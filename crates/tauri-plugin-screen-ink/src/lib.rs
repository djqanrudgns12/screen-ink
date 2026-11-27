//! tauri-plugin-screen-ink — 앱 핵심(InkService)을 Tauri에 붙인다(03 §2·§6).
//!
//! - 명령 `plugin:screen-ink|…` = `InkClient` 메서드(packages/ink-client/src/tauri.js와 1:1)
//! - 이벤트 `screen-ink://state` · `screen-ink://island` · `screen-ink://draw`(판서 세션)
//! - 창: `ink-popover`(트레이 팝오버 S01) · `ink-settings`(S02~S05) · `ink-island`(섬 S17)
//! - 전역 단축키 = tauri-plugin-global-shortcut 어댑터, 트레이 아이콘 = 코드로 그림(tray_art)
//!
//! 스레드 규칙: 서비스는 메인 스레드에서만 만진다(동기 명령·트레이·단축키 처리). 창을 새로 만드는
//! 명령(`run`)만 비동기로 두고(Windows WebView2 교착 회피) 서비스 일은 메인 스레드로 넘긴다.

mod adapters;
mod overlay;
mod pins;
mod rec_frame;
mod tray_art;
mod zoom_hud;
mod update;

use ink_app::{
    HotkeyError, HotkeyOutcome, HotkeyRegistry, ImportPreview, InkService, InkState, IslandEvent, IslandKind, MonitorProbe, Notifier, Ports, PreviewResult, ScreenSource,
    SetShortcutResult, SystemProbe, UpdateResult, WindowCatalog,
};
#[cfg(any(windows, target_os = "macos"))]
use ink_app::SystemEvent;
#[cfg(all(not(windows), not(target_os = "macos")))]
use ink_domain::shortcuts::LiveBinding;
use ink_domain::i18n::Lang;
use ink_domain::shortcuts::{catalog, Platform};
use ink_domain::snapshot::CaptureKind;
use overlay::{FrameStore, TauriOverlayHost};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::image::Image;
use tauri::plugin::{Builder as PluginBuilder, TauriPlugin};
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Rect, RunEvent, Runtime, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tray_art::TrayMode;

// 모든 슉쇽 창이 같은 WebView2 환경을 쓰도록 브라우저 인자를 한 곳에서 정한다.
fn app_window<R: Runtime>(app: &AppHandle<R>, label: impl Into<String>, url: WebviewUrl) -> WebviewWindowBuilder<'_, R, AppHandle<R>> {
    let builder = WebviewWindowBuilder::new(app, label, url);
    #[cfg(windows)]
    let builder = builder.additional_browser_args(concat!(
        // 인자를 직접 주면 Wry 기본값이 교체되므로 메뉴·SmartScreen과 자동 재생 설정을 유지한다.
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection ",
        "--autoplay-policy=no-user-gesture-required ",
        "--disable-background-networking --disable-component-update --disable-domain-reliability --no-pings --disable-sync ",
        // 웹뷰는 로컬 자원만 쓴다. 시스템 프록시 자동 탐색과 외부 이름 조회도 건너뛴다.
        // 앱 업데이트 확인은 웹뷰 밖의 Windows 업데이터가 맡는다.
        "--no-proxy-server --host-resolver-rules=\"MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE *.localhost, EXCLUDE 127.0.0.1, EXCLUDE [::1]\""
    ));
    builder
}

pub const POPOVER: &str = "ink-popover";
pub const SETTINGS: &str = "ink-settings";
pub const ISLAND: &str = "ink-island";
const ISLAND_SIZE: (f64, f64) = (560.0, 88.0);
const TRAY_ID: &str = "screen-ink";
/// 화면에 보이는 앱 이름(docs/brand/README.md). 내부 식별자(플러그인·이벤트·트레이 ID)는 `screen-ink` 그대로.
pub const APP_NAME: &str = "슉쇽";
/// 폴더·파일 이름에 쓰는 영문 이름(사진\Shookshok, %APPDATA%\Shookshok). 화면에 보이는 영문은 `ShookShok`.
pub const APP_DIR: &str = "Shookshok";
/// 이름을 바꾸기 전 설정 폴더 — 새 폴더가 없으면 한 번 옮긴다.
const LEGACY_DIR: &str = "screen-ink";
const EVT_STATE: &str = "screen-ink://state";
const EVT_ISLAND: &str = "screen-ink://island";
const EVT_CAPTURE: &str = "screen-ink://capture";
const EVT_CAPTURE_COMMAND: &str = "screen-ink://capture-command";
const EVT_DRAW: &str = "screen-ink://draw";
/// 다시 시작(화면 기록 권한 반영, M10)
const ACTION_RELAUNCH: &str = "app.relaunch";
/// 권한을 켜러 간 사용자를 기다리는 시간(손쉬운 사용 1초 확인·안내 섬)
#[cfg(target_os = "macos")]
const PERMISSION_WAIT: Duration = Duration::from_secs(120);
const EVT_DRAW_COMMAND: &str = "screen-ink://draw-command";
/// 설정 창에 첫 실행 시트(S06)를 다시 띄우라는 신호(앱 셸이 듣는다)
const EVT_FIRST_RUN: &str = "screen-ink://first-run";
/// 섬 창을 UI가 닫아 주지 않을 때의 안전 시간
const ISLAND_FALLBACK: Duration = Duration::from_secs(8);

/// 관리 상태: 서비스 하나.
pub struct Ink(Mutex<InkService>);

impl Ink {
    fn lock(&self) -> MutexGuard<'_, InkService> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// 팝오버가 포커스를 잃어 막 닫혔는지(트레이 아이콘 클릭이 곧바로 다시 여는 것 막기).
#[derive(Default)]
struct PopoverClock(Mutex<Option<Instant>>);

/// 트레이가 포커스를 받기 직전의 창. 명령 실행 때 프로세스와 속성을 다시 확인한다.
#[cfg(windows)]
#[derive(Default)]
struct TrayPinTarget {
    before_click: Option<(u64, u32)>,
    opened: Option<(u64, u32)>,
}

/// 맥: 팝오버를 붙인 메뉴 막대 아이콘(CG 전역 포인트). 카드 크기가 바뀌면 다시 붙인다.
#[cfg(target_os = "macos")]
#[derive(Default)]
struct PopoverAnchor(Mutex<Option<ink_platform_macos::coords::RectF>>);

/// 맥 팝오버: 카드 둘레 투명 여백(App.svelte `.popover-shell` padding)과 메뉴 막대 아래 틈(pt).
#[cfg(target_os = "macos")]
const POPOVER_PAD: f64 = 8.0;
#[cfg(target_os = "macos")]
const POPOVER_GAP: f64 = 5.0;

#[cfg(target_os = "macos")]
fn anchor_popover<R: Runtime>(app: &AppHandle<R>) {
    let Some(icon) = *app.state::<PopoverAnchor>().0.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    if let Some(ns) = app.get_webview_window(POPOVER).and_then(|w| w.ns_window().ok()) {
        ink_platform_macos::window::place_below(ns, &ink_platform_macos::space(), icon, POPOVER_PAD, POPOVER_GAP);
    }
}

/// 섬 창 세대 — 새 섬이 오면 이전 안전 타이머는 무시.
#[derive(Default)]
struct IslandGen(AtomicU64);

// ───────────────────────── 어댑터 ─────────────────────────

/// 단축키 ID → 칸. 전역 단축키 플러그인 처리기가 읽는다(관리 상태라 플러그인 순서와 무관).
#[derive(Default)]
struct HotkeySlots(Mutex<HashMap<u32, String>>);

struct TauriHotkeys<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> HotkeyRegistry for TauriHotkeys<R> {
    fn register(&mut self, slot: &str, accelerator: &str) -> Result<(), HotkeyError> {
        let sc: Shortcut = accelerator.parse().map_err(|_| HotkeyError::Unsupported)?;
        self.app.global_shortcut().register(sc).map_err(|e| {
            log::info!("hotkey refused: {accelerator} ({e})");
            HotkeyError::Taken
        })?;
        self.app.state::<HotkeySlots>().0.lock().unwrap_or_else(|e| e.into_inner()).insert(sc.id(), slot.to_string());
        Ok(())
    }

    fn unregister(&mut self, accelerator: &str) {
        if let Ok(sc) = accelerator.parse::<Shortcut>() {
            let _ = self.app.global_shortcut().unregister(sc);
            self.app.state::<HotkeySlots>().0.lock().unwrap_or_else(|e| e.into_inner()).remove(&sc.id());
        }
    }
}

struct TauriNotifier<R: Runtime> {
    app: AppHandle<R>,
    /// 마지막으로 그린 (상태, 밝은 작업 표시줄) — 같으면 아이콘을 다시 그리지 않는다
    tray_mode: Mutex<Option<(TrayMode, bool)>>,
    /// 마지막으로 설정한 트레이 툴팁 — 바뀌거나 언어가 바뀔 때만 갱신
    tray_tooltip: Mutex<Option<String>>,
    /// 마지막으로 창 제목에 반영한 언어 — 언어가 바뀌면 열린 창 제목도 갱신
    last_lang: Mutex<Option<Lang>>,
}

impl<R: Runtime> Notifier for TauriNotifier<R> {
    fn state(&self, state: &InkState) {
        let _ = self.app.emit(EVT_STATE, state);
        // 녹화 범위 꺾쇠 창(단계 6.4) — 녹화 중이면 그 모니터를 덮고, 아니면 숨김
        rec_frame::sync(&self.app, &state.recording);
        let mode = if state.recording.active {
            TrayMode::Recording
        } else if state.hotkeys_enabled {
            TrayMode::Ready
        } else {
            TrayMode::Off
        };
        let key = (mode, light_taskbar());
        let icon_changed = {
            let mut last = self.tray_mode.lock().unwrap_or_else(|e| e.into_inner());
            if *last != Some(key) {
                *last = Some(key);
                true
            } else {
                false
            }
        };
        if icon_changed {
            set_tray_icon(&self.app, mode);
        }
        let tip = tooltip(state);
        let tip_changed = {
            let mut last = self.tray_tooltip.lock().unwrap_or_else(|e| e.into_inner());
            if last.as_deref() != Some(tip.as_str()) {
                *last = Some(tip.clone());
                true
            } else {
                false
            }
        };
        if tip_changed {
            if let Some(tray) = self.app.tray_by_id(TRAY_ID) {
                let _ = tray.set_tooltip(Some(tip));
            }
        }
        let lang = state_lang(state);
        let lang_changed = {
            let mut last = self.last_lang.lock().unwrap_or_else(|e| e.into_inner());
            if *last != Some(lang) {
                *last = Some(lang);
                true
            } else {
                false
            }
        };
        if lang_changed {
            refresh_window_titles(&self.app, lang);
        }
    }

    fn island(&self, event: &IslandEvent) {
        log::debug!("island {:?}", event.kind);
        // 모드 끝은 거두기만 — 창을 새로 보이지 않는다(UI가 섬을 치우면 창도 숨김)
        if event.kind != IslandKind::End {
            show_island(&self.app);
        }
        if event.secondary_button.as_ref().is_some_and(|b| b.action == ACTION_TO_DRAW) {
            release_capture_frame_later(&self.app);
        }
        if event.button.as_ref().is_some_and(|b| b.action == ink_app::capture::ACTION_CAPTURE_RESTORE) {
            release_discarded_capture_later(&self.app);
        }
        let _ = self.app.emit(EVT_ISLAND, event);
    }

    fn capture(&self, view: &ink_app::CaptureView) {
        let _ = self.app.emit(EVT_CAPTURE, view);
    }

    fn capture_command(&self, session: u64, monitor_id: u32, command: &str) {
        let _ = self.app.emit(EVT_CAPTURE_COMMAND, serde_json::json!({ "session": session, "monitorId": monitor_id, "command": command }));
    }

    fn draw(&self, session: &ink_app::DrawSession) {
        let _ = self.app.emit(EVT_DRAW, session);
    }

    fn draw_command(&self, session: u64, command: &str) {
        let _ = self.app.emit(EVT_DRAW_COMMAND, serde_json::json!({ "session": session, "command": command }));
    }

    fn zoom(&self, state: Option<&ink_app::LiveZoomState>) {
        zoom_hud::emit(&self.app, state);
    }

    fn record_frame(&self, rect: Option<ink_domain::geometry::PhysRect>) {
        rec_frame::set_rect(&self.app, rect);
    }
}

/// A 근거 어댑터: Windows = UIA 색인 일꾼, 맥 = AX 색인 일꾼(끝나면 메인 스레드에서 서비스에 알림), 그 밖 = 없음.
fn element_source<R: Runtime>(app: &AppHandle<R>) -> Box<dyn ink_app::ElementSource> {
    #[cfg(windows)]
    {
        let handle = app.clone();
        Box::new(ink_platform_windows::uia::UiaIndex::spawn(move |window, exe, took| {
            // 끝난 순간(일꾼 스레드) — 메인 스레드로 넘어오는 지연과 나눠 잰다
            let at = std::time::Instant::now();
            let h = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                if let Some(ink) = h.try_state::<Ink>() {
                    ink.lock().on_elements_ready_at(window, &exe, took, at);
                }
            });
        }))
    }
    // 맥: AX 색인 일꾼(M9) — 끝나면 같은 길로 서비스에 알린다
    #[cfg(target_os = "macos")]
    {
        let handle = app.clone();
        Box::new(ink_platform_macos::ax::AxIndex::spawn(move |window, exe, took| {
            let at = std::time::Instant::now();
            let h = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                if let Some(ink) = h.try_state::<Ink>() {
                    ink.lock().on_elements_ready_at(window, &exe, took, at);
                }
            });
        }))
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = app;
        Box::new(NoElements)
    }
}

