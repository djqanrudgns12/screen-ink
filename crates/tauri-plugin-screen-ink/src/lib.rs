//! tauri-plugin-screen-ink — 앱 핵심(InkService)을 Tauri에 붙인다(03 §2·§6).
//!
//! - 명령 `plugin:screen-ink|…` = `InkClient` 메서드(packages/ink-client/src/tauri.js와 1:1)
//! - 이벤트 `screen-ink://state` · `screen-ink://island`
//! - 창: `ink-popover`(트레이 팝오버 S01) · `ink-settings`(S02~S05) · `ink-island`(섬 S17)
//! - 전역 단축키 = tauri-plugin-global-shortcut 어댑터, 트레이 아이콘 = 코드로 그림(tray_art)
//!
//! 스레드 규칙: 서비스는 메인 스레드에서만 만진다(동기 명령·트레이·단축키 처리). 창을 새로 만드는
//! 명령(`run`)만 비동기로 두고(Windows WebView2 교착 회피) 서비스 일은 메인 스레드로 넘긴다.

mod adapters;
mod overlay;
mod pins;
mod tray_art;

use ink_app::{
    HotkeyError, HotkeyOutcome, HotkeyRegistry, ImportPreview, InkService, InkState, IslandEvent, IslandKind, MonitorProbe, Notifier, Ports, PreviewResult, ScreenSource,
    SetShortcutResult, SystemProbe, UpdateResult, WindowCatalog,
};
#[cfg(any(windows, target_os = "macos"))]
use ink_app::SystemEvent;
#[cfg(all(not(windows), not(target_os = "macos")))]
use ink_domain::shortcuts::LiveBinding;
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

pub const POPOVER: &str = "ink-popover";
pub const SETTINGS: &str = "ink-settings";
pub const ISLAND: &str = "ink-island";
const TRAY_ID: &str = "screen-ink";
/// 화면에 보이는 앱 이름(docs/brand/README.md). 내부 식별자(플러그인·이벤트·트레이 ID)는 `screen-ink` 그대로.
pub const APP_NAME: &str = "슉쇽";
/// 폴더·파일 이름에 쓰는 영문 이름(사진\shookshok, %APPDATA%\shookshok).
pub const APP_DIR: &str = "shookshok";
/// 이름을 바꾸기 전 설정 폴더 — 새 폴더가 없으면 한 번 옮긴다.
const LEGACY_DIR: &str = "screen-ink";
const EVT_STATE: &str = "screen-ink://state";
const EVT_ISLAND: &str = "screen-ink://island";
const EVT_CAPTURE: &str = "screen-ink://capture";
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
}

impl<R: Runtime> Notifier for TauriNotifier<R> {
    fn state(&self, state: &InkState) {
        let _ = self.app.emit(EVT_STATE, state);
        let mode = if state.recording.active {
            TrayMode::Recording
        } else if state.hotkeys_enabled {
            TrayMode::Ready
        } else {
            TrayMode::Off
        };
        let key = (mode, light_taskbar());
        let mut last = self.tray_mode.lock().unwrap_or_else(|e| e.into_inner());
        if *last != Some(key) {
            *last = Some(key);
            set_tray_icon(&self.app, mode, state);
        }
    }

    fn island(&self, event: &IslandEvent) {
        // 모드 끝은 거두기만 — 창을 새로 보이지 않는다(UI가 섬을 치우면 창도 숨김)
        if event.kind != IslandKind::End {
            show_island(&self.app);
        }
        let _ = self.app.emit(EVT_ISLAND, event);
    }

    fn capture(&self, view: &ink_app::CaptureView) {
        let _ = self.app.emit(EVT_CAPTURE, view);
    }
}

/// A 근거 어댑터: Windows = UIA 색인 일꾼(끝나면 메인 스레드에서 서비스에 알림), 그 밖 = 없음.
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
    #[cfg(not(windows))]
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

#[cfg(not(windows))]
struct NoElements;

#[cfg(not(windows))]
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
        (Box::new(WindowsScreen), Box::new(WindowsScreen), Box::new(WindowsScreen))
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

fn tooltip(state: &InkState) -> String {
    let mut t = String::from(APP_NAME);
    if !state.hotkeys_enabled {
        t.push_str(" · 단축키 꺼짐");
    } else if state.conflicts.count > 0 {
        t.push_str(&format!(" · 키 {}개 겹침", state.conflicts.count));
    }
    if state.pinned_count > 0 {
        t.push_str(&format!(" · 고정한 창 {}", state.pinned_count));
    }
    t
}

fn set_tray_icon<R: Runtime>(app: &AppHandle<R>, mode: TrayMode, state: &InkState) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(tray_image(mode)));
        let _ = tray.set_tooltip(Some(tooltip(state)));
    }
}

