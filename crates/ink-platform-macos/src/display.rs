//! 모니터 배치(M1): CG 디스플레이 목록 + NSScreen 작업 영역 → [`coords::Space`].
//! 커서 위치·옮기기·⌥ 상태도 여기(모두 CG 전역 포인트 → `Space`로 변환).

use crate::coords::{cocoa_to_cg, RectF, ScreenInfo, Space};
use ink_domain::geometry::PhysPoint;
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;
use objc2_core_graphics::{
    CGDisplayBounds, CGDisplayCopyDisplayMode, CGDisplayMirrorsDisplay, CGDisplayMode, CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID,
    CGGetActiveDisplayList, CGMainDisplayID, CGWarpMouseCursorPosition,
};
use objc2_core_foundation::{CGPoint, CGRect};
use objc2_foundation::{NSNumber, NSString};
use std::collections::HashMap;

fn rect(r: CGRect) -> RectF {
    RectF::new(r.origin.x, r.origin.y, r.size.width, r.size.height)
}

/// display id → 작업 영역(CG 전역 포인트). NSScreen은 메인 스레드에서만 읽을 수 있어, 아니면 빈 표.
fn visible_frames() -> HashMap<u32, RectF> {
    let Some(mtm) = MainThreadMarker::new() else { return HashMap::new() };
    let screens = NSScreen::screens(mtm);
    let Some(first) = screens.firstObject() else { return HashMap::new() };
    // Cocoa 원점 = screens[0](메뉴 막대가 있는 주 화면)의 왼쪽 아래
    let primary_h = first.frame().size.height;
    let key = NSString::from_str("NSScreenNumber");
    let mut out = HashMap::new();
    for s in screens.iter() {
        let desc = s.deviceDescription();
        let Some(num) = desc.objectForKey(&key) else { continue };
        let Ok(num) = Retained::downcast::<NSNumber>(num) else { continue };
        let v = s.visibleFrame();
        out.insert(num.unsignedIntValue(), cocoa_to_cg(RectF::new(v.origin.x, v.origin.y, v.size.width, v.size.height), primary_h));
    }
    out
}

/// 켜져 있는 화면(미러 사본 제외)의 사실값.
pub fn screens() -> Vec<ScreenInfo> {
    let mut ids = [0u32; 16];
    let mut n = 0u32;
    // SAFETY: 버퍼 길이를 함께 넘긴다.
    let err = unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut n) };
    if err.0 != 0 {
        log::warn!("CGGetActiveDisplayList failed: {}", err.0);
        return Vec::new();
    }
    let main = CGMainDisplayID();
    let visible = visible_frames();
    ids[..n as usize]
        .iter()
        .filter(|&&id| CGDisplayMirrorsDisplay(id) == 0)
        .map(|&id| {
            let frame = rect(CGDisplayBounds(id));
            let mode = CGDisplayCopyDisplayMode(id);
            let (pw, w) = (CGDisplayMode::pixel_width(mode.as_deref()), CGDisplayMode::width(mode.as_deref()));
            let scale = if pw > 0 && w > 0 { pw as f64 / w as f64 } else { 1.0 };
            ScreenInfo { display: id, frame, visible: visible.get(&id).copied().unwrap_or(frame), scale, primary: id == main }
        })
        .collect()
}

pub fn space() -> Space {
    Space::new(&screens())
}

/// 커서(CG 전역 포인트).
pub fn cursor_pt() -> (f64, f64) {
    let e = CGEvent::new(None);
    let p = CGEvent::location(e.as_deref());
    (p.x, p.y)
}

pub fn cursor(space: &Space) -> PhysPoint {
    let (x, y) = cursor_pt();
    space.to_phys(x, y)
}

/// WASD 1 px 이동. 포인트로 돌려 옮긴다(배율 2면 1 px = 0.5pt).
pub fn set_cursor(space: &Space, p: PhysPoint) {
    let (x, y) = space.to_points(p);
    let r = CGWarpMouseCursorPosition(CGPoint { x, y });
    if r.0 != 0 {
        log::debug!("CGWarpMouseCursorPosition failed: {}", r.0);
    }
}

/// ⌥(Option)를 누르고 있는지.
pub fn alt_down() -> bool {
    CGEventSource::flags_state(CGEventSourceStateID::CombinedSessionState).contains(CGEventFlags::MaskAlternate)
}
