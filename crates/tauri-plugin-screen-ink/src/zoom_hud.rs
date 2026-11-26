//! 실시간 확대 HUD(5.5, R-ZOOM-4·6)와 돋보기 묶음.
//!
//! - 창 `ink-zoom-hud`: 클릭이 통과하는 투명 창(커서 주황 후광 44 px + 오른쪽 아래 위치 지도, 04 S13).
//!   미리 만들어 숨겨 두고 확대할 때 그 모니터에 맞춰 보인다. 캡처·녹화에서 빠지고(WDA_EXCLUDEFROMCAPTURE)
//!   돋보기 원본에서도 빠진다(필터 목록 — 확대 화면 안에 HUD·섬이 2중으로 찍히지 않게).
//! - 서비스에는 `Magnifier` 하나로 보인다: 시작 = 제외 목록 → 돋보기 → HUD를 그 위로 → 60 Hz 틱, 끝 = 반대 순서.
//! - 틱은 메인 스레드에서 서비스를 부른다(서비스는 메인 스레드에서만 만진다). 지난 틱이 아직이면 건너뛴다.
//!   멈출 때 틱 스레드를 기다리지 않는다 — 서비스 잠금을 쥔 채 기다리면 교착한다(세대 번호로 스스로 끝남).
//! - 맥(M13)은 `MacMagnifier`(SCStream → CALayer)에 같은 HUD·틱을 붙인다. 그 밖은 확대가 없다(`NoMagnifier`, HUD 창도 만들지 않음).

use ink_app::LiveZoomState;
use tauri::{AppHandle, Emitter, Runtime};

pub const LABEL: &str = "ink-zoom-hud";
pub const EVT_ZOOM: &str = "screen-ink://zoom";

/// HUD 상태를 HUD 창에만(60 Hz라 모든 창에 뿌리지 않는다). `None` = 확대 끝.
pub fn emit<R: Runtime>(app: &AppHandle<R>, state: Option<&LiveZoomState>) {
    let _ = app.emit_to(LABEL, EVT_ZOOM, state);
}

/// HUD 창을 미리 만든다(이벤트 루프 대기열에서 — 플러그인 초기화·명령 안에서 창을 만들면 교착).
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        let w = tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("index.html?view=zoom-hud".into()))
            .title("screen-ink zoom")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focusable(false)
            .focused(false)
            .visible(false)
            .build()?;
        // 클릭·휠은 아래 앱으로(실시간 확대 중에도 원본 앱을 그대로 쓴다)
        let _ = w.set_ignore_cursor_events(true);
        let _ = w.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
        // 웹뷰 표시 상태를 한 번 켜 두고 창만 숨긴다(오버레이와 같은 방식 — 보일 때 렌더러가 이미 돌고 있게).
        // 내용이 투명하고 클릭이 통과하므로 이 순간에도 화면에는 아무것도 보이지 않는다.
        let _ = w.show();
        if let Ok(h) = w.hwnd() {
            ink_platform_windows::events::set_window_shown(h.0 as isize, false);
        }
        crate::exclude_from_capture(app, LABEL);
    }
    // 맥: 같은 HUD 창 + 오버레이 층(돋보기 패널 바로 위)·모든 Space. 한 번 보였다 숨겨 웹뷰가 미리 돌게
    #[cfg(target_os = "macos")]
    {
        let w = tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("index.html?view=zoom-hud".into()))
            .title("screen-ink zoom")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focusable(false)
            .focused(false)
            .visible(false)
            .build()?;
        let _ = w.set_ignore_cursor_events(true);
        let _ = w.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
        if let Ok(ns) = w.ns_window() {
            ink_platform_macos::window::configure(ns, ink_platform_macos::window::Role::Overlay);
        }
        crate::exclude_from_capture(app, LABEL);
        let _ = w.show();
        let _ = w.hide();
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let _ = app;
    Ok(())
}

/// 서비스에 꽂을 돋보기(`InkService::set_magnifier`).
pub fn magnifier<R: Runtime>(app: &AppHandle<R>) -> Box<dyn ink_app::Magnifier> {
    #[cfg(windows)]
    {
        Box::new(win::HudMagnifier::new(app))
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(mac::HudMagnifier::new(app))
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = app;
        Box::new(ink_app::NoMagnifier)
    }
}

