//! 녹화 범위 표시 창(단계 6.4, 04 S11): 네 귀 빨간 꺾쇠 + 바깥 22% 어둡게(`SelectionFrame state="recording"`).
//!
//! - 창 하나(`ink-rec-frame`)를 시작할 때 숨겨 만들어 둔다 — 녹화 중 서비스 잠금 안에서 웹뷰를 새로 만들면 교착될 수 있다.
//! - 클릭 통과·포커스 없음·`WDA_EXCLUDEFROMCAPTURE`(영상에 안 찍힘, R-REC-2). 녹화하는 모니터 전체를 덮고,
//!   범위는 그 창 안 논리 px로 `screen-ink://recframe` 이벤트에 실어 보낸다(UI는 좌표를 바꾸지 않는다, R-DPI-1).
//! - 창 추적 녹화는 창이 움직이면 꺾쇠가 어긋나므로 지금은 띄우지 않는다(녹화 섬만).

use ink_domain::geometry::PhysRect;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const REC_FRAME: &str = "ink-rec-frame";
const EVT_REC_FRAME: &str = "screen-ink://recframe";

/// 지금 보이는 범위(같으면 다시 하지 않는다).
static SHOWN: Mutex<Option<PhysRect>> = Mutex::new(None);

#[derive(Clone, Copy, serde::Serialize)]
struct FramePlace {
    /// 창 안 논리 px
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let w = WebviewWindowBuilder::new(app, REC_FRAME, WebviewUrl::App("index.html?view=recframe".into()))
        .title("슉쇽 녹화 범위")
        .inner_size(320.0, 200.0)
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
    // 아래 앱을 그대로 쓸 수 있게(판서·클릭은 녹화 대상 앱으로)
    let _ = w.set_ignore_cursor_events(true);
    // 맥: 활성화하지 않는 패널 · 모든 Space · 앱 창 위(팝오버 층)
    #[cfg(target_os = "macos")]
    if let Ok(ns) = w.ns_window() {
        ink_platform_macos::window::configure(ns, ink_platform_macos::window::Role::Popover);
    }
    Ok(())
}

/// 상태가 바뀔 때마다(`Notifier::state`): 녹화 중이면 그 모니터를 덮고 범위를 알린다, 아니면 숨긴다.
pub fn sync<R: Runtime>(app: &AppHandle<R>, rec: &ink_app::state::Recording) {
    let want = rec.rect.filter(|_| rec.active && rec.target != Some("window"));
    {
        let mut shown = SHOWN.lock().unwrap_or_else(|e| e.into_inner());
        if *shown == want {
            return;
        }
        *shown = want;
    }
    let Some(w) = app.get_webview_window(REC_FRAME) else { return };
    let Some(r) = want else {
        let _ = w.hide();
        return;
    };
    // 맥: 혼합 배율에서 Tauri 물리 좌표는 옆 화면으로 어긋난다 → 그 모니터를 포인트로 직접 덮는다(오버레이와 같음)
    #[cfg(target_os = "macos")]
    {
        let space = ink_platform_macos::space();
        let center = ink_domain::geometry::PhysPoint { x: r.x + r.w / 2, y: r.y + r.h / 2 };
        let layout = space.layout();
        if let (Some(m), Ok(ns)) = (layout.monitors.iter().find(|m| m.bounds.contains(center)), w.ns_window()) {
            if ink_platform_macos::window::place(ns, &space, m.bounds) {
                let s = m.scale.max(0.5);
                let place = FramePlace { x: (r.x - m.bounds.x) as f64 / s, y: (r.y - m.bounds.y) as f64 / s, width: r.w as f64 / s, height: r.h as f64 / s };
                let _ = app.emit_to(REC_FRAME, EVT_REC_FRAME, place);
                ink_platform_macos::window::order_front(ns);
                let _ = w.show();
                return;
            }
        }
    }
    let (cx, cy) = ((r.x + r.w / 2) as f64, (r.y + r.h / 2) as f64);
    let Some(m) = w.monitor_from_point(cx, cy).ok().flatten() else { return };
    let (pos, size, scale) = (*m.position(), *m.size(), m.scale_factor().max(0.5));
    let _ = w.set_position(PhysicalPosition::new(pos.x, pos.y));
    let _ = w.set_size(PhysicalSize::new(size.width, size.height));
    let place = FramePlace { x: (r.x - pos.x) as f64 / scale, y: (r.y - pos.y) as f64 / scale, width: r.w as f64 / scale, height: r.h as f64 / scale };
    let _ = app.emit_to(REC_FRAME, EVT_REC_FRAME, place);
    #[cfg(windows)]
    if let Ok(h) = w.hwnd() {
        // 활성화하지 않고 맨 위에(녹화 대상 앱이 포커스를 잃지 않게)
        ink_platform_windows::events::set_window_shown(h.0 as isize, true);
        return;
    }
    let _ = w.show();
}