/// V 근거: 전처리가 끝날 때마다 메인 스레드에서 강조 사슬을 다시 만든다.
fn vision_source<R: Runtime>(app: &AppHandle<R>) -> Box<dyn ink_app::VisionSource> {
    let handle = app.clone();
    Box::new(ink_app::VisionEngine::new(move || {
        let at = std::time::Instant::now();
        let h = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            if let Some(ink) = h.try_state::<Ink>() {
                ink.lock().on_vision_ready_at(at);
            }
        });
    }))
}

#[cfg(all(not(windows), not(target_os = "macos")))]
struct NoElements;

#[cfg(all(not(windows), not(target_os = "macos")))]
impl ink_app::ElementSource for NoElements {
    fn request(&self, _window: u64, _exe: &str) {}
    fn elements_at(&self, _window: u64, _p: ink_domain::geometry::PhysPoint) -> Vec<ink_domain::recog::ElementCand> {
        Vec::new()
    }
    fn clear(&self) {}
}

/// 화면 어댑터가 없는 곳(Windows·맥 밖) — 캡처는 실패로 알린다.
#[cfg(all(not(windows), not(target_os = "macos")))]
struct NullScreen;

#[cfg(all(not(windows), not(target_os = "macos")))]
impl ScreenSource for NullScreen {
    fn capture(&mut self, _layout: &ink_domain::geometry::MonitorLayout) -> Result<Vec<ink_app::Frame>, String> {
        Err("screen capture not implemented on this platform yet".into())
    }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
impl WindowCatalog for NullScreen {
    fn windows(&self) -> Vec<ink_domain::snapshot::RawWindow> {
        Vec::new()
    }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
impl MonitorProbe for NullScreen {
    fn layout(&self) -> ink_domain::geometry::MonitorLayout {
        ink_domain::geometry::MonitorLayout::default()
    }

    fn cursor(&self) -> ink_domain::geometry::PhysPoint {
        ink_domain::geometry::PhysPoint::default()
    }
}

fn screen_ports() -> (Box<dyn ScreenSource>, Box<dyn WindowCatalog>, Box<dyn MonitorProbe>) {
    #[cfg(windows)]
    {
        use ink_platform_windows::screen::WindowsScreen;
        // 정지 캡처는 GDI 그대로 + F2 창 캡처는 WGC(단계 6.1)
        (Box::new(ink_platform_windows::wgc_window::WgcScreen::new()), Box::new(WindowsScreen), Box::new(WindowsScreen))
    }
    #[cfg(target_os = "macos")]
    {
        use ink_platform_macos::MacScreen;
        (Box::new(MacScreen), Box::new(MacScreen), Box::new(MacScreen))
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        (Box::new(NullScreen), Box::new(NullScreen), Box::new(NullScreen))
    }
}

/// 섬·팝오버는 캡처·녹화에 안 찍힌다(R-UI-5, S11).
fn exclude_from_capture<R: Runtime>(app: &AppHandle<R>, label: &str) {
    #[cfg(windows)]
    if let Some(w) = app.get_webview_window(label) {
        if let Ok(h) = w.hwnd() {
            if !ink_platform_windows::events::exclude_from_capture(h.0 as isize, true) {
                log::info!("exclude from capture not available for {label}");
            }
        }
    }
    #[cfg(target_os = "macos")]
    if let Some(w) = app.get_webview_window(label) {
        if !w.ns_window().is_ok_and(ink_platform_macos::window::exclude_from_capture) {
            log::info!("exclude from capture not available for {label}");
        }
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let _ = (app, label);
}

/// 어댑터가 없는 곳(Windows·맥 밖) — 아무 앱도 없다고 본다.
#[cfg(all(not(windows), not(target_os = "macos")))]
struct NullProbe;

#[cfg(all(not(windows), not(target_os = "macos")))]
impl SystemProbe for NullProbe {
    fn running_apps(&self) -> Vec<String> {
        Vec::new()
    }

    fn live_bindings(&self) -> Option<Vec<LiveBinding>> {
        Some(Vec::new())
    }
}

fn probe() -> Box<dyn SystemProbe> {
    #[cfg(windows)]
    {
        Box::new(ink_platform_windows::WindowsProbe::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(ink_platform_macos::MacProbe)
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Box::new(NullProbe)
    }
}

fn light_taskbar() -> bool {
    #[cfg(windows)]
    {
        ink_platform_windows::taskbar_is_light()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

// ───────────────────────── 트레이 · 창 ─────────────────────────

fn tray_image(mode: TrayMode) -> Image<'static> {
    let (rgba, w, h) = tray_art::render(mode, light_taskbar());
    Image::new_owned(rgba, w, h)
}

fn tooltip_for(lang: Lang, hotkeys_enabled: bool, conflicts: usize, pinned_count: u32) -> String {
    let mut t = tr_for(lang, "common.appTitle");
    if !hotkeys_enabled {
        t.push_str(" · ");
        t.push_str(&tr_for(lang, "plugin.tray.paused"));
    } else if conflicts == 1 {
        t.push_str(" · ");
        t.push_str(&tr_for(lang, "plugin.tray.conflicts.one"));
    } else if conflicts > 1 {
        t.push_str(" · ");
        t.push_str(&trf_for(lang, "plugin.tray.conflicts.other", &[("n", &conflicts)]));
    }
    if pinned_count == 1 {
        t.push_str(" · ");
        t.push_str(&tr_for(lang, "plugin.tray.topmost.one"));
    } else if pinned_count > 1 {
        t.push_str(" · ");
        t.push_str(&trf_for(lang, "plugin.tray.topmost.other", &[("n", &pinned_count)]));
    }
    t
}

fn tooltip(state: &InkState) -> String {
    tooltip_for(state_lang(state), state.hotkeys_enabled, state.conflicts.count, state.pinned_count)
}

fn set_tray_icon<R: Runtime>(app: &AppHandle<R>, mode: TrayMode) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(tray_image(mode)));
    }
}

fn refresh_window_titles<R: Runtime>(app: &AppHandle<R>, lang: Lang) {
    for (label, key) in [
        (POPOVER, "plugin.window.popover"),
        (ISLAND, "plugin.window.island"),
        (SETTINGS, "plugin.window.settings"),
        (rec_frame::REC_FRAME, "plugin.window.recFrame"),
        (zoom_hud::LABEL, "plugin.window.zoomHud"),
    ] {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.set_title(&tr_for(lang, key));
        }
    }
    for (label, w) in app.webview_windows() {
        if overlay::is_overlay_label(&label) {
            let _ = w.set_title(&tr_for(lang, "plugin.window.overlay"));
        } else if label.starts_with(pins::PREFIX) {
            let _ = w.set_title(&tr_for(lang, "plugin.window.pin"));
        }
    }
}

fn show_island<R: Runtime>(app: &AppHandle<R>) {
    let gen = app.state::<IslandGen>().0.fetch_add(1, Ordering::SeqCst) + 1;
    #[cfg(windows)]
    {
        let cursor = ink_platform_windows::screen::cursor_pos();
        // SetWindowPos는 WebView2 창 절차에 재진입한다. 서비스 잠금을 놓은 다음 창을 옮긴다.
        overlay::defer(app, move |app| {
            if app.state::<IslandGen>().0.load(Ordering::SeqCst) != gen {
                return;
            }
            if let Some(w) = app.get_webview_window(ISLAND) {
                if let Ok(h) = w.hwnd() {
                    ink_platform_windows::screen::show_island(h.0 as isize, cursor, ISLAND_SIZE);
                }
                let _ = w.show();
            }
        });
    }
    #[cfg(not(windows))]
    if let Some(w) = app.get_webview_window(ISLAND) {
        place_island(app, &w);
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(ISLAND_FALLBACK);
        if app.state::<IslandGen>().0.load(Ordering::SeqCst) == gen {
            if let Some(w) = app.get_webview_window(ISLAND) {
                let _ = w.hide();
            }
        }
    });
}