#[cfg(windows)]
mod win {
    use super::LABEL;
    use crate::{Ink, ISLAND, POPOVER};
    use ink_app::Magnifier;
    use ink_domain::geometry::PhysRect;
    use ink_platform_windows::events::set_window_shown;
    use ink_platform_windows::magnifier::WinMagnifier;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime};

    /// 서비스 틱 간격(≈60 Hz). 돋보기 다시 그리기는 돋보기 스레드가 따로 한다.
    const TICK: Duration = Duration::from_millis(16);

    pub struct HudMagnifier<R: Runtime> {
        app: AppHandle<R>,
        inner: WinMagnifier,
        /// 틱 세대 — 바뀌면 이전 틱 스레드는 스스로 끝난다
        gen: Arc<AtomicU64>,
    }

    impl<R: Runtime> HudMagnifier<R> {
        pub fn new(app: &AppHandle<R>) -> Self {
            let h = app.clone();
            // 훅 스레드: 잠금 없이 메인 스레드 대기열로만 넘긴다(돋보기를 멈출 때 그 스레드를 기다리므로)
            let mut inner = WinMagnifier::new(move |dy| {
                let h2 = h.clone();
                let _ = h.run_on_main_thread(move || {
                    if let Some(ink) = h2.try_state::<Ink>() {
                        ink.lock().zoom_input(ink_app::ZoomInput::Wheel { dy });
                    }
                });
            });
            // 첫 WC_MAGNIFIER 생성 ≈ 0.9 s(REPORT §3) — 앱 시작 때 숨겨서 미리 만든다(기다리지 않음)
            inner.prewarm();
            HudMagnifier { app: app.clone(), inner, gen: Arc::new(AtomicU64::new(0)) }
        }

        fn hwnd(&self, label: &str) -> Option<isize> {
            self.app.get_webview_window(label).and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize)
        }

        fn spawn_ticks(&self) {
            let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
            let (g, app) = (self.gen.clone(), self.app.clone());
            let pending = Arc::new(AtomicBool::new(false));
            let spawned = std::thread::Builder::new().name("ink-zoom-tick".into()).spawn(move || {
                let mut next = Instant::now();
                loop {
                    next += TICK;
                    let now = Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else {
                        next = now;
                    }
                    if g.load(Ordering::SeqCst) != gen {
                        break;
                    }
                    // 메인 스레드가 지난 틱을 아직 못 돌렸으면 쌓지 않는다
                    if pending.swap(true, Ordering::SeqCst) {
                        continue;
                    }
                    let (h, p, g2) = (app.clone(), pending.clone(), g.clone());
                    let posted = app.run_on_main_thread(move || {
                        p.store(false, Ordering::SeqCst);
                        if g2.load(Ordering::SeqCst) != gen {
                            return;
                        }
                        let alive = h.try_state::<Ink>().is_some_and(|ink| ink.lock().tick_zoom());
                        if !alive {
                            let _ = g2.compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst);
                        }
                    });
                    if posted.is_err() {
                        break;
                    }
                }
            });
            if let Err(e) = spawned {
                log::warn!("zoom tick thread: {e}");
            }
        }

        fn show_hud(&self, monitor: PhysRect) {
            let Some(w) = self.app.get_webview_window(LABEL) else { return };
            // 위치 먼저(그 모니터 배율을 받게) → 크기
            let _ = w.set_position(PhysicalPosition::new(monitor.x, monitor.y));
            let _ = w.set_size(PhysicalSize::new(monitor.w.max(1) as u32, monitor.h.max(1) as u32));
            if let Ok(h) = w.hwnd() {
                // 맨 위 무리의 맨 앞 = 돋보기 호스트 위(섬은 그 뒤에 섬 알림이 다시 올린다)
                set_window_shown(h.0 as isize, true);
            }
        }

        fn hide_hud(&self) {
            if let Some(h) = self.hwnd(LABEL) {
                set_window_shown(h, false);
            }
        }
    }

    impl<R: Runtime> Magnifier for HudMagnifier<R> {
        fn start(&mut self, monitor: PhysRect, source: PhysRect, scale: f64) -> Result<(), String> {
            let exclude: Vec<isize> = [LABEL, ISLAND, POPOVER].into_iter().filter_map(|l| self.hwnd(l)).collect();
            self.inner.set_exclude(exclude);
            self.inner.start(monitor, source, scale)?;
            self.show_hud(monitor);
            self.spawn_ticks();
            Ok(())
        }

        fn update(&mut self, source: PhysRect, scale: f64) -> Result<(), String> {
            self.inner.update(source, scale)
        }

        fn stop(&mut self) {
            self.gen.fetch_add(1, Ordering::SeqCst);
            self.inner.stop();
            self.hide_hud();
        }
    }
}

