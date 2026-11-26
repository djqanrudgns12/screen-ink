//! 모니터별 오버레이 창(`ink-overlay-<id>`)과 정지 이미지 프로토콜(`ink://frame/<세션>/<모니터>`).
//!
//! - 창은 미리 만들어 숨겨 둔다(R-CAP-1). 만들기는 언제나 이벤트 루프 대기열로 미룬다 — 플러그인
//!   초기화·동기 명령 안에서 창을 만들면 교착된다.
//! - 정지 이미지는 base64로 보내지 않는다. 메모리의 BMP를 프로토콜이 바로 내주고, 모드가 끝나면 버린다(03 §4).
//! - 보일 때마다 웹뷰 배경 투명을 다시 지정한다(R-SYS-6).

use ink_app::{Frame, OverlayHost, OverlaySession};
use ink_domain::geometry::{MonitorLayout, PhysRect};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::http::{Request, Response};
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, UriSchemeContext, WebviewUrl, WebviewWindowBuilder};

pub const SCHEME: &str = "ink";
pub const EVT_OVERLAY: &str = "screen-ink://overlay";
const PREFIX: &str = "ink-overlay-";

pub fn label(monitor: u32) -> String {
    format!("{PREFIX}{monitor}")
}

/// 오버레이 창 이름 → 모니터 id(`label`의 반대). 오버레이 창이 아니면 None.
/// 지금은 맥 입력 경로만 쓴다(순수 함수라 시험은 모든 OS에서 돈다).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn monitor_of(label: &str) -> Option<u32> {
    label.strip_prefix(PREFIX)?.parse().ok()
}

/// 오버레이가 보낸 포인터 점이 자기 모니터 밖이면 `outside()`(실제 커서 등)로 바꾼다.
/// 맥은 누른 모니터의 웹뷰가 창 밖 이동까지 받고 자기 배율로 환산하므로, 배율이 다른 옆 모니터에서는 점이 어긋난다.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn pointer_inside(input: ink_app::CaptureInput, monitor: PhysRect, outside: impl FnOnce() -> ink_domain::geometry::PhysPoint) -> ink_app::CaptureInput {
    use ink_app::CaptureInput as I;
    let (x, y) = match &input {
        I::Move { x, y } | I::Down { x, y, .. } | I::Up { x, y, .. } | I::Dblclick { x, y } => (*x, *y),
        _ => return input,
    };
    if monitor.contains(ink_domain::geometry::PhysPoint { x, y }) {
        return input;
    }
    let p = outside();
    match input {
        I::Move { .. } => I::Move { x: p.x, y: p.y },
        I::Down { button, .. } => I::Down { x: p.x, y: p.y, button },
        I::Up { button, .. } => I::Up { x: p.x, y: p.y, button },
        I::Dblclick { .. } => I::Dblclick { x: p.x, y: p.y },
        other => other,
    }
}

/// 웹뷰에서 쓸 이미지 주소(Windows WebView2는 `http://<scheme>.localhost/`로 사용자 프로토콜을 연다).
pub fn frame_url(session: u64, monitor: u32) -> String {
    if cfg!(windows) || cfg!(target_os = "android") {
        format!("http://{SCHEME}.localhost/frame/{session}/{monitor}")
    } else {
        format!("{SCHEME}://localhost/frame/{session}/{monitor}")
    }
}

type FrameMap = HashMap<(u64, u32), Frame>;

/// (세션, 모니터) → 정지 이미지(BGRA). 지금 세션 것만 둔다. 픽셀은 `Arc`라 복사 없이 함께 쓴다.
#[derive(Default, Clone)]
pub struct FrameStore(Arc<Mutex<FrameMap>>);

impl FrameStore {
    fn put(&self, session: u64, frames: &[Frame]) {
        let mut m = self.0.lock().unwrap_or_else(|e| e.into_inner());
        m.clear();
        for f in frames {
            m.insert((session, f.monitor_id), f.clone());
        }
    }

    fn drop_session(&self, session: u64) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).retain(|(s, _), _| *s != session);
    }

    pub fn get(&self, session: u64, monitor: u32) -> Option<Frame> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).get(&(session, monitor)).cloned()
    }
}