#[cfg(not(windows))]
fn place_island<R: Runtime>(app: &AppHandle<R>, w: &tauri::WebviewWindow<R>) {
    // 커서가 있는 모니터의 위 가운데(02 §7: 단축키 순간 커서 모니터가 대상)
    // 맥: 혼합 배율에서 Tauri 물리 좌표는 옆 화면에 걸쳐 놓인다 → 포인트로 직접 놓는다
    #[cfg(target_os = "macos")]
    let placed = w.ns_window().is_ok_and(|ns| ink_platform_macos::window::place_top_center(ns, &ink_platform_macos::space(), ink_platform_macos::display::cursor_pt(), 12.0));
    #[cfg(not(target_os = "macos"))]
    let placed = false;
    let monitor = app.cursor_position().ok().and_then(|p| w.monitor_from_point(p.x, p.y).ok().flatten()).or_else(|| w.primary_monitor().ok().flatten());
    if let (false, Some(m), Ok(size)) = (placed, monitor, w.outer_size()) {
        let wa = m.work_area();
        let x = wa.position.x + (wa.size.width as i32 - size.width as i32) / 2;
        let y = wa.position.y + (12.0 * m.scale_factor()) as i32;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
    // 맥: 지금 Space(전체 화면 앱 포함)에 바로 올린다(Tauri show는 나중으로 미룬다)
    #[cfg(target_os = "macos")]
    if let Ok(ns) = w.ns_window() {
        ink_platform_macos::window::order_front(ns);
    }
    let _ = w.show();
}

fn build_static_windows<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let popover = app_window(app, POPOVER, WebviewUrl::App("index.html?view=popover".into()))
        .title(tr(app, "plugin.window.popover"))
        // 카드 252 × 약 418~448 + 둘레 투명 여백 8씩(App.svelte `.popover-shell`) — 창 = 카드 + 그림자만(R2-UI-TRAY-1)
        .inner_size(268.0, 464.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
    let handle = app.clone();
    popover.on_window_event(move |e| {
        // 맥: 카드에 맞춰 창 크기가 바뀌면 왼쪽 아래가 고정돼 윗변이 움직인다 → 아이콘 아래에 다시 붙인다
        #[cfg(target_os = "macos")]
        if let WindowEvent::Resized(_) = e {
            anchor_popover(&handle);
        }
        if let WindowEvent::Focused(false) = e {
            if let Some(w) = handle.get_webview_window(POPOVER) {
                let _ = w.hide();
            }
            *handle.state::<PopoverClock>().0.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        }
    });

    app_window(app, ISLAND, WebviewUrl::App("index.html?view=island".into()))
        .title(tr(app, "plugin.window.island"))
        .inner_size(ISLAND_SIZE.0, ISLAND_SIZE.1)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focusable(false)
        .focused(false)
        .visible(false)
        .build()?;
    rec_frame::build(app)?;
    Ok(())
}

/// 트레이 팝오버 열기·닫기. 위치: Windows = 아이콘 위(작업 표시줄이 위면 아래), 맥 = 메뉴 막대 아이콘 바로 아래.
fn toggle_popover<R: Runtime>(app: &AppHandle<R>, tray_rect: Rect) {
    #[cfg(windows)]
    let clicked = std::time::Instant::now();
    let Some(w) = app.get_webview_window(POPOVER) else { return };
    let visible = w.is_visible().unwrap_or(false);
    // 맥: 다른 Space(전체 화면 앱 등)에서 "보임"으로 남은 팝오버는 숨기지 않고 지금 Space로 데려온다(R2-BUG-TRAY-3)
    #[cfg(target_os = "macos")]
    let on_space = w.ns_window().is_ok_and(ink_platform_macos::window::on_active_space);
    #[cfg(not(target_os = "macos"))]
    let on_space = true;
    if visible && on_space {
        log::info!("tray click: hide (visible on this space)");
        let _ = w.hide();
        return;
    }
    let just_closed = app.state::<PopoverClock>().0.lock().unwrap_or_else(|e| e.into_inner()).is_some_and(|t| t.elapsed() < Duration::from_millis(250));
    if just_closed {
        log::info!("tray click: ignored (closed {} ms ago)", app.state::<PopoverClock>().0.lock().unwrap_or_else(|e| e.into_inner()).map_or(0, |t| t.elapsed().as_millis()));
        return;
    }
    // 맥: tray-icon 사각형은 "포인트 × 그 화면 배율" → 포인트로 돌려 아이콘 바로 아래에 붙인다(혼합 배율에서도 정확)
    #[cfg(target_os = "macos")]
    {
        let scale = w.scale_factor().unwrap_or(1.0);
        let p = tray_rect.position.to_physical::<f64>(scale);
        let z = tray_rect.size.to_physical::<f64>(scale);
        // 클릭 순간 커서(포인트)로 어느 화면 배율인지 가린다(혼합 배율에서 숫자만으로는 모호)
        let icon = ink_platform_macos::space().unscale_rect(p.x, p.y, z.width, z.height, Some(ink_platform_macos::display::cursor_pt()));
        *app.state::<PopoverAnchor>().0.lock().unwrap_or_else(|e| e.into_inner()) = Some(icon);
        anchor_popover(app);
    }
    #[cfg(not(target_os = "macos"))]
    place_popover_near(&w, tray_rect);
    #[cfg(windows)]
    {
        let target = app.state::<Ink>().lock().foreground_for_tray();
        let state = app.state::<Mutex<TrayPinTarget>>();
        let mut saved = state.lock().unwrap_or_else(|e| e.into_inner());
        saved.opened = target.or(saved.before_click.take());
    }
    {
        let ink = app.state::<Ink>();
        let mut s = ink.lock();
        s.refresh_environment();
        let st = s.state();
        let _ = app.emit(EVT_STATE, &st);
    }
    // 맥: 활성화하지 않는 패널을 지금 Space(전체 화면 앱 포함)에 올리고 키 창으로만 — 앱을 활성화하면 Space가 넘어간다.
    // Tauri `show()`는 부르지 않는다(층·순서를 다시 써 전체 화면 창 뒤로 갈 수 있다 — R2-BUG-TRAY-4). 창 보임은 NSWindow가 판단한다.
    // 클릭마다 판단 근거를 한 줄 남긴다(R2-BUG-TRAY-1 — 숫자·참거짓만): 재발하면 이 줄로 가설을 가린다.
    #[cfg(target_os = "macos")]
    if let Ok(ns) = w.ns_window() {
        use ink_platform_macos::window as mw;
        let active_before = mw::app_active();
        mw::order_front(ns);
        mw::make_key(ns);
        log::info!(
            "tray click: show (was visible {visible}, on space {on_space}, app active {active_before}->{}, key {}, on space now {}, level {})",
            mw::app_active(),
            mw::is_key(ns),
            mw::on_active_space(ns),
            mw::level(ns)
        );
        return;
    }
    let shown = w.show();
    #[cfg(windows)]
    if shown.is_ok() {
        log::info!("트레이 팝오버 표시 [{:.1} ms]", clicked.elapsed().as_secs_f64() * 1000.0);
    }
    let _ = shown;
    let _ = w.set_focus();
}

/// 트레이 아이콘 위(작업 표시줄이 위면 아래)에, 모니터 작업 영역 안으로 가둔다(Windows).
#[cfg(not(target_os = "macos"))]
fn place_popover_near<R: Runtime>(w: &tauri::WebviewWindow<R>, tray_rect: Rect) {
    let scale = w.scale_factor().unwrap_or(1.0);
    let pos = tray_rect.position.to_physical::<i32>(scale);
    let size = tray_rect.size.to_physical::<i32>(scale);
    if let Ok(win) = w.outer_size() {
        let (ww, wh) = (win.width as i32, win.height as i32);
        let gap = (6.0 * scale) as i32;
        let mut x = pos.x + size.width / 2 - ww / 2;
        let mut y = pos.y - wh - gap;
        if let Ok(Some(m)) = w.monitor_from_point(pos.x as f64, pos.y as f64) {
            let wa = m.work_area();
            let (left, top) = (wa.position.x, wa.position.y);
            let (right, bottom) = (left + wa.size.width as i32, top + wa.size.height as i32);
            if y < top {
                y = pos.y + size.height + gap; // 작업 표시줄이 위
            }
            x = x.clamp(left + gap, (right - ww - gap).max(left));
            y = y.clamp(top, (bottom - wh).max(top));
        }
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
}

/// 설정 창(S02~S05). 이미 있으면 앞으로(05 §11-1 단일 창).
pub fn open_settings<R: Runtime>(app: &AppHandle<R>) {
    open_settings_with(app, false);
}

/// `first_run`이면 첫 실행 시트(S06)를 위에 띄운다.
fn open_settings_with<R: Runtime>(app: &AppHandle<R>, first_run: bool) {
    if let Some(p) = app.get_webview_window(POPOVER) {
        let _ = p.hide();
    }
    if let Some(w) = app.get_webview_window(SETTINGS) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let url = if first_run { "index.html?view=settings&first=1" } else { "index.html?view=settings" };
    let built = app_window(app, SETTINGS, WebviewUrl::App(url.into()))
        .title(tr(app, "plugin.window.settings"))
        .inner_size(900.0, 600.0)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        .center()
        .build();
    match built {
        Ok(w) => {
            // 녹화 중 설정 창이 뒤로 가거나 닫히면 전역 키를 바로 되살린다(R2-KEY-REC-2).
            // 서비스가 잠금을 쥔 채 창을 닫는 길이 있어(settings.close) 여기서 바로 잠그지 않고 메인 대기열로 미룬다.
            let h = app.clone();
            w.on_window_event(move |e| {
                if matches!(e, WindowEvent::Focused(false) | WindowEvent::Destroyed | WindowEvent::CloseRequested { .. }) {
                    let h2 = h.clone();
                    let _ = h.run_on_main_thread(move || {
                        if let Some(ink) = h2.try_state::<Ink>() {
                            ink.lock().end_shortcut_capture();
                        }
                    });
                }
            });
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || handle.state::<Ink>().lock().begin_settings_session());
        }
        Err(e) => log::error!("settings window: {e}"),
    }
}

fn output_folder<R: Runtime>(app: &AppHandle<R>) -> Option<std::path::PathBuf> {
    let configured = app.state::<Ink>().lock().settings().str("output.folder").to_string();
    if !configured.is_empty() {
        return Some(configured.into());
    }
    app.path().picture_dir().ok().map(|p| p.join(APP_DIR))
}

/// 파일이면 그 파일을 고른 채로 폴더를 연다(맥 Finder `open -R`, Windows 탐색기 `/select,`). 폴더면 그 폴더.
fn reveal_path(path: &std::path::Path) {
    if !path.is_file() {
        open_folder(path);
        return;
    }
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg("-R").arg(path).spawn();
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(path.parent().unwrap_or(path)).spawn();
    if let Err(e) = r {
        log::warn!("reveal failed: {e}");
    }
}

/// 파일을 그 OS의 기본 앱으로 연다.
fn open_file(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(path).spawn();
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(path).spawn();
    if let Err(e) = r {
        log::warn!("open file failed: {e}");
    }
}

fn open_folder(path: &std::path::Path) {
    let _ = std::fs::create_dir_all(path);
    #[cfg(windows)]
    let cmd = "explorer";
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let cmd = "xdg-open";
    if let Err(e) = std::process::Command::new(cmd).arg(path).spawn() {
        log::warn!("open folder failed: {e}");
    }
}

/// 결과 섬 [판서로](3e-handoff) 동작 이름 — `InkClient.run('capture.toDraw')`.
const ACTION_TO_DRAW: &str = "capture.toDraw";

/// [판서로]가 쥔 정지 프레임(모니터 한 장, 4K ≈ 33 MB)을 30초 뒤에 놓는다(B §5, M3 대기 메모리).
/// 서비스 잠금을 쥔 채 불린다 — 여기서는 스레드만 띄우고, 놓기는 그 뒤 메인 스레드에서(새 결과면 서비스가 나이로 거른다).
fn release_capture_frame_later<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(ink_app::capture::CAPTURE_FRAME_TTL + Duration::from_millis(200));
        let h = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(ink) = h.try_state::<Ink>() {
                if ink.lock().expire_capture_frame() {
                    log::debug!("capture frame released (to-draw idle)");
                }
            }
        });
    });
}

/// 주석을 버린 뒤 띄운 5초 [되살리기] 대기가 끝나면 정지 프레임을 놓는다(R2-UNDO-1).
fn release_discarded_capture_later<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(ink_app::capture::CAPTURE_UNDO_TTL + Duration::from_millis(100));
        let h = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(ink) = h.try_state::<Ink>() {
                if ink.lock().expire_discarded_capture() {
                    log::debug!("discarded capture frames released (undo ttl expired)");
                }
            }
        });
    });
}

/// 아직 만들지 않은 모드(단계 2 이후) — 눌린 것은 알려 준다.
fn not_ready<R: Runtime>(app: &AppHandle<R>, slot_or_action: &str) {
    let ink = app.state::<Ink>();
    let s = ink.lock();
    let lang = s.lang();
    let label = catalog().slot_label(slot_or_action, lang).unwrap_or_else(|| slot_or_action.to_string());
    s.island(IslandEvent::new(IslandKind::Info, tr_for(lang, "plugin.notReady")).detail(label));
}

fn on_hotkey<R: Runtime>(app: &AppHandle<R>, slot: &str, pressed: Instant) {
    log::debug!("hotkey {slot}");
    let outcome = app.state::<Ink>().lock().on_hotkey_at(slot, pressed);
    if let HotkeyOutcome::Start(slot) = outcome {
        not_ready(app, &slot);
    }
}

/// 트레이·명령에서 캡처 시작(단축키가 꺼져 있어도).
fn start_capture<R: Runtime>(app: &AppHandle<R>, kind: CaptureKind) {
    if let Some(p) = app.get_webview_window(POPOVER) {
        let _ = p.hide();
    }
    app.state::<Ink>().lock().start_capture(kind);
}

/// 트레이·명령에서 판서·칠판·직전 판서 열기(단축키가 꺼져 있어도).
fn start_draw<R: Runtime>(app: &AppHandle<R>, kind: ink_app::DrawKind) {
    if let Some(p) = app.get_webview_window(POPOVER) {
        let _ = p.hide();
    }
    app.state::<Ink>().lock().start_draw(kind);
}

/// `draw_export` 머리글 → 메타(머리글 이름은 소문자, 값은 글자).
fn export_meta(headers: &tauri::http::HeaderMap) -> Result<ink_app::DrawExportMeta, String> {
    ink_app::DrawExportMeta::parse(|name| headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_string))
}

#[cfg(not(debug_assertions))]
fn sync_autostart<R: Runtime>(app: &AppHandle<R>, on: bool) {
    let channel = app.state::<Ink>().lock().install_channel();
    match ink_domain::package::autostart_backend(channel) {
        ink_domain::package::AutostartBackend::StartupTask => {
            let (lang, res) = {
                let ink = app.state::<Ink>();
                let mut s = ink.lock();
                (s.lang(), s.sync_autostart_via_port(on))
            };
            match res {
                Ok(notice) => {
                    if let Some(msg) = notice.guidance(lang) {
                        log::info!("startup task notice: {msg}");
                    }
                }
                Err(e) => log::warn!("startup task sync failed: {e}"),
            }
        }
        ink_domain::package::AutostartBackend::RegistryRun => {
            app.state::<Ink>().lock().set_autostart_notice(None);
            use tauri_plugin_autostart::ManagerExt;
            let m = app.autolaunch();
            let now = m.is_enabled().unwrap_or(false);
            let r = match (on, now) {
                (true, false) => m.enable(),
                (false, true) => m.disable(),
                _ => Ok(()),
            };
            if let Err(e) = r {
                log::warn!("autostart sync failed: {e}");
            }
        }
    }
}

/// 개발 빌드는 이 PC 로그인 항목을 건드리지 않는다.
#[cfg(debug_assertions)]
fn sync_autostart<R: Runtime>(_app: &AppHandle<R>, on: bool) {
    log::debug!("autostart sync skipped in debug build (want {on})");
}

// ───────────────────────── 명령 ─────────────────────────

#[tauri::command]
fn get_state(ink: State<'_, Ink>) -> InkState {
    ink.lock().state()
}

#[tauri::command]
fn update_setting<R: Runtime>(app: AppHandle<R>, ink: State<'_, Ink>, path: String, value: Value) -> UpdateResult {
    let mut r = ink.lock().update_setting(&path, &value);
    if r.ok && path == "general.autostart" {
        sync_autostart(&app, value.as_bool().unwrap_or(false));
        r.state = ink.lock().state();
    }
    r
}

#[tauri::command]
fn preview_shortcut(ink: State<'_, Ink>, id: String, combo: String) -> PreviewResult {
    ink.lock().preview_shortcut(&id, &combo)
}

#[tauri::command]
fn set_shortcut(ink: State<'_, Ink>, id: String, combo: String) -> SetShortcutResult {
    ink.lock().set_shortcut(&id, &combo)
}

#[tauri::command]
fn reset_shortcut_group(ink: State<'_, Ink>, group: String) -> InkState {
    ink.lock().reset_shortcut_group(&group)
}

#[tauri::command]
fn undo(ink: State<'_, Ink>) -> UpdateResult {
    ink.lock().undo()
}

#[tauri::command]
fn set_hotkeys_enabled(ink: State<'_, Ink>, enabled: bool) -> InkState {
    ink.lock().set_hotkeys_enabled(enabled)
}

/// 단축키 녹화가 이 시간보다 길면 전역 키를 되살린다(R2-KEY-REC-2 — 웹뷰가 끝을 못 알려도 갇히지 않게).
const SHORTCUT_CAPTURE_LIMIT: Duration = Duration::from_secs(20);
/// 녹화 세대 — 새 녹화가 시작되면 지난 시간 제한은 무시된다.
static SHORTCUT_CAPTURE_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 설정 › 단축키 녹화 칸이 키를 받는 동안 전역 키를 조용히 푼다(true) / 다시 등록한다(false).
#[tauri::command]
fn shortcut_capture<R: Runtime>(app: AppHandle<R>, ink: State<'_, Ink>, active: bool) {
    use std::sync::atomic::Ordering;
    let gen = SHORTCUT_CAPTURE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    if !active {
        ink.lock().end_shortcut_capture();
        return;
    }
    ink.lock().begin_shortcut_capture();
    std::thread::spawn(move || {
        std::thread::sleep(SHORTCUT_CAPTURE_LIMIT);
        if SHORTCUT_CAPTURE_GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        let h = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(ink) = h.try_state::<Ink>() {
                let mut ink = ink.lock();
                if ink.capturing_shortcut() {
                    log::info!("shortcut capture timed out — hotkeys restored");
                    ink.end_shortcut_capture();
                }
            }
        });
    });
}

