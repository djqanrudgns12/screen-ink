//! 녹화 범위 표시 창(단계 6.4, 04 S11): 네 귀 빨간 꺾쇠 + 바깥 22% 어둡게(`SelectionFrame state="recording"`).
//!
//! - 창 하나(`ink-rec-frame`)를 시작할 때 숨겨 만들어 둔다 — 녹화 중 서비스 잠금 안에서 웹뷰를 새로 만들면 교착될 수 있다.
//! - 클릭 통과·포커스 없음·`WDA_EXCLUDEFROMCAPTURE`(영상에 안 찍힘, R-REC-2). 녹화하는 모니터 전체를 덮고,
//!   범위는 그 창 안 논리 px로 `screen-ink://recframe` 이벤트에 실어 보낸다(UI는 좌표를 바꾸지 않는다, R-DPI-1).
//! - 창 추적 녹화(단계 6.4): 대상 창이 움직이거나 크기가 바뀌면 녹화기의 자리 변화 알림으로 꺾쇠 창을 옮기고,
//!   최소화·다른 Space로 숨으면 꺾쇠를 숨긴다.

use ink_domain::geometry::PhysRect;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const REC_FRAME: &str = "ink-rec-frame";
const EVT_REC_FRAME: &str = "screen-ink://recframe";

#[derive(Clone, Copy, PartialEq, Eq)]
struct Shown {
    rect: PhysRect,
    monitor_id: Option<u32>,
}

/// 지금 보이는 범위와 모니터(같으면 다시 하지 않는다).
static SHOWN: Mutex<Option<Shown>> = Mutex::new(None);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyncAction {
    Set(Option<PhysRect>),
    Keep,
}

pub(crate) fn sync_action(active: bool, target: Option<&str>, rect: Option<PhysRect>) -> SyncAction {
    if !active {
        SyncAction::Set(None)
    } else if target == Some("window") {
        SyncAction::Keep
    } else {
        SyncAction::Set(rect)
    }
}

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
        .title(crate::tr(app, "plugin.window.recFrame"))
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
/// 창 추적(`window`) 중에는 상태 틱마다 덮지 않고 `Notifier::record_frame` → `set_rect`만 따른다.
pub fn sync<R: Runtime>(app: &AppHandle<R>, rec: &ink_app::state::Recording) {
    if let SyncAction::Set(want) = sync_action(rec.active, rec.target, rec.rect) {
        set_rect(app, want);
    }
}

/// 녹화 꺾쇠 창의 범위·보임 상태를 맞춘다(`None` = 숨김).
pub fn set_rect<R: Runtime>(app: &AppHandle<R>, want: Option<PhysRect>) {
    let prev_monitor = {
        let mut shown = SHOWN.lock().unwrap_or_else(|e| e.into_inner());
        if shown.map(|s| s.rect) == want {
            return;
        }
        if want.is_none() {
            *shown = None;
        }
        shown.and_then(|s| s.monitor_id)
    };
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
        if let (Some(m), Ok(ns)) = (layout.monitor_at(center).or_else(|| layout.nearest(center)), w.ns_window()) {
            let need_window_move = prev_monitor != Some(m.id);
            let placed = !need_window_move || ink_platform_macos::window::place(ns, &space, m.bounds);
            if placed {
                *SHOWN.lock().unwrap_or_else(|e| e.into_inner()) = Some(Shown { rect: r, monitor_id: Some(m.id) });
                let s = m.scale.max(0.5);
                let place = FramePlace { x: (r.x - m.bounds.x) as f64 / s, y: (r.y - m.bounds.y) as f64 / s, width: r.w as f64 / s, height: r.h as f64 / s };
                let _ = app.emit_to(REC_FRAME, EVT_REC_FRAME, place);
                if need_window_move {
                    ink_platform_macos::window::order_front(ns);
                    let _ = w.show();
                }
                return;
            }
        }
    }
    let _ = prev_monitor;
    let (cx, cy) = ((r.x + r.w / 2) as f64, (r.y + r.h / 2) as f64);
    let Some(m) = w.monitor_from_point(cx, cy).ok().flatten() else { return };
    *SHOWN.lock().unwrap_or_else(|e| e.into_inner()) = Some(Shown { rect: r, monitor_id: None });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 창_추적과_일반_녹화의_상태_동기화_판정() {
        let r = PhysRect::new(10, 20, 300, 200);
        assert_eq!(sync_action(false, Some("window"), Some(r)), SyncAction::Set(None));
        assert_eq!(sync_action(false, Some("region"), Some(r)), SyncAction::Set(None));
        assert_eq!(sync_action(true, Some("region"), Some(r)), SyncAction::Set(Some(r)));
        assert_eq!(sync_action(true, Some("monitor"), Some(r)), SyncAction::Set(Some(r)));
        assert_eq!(sync_action(true, Some("window"), Some(r)), SyncAction::Keep);
    }
}