/// `ink://` 프로토콜 처리기.
pub fn serve<R: Runtime>(ctx: UriSchemeContext<'_, R>, req: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let not_found = || Response::builder().status(404).body(Vec::new()).unwrap();
    let path = req.uri().path().trim_start_matches('/');
    let mut parts = path.split('/');
    let first = parts.next();
    if first == Some("thumb") {
        let Some(id) = parts.next().and_then(|s| s.parse::<u64>().ok()) else { return not_found() };
        let Some(bmp) = ctx.app_handle().try_state::<crate::adapters::ThumbStore>().and_then(|t| t.bmp(id)) else { return not_found() };
        return Response::builder().header("Content-Type", "image/bmp").header("Cache-Control", "no-store").header("Access-Control-Allow-Origin", "*").body(bmp).unwrap_or_else(|_| not_found());
    }
    if first == Some("pin") {
        let Some(id) = parts.next().and_then(|s| s.parse::<u64>().ok()) else { return not_found() };
        let Some(p) = ctx.app_handle().try_state::<crate::pins::PinStore>().and_then(|s| s.get(id)) else { return not_found() };
        return Response::builder()
            .header("Content-Type", "image/bmp")
            .header("Cache-Control", "no-store")
            .header("Access-Control-Allow-Origin", "*")
            .body(ink_domain::bmp::encode(p.w, p.h, &p.bgra))
            .unwrap_or_else(|_| not_found());
    }
    let (Some("frame"), Some(s), Some(m)) = (first, parts.next(), parts.next()) else { return not_found() };
    let (Ok(session), Ok(monitor)) = (s.parse::<u64>(), m.parse::<u32>()) else { return not_found() };
    let Some(store) = ctx.app_handle().try_state::<FrameStore>() else { return not_found() };
    let Some(f) = store.get(session, monitor) else { return not_found() };
    Response::builder()
        .header("Content-Type", "image/bmp")
        .header("Cache-Control", "no-store")
        // 확대경이 캔버스로 픽셀을 읽을 수 있게(crossorigin="anonymous")
        .header("Access-Control-Allow-Origin", "*")
        .body(ink_domain::bmp::encode(f.width, f.height, &f.bgra))
        .unwrap_or_else(|_| not_found())
}

/// 배치에 맞춰 오버레이 창을 만들고(없으면)·옮기고·남는 창을 닫는다. 메인 스레드 대기열에서만 부른다.
pub fn ensure_overlays<R: Runtime>(app: &AppHandle<R>, layout: &MonitorLayout) {
    #[cfg(target_os = "macos")]
    let space = ink_platform_macos::space();
    for m in &layout.monitors {
        let l = label(m.id);
        let w = match app.get_webview_window(&l) {
            Some(w) => w,
            None => match WebviewWindowBuilder::new(app, &l, WebviewUrl::App(format!("index.html?view=overlay&monitor={}", m.id).into()))
                .title("screen-ink overlay")
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .focused(false)
                .build()
            {
                Ok(w) => {
                    // 웹뷰 표시 상태를 한 번 켜 두고 창만 숨긴다(이후 보이기·숨기기는 Win32로, 렌더러는 깨어 있게).
                    // 내용이 투명하므로 이 순간에도 화면에는 아무것도 보이지 않는다.
                    let _ = w.show();
                    let _ = show_raw(&w, false);
                    w
                }
                Err(e) => {
                    log::error!("overlay window {l}: {e}");
                    continue;
                }
            },
        };
        // 맥: 계약 물리 px → 그 화면 포인트로 직접 놓는다(혼합 배율 배치는 Tauri 물리 좌표와 다르다).
        // 메뉴 막대·Dock 위 층, 모든 Space·전체 화면 앱 위
        #[cfg(target_os = "macos")]
        let placed = w.ns_window().is_ok_and(|ns| {
            ink_platform_macos::window::configure(ns, ink_platform_macos::window::Role::Overlay);
            ink_platform_macos::window::place(ns, &space, m.bounds)
        });
        #[cfg(not(target_os = "macos"))]
        let placed = false;
        if !placed {
            // 위치 먼저(그 모니터 배율을 받게) → 크기
            let _ = w.set_position(PhysicalPosition::new(m.bounds.x, m.bounds.y));
            let _ = w.set_size(PhysicalSize::new(m.bounds.w as u32, m.bounds.h as u32));
        }
        let _ = w.set_background_color(Some(Color(0, 0, 0, 0)));
    }
    #[cfg(target_os = "macos")]
    ink_platform_macos::freeze::prepare(&layout.monitors.iter().map(|m| m.id).collect::<Vec<_>>());
    let keep: Vec<String> = layout.monitors.iter().map(|m| label(m.id)).collect();
    for (l, w) in app.webview_windows() {
        if l.starts_with(PREFIX) && !keep.contains(&l) {
            let _ = w.destroy();
        }
    }
}