/// 맥(M13): Windows `win::HudMagnifier`와 같은 틀 — 돋보기 → HUD를 그 위로 → 60 Hz 틱, 끝 = 반대 순서.
/// 틱 스레드는 Windows 것을 그대로 옮겼다(Windows 블록은 건드리지 않는다).
#[cfg(target_os = "macos")]
mod mac {
    use super::LABEL;
    use crate::Ink;
    use ink_app::Magnifier;
    use ink_domain::geometry::PhysRect;
    use ink_platform_macos::magnifier::MacMagnifier;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tauri::{AppHandle, Manager, Runtime};

    /// 서비스 틱 간격(≈60 Hz). 프레임 붙이기는 SCK가 따로 한다.
    const TICK: Duration = Duration::from_millis(16);

    pub struct HudMagnifier<R: Runtime> {
        app: AppHandle<R>,
        inner: MacMagnifier,
        /// 틱 세대 — 바뀌면 이전 틱 스레드는 스스로 끝난다
        gen: Arc<AtomicU64>,
    }

    impl<R: Runtime> HudMagnifier<R> {
        pub fn new(app: &AppHandle<R>) -> Self {
            let h = app.clone();
            // 가로채기 스레드(휠·트랙패드·Esc): 잠금 없이 메인 스레드 대기열로만 넘긴다(돋보기를 멈출 때 그 스레드를 기다리므로)
            let inner = MacMagnifier::new(move |input| {
                let h2 = h.clone();
                let _ = h.run_on_main_thread(move || {
                    if let Some(ink) = h2.try_state::<Ink>() {
                        ink.lock().zoom_input(input);
                    }
                });
            });
            HudMagnifier { app: app.clone(), inner, gen: Arc::new(AtomicU64::new(0)) }
        }

        fn spawn_ticks(&self) {
            let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
            let (g, app) = (self.gen.clone(), self.app.clone());
            let pending = Arc::new(AtomicBool::new(false));
            let spawned = std::thread::Builder::new().name("ink-zoom-tick".into()).spawn(move || {
                let mut next = Instant::now();
                loop {
                    next += TICK;
                    let now = Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else {
                        next = now;
                    }
                    if g.load(Ordering::SeqCst) != gen {
                        break;
                    }
                    // 메인 스레드가 지난 틱을 아직 못 돌렸으면 쌓지 않는다
                    if pending.swap(true, Ordering::SeqCst) {
                        continue;
                    }
                    let (h, p, g2) = (app.clone(), pending.clone(), g.clone());
                    let posted = app.run_on_main_thread(move || {
                        p.store(false, Ordering::SeqCst);
                        if g2.load(Ordering::SeqCst) != gen {
                            return;
                        }
                        let alive = h.try_state::<Ink>().is_some_and(|ink| ink.lock().tick_zoom());
                        if !alive {
                            let _ = g2.compare_exchange(gen, gen + 1, Ordering::SeqCst, Ordering::SeqCst);
                        }
                    });
                    if posted.is_err() {
                        break;
                    }
                }
            });
            if let Err(e) = spawned {
                log::warn!("zoom tick thread: {e}");
            }
        }

        fn show_hud(&self, monitor: PhysRect) {
            let Some(w) = self.app.get_webview_window(LABEL) else { return };
            if let Ok(ns) = w.ns_window() {
                // 혼합 배율에서 Tauri 물리 좌표는 어긋난다 — 그 화면 포인트로 직접 놓고 지금 Space 맨 앞으로
                ink_platform_macos::window::place(ns, &ink_platform_macos::space(), monitor);
                ink_platform_macos::window::order_front(ns);
            }
            let _ = w.show();
        }

        fn hide_hud(&self) {
            if let Some(w) = self.app.get_webview_window(LABEL) {
                let _ = w.hide();
            }
        }
    }

    impl<R: Runtime> Magnifier for HudMagnifier<R> {
        fn start(&mut self, monitor: PhysRect, source: PhysRect, scale: f64) -> Result<(), String> {
            self.inner.start(monitor, source, scale)?;
            self.show_hud(monitor);
            self.spawn_ticks();
            Ok(())
        }

        fn update(&mut self, source: PhysRect, scale: f64) -> Result<(), String> {
            self.inner.update(source, scale)
        }

        fn stop(&mut self) {
            self.gen.fetch_add(1, Ordering::SeqCst);
            self.inner.stop();
            self.hide_hud();
        }

        fn wait_hidden(&mut self) {
            self.inner.wait_hidden();
        }
    }
}
