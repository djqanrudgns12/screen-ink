//! 우리 창의 층·Space 동작·캡처 제외(M7·M8). Tauri 창의 `ns_window()` 포인터를 받는다.
//! 메인 스레드에서만 부른다(AppKit 규칙).

use crate::coords::{RectF, Space};
use ink_domain::geometry::PhysRect;
use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::ffi::c_void;

/// NSWindowCollectionBehavior: 모든 Space · 고정 · 창 순환 제외 · 전체 화면 앱 위.
const ALL_SPACES: usize = 1 << 0 | 1 << 4 | 1 << 6 | 1 << 8;
/// `NSScreenSaverWindowLevel` — 메뉴 막대(24)·Dock(20)·전체 화면 앱 위.
pub const OVERLAY_LEVEL: isize = 1000;
/// 섬은 오버레이 위에.
pub const ISLAND_LEVEL: isize = 1001;

/// 창 역할.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// 모니터별 정지 화면 오버레이
    Overlay,
    /// 섬(결과·안내)
    Island,
}

fn window<'a>(ns_window: *mut c_void) -> Option<&'a AnyObject> {
    // SAFETY: Tauri가 돌려준 NSWindow 포인터(창이 살아 있는 동안 유효).
    unsafe { (ns_window as *const AnyObject).as_ref() }
}

/// 층·Space 동작·그림자를 역할에 맞춘다.
pub fn configure(ns_window: *mut c_void, role: Role) {
    let Some(w) = window(ns_window) else { return };
    let level = match role {
        Role::Overlay => OVERLAY_LEVEL,
        Role::Island => ISLAND_LEVEL,
    };
    // SAFETY: NSWindow 공개 메서드, 인자 형식은 NSInteger·NSUInteger·BOOL.
    unsafe {
        let _: () = msg_send![w, setLevel: level];
        let _: () = msg_send![w, setCollectionBehavior: ALL_SPACES];
        let _: () = msg_send![w, setHasShadow: false];
    }
}

/// 스크린숏·화면 기록에 찍히지 않게(`NSWindowSharingNone`, Windows `WDA_EXCLUDEFROMCAPTURE`와 같은 뜻).
pub fn exclude_from_capture(ns_window: *mut c_void) -> bool {
    let Some(w) = window(ns_window) else { return false };
    // SAFETY: NSWindow 공개 메서드(NSUInteger 0 = None).
    unsafe {
        let _: () = msg_send![w, setSharingType: 0usize];
    }
    true
}

/// 계약 모니터 경계(물리 px)에 창을 맞춘다 — 포인트로 돌려 Cocoa 좌표(왼쪽 아래 원점)로 놓는다.
/// 메뉴 막대·Dock 위까지 덮는다(테두리 없는 창은 화면 안으로 밀리지 않는다).
pub fn place(ns_window: *mut c_void, space: &Space, bounds: PhysRect) -> bool {
    let (Some(w), Some(mtm)) = (window(ns_window), MainThreadMarker::new()) else { return false };
    let Some(s) = space.screen_of(bounds) else { return false };
    let Some(primary_h) = NSScreen::screens(mtm).firstObject().map(|p| p.frame().size.height) else { return false };
    let RectF { x, y, w: pw, h: ph } = s.pt;
    let frame = NSRect::new(NSPoint::new(x, primary_h - (y + ph)), NSSize::new(pw, ph));
    // SAFETY: NSWindow 공개 메서드.
    unsafe {
        let _: () = msg_send![w, setFrame: frame, display: false];
    }
    true
}

/// 활성화하지 않고 맨 앞으로(다른 앱의 포커스를 뺏지 않고 보이기만).
pub fn order_front(ns_window: *mut c_void) {
    let Some(w) = window(ns_window) else { return };
    // SAFETY: NSWindow 공개 메서드.
    unsafe {
        let _: () = msg_send![w, orderFrontRegardless];
    }
}