/// 다른 스레드에서 메인 스레드 대기열로(즉시 실행이 아니라 이벤트 루프가 돈 뒤에).
pub fn defer<R: Runtime>(app: &AppHandle<R>, f: impl FnOnce(&AppHandle<R>) + Send + 'static) {
    let h = app.clone();
    std::thread::spawn(move || {
        let inner = h.clone();
        if let Err(e) = h.run_on_main_thread(move || f(&inner)) {
            log::error!("defer to main thread failed: {e}");
        }
    });
}

pub struct TauriOverlayHost<R: Runtime> {
    pub app: AppHandle<R>,
    pub frames: FrameStore,
    #[cfg(windows)]
    pub freeze: ink_platform_windows::freeze::Freeze,
    /// 지금 세션의 모니터 배치(강조를 모니터별로 나눌 때)
    layout: Option<MonitorLayout>,
    thumbs: crate::adapters::ThumbStore,
}

/// 썸네일 주소(`frame_url`과 같은 규칙).
pub fn thumb_url(id: u64) -> String {
    if cfg!(windows) || cfg!(target_os = "android") {
        format!("http://{SCHEME}.localhost/thumb/{id}")
    } else {
        format!("{SCHEME}://localhost/thumb/{id}")
    }
}

impl<R: Runtime> TauriOverlayHost<R> {
    pub fn new(app: AppHandle<R>, frames: FrameStore, thumbs: crate::adapters::ThumbStore) -> Self {
        Self {
            app,
            frames,
            #[cfg(windows)]
            freeze: Default::default(),
            layout: None,
            thumbs,
        }
    }

    /// 정지 화면을 네이티브 창에 바로 그린다(Windows GDI·맥 CALayer). 모든 모니터에 그렸으면 true.
    fn paint_native(&mut self, session: &OverlaySession, frames: &[Frame]) -> bool {
        #[cfg(windows)]
        {
            let mut all = !frames.is_empty();
            for f in frames {
                let Some(m) = session.monitors.iter().find(|m| m.id == f.monitor_id) else { continue };
                let local = session.highlight.and_then(|r| r.intersect(&m.bounds)).map(|r| PhysRect::new(r.x - m.bounds.x, r.y - m.bounds.y, r.w, r.h));
                all &= self.freeze.show(m.id, m.bounds, f.bgra.clone(), f.width, f.height, local);
            }
            if !all {
                self.freeze.hide_all();
            }
            all
        }
        #[cfg(target_os = "macos")]
        {
            use ink_platform_macos::freeze;
            let space = ink_platform_macos::space();
            let mut all = !frames.is_empty();
            for f in frames {
                let Some(m) = session.monitors.iter().find(|m| m.id == f.monitor_id) else { continue };
                let local = session.highlight.and_then(|r| r.intersect(&m.bounds)).map(|r| PhysRect::new(r.x - m.bounds.x, r.y - m.bounds.y, r.w, r.h));
                all &= freeze::show(m.id, &space, m.bounds, f.width, f.height, &f.bgra, local);
            }
            if !all {
                freeze::hide_all();
            }
            all
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            let _ = (session, frames);
            false
        }
    }
}

/// 맥: 정지 화면 창은 메인 스레드에서만 만진다 — 다른 스레드면 메인 대기열로 넘긴다.
#[cfg(target_os = "macos")]
fn on_main<R: Runtime>(app: &AppHandle<R>, f: impl FnOnce() + Send + 'static) {
    if ink_platform_macos::freeze::on_main_thread() {
        f();
    } else if let Err(e) = app.run_on_main_thread(f) {
        log::error!("freeze to main thread failed: {e}");
    }
}

