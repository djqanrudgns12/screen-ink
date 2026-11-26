//! 모니터별 오버레이 창(`ink-overlay-<id>`)과 정지 이미지 프로토콜(`ink://frame/<세션>/<모니터>`).
//!
//! - 창은 미리 만들어 숨겨 둔다(R-CAP-1). 만들기는 언제나 이벤트 루프 대기열로 미룬다 — 플러그인
//!   초기화·동기 명령 안에서 창을 만들면 교착된다.
//! - 정지 이미지는 base64로 보내지 않는다. 메모리의 BMP를 프로토콜이 바로 내주고, 모드가 끝나면 버린다(03 §4).
//! - 보일 때마다 웹뷰 배경 투명을 다시 지정한다(R-SYS-6).

use ink_app::{Frame, OverlayHost, OverlaySession};
use ink_domain::geometry::{MonitorLayout, PhysRect};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::http::{Request, Response};
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, UriSchemeContext, WebviewUrl, WebviewWindowBuilder};

pub const SCHEME: &str = "ink";
pub const EVT_OVERLAY: &str = "screen-ink://overlay";
pub const EVT_AFTERIMAGE: &str = "screen-ink://afterimage";
/// 캡처 주석(4.9): 주석 층을 보내 달라는 부탁(오버레이 → `capture_commit_annotated`로 답).
pub const EVT_CAPTURE_COMMIT: &str = "screen-ink://capture-commit";
/// 확정 직후 고른 자리 주황 잔상 지속 시간(R-CAP-26 · 10 §6).
pub const AFTERIMAGE_DURATION_MS: u64 = 400;
const PREFIX: &str = "ink-overlay-";

/// 잔상 알림 — 창마다 그 창 CSS 사각형을 담아 `emit_to`로 보낸다. 오버레이 화면(`CaptureOverlay.svelte`)이
/// `SelectionFrame state="afterimage"`로 그리고, `prefers-reduced-motion: reduce`면 화면 쪽에서 건너뛴다.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterimagePayload {
    pub session: u64,
    pub rect: PhysRect,
    pub duration_ms: u64,
    /// 이 알림을 받는 오버레이 창의 모니터
    pub monitor_id: u32,
    /// 그 창 기준 CSS 사각형
    pub css: CssRect,
    /// 세대(늦게 끝난 타이머가 새 잔상을 지우지 않게)
    pub token: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct CssRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// 확정 사각형(물리 px)이 겹치는 모니터마다 오버레이 창 안 CSS 좌표를 구한다.
pub fn afterimage_targets(rect: PhysRect, layout: &MonitorLayout) -> Vec<(u32, CssRect)> {
    if rect.w <= 0 || rect.h <= 0 {
        return Vec::new();
    }
    layout
        .monitors
        .iter()
        .filter_map(|m| {
            rect.intersect(&m.bounds)?;
            let scale = if m.scale.is_finite() && m.scale > 0.0 { m.scale } else { 1.0 };
            Some((
                m.id,
                CssRect {
                    x: f64::from(rect.x - m.bounds.x) / scale,
                    y: f64::from(rect.y - m.bounds.y) / scale,
                    w: f64::from(rect.w) / scale,
                    h: f64::from(rect.h) / scale,
                },
            ))
        })
        .collect()
}