#[tauri::command]
fn reset_all(ink: State<'_, Ink>) -> InkState {
    ink.lock().reset_all()
}

/// 오버레이가 정지 이미지를 그렸다 → 진입 계측(M1) 한 줄.
#[tauri::command]
fn overlay_ready(ink: State<'_, Ink>, session: u64, load_ms: Option<f64>, paint_ms: Option<f64>) {
    ink.lock().overlay_ready_with(session, load_ms, paint_ms);
}

/// 오버레이 입력 한 건(포인터·휠·키). 판단은 서비스의 고르기 상태 기계가 한다.
#[tauri::command]
fn capture_input<R: Runtime>(window: tauri::WebviewWindow<R>, ink: State<'_, Ink>, input: ink_app::CaptureInput) {
    // 맥: 모니터를 넘어 끌면 누른 모니터의 웹뷰가 창 밖 점을 자기 배율로 환산해 보낸다 → 배율이 다른 옆 모니터에서 어긋남.
    // 자기 모니터 밖 점은 실제 커서(좌표 변환 함수 하나)로 바꾼다.
    #[cfg(target_os = "macos")]
    let input = {
        let space = ink_platform_macos::space();
        match overlay::monitor_of(window.label()).and_then(|id| space.screens.iter().find(|s| s.id == id)) {
            Some(s) => overlay::pointer_inside(input, s.phys, || ink_platform_macos::display::cursor(&space)),
            None => input,
        }
    };
    #[cfg(not(target_os = "macos"))]
    let _ = &window;
    ink.lock().capture_input(input);
}

/// 캡처 주석(4.9): 오버레이 창(모니터)이 자기 주석 개수와 되살리기용 주석 데이터를 알린다.
#[tauri::command]
fn capture_annotations(ink: State<'_, Ink>, session: u64, monitor_id: u32, items: u32, data: Option<Value>) {
    ink.lock().capture_annotations_with_data(session, monitor_id, items, data);
}

/// 캡처 주석(4.9): `screen-ink://capture-commit` 부탁에 대한 답 — 원본 바이트 본문
/// `[머리 길이 u32 LE][머리 JSON][주석 층 RGBA]`(base64 없음, B §2.1). 합성·복사·저장·핀은 서비스가.
#[tauri::command]
fn capture_commit_annotated(ink: State<'_, Ink>, request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let tauri::ipc::InvokeBody::Raw(body) = request.body() else {
        return Err("annotation layer must be sent as raw bytes".into());
    };
    let commit = ink_app::capture::AnnotatedCommit::decode(body)?;
    ink.lock().capture_commit_annotated(commit);
    Ok(())
}

/// 글 상자가 초점을 받았다(R2-TXT-0 맥 한글 조합): 그 창의 웹뷰 입력 문맥을 지금 입력 문맥으로 바로잡는다
/// (`ink_platform_macos::window::ensure_text_input`). 바로잡았을 때만 기록 한 줄, 개발 실행이면 뒤 상태도 한 줄.
/// 창 제목·입력한 글자는 담지 않는다. Windows는 할 일 없음(TSF가 창 초점을 따른다).
#[tauri::command]
fn text_focus<R: Runtime>(app: AppHandle<R>, window: tauri::WebviewWindow<R>, place: String) {
    #[cfg(target_os = "macos")]
    {
        let from = window.label().to_string();
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let Some(window) = handle.get_webview_window(&from) else { return };
            if !window.is_visible().unwrap_or(false) {
                return;
            }
            let Ok(ptr) = window.ns_window() else { return };
            if let Some(before) = ink_platform_macos::window::ensure_text_input(ptr) {
                log::info!("text input context fixed [{place}] on {from}: {before}");
            }
            if cfg!(debug_assertions) {
                let (_, info) = ink_platform_macos::window::input_state();
                log::info!("input state [{place}] on {from}: {info}");
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, window, place);
}

/// 이 컴퓨터 글꼴 집 이름(글자 도구 글꼴 고르기 R2-TXT-4) — 이름만, 화면에만(로그·파일에 쓰지 않는다).
/// 맥 = `NSFontManager.availableFontFamilies`, Windows = 레지스트리 글꼴 목록에서 집 이름만. 숨은 글꼴(`.`으로 시작)은 뺀다.
#[tauri::command]
async fn system_fonts<R: Runtime>(app: AppHandle<R>) -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = app.run_on_main_thread(move || {
            let _ = tx.send(ink_platform_macos::fonts::families());
        });
        let mut v = rx.recv_timeout(Duration::from_secs(3)).unwrap_or_default();
        tidy_font_names(&mut v);
        v
    }
    #[cfg(windows)]
    {
        let _ = app;
        let mut v = windows_font_families();
        tidy_font_names(&mut v);
        v
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = app;
        Vec::new()
    }
}

/// 정렬·중복 제거·숨은 글꼴 빼기.
fn tidy_font_names(v: &mut Vec<String>) {
    v.retain(|n| !n.is_empty() && !n.starts_with('.') && !n.starts_with('#'));
    v.sort_by_key(|n| n.to_lowercase());
    v.dedup();
}

#[derive(serde::Serialize)]
struct AudioInput {
    id: String,
    label: String,
}

/// 이 컴퓨터 마이크 장치 목록(설정 › 녹화). 이름만 화면에 쓰고 로그·기록에 남기지 않는다.
#[tauri::command]
fn audio_inputs(ink: State<'_, Ink>) -> Vec<AudioInput> {
    ink.lock().audio_inputs().into_iter().map(|(id, label)| AudioInput { id, label }).collect()
}

/// `HKLM\…\Fonts` 값 이름("Malgun Gothic Bold (TrueType)", "Segoe UI & Segoe UI Semilight (TrueType)")에서 집 이름만.
#[cfg(any(windows, test))]
fn windows_family_names(value_name: &str) -> Vec<String> {
    const STYLES: [&str; 12] = [" Bold Italic", " Bold", " Italic", " Semibold", " SemiBold", " Semilight", " Light", " Black", " ExtraLight", " Regular", " Medium", " Oblique"];
    let mut out = Vec::new();
    let name = value_name.split(" (").next().unwrap_or(value_name);
    for part in name.split(" & ") {
        let mut family = part.trim();
        for style in STYLES {
            if let Some(base) = family.strip_suffix(style) {
                family = base;
            }
        }
        if !family.is_empty() {
            out.push(family.to_owned());
        }
    }
    out
}

#[cfg(windows)]
fn windows_font_families() -> Vec<String> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_SZ};
    use winreg::RegKey;
    let mut all = Vec::new();
    for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let Ok(key) = RegKey::predef(root).open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts") else { continue };
        for (name, value) in key.enum_values().flatten() {
            if value.vtype == REG_SZ {
                all.extend(windows_family_names(&name));
            }
        }
    }
    all
}

/// 마지막으로 저장한 PDF(저장 섬 [열기]가 연다).
static LAST_PDF: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);
const ACTION_PDF_OPEN: &str = "pdf.open";
/// 마지막으로 저장한 문제 보고용 기록 zip(저장 섬 [폴더 열기]가 선택해 연다).
static LAST_DIAG_ZIP: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);
const ACTION_DIAG_OPEN_FOLDER: &str = "diag.openFolder";

// ───────────────────────── 문제 보고용 기록 묶기(R2-DIAG-1) ─────────────────────────

mod diag {
    use ink_domain::geometry::MonitorLayout;
    use ink_domain::i18n::Lang;
    use ink_domain::package::InstallChannel;
    use serde_json::{json, Value};
    use std::io::Read;
    use std::path::Path;

    const MAX_LOG_BYTES_PER_FILE: usize = 2 * 1024 * 1024;
    const MAX_LOG_FILES: usize = 5;

    pub(crate) struct DiagInput<'a> {
        pub version: &'a str,
        pub channel: InstallChannel,
        pub lang: Lang,
        pub layout: &'a MonitorLayout,
        pub exported_settings: &'a Value,
        pub include_shortcuts: bool,
        pub logs: &'a [String],
        pub date: &'a str,
    }

    pub(crate) fn diag_zip_name(lang: Lang, date: &str) -> String {
        match lang {
            Lang::Ko => format!("{}-기록-{date}.zip", crate::APP_DIR),
            Lang::En => format!("{}-logs-{date}.zip", crate::APP_DIR),
        }
    }

    /// 설정 JSON에서 폴더 경로·파일 이름 패턴·예외 앱 목록 같은 개인정보 가능 항목을 가린다.
    pub(crate) fn sanitize_settings_for_diag(exported: &Value, include_shortcuts: bool) -> Value {
        let mut root = exported.clone();
        if let Some(settings) = root.get_mut("settings").and_then(Value::as_object_mut) {
            if !include_shortcuts {
                settings.remove("shortcuts");
            }
            if let Some(values) = settings.get_mut("values").and_then(Value::as_object_mut) {
                for key in ["output.folder", "output.template", "record.micDevice", "annotate.font"] {
                    if let Some(v) = values.get_mut(key) {
                        if v.as_str().is_some_and(|s| !s.is_empty()) {
                            *v = Value::String("<redacted>".into());
                        }
                    }
                }
                if let Some(excluded) = values.get_mut("capture.excluded") {
                    if let Some(arr) = excluded.as_array() {
                        let count = arr.len();
                        *excluded = Value::Array(Vec::new());
                        values.insert("capture.excludedCount".into(), json!(count));
                    }
                }
            }
        }
        if let Some(obj) = root.as_object_mut() {
            obj.insert("shortcutsIncluded".into(), Value::Bool(include_shortcuts));
        }
        root
    }

    /// 따옴표 없는 경로에는 공백도 들어갈 수 있어 첫 홈 경로부터 줄 끝까지 가린다.
    /// 한 줄에 홈 경로가 여럿이면 각 경로의 OS 구분만 남긴다.
    fn redact_home_paths(line: &str) -> String {
        let normalized = line.to_ascii_lowercase().replace('\\', "/");
        let mut out = String::new();
        for (start, _) in normalized.match_indices('/') {
            let tail = &normalized[start..];
            let Some(root) = ["/users/", "/home/"].into_iter().find(|root| tail.starts_with(root)) else { continue };
            if out.is_empty() {
                out.push_str(&line[..start]);
            } else {
                out.push(' ');
            }
            let end = start + root.len();
            out.push_str(&line[start..end]);
            out.push_str("<user>");
            out.push(line.as_bytes()[end - 1] as char);
            out.push('…');
        }
        if out.is_empty() { line.to_owned() } else { out }
    }

    /// 로그에서 창 제목 관련 줄과 OS 경로 안의 사용자 이름을 가린다.
    pub(crate) fn sanitize_log_text(raw: &str) -> String {
        let mut lines = Vec::new();
        for line in raw.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("window_title") || lower.contains("windowtitle") || lower.contains("title=") {
                continue;
            }
            lines.push(redact_home_paths(line));
        }
        let mut out = lines.join("\n");
        if !out.is_empty() && raw.ends_with('\n') {
            out.push('\n');
        }
        out
    }

    pub(crate) fn build_diag_entries(input: &DiagInput<'_>) -> Vec<(String, Vec<u8>)> {
        let scales: Vec<f64> = input.layout.monitors.iter().map(|m| m.scale).collect();
        let resolutions: Vec<Value> = input
            .layout
            .monitors
            .iter()
            .map(|m| json!({ "w": m.bounds.w, "h": m.bounds.h, "scale": m.scale, "primary": m.primary }))
            .collect();
        let env_json = json!({
            "app": "ShookShok",
            "version": input.version,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "channel": input.channel,
            "lang": input.lang.code(),
            "monitors": {
                "count": input.layout.monitors.len(),
                "scales": scales,
                "resolutions": resolutions,
            },
            "generatedDate": input.date,
        });
        let settings_json = sanitize_settings_for_diag(input.exported_settings, input.include_shortcuts);
        let mut entries = vec![
            ("environment.json".to_string(), serde_json::to_vec_pretty(&env_json).unwrap_or_default()),
            ("settings.json".to_string(), serde_json::to_vec_pretty(&settings_json).unwrap_or_default()),
        ];
        for (idx, raw) in input.logs.iter().enumerate() {
            let clean = sanitize_log_text(raw);
            entries.push((format!("logs/shookshok-{}.log", idx + 1), clean.into_bytes()));
        }
        entries
    }

    pub(crate) fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    /// 외부 의존성 없이 순수 Rust로 PKZIP(Stored, Method 0, UTF-8 이름)을 만든다.
    pub(crate) fn encode_zip_stored(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, data) in entries {
            let name_bytes = name.as_bytes();
            let name_len = name_bytes.len().min(u16::MAX as usize) as u16;
            let data_len = data.len().min(u32::MAX as usize) as u32;
            let crc = crc32(&data[..data_len as usize]);
            let offset = out.len() as u32;

            // Local File Header (30 bytes + name)
            out.extend_from_slice(&0x0403_4B50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes()); // version needed
            out.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 flag
            out.extend_from_slice(&0u16.to_le_bytes()); // method = Stored
            out.extend_from_slice(&0u16.to_le_bytes()); // mod time
            out.extend_from_slice(&0u16.to_le_bytes()); // mod date
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&data_len.to_le_bytes());
            out.extend_from_slice(&data_len.to_le_bytes());
            out.extend_from_slice(&name_len.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // extra len
            out.extend_from_slice(&name_bytes[..name_len as usize]);
            out.extend_from_slice(&data[..data_len as usize]);

            // Central Directory Header (46 bytes + name)
            central.extend_from_slice(&0x0201_4B50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes()); // version made by
            central.extend_from_slice(&20u16.to_le_bytes()); // version needed
            central.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 flag
            central.extend_from_slice(&0u16.to_le_bytes()); // method = Stored
            central.extend_from_slice(&0u16.to_le_bytes()); // mod time
            central.extend_from_slice(&0u16.to_le_bytes()); // mod date
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&data_len.to_le_bytes());
            central.extend_from_slice(&data_len.to_le_bytes());
            central.extend_from_slice(&name_len.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // extra len
            central.extend_from_slice(&0u16.to_le_bytes()); // comment len
            central.extend_from_slice(&0u16.to_le_bytes()); // disk number start
            central.extend_from_slice(&0u16.to_le_bytes()); // internal file attrs
            central.extend_from_slice(&0u32.to_le_bytes()); // external file attrs
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(&name_bytes[..name_len as usize]);
        }

        let cd_offset = out.len() as u32;
        let cd_size = central.len() as u32;
        let count = entries.len().min(u16::MAX as usize) as u16;
        out.extend_from_slice(&central);

        // End of Central Directory (22 bytes)
        out.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // disk number
        out.extend_from_slice(&0u16.to_le_bytes()); // start disk
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // comment len
        out
    }

    pub(crate) fn build_diag_zip(input: &DiagInput<'_>) -> Vec<u8> {
        encode_zip_stored(&build_diag_entries(input))
    }

    fn read_log_tail(path: &Path) -> Option<String> {
        let mut f = std::fs::File::open(path).ok()?;
        let len = f.metadata().ok()?.len() as usize;
        let mut buf = Vec::new();
        if len > MAX_LOG_BYTES_PER_FILE {
            use std::io::{Seek, SeekFrom};
            f.seek(SeekFrom::End(-(MAX_LOG_BYTES_PER_FILE as i64))).ok()?;
            f.read_to_end(&mut buf).ok()?;
            let s = String::from_utf8_lossy(&buf);
            Some(s.split_once('\n').map(|(_, rest)| rest.to_string()).unwrap_or_else(|| s.into_owned()))
        } else {
            f.read_to_end(&mut buf).ok()?;
            Some(String::from_utf8_lossy(&buf).into_owned())
        }
    }

    /// 로그 폴더에서 최근 `.log` 파일 내용을 읽는다(원래 파일 이름은 쓰지 않는다).
    pub(crate) fn collect_log_texts(log_dir: &Path) -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(log_dir) else { return Vec::new() };
        let mut files: Vec<(std::time::SystemTime, String, std::path::PathBuf)> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let p = e.path();
                if !p.is_file() || p.extension().and_then(|s| s.to_str()) != Some("log") {
                    return None;
                }
                let modified = e.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(std::time::UNIX_EPOCH);
                let name = p.file_name()?.to_string_lossy().into_owned();
                Some((modified, name, p))
            })
            .collect();
        files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        files.truncate(MAX_LOG_FILES);
        files.into_iter().filter_map(|(_, _, p)| read_log_tail(&p)).collect()
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagExportResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn write_unique_zip(dir: &std::path::Path, file_name: &str, bytes: &[u8]) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(dir)?;
    let stem = file_name.strip_suffix(".zip").unwrap_or(file_name);
    let unique = ink_domain::naming::unique_file_name(stem, "zip", |n| dir.join(n).exists());
    let path = dir.join(unique);
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
    std::io::Write::write_all(&mut f, bytes)?;
    Ok(path)
}