impl<R: Runtime> OverlayHost for TauriOverlayHost<R> {
    fn prepare(&mut self, layout: &MonitorLayout) {
        #[cfg(windows)]
        self.freeze.retain(&layout.monitors.iter().map(|m| m.id).collect::<Vec<_>>());
        let layout = layout.clone();
        defer(&self.app, move |app| ensure_overlays(app, &layout));
    }

    fn thumbnail(&mut self, w: u32, h: u32, bgra: Vec<u8>) -> Option<String> {
        (w > 0 && h > 0).then(|| thumb_url(self.thumbs.put(w, h, bgra)))
    }

    fn highlight(&mut self, session: u64, rect: Option<PhysRect>) {
        let _ = session;
        #[cfg(windows)]
        if let Some(layout) = self.layout.clone() {
            for m in &layout.monitors {
                let local = rect.and_then(|r| r.intersect(&m.bounds)).map(|r| PhysRect::new(r.x - m.bounds.x, r.y - m.bounds.y, r.w, r.h));
                self.freeze.set_highlight(m.id, local);
            }
        }
        #[cfg(target_os = "macos")]
        if let Some(layout) = self.layout.clone() {
            on_main(&self.app, move || {
                for m in &layout.monitors {
                    let local = rect.and_then(|r| r.intersect(&m.bounds)).map(|r| PhysRect::new(r.x - m.bounds.x, r.y - m.bounds.y, r.w, r.h));
                    ink_platform_macos::freeze::set_highlight(m.id, local);
                }
            });
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        let _ = rect;
    }

    fn show(&mut self, session: &OverlaySession, frames: Vec<Frame>) -> Result<Option<std::time::Instant>, String> {
        let windows: Vec<_> = session.monitors.iter().map(|m| (m, self.app.get_webview_window(&label(m.id)))).collect();
        if windows.iter().any(|(_, w)| w.is_none()) {
            // 아직 창을 만드는 중(시작 직후·모니터를 막 꽂음) — 다음 진입부터 된다
            self.prepare(&MonitorLayout::new(session.monitors.iter().map(|m| ink_domain::geometry::Monitor { id: m.id, bounds: m.bounds, work: m.work, scale: m.scale, primary: m.primary }).collect()));
            return Err("overlay windows not ready".into());
        }
        self.frames.put(session.session, &frames);
        self.layout = Some(MonitorLayout::new(
            session.monitors.iter().map(|m| ink_domain::geometry::Monitor { id: m.id, bounds: m.bounds, work: m.work, scale: m.scale, primary: m.primary }).collect(),
        ));
        // 1) 정지 화면(어둡게 + 강조만 밝게)을 네이티브 창에 바로 그린다 — 눈에 보이는 첫 변화(진입 시간의 끝점)
        let native = self.paint_native(session, &frames);
        let painted_at = native.then(std::time::Instant::now);
        let mut payload = session.clone();
        payload.native_frame = native;
        for m in &mut payload.monitors {
            m.frame_url = Some(frame_url(session.session, m.id));
        }
        // 2) 웹뷰 오버레이(투명: 어둡게·선택·확대경)를 그 위에. 위치·크기·투명 배경은 prepare·hide에서 미리 맞춰 둠
        self.app.emit(EVT_OVERLAY, &payload).map_err(|e| e.to_string())?;
        let cursor = session.cursor;
        for (_, w) in &windows {
            show_raw(w.as_ref().expect("checked above"), true).map_err(|e| e.to_string())?;
        }
        // 키(Esc·Enter)는 커서가 있는 모니터 창이 받는다
        let focus = session.monitors.iter().find(|m| m.bounds.contains(cursor)).or(session.monitors.first());
        if let Some(m) = focus {
            if let Some(w) = self.app.get_webview_window(&label(m.id)) {
                focus_raw(&w);
            }
        }
        Ok(painted_at)
    }

    fn hide(&mut self, session: u64) {
        #[cfg(windows)]
        self.freeze.hide_all();
        #[cfg(target_os = "macos")]
        on_main(&self.app, ink_platform_macos::freeze::hide_all);
        for (l, w) in self.app.webview_windows() {
            if l.starts_with(PREFIX) {
                let _ = show_raw(&w, false);
                // R-SYS-6: 투명 배경 다시 지정은 숨긴 뒤에(다음 진입의 시간을 쓰지 않게)
                let _ = w.set_background_color(Some(Color(0, 0, 0, 0)));
            }
        }
        self.frames.drop_session(session);
        // 맥: 설정 창을 쓰는 중이 아니면 포커스를 원래 앱으로(전체 화면 앱에서 키가 계속 먹게)
        #[cfg(target_os = "macos")]
        if !self.app.get_webview_window(crate::SETTINGS).is_some_and(|w| w.is_visible().unwrap_or(false)) {
            ink_platform_macos::window::deactivate();
        }
    }
}

/// Windows: 창만 보이기·숨기기(웹뷰 렌더러는 깨어 있게). 다른 곳: Tauri 기본.
fn show_raw<R: Runtime>(w: &tauri::WebviewWindow<R>, show: bool) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        let h = w.hwnd()?;
        ink_platform_windows::events::set_window_shown(h.0 as isize, show);
        Ok(())
    }
    // 맥: Tauri show()는 맨 앞으로 올리기를 나중으로 미룬다 → 그 전에 활성화하면 우리 창이 있던 다른 Space로
    // 넘어간다. 지금 Space에 바로 올린 뒤 Tauri 상태를 맞춘다.
    #[cfg(target_os = "macos")]
    if show {
        if let Ok(ns) = w.ns_window() {
            ink_platform_macos::window::order_front(ns);
        }
    }
    #[cfg(not(windows))]
    {
        if show {
            w.show()
        } else {
            w.hide()
        }
    }
}