/// 잔상 대상 창마다 보낼 알림(창 이름, 알림). 순수 함수 — 시험은 모든 OS에서 돈다.
pub fn afterimage_payloads(session: u64, rect: PhysRect, layout: &MonitorLayout, token: u64) -> Vec<(String, AfterimagePayload)> {
    afterimage_targets(rect, layout)
        .into_iter()
        .map(|(monitor_id, css)| (label(monitor_id), AfterimagePayload { session, rect, duration_ms: AFTERIMAGE_DURATION_MS, monitor_id, css, token }))
        .collect()
}

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
                .title("ShookShok overlay")
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
    /// 잔상 400 ms 타이머 세대 — 새 세션 시작·숨기기가 오면 이전 타이머는 무시
    afterimage_gen: Arc<AtomicU64>,
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
            afterimage_gen: Arc::new(AtomicU64::new(0)),
        }
    }

    fn bump_afterimage_gen(&self) -> u64 {
        self.afterimage_gen.fetch_add(1, Ordering::SeqCst) + 1
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
        // 진행 중인 0.4초 잔상 타이머를 즉시 취소하고 클릭 통과를 푼다
        self.bump_afterimage_gen();
        let windows: Vec<_> = session.monitors.iter().map(|m| (m, self.app.get_webview_window(&label(m.id)))).collect();
        if windows.iter().any(|(_, w)| w.is_none()) {
            // 아직 창을 만드는 중(시작 직후·모니터를 막 꽂음) — 다음 진입부터 된다
            self.prepare(&MonitorLayout::new(session.monitors.iter().map(|m| ink_domain::geometry::Monitor { id: m.id, bounds: m.bounds, work: m.work, scale: m.scale, primary: m.primary }).collect()));
            return Err("overlay windows not ready".into());
        }
        // 화면 쪽 잔상은 새 `overlay` 알림을 받으면 스스로 걷는다
        for (l, w) in self.app.webview_windows() {
            if l.starts_with(PREFIX) {
                let _ = w.set_ignore_cursor_events(false);
            }
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

    fn show_draw(&mut self, session: &ink_app::DrawSession, frame: Option<Frame>) -> Result<Option<String>, String> {
        let m = &session.monitor;
        // 창은 prepare(배치가 바뀌면 다시 맞춤)가 미리 만들어 둔다 — 아직이면 다음 진입부터
        let w = self.app.get_webview_window(&label(m.id)).ok_or("overlay window not ready")?;
        // 확정 직후 0.4초 잔상 중에 판서가 열릴 수 있다([판서로]·Alt+1을 바로 누름) — 잔상 타이머가 이 창을 숨기지 않게
        // 취소하고 클릭 통과를 푼다. 잔상 그림은 화면(CaptureOverlay)이 `screen-ink://draw` 알림을 받으면 스스로 걷는다.
        // 다른 모니터의 잔상 창은 여기서 바로 숨긴다.
        self.bump_afterimage_gen();
        let target = label(m.id);
        for (l, other) in self.app.webview_windows() {
            if !l.starts_with(PREFIX) {
                continue;
            }
            let _ = other.set_ignore_cursor_events(false);
            if l != target {
                let _ = show_raw(&other, false);
            }
        }
        // 판서는 네이티브 정지 창·강조를 쓰지 않는다(배경도 확대·이동과 같은 변환으로 웹뷰가 그림, A §2.5)
        self.layout = None;
        let url = match &frame {
            Some(f) => {
                self.frames.put(session.session, std::slice::from_ref(f));
                Some(frame_url(session.session, m.id))
            }
            None => {
                self.frames.put(session.session, &[]);
                None
            }
        };
        // 투명 창을 먼저 보여도 화면은 그대로다(배경 = 방금 찍은 그 화면). 세션 알림은 서비스가 곧 보낸다.
        show_raw(&w, true).map_err(|e| e.to_string())?;
        focus_raw(&w);
        Ok(url)
    }

    fn hide(&mut self, session: u64) {
        self.bump_afterimage_gen();
        #[cfg(windows)]
        self.freeze.hide_all();
        // 맥: 창을 숨기기 전에 커서를 화살표로(판서 `cursor: none`·캡처 십자가 숨긴 뒤에도 남지 않게).
        // on_main은 아래 Tauri hide()보다 먼저 메인 대기열에 들어간다.
        #[cfg(target_os = "macos")]
        on_main(&self.app, || {
            ink_platform_macos::window::reset_cursor();
            ink_platform_macos::freeze::hide_all();
        });
        for (l, w) in self.app.webview_windows() {
            if l.starts_with(PREFIX) {
                let _ = show_raw(&w, false);
                let _ = w.set_ignore_cursor_events(false);
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

    fn flash_afterimage(&mut self, session: u64, rect: PhysRect, layout: &MonitorLayout) {
        if afterimage_targets(rect, layout).is_empty() {
            return;
        }
        let token = self.bump_afterimage_gen();
        let targets = afterimage_payloads(session, rect, layout, token);
        let mut shown_labels = Vec::with_capacity(targets.len());
        for (l, payload) in targets {
            let Some(w) = self.app.get_webview_window(&l) else { continue };
            let _ = w.set_ignore_cursor_events(true);
            // 그 창 기준 CSS 사각형을 그 창에만 — 화면(CaptureOverlay)이 SelectionFrame state="afterimage"로 그린다
            let _ = self.app.emit_to(l.as_str(), EVT_AFTERIMAGE, &payload);
            if show_raw(&w, true).is_ok() {
                shown_labels.push(l);
            } else {
                let _ = w.set_ignore_cursor_events(false);
            }
        }
        if shown_labels.is_empty() {
            return;
        }
        let app = self.app.clone();
        let gen = self.afterimage_gen.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(AFTERIMAGE_DURATION_MS));
            if gen.load(Ordering::SeqCst) != token {
                return;
            }
            let h = app.clone();
            let _ = app.run_on_main_thread(move || {
                if gen.load(Ordering::SeqCst) != token {
                    return;
                }
                for l in &shown_labels {
                    if let Some(w) = h.get_webview_window(l) {
                        let _ = show_raw(&w, false);
                        let _ = w.set_ignore_cursor_events(false);
                        let _ = w.set_background_color(Some(Color(0, 0, 0, 0)));
                    }
                }
            });
        });
    }

    fn request_commit(&mut self, request: &ink_app::capture::CommitRequest) -> bool {
        // 이 세션의 오버레이 창마다(주석을 쥔 창만 답한다)
        let labels: Vec<String> = match &self.layout {
            Some(layout) => layout.monitors.iter().map(|m| label(m.id)).collect(),
            None => return false,
        };
        let mut sent = false;
        for l in labels {
            if self.app.get_webview_window(&l).is_some() {
                sent |= self.app.emit_to(l.as_str(), EVT_CAPTURE_COMMIT, request).is_ok();
            }
        }
        sent
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

    #[test]
    fn 잔상_대상_모니터와_혼합_배율_좌표() {
        use ink_domain::geometry::Monitor;
        let layout = MonitorLayout::new(vec![
            Monitor { id: 0, bounds: PhysRect::new(0, 0, 1920, 1080), work: PhysRect::new(0, 0, 1920, 1040), scale: 1.0, primary: true },
            Monitor { id: 1, bounds: PhysRect::new(1920, 0, 2880, 1800), work: PhysRect::new(1920, 0, 2880, 1728), scale: 1.5, primary: false },
        ]);
        // 두 모니터에 걸친 사각형
        let targets = afterimage_targets(PhysRect::new(1820, 300, 400, 150), &layout);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0], (0, CssRect { x: 1820.0, y: 300.0, w: 400.0, h: 150.0 }));
        assert_eq!(targets[1].0, 1);
        assert!((targets[1].1.x - (-100.0 / 1.5)).abs() < 1e-6);
        assert!((targets[1].1.y - 200.0).abs() < 1e-6);
        assert!((targets[1].1.w - (400.0 / 1.5)).abs() < 1e-6);
        assert!((targets[1].1.h - 100.0).abs() < 1e-6);
        // 빈 사각형은 대상 없음
        assert!(afterimage_targets(PhysRect::new(0, 0, 0, 10), &layout).is_empty());
        // 창마다 그 창 CSS 사각형을 담은 알림(eval 스크립트 없음 — 화면이 SelectionFrame으로 그림)
        let sent = afterimage_payloads(4, PhysRect::new(1820, 300, 400, 150), &layout, 9);
        assert_eq!(sent.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(), ["ink-overlay-0", "ink-overlay-1"]);
        assert_eq!(sent[1].1.monitor_id, 1);
        assert_eq!(sent[1].1.css, targets[1].1);
        let json = serde_json::to_value(&sent[0].1).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "session": 4, "rect": { "x": 1820, "y": 300, "w": 400, "h": 150 }, "durationMs": 400, "monitorId": 0, "css": { "x": 1820.0, "y": 300.0, "w": 400.0, "h": 150.0 }, "token": 9 })
        );
        assert!(afterimage_payloads(4, PhysRect::new(0, 0, 0, 10), &layout, 9).is_empty());
    }
}