/// 설정 › 정보(일반) [기록 묶기](R2-DIAG-1): 앱 로그·환경·설정을 zip 하나로 다운로드 폴더에 저장한다. 통신 없음.
#[tauri::command]
async fn export_diagnostics<R: Runtime>(app: AppHandle<R>, include_shortcuts: Option<bool>) -> DiagExportResult {
    let include = include_shortcuts.unwrap_or(true);
    let (version, channel, lang, exported_settings, layout) = {
        let ink = app.state::<Ink>();
        let s = ink.lock();
        let st = s.state();
        (st.version, st.channel, s.lang(), s.export_file(), s.monitor_layout())
    };
    let logs = app.path().app_log_dir().ok().map(|d| diag::collect_log_texts(&d)).unwrap_or_default();
    let date = today();
    let bytes = diag::build_diag_zip(&diag::DiagInput {
        version: &version,
        channel,
        lang,
        layout: &layout,
        exported_settings: &exported_settings,
        include_shortcuts: include,
        logs: &logs,
        date: &date,
    });
    let dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().document_dir())
        .or_else(|_| app.path().home_dir())
        .map_err(|e| e.to_string());
    let saved = dir.and_then(|d| {
        let wanted = diag::diag_zip_name(lang, &date);
        write_unique_zip(&d, &wanted, &bytes).map_err(|e| e.to_string())
    });
    match saved {
        Ok(path) => {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            *LAST_DIAG_ZIP.lock().unwrap_or_else(|e| e.into_inner()) = Some(path);
            let h = app.clone();
            let detail = name.clone();
            let _ = app.run_on_main_thread(move || {
                let ink = h.state::<Ink>();
                let s = ink.lock();
                let l = s.lang();
                let ev = IslandEvent::new(IslandKind::Result, tr_for(l, "island.diag.saved"))
                    .variant("result")
                    .detail(detail)
                    .button(&tr_for(l, "island.action.openFolder"), ACTION_DIAG_OPEN_FOLDER, None);
                s.island(ev);
            });
            DiagExportResult { ok: true, name: Some(name), error: None }
        }
        Err(e) => {
            log::warn!("diag export failed: {e}");
            let h = app.clone();
            let _ = app.run_on_main_thread(move || {
                let ink = h.state::<Ink>();
                let s = ink.lock();
                s.island(IslandEvent::new(IslandKind::Problem, tr_for(s.lang(), "island.diag.failed")));
            });
            DiagExportResult { ok: false, name: None, error: Some(e) }
        }
    }
}

pub(crate) fn state_lang(state: &InkState) -> Lang {
    state.settings.get("general.lang").and_then(|v| v.as_str()).map(Lang::from_setting).unwrap_or(Lang::Ko)
}

pub(crate) fn lang_of<R: Runtime>(app: &AppHandle<R>) -> Lang {
    app.state::<Ink>().lock().lang()
}

pub(crate) fn tr_for(lang: Lang, key: &str) -> String {
    ink_domain::i18n::tr_os(lang, Platform::current(), key)
}

pub(crate) fn trf_for(lang: Lang, key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    ink_domain::i18n::trf_os(lang, Platform::current(), key, args)
}

/// 지금 언어·OS의 글(플러그인이 만드는 섬·대화상자 글 — R2-I18N).
pub(crate) fn tr<R: Runtime>(app: &AppHandle<R>, key: &str) -> String {
    tr_for(lang_of(app), key)
}

/// 화면이 만든 PDF(내 단축키 한 장, R2-PDF)를 저장 대화상자로 받아 쓴다. 이름은 화면이 정하고(경로 글자는 지움),
/// 기본 위치는 저장 폴더. 저장하면 섬 + [열기]. 취소하면 아무 일도 없다.
#[tauri::command]
async fn save_pdf<R: Runtime>(app: AppHandle<R>, bytes: Vec<u8>, name: String) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    if !bytes.starts_with(b"%PDF-") {
        return Err("not a pdf".into());
    }
    let safe: String = name.chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '\0')).take(120).collect();
    let safe = if safe.to_ascii_lowercase().ends_with(".pdf") { safe } else { format!("{safe}.pdf") };
    let mut dialog = app.dialog().file().add_filter(tr(&app, "file.pdf.filter"), &["pdf"]).set_file_name(&safe);
    if let Some(dir) = output_folder(&app) {
        let _ = std::fs::create_dir_all(&dir);
        dialog = dialog.set_directory(dir);
    }
    let Some(picked) = dialog.blocking_save_file() else { return Ok(false) };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let ok = std::fs::write(&path, &bytes).is_ok();
    if ok {
        *LAST_PDF.lock().unwrap_or_else(|e| e.into_inner()) = Some(path);
    }
    let (saved, failed, open) = (tr(&app, "file.pdf.saved"), tr(&app, "file.pdf.failed"), tr(&app, "file.open"));
    let h = app.clone();
    app.run_on_main_thread(move || {
        let ev = if ok { IslandEvent::new(IslandKind::Result, saved).button(&open, ACTION_PDF_OPEN, None) } else { IslandEvent::new(IslandKind::Problem, failed) };
        h.state::<Ink>().lock().island(ev);
    })
    .map_err(|e| e.to_string())?;
    Ok(ok)
}

/// 화면(웹뷰)에서 난 오류를 앱 로그로(진단용 — 사용자 문제를 로그 하나로 볼 수 있게). 400자까지, 창 이름만 붙인다.
#[tauri::command]
fn ui_log<R: Runtime>(window: tauri::WebviewWindow<R>, level: String, message: String) {
    let msg: String = message.chars().take(400).collect();
    if level == "error" {
        log::error!("ui {}: {msg}", window.label());
    } else {
        log::warn!("ui {}: {msg}", window.label());
    }
}

/// 판서 원본을 요청한 모니터의 웹뷰에만 보낸다. 미지원 환경은 기존 이미지 주소를 쓴다.
#[tauri::command]
async fn draw_frame<R: Runtime>(window: tauri::WebviewWindow<R>, session: u64) -> Result<(), String> {
    #[cfg(windows)]
    {
        let monitor = overlay::monitor_of(window.label()).ok_or("판서 창을 찾지 못했습니다")?;
        let frame = window.state::<overlay::FrameStore>().get(session, monitor).ok_or("종료된 판서입니다")?;
        let (tx, mut rx) = tauri::async_runtime::channel(1);
        window
            .with_webview(move |view| {
                let result = overlay::post_draw_frame(view, session, &frame);
                let _ = tx.try_send(result.map_err(|e| e.to_string()));
            })
            .map_err(|e| e.to_string())?;
        rx.recv().await.ok_or("판서 창이 닫혔습니다")?
    }
    #[cfg(not(windows))]
    {
        let _ = (window, session);
        Err("공유 메모리를 지원하지 않습니다".into())
    }
}

/// 판서 배경을 그렸다 → 진입 계측(M4) `mode enter draw [N ms]` 한 줄.
#[tauri::command]
fn draw_ready(ink: State<'_, Ink>, session: u64, paint_ms: Option<f64>, load_ms: Option<f64>, render_ms: Option<f64>) {
    if ink.lock().draw_ready(session, paint_ms).is_some() {
        if let (Some(ui), Some(load), Some(render)) = (paint_ms, load_ms, render_ms) {
            log::debug!("판서 첫 그림 {session}: 원본 준비 {load:.1} ms, 그리기 대기 {:.1} ms, 캔버스 {render:.1} ms", (ui - load - render).max(0.0));
        }
    }
}

/// 판서 끝(Esc·우클릭 1×) 또는 Rust가 먼저 끝낸 세션의 늦은 끝 — 항목이 있으면 보관.
#[tauri::command]
fn draw_finish(ink: State<'_, Ink>, payload: ink_app::DrawFinish) {
    ink.lock().draw_finish(payload);
}

/// Ctrl+Shift+R · Alt+Shift+1 복원 판정(모니터·배율 규칙).
#[tauri::command]
fn draw_restore(ink: State<'_, Ink>, session: u64) -> ink_app::DrawRestoreResult {
    ink.lock().draw_restore(session)
}

/// 합성 결과 원본 바이트(base64 아님, 03 §4) — 본문 = RGBA, 머리글 `x-ink-action`·`x-ink-w`·`x-ink-h`.
/// 서비스는 메인 스레드에서만 만지므로 동기 명령(캡처 Enter 저장과 같은 길).
#[tauri::command]
fn draw_export(ink: State<'_, Ink>, request: tauri::ipc::Request<'_>) -> Result<ink_app::DrawExported, String> {
    let meta = export_meta(request.headers())?;
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else { return Err("draw_export expects raw bytes".into()) };
    Ok(ink.lock().draw_export(meta, bytes.clone()))
}

/// 판서 섬 한 줄 갱신(색·도구·굵기·배율·쪽·안내).
#[tauri::command]
fn draw_island(ink: State<'_, Ink>, patch: ink_app::DrawIslandPatch) {
    ink.lock().draw_island(patch);
}

/// 펜 지연(M5) 요약을 로그 한 줄로.
#[tauri::command]
fn draw_pen_metrics(ink: State<'_, Ink>, sample: ink_app::PenSample) {
    ink.lock().draw_pen_metrics(sample);
}

/// 실시간 확대 입력(휠·확대 안 키·끝) — HUD·미리보기용. 실제 앱의 휠은 돋보기 훅이 넘긴다.
#[tauri::command]
fn zoom_input(ink: State<'_, Ink>, input: ink_app::ZoomInput) {
    ink.lock().zoom_input(input);
}

/// 핀 화면이 처음 묻는 것: 그림 크기·주소·배율(R-CAP-29).
#[tauri::command]
fn pin_info(store: State<'_, pins::PinStore>, id: u64) -> Option<pins::PinView> {
    pins::view(&store, id)
}