fn focus_raw<R: Runtime>(w: &tauri::WebviewWindow<R>) {
    #[cfg(windows)]
    if let Ok(h) = w.hwnd() {
        ink_platform_windows::events::focus_window(h.0 as isize);
        return;
    }
    #[cfg(target_os = "macos")]
    if let Ok(ns) = w.ns_window() {
        ink_platform_macos::window::make_key(ns);
        return;
    }
    let _ = w.set_focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 자기_모니터_밖_점만_실제_커서로() {
        use ink_app::CaptureInput as I;
        use ink_domain::geometry::PhysPoint;
        let mon = PhysRect::new(0, 0, 3840, 2160);
        let cursor = || PhysPoint { x: 3940, y: 300 };
        // 모니터 안: 그대로(커서를 읽지 않는다)
        let kept = pointer_inside(I::Down { x: 3000, y: 1000, button: 0 }, mon, || panic!("안쪽 점은 커서를 읽지 않는다"));
        assert!(matches!(kept, I::Down { x: 3000, y: 1000, button: 0 }));
        // 4K(2배) 웹뷰가 옆 FHD(1배) 점을 자기 배율로 환산한 값 → 실제 커서
        assert!(matches!(pointer_inside(I::Move { x: 4040, y: 600 }, mon, cursor), I::Move { x: 3940, y: 300 }));
        assert!(matches!(pointer_inside(I::Up { x: 4040, y: 600, button: 0 }, mon, cursor), I::Up { x: 3940, y: 300, button: 0 }));
        // 점이 없는 입력은 그대로
        assert!(matches!(pointer_inside(I::Wheel { dy: 1 }, mon, || panic!("점 없음")), I::Wheel { dy: 1 }));
        assert_eq!(monitor_of(&label(1)), Some(1));
        assert_eq!(monitor_of("ink-island"), None);
    }

    #[test]
    fn 이미지_주소와_창_이름() {
        assert_eq!(label(1), "ink-overlay-1");
        let u = frame_url(7, 1);
        assert!(u.ends_with("/frame/7/1"));
        if cfg!(windows) {
            assert_eq!(u, "http://ink.localhost/frame/7/1");
        }
    }

    #[test]
    fn 프레임은_지금_세션만_쥔다() {
        let s = FrameStore::default();
        s.put(1, &[Frame { monitor_id: 0, width: 1, height: 1, bgra: Arc::new(vec![1]) }]);
        s.put(2, &[Frame { monitor_id: 0, width: 1, height: 1, bgra: Arc::new(vec![2]) }]);
        assert!(s.get(1, 0).is_none());
        assert_eq!(s.get(2, 0).unwrap().bgra.as_slice(), &[2]);
        s.drop_session(2);
        assert!(s.get(2, 0).is_none());
    }
}