fn show_island<R: Runtime>(app: &AppHandle<R>) {
    let Some(w) = app.get_webview_window(ISLAND) else { return };
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
    // 캡처 중이면 정지 화면·오버레이 창이 나중에 맨 위로 올라와 있다 → 섬을 그 위로(맨 위 무리의 맨 앞)
    #[cfg(windows)]
    if let Ok(h) = w.hwnd() {
        ink_platform_windows::events::set_window_shown(h.0 as isize, true);
    }
    let gen = app.state::<IslandGen>().0.fetch_add(1, Ordering::SeqCst) + 1;
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

fn build_static_windows<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let popover = WebviewWindowBuilder::new(app, POPOVER, WebviewUrl::App("index.html?view=popover".into()))
        .title(APP_NAME)
        .inner_size(304.0, 460.0)
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

    WebviewWindowBuilder::new(app, ISLAND, WebviewUrl::App("index.html?view=island".into()))
        .title("shookshok island")
        .inner_size(560.0, 88.0)
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
    Ok(())
}

/// 트레이 팝오버 열기·닫기. 위치: Windows = 아이콘 위(작업 표시줄이 위면 아래), 맥 = 메뉴 막대 아이콘 바로 아래.
fn toggle_popover<R: Runtime>(app: &AppHandle<R>, tray_rect: Rect) {
    let Some(w) = app.get_webview_window(POPOVER) else { return };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    let just_closed = app.state::<PopoverClock>().0.lock().unwrap_or_else(|e| e.into_inner()).is_some_and(|t| t.elapsed() < Duration::from_millis(250));
    if just_closed {
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
    {
        let ink = app.state::<Ink>();
        let mut s = ink.lock();
        s.refresh_environment();
        let st = s.state();
        let _ = app.emit(EVT_STATE, &st);
    }
    // 맥: 활성화하지 않는 패널을 지금 Space(전체 화면 앱 포함)에 올리고 키 창으로만 — 앱을 활성화하면 Space가 넘어간다
    #[cfg(target_os = "macos")]
    if let Ok(ns) = w.ns_window() {
        ink_platform_macos::window::order_front(ns);
        let _ = w.show();
        ink_platform_macos::window::make_key(ns);
        return;
    }
    let _ = w.show();
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
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App(url.into()))
        .title(format!("{APP_NAME} 설정"))
        .inner_size(900.0, 600.0)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        .center()
        .build();
    match built {
        Ok(_) => {
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

/// 아직 만들지 않은 모드(단계 2 이후) — 눌린 것은 알려 준다.
fn not_ready<R: Runtime>(app: &AppHandle<R>, slot_or_action: &str) {
    let label = catalog().slot(slot_or_action).map(|s| s.label.clone()).unwrap_or_else(|| slot_or_action.to_string());
    app.state::<Ink>().lock().island(IslandEvent::new(IslandKind::Info, "준비 중이에요").detail(label));
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

#[cfg(not(debug_assertions))]
fn sync_autostart<R: Runtime>(app: &AppHandle<R>, on: bool) {
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
    let r = ink.lock().update_setting(&path, &value);
    if r.ok && path == "general.autostart" {
        sync_autostart(&app, value.as_bool().unwrap_or(false));
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

/// 캡처 주석(4.9): 오버레이 창(모니터)이 자기 주석 개수를 알린다(0 ↔ n이 바뀔 때).
#[tauri::command]
fn capture_annotations(ink: State<'_, Ink>, session: u64, monitor_id: u32, items: u32) {
    ink.lock().capture_annotations(session, monitor_id, items);
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

fn read_json_file(path: &std::path::Path) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|_| "파일을 읽을 수 없어요".to_string())?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    serde_json::from_slice(bytes).map_err(|_| format!("이 파일은 {APP_NAME} 설정이 아니에요"))
}

/// 가져오기 1단계: 파일 고르기 → 무엇이 바뀌는지(시트). 저장은 하지 않는다.
#[tauri::command]
async fn preview_import<R: Runtime>(app: AppHandle<R>) -> Result<ImportPick, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(picked) = app.dialog().file().add_filter(format!("{APP_NAME} 설정"), &["json"]).blocking_pick_file() else {
        return Ok(ImportPick { cancelled: true, path: None, preview: None });
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let preview = match read_json_file(&path) {
        Ok(file) => app.state::<Ink>().lock().preview_import(&file),
        Err(e) => ImportPreview { ok: false, error: Some(e), changes: vec![], warnings: vec![] },
    };
    Ok(ImportPick { cancelled: false, path: Some(path.to_string_lossy().into_owned()), preview: Some(preview) })
}

/// 가져오기 2단계: 시트에서 [가져오기]를 눌렀을 때.
#[tauri::command]
fn apply_import(ink: State<'_, Ink>, path: String) -> UpdateResult {
    match read_json_file(std::path::Path::new(&path)) {
        Ok(file) => ink.lock().apply_import(&file),
        Err(e) => UpdateResult { ok: false, error: Some(e), state: ink.lock().state() },
    }
}

/// UTC 날짜(내보내기 파일 이름용) — 1970-01-01부터의 날 수 → 연·월·일(Hinnant).
fn today() -> String {
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
    let Some(picked) = app.dialog().file().add_filter(format!("{APP_NAME} 설정"), &["json"]).set_file_name(&name).blocking_save_file() else {
        return Ok(());
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let body = serde_json::to_string_pretty(&app.state::<Ink>().lock().export_file()).map_err(|e| e.to_string())?;
    let ok = std::fs::write(&path, body).is_ok();
    let h = app.clone();
    app.run_on_main_thread(move || {
        let ev = if ok { IslandEvent::new(IslandKind::Result, "설정을 내보냈어요") } else { IslandEvent::new(IslandKind::Problem, "내보내지 못했어요") };
        h.state::<Ink>().lock().island(ev);
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
async fn run<R: Runtime>(app: AppHandle<R>, action: String) -> Result<(), String> {
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
        "island.hidden" => hide(ISLAND),
        "folder.open" => {
            hide(POPOVER);
            if let Some(p) = output_folder(&app) {
                open_folder(&p);
            }
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
                let n = s.clear_ranges();
                let text = if n == 0 { "지울 기록이 없어요".to_string() } else { format!("캡처 기록 {n}개를 지웠어요") };
                s.island(IslandEvent::new(IslandKind::Info, text));
            })
            .map_err(|e| e.to_string())?;
        }
        "capture.saveLast" => {
            let h = app.clone();
            app.run_on_main_thread(move || h.state::<Ink>().lock().save_last()).map_err(|e| e.to_string())?;
        }
        "capture.copyHex" => {
            let h = app.clone();
            app.run_on_main_thread(move || h.state::<Ink>().lock().copy_last_hex()).map_err(|e| e.to_string())?;
        }
        "app.quit" => {
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
        "capture.region" | "capture.window" | "record.toggle" => {
            let kind = match action.as_str() {
                "capture.region" => CaptureKind::Region,
                "capture.window" => CaptureKind::Window,
                _ => CaptureKind::Record,
            };
            let h = app.clone();
            app.run_on_main_thread(move || start_capture(&h, kind)).map_err(|e| e.to_string())?;
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

fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    app.manage(PopoverClock::default());
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
    let (dir, portable) = ink_store::resolve_dir(&exe_dir, &user_dir);
    let (screen, windows, monitors) = screen_ports();
    let ports = Ports {
        hotkeys: Box::new(TauriHotkeys { app: app.clone() }),
        probe: probe(),
        store: Box::new(ink_store::JsonFileStore::new(dir, portable)),
        notifier: Box::new(TauriNotifier { app: app.clone(), tray_mode: Mutex::new(None) }),
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
    // 개발용 사람 정답 도구: 확정할 때마다 인식 시험 사례를 쓴다(시험 문서 화면에서만, 08 §5.2)
    if let Ok(dir) = std::env::var("SCREEN_INK_CASE_DIR") {
        log::warn!("recognition cases will be written on every confirm");
        service.set_case_dir(Some(dir));
    }
    let failed = service.start();
    let first_run = service.is_first_run();
    let autostart = service.settings().bool("general.autostart");
    let initial = service.state();
    log::info!("shookshok ready: {} hotkey(s) not registered", failed);
    app.manage(Ink(Mutex::new(service)));

    // 창·트레이는 플러그인 초기화가 끝난 뒤 이벤트 루프에서 만든다. 플러그인 setup은 플러그인 목록
    // 잠금을 쥔 채 돌고, 창을 만들면 창 생성 훅이 같은 잠금을 잡아 교착된다. 다른 스레드에서
    // run_on_main_thread를 부르면 즉시 실행이 아니라 이벤트 루프 대기열로 들어간다.
    let handle = app.clone();
    std::thread::spawn(move || {
        let h = handle.clone();
        if let Err(e) = handle.run_on_main_thread(move || {
            if let Err(e) = finish_setup(&h, initial, first_run, autostart, started) {
                log::error!("shookshok setup failed: {e}");
            }
        }) {
            log::error!("shookshok setup could not be scheduled: {e}");
        }
    });
    Ok(())
}

fn finish_setup<R: Runtime>(app: &AppHandle<R>, initial: InkState, first_run: bool, autostart: bool, started: Instant) -> tauri::Result<()> {
    build_static_windows(app)?;
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
    listen_system_events(app);
    let mode = if initial.hotkeys_enabled { TrayMode::Ready } else { TrayMode::Off };
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_image(mode))
        .tooltip(tooltip(&initial))
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
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
            run,
            preview_import,
            apply_import,
            reset_all,
            overlay_ready,
            capture_input,
            pin_info,
            pin_action,
            pin_zoom,
            capture_annotations,
            capture_commit_annotated
        ])
        .register_uri_scheme_protocol(overlay::SCHEME, |ctx, req| overlay::serve(ctx, req))
        .setup(|app, _api| setup(app))
        .on_event(|_app, event| {
            // 트레이 상주 앱: 마지막 창을 닫아도 끝나지 않는다(끝내기는 팝오버 [끝내기] = exit(0))
            if let RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        })
        .build()
}