/// 핀 화면 동작: close · closeAll · copy · save · drag(끌어 옮기기) · menu(오른쪽 클릭 메뉴).
#[tauri::command]
fn pin_action<R: Runtime>(window: tauri::WebviewWindow<R>, ink: State<'_, Ink>, id: u64, action: String) -> Result<(), String> {
    match action.as_str() {
        "close" => ink.lock().pin_close(id),
        "closeAll" => {
            ink.lock().pin_close_all();
        }
        "copy" => ink.lock().pin_copy(id),
        "save" => ink.lock().pin_save(id),
        "drag" => window.start_dragging().map_err(|e| {
            log::warn!("pin {id} drag: {e}");
            e.to_string()
        })?,
        "menu" => {
            // 앞에 있지 않은 창의 팝업 메뉴는 바로 닫힌다 → 먼저 초점
            let _ = window.set_focus();
            pins::popup_menu(&window, id).map_err(|e| {
                log::warn!("pin {id} menu: {e}");
                e.to_string()
            })?
        }
        other => return Err(format!("unknown pin action: {other}")),
    }
    Ok(())
}

/// 핀 휠 한 칸 확대·축소(커서 자리 `fx`·`fy` = 창 안 비율). 새 배율(%).
#[tauri::command]
fn pin_zoom<R: Runtime>(window: tauri::WebviewWindow<R>, store: State<'_, pins::PinStore>, id: u64, dy: i32, fx: f64, fy: f64) -> Option<u32> {
    pins::zoom(&window, &store, id, dy, fx, fy)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportPick {
    /// 사용자가 파일 고르기를 취소함
    cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(flatten)]
    preview: Option<ImportPreview>,
}

fn read_json_file(path: &std::path::Path, lang: Lang) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|_| tr_for(lang, "plugin.settings.readError"))?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    serde_json::from_slice(bytes).map_err(|_| tr_for(lang, "error.settings.notSettings"))
}

/// 가져오기 1단계: 파일 고르기 → 무엇이 바뀌는지(시트). 저장은 하지 않는다.
#[tauri::command]
async fn preview_import<R: Runtime>(app: AppHandle<R>) -> Result<ImportPick, String> {
    use tauri_plugin_dialog::DialogExt;
    let lang = lang_of(&app);
    let Some(picked) = app.dialog().file().add_filter(tr_for(lang, "plugin.settings.fileFilter"), &["json"]).blocking_pick_file() else {
        return Ok(ImportPick { cancelled: true, path: None, preview: None });
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let preview = match read_json_file(&path, lang) {
        Ok(file) => app.state::<Ink>().lock().preview_import(&file),
        Err(e) => ImportPreview { ok: false, error: Some(e), changes: vec![], warnings: vec![] },
    };
    Ok(ImportPick { cancelled: false, path: Some(path.to_string_lossy().into_owned()), preview: Some(preview) })
}

/// 가져오기 2단계: 시트에서 [가져오기]를 눌렀을 때.
#[tauri::command]
fn apply_import(ink: State<'_, Ink>, path: String) -> UpdateResult {
    let lang = ink.lock().lang();
    match read_json_file(std::path::Path::new(&path), lang) {
        Ok(file) => ink.lock().apply_import(&file),
        Err(e) => UpdateResult { ok: false, error: Some(e), state: ink.lock().state() },
    }
}

/// UTC 날짜(내보내기 파일 이름용) — 1970-01-01부터의 날 수 → 연·월·일(Hinnant).
pub(crate) fn today() -> String {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

async fn export_settings<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;
    let name = format!("{APP_DIR}-settings-{}.json", today());
    let Some(picked) = app.dialog().file().add_filter(tr(app, "plugin.settings.fileFilter"), &["json"]).set_file_name(&name).blocking_save_file() else {
        return Ok(());
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let body = serde_json::to_string_pretty(&app.state::<Ink>().lock().export_file()).map_err(|e| e.to_string())?;
    let ok = std::fs::write(&path, body).is_ok();
    let h = app.clone();
    app.run_on_main_thread(move || {
        let ink = h.state::<Ink>();
        let s = ink.lock();
        let lang = s.lang();
        let ev = if ok {
            IslandEvent::new(IslandKind::Result, tr_for(lang, "plugin.settings.exported"))
        } else {
            IslandEvent::new(IslandKind::Problem, tr_for(lang, "plugin.settings.exportFailed"))
        };
        s.island(ev);
    })
    .map_err(|e| e.to_string())
}

async fn pick_output_folder<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(picked) = app.dialog().file().blocking_pick_folder() else { return Ok(()) };
    let path = picked.into_path().map_err(|e| e.to_string())?.to_string_lossy().into_owned();
    let h = app.clone();
    app.run_on_main_thread(move || {
        h.state::<Ink>().lock().update_setting("output.folder", &Value::String(path));
    })
    .map_err(|e| e.to_string())
}

/// 화면 동작(`InkAction`). 창을 만들 수 있어 비동기, 서비스 일은 메인 스레드로.
#[tauri::command]
async fn run<R: Runtime>(app: AppHandle<R>, action: String, id: Option<u64>) -> Result<(), String> {
    let hide = |label: &str| {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.hide();
        }
    };
    match action.as_str() {
        "settings.open" | "snipaste.howto" => open_settings(&app),
        "firstRun.open" => {
            // 일반 > 첫 실행 안내 [다시 보기]: 열린 설정 창이면 그 창에 시트를 띄우라고 알린다
            if app.get_webview_window(SETTINGS).is_some() {
                open_settings(&app);
                let _ = app.emit_to(SETTINGS, EVT_FIRST_RUN, ());
            } else {
                open_settings_with(&app, true);
            }
        }
        "settings.close" => {
            if let Some(w) = app.get_webview_window(SETTINGS) {
                let _ = w.close();
            }
        }
        "popover.close" => hide(POPOVER),
        #[cfg(windows)]
        "island.ready" => {
            let handle = app.clone();
            app.run_on_main_thread(move || handle.state::<Ink>().lock().announce_topmost_cleanup()).map_err(|e| e.to_string())?;
        }
        #[cfg(windows)]
        "pin.toggle" | "pin.releaseAll" => {
            let handle = app.clone();
            app.run_on_main_thread(move || {
                if let Some(w) = handle.get_webview_window(POPOVER) { let _ = w.hide(); }
                let ink = handle.state::<Ink>();
                let mut service = ink.lock();
                if action == "pin.releaseAll" {
                    service.unpin_all(true);
                } else {
                    let target = handle.state::<Mutex<TrayPinTarget>>().lock().unwrap_or_else(|e| e.into_inner()).opened.take();
                    service.toggle_tray_target(target);
                }
            }).map_err(|e| e.to_string())?;
        }
        "island.hidden" => {
            log::debug!("island hidden by ui");
            #[cfg(windows)]
            app.state::<IslandGen>().0.fetch_add(1, Ordering::SeqCst);
            hide(ISLAND)
        }
        "folder.open" => {
            hide(POPOVER);
            if let Some(p) = output_folder(&app) {
                open_folder(&p);
            }
        }
        // 자동 업데이트(R2-UPD): [지금 확인]은 뒤 스레드(통신은 잠금 밖), [지금 업데이트]·오늘은 그만은 메인 스레드
        update::ACTION_CHECK => {
            let h = app.clone();
            std::thread::spawn(move || update::check(&h, true));
        }
        update::ACTION_INSTALL => {
            let h = app.clone();
            app.run_on_main_thread(move || update::install(&h)).map_err(|e| e.to_string())?;
        }
        update::ACTION_SNOOZE => {
            let h = app.clone();
            app.run_on_main_thread(move || update::snooze(&h)).map_err(|e| e.to_string())?;
        }
        ACTION_PDF_OPEN => {
            if let Some(p) = LAST_PDF.lock().unwrap_or_else(|e| e.into_inner()).clone() {
                open_file(&p);
            }
        }
        ACTION_DIAG_OPEN_FOLDER => {
            let last = LAST_DIAG_ZIP.lock().unwrap_or_else(|e| e.into_inner()).clone();
            if let Some(p) = last {
                reveal_path(&p);
            } else if let Ok(dir) = app.path().download_dir() {
                open_folder(&dir);
            }
        }
        ink_app::capture::ACTION_CAPTURE_RESTORE => {
            hide(POPOVER);
            let h = app.clone();
            app.run_on_main_thread(move || {
                h.state::<Ink>().lock().restore_discarded_capture();
            })
            .map_err(|e| e.to_string())?;
        }
        // 설정 › 일반 › 설정 위치 [열기](R2-FIX-1): 저장 폴더가 아니라 설정 파일이 있는 곳을 그 파일을 고른 채로
        "settings.reveal" => {
            let path = app.state::<Ink>().lock().state().location.path;
            reveal_path(std::path::Path::new(&path));
        }
        "folder.pick" => pick_output_folder(&app).await?,
        "settings.export" => export_settings(&app).await?,
        "logs.open" => {
            if let Ok(dir) = app.path().app_log_dir() {
                open_folder(&dir);
            }
        }
        "capture.history.clear" => {
            let h = app.clone();
            app.run_on_main_thread(move || {
                let ink = h.state::<Ink>();
                let mut s = ink.lock();
                let lang = s.lang();
                let n = s.clear_ranges();
                let text = match n {
                    0 => tr_for(lang, "plugin.history.empty"),
                    1 => tr_for(lang, "plugin.history.cleared.one"),
                    _ => trf_for(lang, "plugin.history.cleared.other", &[("n", &n)]),
                };
                s.island(IslandEvent::new(IslandKind::Info, text));
            })
            .map_err(|e| e.to_string())?;
        }
        "capture.reopen" => {
            let id = id.ok_or("캡처 번호가 필요합니다")?;
            hide(POPOVER);
            let h = app.clone();
            app.run_on_main_thread(move || h.state::<Ink>().lock().reopen_capture(id)).map_err(|e| e.to_string())?;
        }
        "capture.saveLast" => {
            let h = app.clone();
            app.run_on_main_thread(move || h.state::<Ink>().lock().save_last()).map_err(|e| e.to_string())?;
        }
        "capture.copyHex" => {
            let h = app.clone();
            app.run_on_main_thread(move || h.state::<Ink>().lock().copy_last_hex()).map_err(|e| e.to_string())?;
        }
        // 결과 섬 [판서로]: 방금 고른 범위를 확대한 채 그 모니터 판서로(정지 프레임 재사용, ≤ 150 ms)
        ACTION_TO_DRAW => {
            hide(POPOVER);
            let pressed = Instant::now();
            let h = app.clone();
            app.run_on_main_thread(move || h.state::<Ink>().lock().capture_to_draw_at(pressed)).map_err(|e| e.to_string())?;
        }
        "app.quit" => {
            #[cfg(windows)]
            app.state::<Ink>().lock().unpin_all(false);
            let _ = app.global_shortcut().unregister_all();
            app.exit(0);
        }
        "undo" => {
            let h = app.clone();
            app.run_on_main_thread(move || {
                h.state::<Ink>().lock().undo();
            })
            .map_err(|e| e.to_string())?;
        }
        // 트레이 타일 [전체](R2-UI-TRAY-2): 커서 모니터 바로 캡처 — 단축키를 비워 두어도 된다
        "capture.full" => {
            hide(POPOVER);
            let h = app.clone();
            app.run_on_main_thread(move || h.state::<Ink>().lock().start_quick("full")).map_err(|e| e.to_string())?;
        }
        "capture.region" | "capture.window" => {
            let kind = if action == "capture.region" { CaptureKind::Region } else { CaptureKind::Window };
            let h = app.clone();
            app.run_on_main_thread(move || start_capture(&h, kind)).map_err(|e| e.to_string())?;
        }
        "draw" | "board" | "draw.restore" => {
            let kind = match action.as_str() {
                "draw" => ink_app::DrawKind::Draw,
                "board" => ink_app::DrawKind::Board,
                _ => ink_app::DrawKind::Restore,
            };
            let h = app.clone();
            app.run_on_main_thread(move || start_draw(&h, kind)).map_err(|e| e.to_string())?;
        }
        // 섬 단추 → 판서 엔진(멈춤 도형 [바꾸기] · 모두 지우기 [되돌리기])
        "draw.acceptSuggestion" | "draw.undo" => {
            let command = action.trim_start_matches("draw.").to_string();
            let h = app.clone();
            app.run_on_main_thread(move || {
                h.state::<Ink>().lock().draw_command(&command);
            })
            .map_err(|e| e.to_string())?;
        }
        // 녹화(단계 6.3·6.5): 트레이 시작·중지 · 녹화 섬 [중지]·[버리기] · 복구 섬 [저장 시도]·[지우기]
        "record.toggle" | "record.stop" | "record.discard" | "record.recoverSave" | "record.recoverDelete" => {
            hide(POPOVER);
            let h = app.clone();
            let action = action.clone();
            app.run_on_main_thread(move || {
                let ink = h.state::<Ink>();
                let mut s = ink.lock();
                match action.as_str() {
                    "record.toggle" => s.toggle_record(),
                    "record.stop" => s.stop_recording(),
                    "record.discard" => s.discard_recording(),
                    "record.recoverSave" => s.recover_leftovers(true),
                    _ => s.recover_leftovers(false),
                }
            })
            .map_err(|e| e.to_string())?;
        }
        "pins.closeAll" => {
            let h = app.clone();
            app.run_on_main_thread(move || {
                h.state::<Ink>().lock().pin_close_all();
            })
            .map_err(|e| e.to_string())?;
        }
        "capture.cancel" => {
            let h = app.clone();
            app.run_on_main_thread(move || h.state::<Ink>().lock().end_capture()).map_err(|e| e.to_string())?;
        }
        "zoom" => {
            hide(POPOVER);
            let h = app.clone();
            app.run_on_main_thread(move || h.state::<Ink>().lock().start_zoom()).map_err(|e| e.to_string())?;
        }
        // 첫 실행 카드·설정 점검이 1초마다(R2-ONB-2): 권한을 다시 읽고 바뀌었을 때만 상태를 보낸다(두 OS — Windows는 아무 일도 없음)
        ink_app::ACTION_PERM_REFRESH => {
            let h = app.clone();
            app.run_on_main_thread(move || {
                h.state::<Ink>().lock().poll_permissions();
            })
            .map_err(|e| e.to_string())?;
        }
        // 맥 권한(M10): 단추 하나로 요청 + 정확한 설정 칸 열기. 켜는 것은 사용자가 한다
        #[cfg(target_os = "macos")]
        ink_app::ACTION_PERM_SCREEN => {
            hide(POPOVER);
            let h = app.clone();
            app.run_on_main_thread(move || {
                let ink = h.state::<Ink>();
                let mut s = ink.lock();
                let lang = s.lang();
                s.end_capture();
                ink_platform_macos::permissions::request_screen();
                // 켠 뒤에는 다시 시작해야 반영된다(그 순간 macOS도 "종료 후 다시 열기"를 묻는다)
                s.island(
                    IslandEvent::new(IslandKind::Info, tr_for(lang, "plugin.perm.screenGuide"))
                        .detail(tr_for(lang, "plugin.perm.relaunchDetail"))
                        .button(&tr_for(lang, "plugin.perm.relaunchBtn"), ACTION_RELAUNCH, None)
                        .duration_ms(PERMISSION_WAIT.as_millis() as u32),
                );
            })
            .map_err(|e| e.to_string())?;
        }
        #[cfg(target_os = "macos")]
        ink_app::ACTION_PERM_AX => {
            hide(POPOVER);
            let h = app.clone();
            app.run_on_main_thread(move || {
                let ink = h.state::<Ink>();
                let mut s = ink.lock();
                let lang = s.lang();
                s.end_capture();
                ink_platform_macos::permissions::request_accessibility();
                s.island(
                    IslandEvent::new(IslandKind::Info, tr_for(lang, "plugin.perm.axGuide"))
                        .detail(tr_for(lang, "plugin.perm.axDetail"))
                        .duration_ms(PERMISSION_WAIT.as_millis() as u32),
                );
            })
            .map_err(|e| e.to_string())?;
            // 손쉬운 사용은 켜자마자 반영된다 → 1초마다 보고 켜지면 알린다(다시 시작 필요 없음)
            let h = app.clone();
            std::thread::spawn(move || {
                let until = Instant::now() + PERMISSION_WAIT;
                while Instant::now() < until {
                    std::thread::sleep(Duration::from_secs(1));
                    if ink_platform_macos::permissions::accessibility() {
                        let app = h.clone();
                        let _ = h.run_on_main_thread(move || {
                            let ink = app.state::<Ink>();
                            let mut s = ink.lock();
                            let lang = s.lang();
                            s.refresh_permissions();
                            s.island(IslandEvent::new(IslandKind::Result, tr_for(lang, "plugin.perm.axGranted")));
                            let _ = app.emit(EVT_STATE, &s.state());
                        });
                        return;
                    }
                }
            });
        }
        #[cfg(target_os = "macos")]
        ink_app::ACTION_PERM_MIC => {
            hide(POPOVER);
            // 아직 묻지 않았으면 macOS 허용 창, 이미 정해졌으면(꺼짐) 시스템 설정 › 마이크 칸
            let asked = app.state::<Ink>().lock().request_microphone_if_undetermined();
            if !asked {
                ink_platform_macos::permissions::open_microphone_settings();
            }
        }
        #[cfg(windows)]
        ink_app::ACTION_PERM_MIC => {
            hide(POPOVER);
            ink_platform_windows::microphone::open_settings()?;
        }
        ACTION_RELAUNCH => app.restart(),
        other => {
            hide(POPOVER);
            let h = app.clone();
            let other = other.to_string();
            app.run_on_main_thread(move || not_ready(&h, &other)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ───────────────────────── 조립 ─────────────────────────

fn package_runtime() -> Box<dyn ink_app::PackageRuntime> {
    #[cfg(windows)]
    {
        Box::new(ink_platform_windows::package::WindowsPackageRuntime)
    }
    #[cfg(not(windows))]
    {
        Box::new(ink_app::UnpackagedRuntime)
    }
}

fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    app.manage(PopoverClock::default());
    #[cfg(windows)]
    app.manage(Mutex::new(TrayPinTarget::default()));
    #[cfg(target_os = "macos")]
    app.manage(PopoverAnchor::default());
    app.manage(IslandGen::default());
    app.manage(HotkeySlots::default());
    let frames = FrameStore::default();
    app.manage(frames.clone());
    let thumbs = adapters::ThumbStore::default();
    app.manage(thumbs.clone());
    let pin_store = pins::PinStore::default();
    app.manage(pin_store.clone());
    // 핀 오른쪽 클릭 메뉴(pin:<동작>:<id>) → 서비스
    app.on_menu_event(|app, event| {
        let Some((action, id)) = pins::parse_menu_id(event.id().as_ref()) else { return };
        let mut ink = app.state::<Ink>().inner().lock();
        match action {
            "copy" => ink.pin_copy(id),
            "save" => ink.pin_save(id),
            "close" => ink.pin_close(id),
            "closeAll" => {
                ink.pin_close_all();
            }
            _ => {}
        }
    });

    let exe_dir = std::env::current_exe()?.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let config_dir = app.path().config_dir()?;
    let user_dir = config_dir.join(APP_DIR);
    let legacy = config_dir.join(LEGACY_DIR);
    if !user_dir.exists() && legacy.is_dir() {
        // 이름 바꾸기 전에 쓰던 설정을 그대로 가져간다. 실패하면 새 폴더에서 처음부터.
        let _ = std::fs::rename(&legacy, &user_dir);
    }
    let pkg_runtime = package_runtime();
    let channel = pkg_runtime.channel();
    let (dir, portable) = ink_store::resolve_dir(&exe_dir, &user_dir, channel);
    let (screen, windows, monitors) = screen_ports();
    let ports = Ports {
        hotkeys: Box::new(TauriHotkeys { app: app.clone() }),
        probe: probe(),
        store: Box::new(ink_store::JsonFileStore::new(dir, portable)),
        notifier: Box::new(TauriNotifier {
            app: app.clone(),
            tray_mode: Mutex::new(None),
            tray_tooltip: Mutex::new(None),
            last_lang: Mutex::new(None),
        }),
        screen,
        windows,
        monitors,
        overlay: Box::new(TauriOverlayHost::new(app.clone(), frames, thumbs)),
        elements: element_source(app),
        vision: vision_source(app),
        clipboard: Box::new(adapters::TauriClipboard { app: app.clone() }),
        files: Box::new(adapters::TauriFiles { app: app.clone() }),
        clock: Box::new(adapters::SystemClock),
        pins: Box::new(pins::TauriPinHost::new(app.clone(), pin_store)),
    };
    let mut service = InkService::new(ports, Platform::current(), app.package_info().version.to_string());
    #[cfg(windows)]
    {
        service.set_window_pinner(Box::new(ink_platform_windows::topmost::WinWindowPinner::default()));
        service.cleanup_on_start();
    }
    service.set_package_runtime(pkg_runtime);
    #[cfg(windows)]
    if channel.is_packaged() {
        service.set_autostart(Box::new(ink_platform_windows::package::WindowsStartupTaskAutostart::default()));
    }
    // 실시간 확대(Alt+2): Windows = Magnification API + HUD, 맥 = SCStream 돋보기 + HUD(M13), 그 밖 = 없음
    service.set_magnifier(zoom_hud::magnifier(app));
    // 녹화(단계 6.3): 임시 파일은 %TEMP%의 shookshok 폴더, 저장 폴더가 비면 사진 폴더의 shookshok
    #[cfg(windows)]
    {
        let pictures = app.path().picture_dir().unwrap_or_else(|_| std::env::temp_dir()).join(APP_DIR);
        service.set_recorder(Box::new(ink_platform_windows::recorder::WinRecorder::new(std::env::temp_dir().join(APP_DIR), pictures)));
    }
    // 맥 녹화(M15): SCStream → AVAssetWriter(H.264 + AAC). 임시 폴더 = $TMPDIR/Shookshok, 기본 저장 = 사진/Shookshok
    #[cfg(target_os = "macos")]
    {
        let pictures = app.path().picture_dir().unwrap_or_else(|_| std::env::temp_dir()).join(APP_DIR);
        let h = app.clone();
        let rec = ink_platform_macos::recorder::MacRecorder::new(std::env::temp_dir().join(APP_DIR), pictures).with_window_rect_callback(move |rect| {
            let app = h.clone();
            let _ = h.run_on_main_thread(move || {
                if let Some(ink) = app.try_state::<Ink>() {
                    ink.lock().on_tracked_window_rect(rect);
                }
            });
        });
        service.set_recorder(Box::new(rec));
    }
    // 개발용 사람 정답 도구: 확정할 때마다 인식 시험 사례를 쓴다(시험 문서 화면에서만, 08 §5.2)
    if let Ok(dir) = std::env::var("SCREEN_INK_CASE_DIR") {
        log::warn!("recognition cases will be written on every confirm");
        service.set_case_dir(Some(dir));
    }
    let failed = service.start();
    let first_run = service.is_first_run();
    let autostart = service.settings().bool("general.autostart");
    let initial = service.state();
    log::info!("ShookShok ready: {} hotkey(s) not registered", failed);
    app.manage(Ink(Mutex::new(service)));

    // 창·트레이는 플러그인 초기화가 끝난 뒤 이벤트 루프에서 만든다. 플러그인 setup은 플러그인 목록
    // 잠금을 쥔 채 돌고, 창을 만들면 창 생성 훅이 같은 잠금을 잡아 교착된다. 다른 스레드에서
    // run_on_main_thread를 부르면 즉시 실행이 아니라 이벤트 루프 대기열로 들어간다.
    let handle = app.clone();
    std::thread::spawn(move || {
        let h = handle.clone();
        if let Err(e) = handle.run_on_main_thread(move || {
            if let Err(e) = finish_setup(&h, initial, first_run, autostart, started) {
                log::error!("ShookShok setup failed: {e}");
            }
        }) {
            log::error!("ShookShok setup could not be scheduled: {e}");
        }
    });
    Ok(())
}

fn finish_setup<R: Runtime>(app: &AppHandle<R>, initial: InkState, first_run: bool, autostart: bool, started: Instant) -> tauri::Result<()> {
    build_static_windows(app)?;
    // 업데이트·재서명 뒤 화면 기록 권한이 풀렸는지(M10) — 섬 화면이 준비된 뒤에 알린다
    let h = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        let app = h.clone();
        let _ = h.run_on_main_thread(move || app.state::<Ink>().lock().startup_permission_check());
    });
    if let Err(e) = zoom_hud::build(app) {
        log::error!("zoom hud window: {e}");
    }
    // 자동 업데이트: 켠 뒤 10분부터 하루 한 번(설정에서 끄면 통신 0)
    update::schedule(app);
    // 맥: 독 아이콘 없이 메뉴 막대에만(트레이 앱), 섬은 오버레이 위·모든 Space
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        use ink_platform_macos::window::{configure, Role};
        for (label, role) in [(ISLAND, Role::Island), (POPOVER, Role::Popover)] {
            if let Some(ns) = app.get_webview_window(label).and_then(|w| w.ns_window().ok()) {
                configure(ns, role);
            }
        }
    }
    exclude_from_capture(app, ISLAND);
    exclude_from_capture(app, POPOVER);
    exclude_from_capture(app, rec_frame::REC_FRAME);
    listen_system_events(app);
    start_ticker(app);
    let mode = if initial.hotkeys_enabled { TrayMode::Ready } else { TrayMode::Off };
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_image(mode))
        .tooltip(tooltip(&initial))
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            // 클릭하면 작업 표시줄이 먼저 활성화될 수 있어 아이콘에 들어온 순간의 대상도 기억한다.
            #[cfg(windows)]
            if matches!(event, TrayIconEvent::Enter { .. } | TrayIconEvent::Leave { .. }) {
                let app = tray.app_handle();
                let target = if matches!(event, TrayIconEvent::Enter { .. }) {
                    app.state::<Ink>().lock().foreground_for_tray()
                } else {
                    None
                };
                app.state::<Mutex<TrayPinTarget>>().lock().unwrap_or_else(|e| e.into_inner()).before_click = target;
            }
            // 왼쪽·오른쪽 클릭 모두 같은 팝오버(S01, Windows 관례)
            if let TrayIconEvent::Click { button_state: MouseButtonState::Up, rect, .. } = event {
                toggle_popover(tray.app_handle(), rect);
            }
        })
        .build(app)?;

    sync_autostart(app, autostart);
    // 계측 줄(05 §9): 창 제목·파일 이름 없이 시간만
    log::info!("launch time {} ms", started.elapsed().as_millis());
    if first_run {
        app.state::<Ink>().lock().update_setting("onboarding.done", &Value::Bool(true));
        open_settings_with(app, true);
    }
    Ok(())
}

/// 서비스 틱(250 ms): 지연 캡처 카운트다운 · 녹화 섬 경과 · 녹화기 스스로 멈춤 · 남은 녹화 복구 제안(단계 6).
/// 창이 다 만들어진 뒤에 시작해 첫 틱의 복구 섬이 섬 창에 닿게 한다.
fn start_ticker<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    let _ = std::thread::Builder::new().name("shookshok-tick".into()).spawn(move || loop {
        std::thread::sleep(Duration::from_millis(250));
        let h = handle.clone();
        let sent = handle.run_on_main_thread(move || {
            if let Some(ink) = h.try_state::<Ink>() {
                ink.lock().tick();
            }
        });
        if sent.is_err() {
            break;
        }
    });
}

/// 잠금·절전·디스플레이 변경·테마(R-SYS-3). 사건 스레드 → 메인 스레드로 넘겨 서비스에.
fn listen_system_events<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(windows)]
    {
        let handle = app.clone();
        ink_platform_windows::events::listen(move |ev: SystemEvent| {
            let h = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                log::info!("system event {ev:?}");
                h.state::<Ink>().lock().on_system_event(ev);
            });
        });
    }
    #[cfg(target_os = "macos")]
    {
        let handle = app.clone();
        ink_platform_macos::events::listen(move |ev: SystemEvent| {
            let h = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                log::info!("system event {ev:?}");
                h.state::<Ink>().lock().on_system_event(ev);
            });
        });
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let _ = app;
}

/// 전역 단축키 플러그인(처리기 = 눌린 칸을 서비스로). screen-ink 플러그인보다 **먼저** 붙여야 한다.
pub fn global_shortcut<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            // 진입 계측(M1)의 시작점 = 키 메시지를 받은 이 순간
            let pressed = Instant::now();
            let Some(slots) = app.try_state::<HotkeySlots>() else { return };
            let slot = slots.0.lock().unwrap_or_else(|e| e.into_inner()).get(&shortcut.id()).cloned();
            if let Some(slot) = slot {
                let h = app.clone();
                let _ = app.run_on_main_thread(move || on_hotkey(&h, &slot, pressed));
            }
        })
        .build()
}

/// 앱에 붙이는 문 — 필요한 플러그인을 순서대로 붙인다(플러그인 setup 안에서 `app.plugin()`을 부르면
/// 플러그인 목록 잠금을 다시 잡아 교착되므로 Builder 단계에서). Tidy도 같은 한 줄로 붙인다.
pub fn install<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .plugin(global_shortcut())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_dialog::init())
        .plugin(init())
}

/// screen-ink 플러그인 자체. 보통은 [`install`]을 쓴다.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("screen-ink")
        .invoke_handler(tauri::generate_handler![
            get_state,
            update_setting,
            preview_shortcut,
            set_shortcut,
            reset_shortcut_group,
            undo,
            set_hotkeys_enabled,
            shortcut_capture,
            save_pdf,
            system_fonts,
            audio_inputs,
            text_focus,
            run,
            preview_import,
            apply_import,
            reset_all,
            overlay_ready,
            capture_input,
            pin_info,
            pin_action,
            pin_zoom,
            draw_ready,
            draw_frame,
            draw_finish,
            draw_restore,
            draw_export,
            draw_island,
            draw_pen_metrics,
            zoom_input,
            capture_annotations,
            capture_commit_annotated,
            export_diagnostics,
            ui_log
        ])
        .register_uri_scheme_protocol(overlay::SCHEME, |ctx, req| overlay::serve(ctx, req))
        .setup(|app, _api| setup(app))
        .on_event(|_app, event| {
            #[cfg(windows)]
            if matches!(event, RunEvent::ExitRequested { code: Some(_), .. } | RunEvent::Exit) {
                if let Some(ink) = _app.try_state::<Ink>() { ink.lock().unpin_all(false); }
            }
            // 트레이 상주 앱: 마지막 창을 닫아도 끝나지 않는다(끝내기는 팝오버 [끝내기] = exit(0))
            if let RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        })
        .build()
}

#[cfg(test)]
mod tests {
    #[test]
    fn windows_글꼴_목록에서_집_이름만() {
        let names = ["Malgun Gothic (TrueType)", "Malgun Gothic Bold (TrueType)", "Segoe UI & Segoe UI Semilight (TrueType)"];
        let mut v = names.into_iter().flat_map(super::windows_family_names).collect();
        super::tidy_font_names(&mut v);
        assert_eq!(v, ["Malgun Gothic", "Segoe UI"]);
        assert_eq!(super::windows_family_names("맑은 고딕 Bold (TrueType)"), ["맑은 고딕"]);
    }

    use super::*;
    use tauri::http::{HeaderMap, HeaderName, HeaderValue};

    #[test]
    fn 내보내기_머리글은_대소문자와_무관하게_읽는다() {
        let mut h = HeaderMap::new();
        for (k, v) in [("X-Ink-Action", "save"), ("X-INK-W", "1920"), ("x-ink-h", "1080"), ("X-Ink-Session", "4")] {
            h.insert(HeaderName::from_bytes(k.as_bytes()).unwrap(), HeaderValue::from_static(v));
        }
        let m = export_meta(&h).unwrap();
        assert_eq!((m.action, m.w, m.h, m.session, m.bgra), (ink_app::ExportAction::Save, 1920, 1080, Some(4), false));
        h.remove("x-ink-w");
        assert!(export_meta(&h).is_err());
    }

    #[test]
    fn 트레이_툴팁은_언어와_상태에_따라_바뀐다() {
        assert_eq!(tooltip_for(Lang::Ko, true, 0, 0), "슉쇽");
        assert_eq!(tooltip_for(Lang::En, true, 0, 0), "ShookShok");
        assert_eq!(tooltip_for(Lang::Ko, false, 2, 1), "슉쇽 · 단축키 꺼짐 · 맨 위 창 1개");
        assert_eq!(tooltip_for(Lang::En, false, 2, 1), "ShookShok · Shortcuts paused · 1 window on top");
        assert_eq!(tooltip_for(Lang::Ko, true, 1, 3), "슉쇽 · 키 1개 겹침 · 맨 위 창 3개");
        assert_eq!(tooltip_for(Lang::En, true, 2, 2), "ShookShok · 2 key conflicts · 2 windows on top");
    }

    #[test]
    fn 플러그인_번역_키가_모두_카탈로그에_있다() {
        let keys = [
            "common.appTitle",
            "error.settings.notSettings",
            "island.action.openFolder",
            "island.capture.annotDiscarded",
            "island.capture.annotRestoreBtn",
            "island.diag.failed",
            "island.diag.saved",
            "plugin.history.cleared.one",
            "plugin.history.cleared.other",
            "plugin.history.empty",
            "plugin.notReady",
            "plugin.perm.axDetail",
            "plugin.perm.axGranted",
            "plugin.perm.axGuide",
            "plugin.perm.relaunchBtn",
            "plugin.perm.relaunchDetail",
            "plugin.perm.screenGuide",
            "plugin.pinMenu.close",
            "plugin.pinMenu.closeAll",
            "plugin.pinMenu.copy",
            "plugin.pinMenu.save",
            "plugin.settings.exportFailed",
            "plugin.settings.exported",
            "plugin.settings.fileFilter",
            "plugin.settings.readError",
            "plugin.tray.conflicts.one",
            "plugin.tray.conflicts.other",
            "plugin.tray.paused",
            "plugin.tray.topmost.one",
            "plugin.tray.topmost.other",
            "plugin.window.island",
            "plugin.window.overlay",
            "plugin.window.pin",
            "plugin.window.popover",
            "plugin.window.recFrame",
            "plugin.window.settings",
            "plugin.window.zoomHud",
        ];
        for key in keys {
            let ko = tr_for(Lang::Ko, key);
            let en = tr_for(Lang::En, key);
            assert_ne!(ko, key, "ko missing: {key}");
            assert_ne!(en, key, "en missing: {key}");
            assert!(!en.chars().any(|c| ('가'..='힣').contains(&c)), "en has hangul for {key}: {en}");
        }
    }

    #[test]
    fn 진단_로그에서_홈_이름과_하위_경로를_가린다() {
        for path in [
            "/Users/alice", "/home/alice/개인 문서", r"C:\Users\bob",
            r"C:\Users\Kim Minsoo\개인 문서", "C:/Users/bob/개인 문서",
            r"C:\\Users\\bob\\개인 문서",
        ] {
            let log = diag::sanitize_log_text(&format!("경고 {path}\n다음 줄"));
            assert!(log.starts_with("경고 ") && log.ends_with("\n다음 줄"));
            for private in ["alice", "bob", "Kim", "Minsoo", "개인", "문서"] {
                assert!(!log.contains(private), "{log}");
            }
        }
    }

    #[test]
    fn 진단_zip은_언어별_이름과_개인정보_제외_규칙을_지킨다() {
        use ink_domain::geometry::{Monitor, MonitorLayout, PhysRect};
        use ink_domain::package::InstallChannel;

        assert_eq!(diag::diag_zip_name(Lang::Ko, "2026-11-27"), "Shookshok-기록-2026-11-27.zip");
        assert_eq!(diag::diag_zip_name(Lang::En, "2026-11-27"), "Shookshok-logs-2026-11-27.zip");

        let layout = MonitorLayout::new(vec![
            Monitor { id: 0, bounds: PhysRect::new(0, 0, 2560, 1440), work: PhysRect::new(0, 0, 2560, 1400), scale: 2.0, primary: true },
        ]);
        let exported = serde_json::json!({
            "schemaVersion": 1,
            "app": "shookshok",
            "settings": {
                "values": {
                    "output.folder": "/Users/alice/Pictures/SecretClass",
                    "output.template": "비밀수업_{n}",
                    "record.micDevice": "Alice AirPods",
                    "annotate.font": "개인 서체",
                    "capture.excluded": [{ "exe": "SecretApp", "mode": "exclude", "auto": false }],
                    "general.lang": "ko"
                },
                "shortcuts": { "region": "Option+F1" }
            }
        });
        let logs = vec![
            "[INFO] capture ready 12 ms\n[DEBUG] window_title=개인정보문서\n[WARN] failed /Users/alice/Desktop/a.png and C:\\Users\\bob\\doc.hwp\n".to_string(),
        ];

        // 단축키 제외 옵션
        let entries_no_sc = diag::build_diag_entries(&diag::DiagInput {
            version: "0.2.0",
            channel: InstallChannel::Unpackaged,
            lang: Lang::Ko,
            layout: &layout,
            exported_settings: &exported,
            include_shortcuts: false,
            logs: &logs,
            date: "2026-11-27",
        });
        let names: Vec<&str> = entries_no_sc.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["environment.json", "settings.json", "logs/shookshok-1.log"]);

        let env_v: Value = serde_json::from_slice(&entries_no_sc[0].1).unwrap();
        assert_eq!(env_v["version"], "0.2.0");
        assert_eq!(env_v["monitors"]["count"], 1);
        assert_eq!(env_v["monitors"]["scales"], serde_json::json!([2.0]));
        assert_eq!(env_v["os"], std::env::consts::OS);

        let set_v: Value = serde_json::from_slice(&entries_no_sc[1].1).unwrap();
        assert_eq!(set_v["shortcutsIncluded"], false);
        assert!(set_v["settings"].get("shortcuts").is_none());
        assert_eq!(set_v["settings"]["values"]["output.folder"], "<redacted>");
        assert_eq!(set_v["settings"]["values"]["output.template"], "<redacted>");
        assert_eq!(set_v["settings"]["values"]["record.micDevice"], "<redacted>");
        assert_eq!(set_v["settings"]["values"]["annotate.font"], "<redacted>");
        assert_eq!(set_v["settings"]["values"]["capture.excludedCount"], 1);
        let set_raw = String::from_utf8(entries_no_sc[1].1.clone()).unwrap();
        assert!(!set_raw.contains("alice") && !set_raw.contains("SecretClass") && !set_raw.contains("SecretApp"));

        let log_raw = String::from_utf8(entries_no_sc[2].1.clone()).unwrap();
        assert!(!log_raw.contains("개인정보문서"));
        assert!(!log_raw.contains("/Users/alice/"));
        assert!(!log_raw.contains("\\Users\\bob\\"));
        assert!(log_raw.contains("/Users/<user>/"));
        assert!(log_raw.contains("\\Users\\<user>\\"));

        // 단축키 포함 + ZIP 포맷 검증(로컬 헤더·중앙 디렉터리·EOCD·CRC32)
        let zip = diag::build_diag_zip(&diag::DiagInput {
            version: "0.2.0",
            channel: InstallChannel::Unpackaged,
            lang: Lang::En,
            layout: &layout,
            exported_settings: &exported,
            include_shortcuts: true,
            logs: &logs,
            date: "2026-11-27",
        });
        assert_eq!(&zip[0..4], &[0x50, 0x4B, 0x03, 0x04]);
        let eocd = zip.windows(4).rposition(|w| w == [0x50, 0x4B, 0x05, 0x06]).expect("EOCD");
        let total_entries = u16::from_le_bytes([zip[eocd + 10], zip[eocd + 11]]);
        assert_eq!(total_entries, 3);
        let cd_offset = u32::from_le_bytes([zip[eocd + 16], zip[eocd + 17], zip[eocd + 18], zip[eocd + 19]]) as usize;
        assert_eq!(&zip[cd_offset..cd_offset + 4], &[0x50, 0x4B, 0x01, 0x02]);
        // 첫 번째 파일(environment.json) 크기·CRC 검증
        let crc_stored = u32::from_le_bytes([zip[14], zip[15], zip[16], zip[17]]);
        let size_stored = u32::from_le_bytes([zip[18], zip[19], zip[20], zip[21]]) as usize;
        let name_len = u16::from_le_bytes([zip[26], zip[27]]) as usize;
        let data_start = 30 + name_len;
        assert_eq!(&zip[30..data_start], b"environment.json");
        assert_eq!(diag::crc32(&zip[data_start..data_start + size_stored]), crc_stored);
    }
}
