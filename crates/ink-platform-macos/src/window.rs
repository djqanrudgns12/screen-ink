//! 우리 창의 층·Space 동작·캡처 제외(M7·M8). Tauri 창의 `ns_window()` 포인터를 받는다.
//! 메인 스레드에서만 부른다(AppKit 규칙).
//!
//! 전체 화면 앱 위에 뜨기: 일반 NSWindow는 다른 앱의 전체 화면 Space에 끼어들지 못해, 앞으로 올리면
//! macOS가 그 창이 있는 Space로 넘어간다. 그래서 오버레이·섬은 **활성화하지 않는 패널**(NSPanel +
//! `NonactivatingPanel`)로 바꾼다. 앱을 활성화하지 않고도 키 창이 되어 Esc·Enter를 받는다.
//! 클래스만 바꾼다(`object_setClass`): NSPanel은 NSWindow에 필드를 더하지 않는다(둘 다 528바이트,
//! 2026-11-25 macOS 27 확인). tao 창(`TaoWindow`)은 그 뒤에 자기 상태 칸을 붙여 더 크다(536바이트) →
//! 패널 클래스에도 같은 크기의 빈 칸을 붙여 크기를 맞춘다(우리는 그 칸을 읽거나 쓰지 않는다. tao 상태는 그대로).
//! tao가 덮어쓴 것(키 창 여부·창 끌기)은 여기서 다시 정한다.

use crate::coords::{RectF, Space};
use ink_domain::geometry::PhysRect;
use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::{sel, ClassType};
use objc2_app_kit::NSPanel;
use std::sync::OnceLock;
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
/// 팝오버 = `NSPopUpMenuWindowLevel`(메뉴 막대 위, 오버레이 아래).
pub const POPOVER_LEVEL: isize = 101;

/// 창 역할.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// 모니터별 정지 화면 오버레이
    Overlay,
    /// 섬(결과·안내)
    Island,
    /// 트레이 팝오버(메뉴 막대 바로 위 층)
    Popover,
}

fn window<'a>(ns_window: *mut c_void) -> Option<&'a AnyObject> {
    // SAFETY: Tauri가 돌려준 NSWindow 포인터(창이 살아 있는 동안 유효).
    unsafe { (ns_window as *const AnyObject).as_ref() }
}

extern "C-unwind" fn yes(_this: *mut AnyObject, _sel: Sel) -> Bool {
    Bool::YES
}

extern "C-unwind" fn no(_this: *mut AnyObject, _sel: Sel) -> Bool {
    Bool::NO
}

/// 키 창이 될 수 있는(오버레이·팝오버) / 없는(섬) 패널 클래스. `size` = 바꿀 창 클래스의 인스턴스 크기. 한 번만 만든다.
fn panel_class(key: bool, size: usize) -> &'static AnyClass {
    static KEY: OnceLock<&'static AnyClass> = OnceLock::new();
    static NO_KEY: OnceLock<&'static AnyClass> = OnceLock::new();
    let (cell, name) = if key { (&KEY, c"ScreenInkKeyPanel") } else { (&NO_KEY, c"ScreenInkPanel") };
    cell.get_or_init(|| {
        let mut b = ClassBuilder::new(name, NSPanel::class()).expect("panel class name is unique");
        // tao 상태 칸만큼 빈 칸(바이트 단위라 정렬 틈이 생기지 않는다)
        for i in 0..size.saturating_sub(NSPanel::class().instance_size()) {
            let n = std::ffi::CString::new(format!("_screenInkPad{i}")).expect("no nul");
            b.add_ivar::<u8>(&n);
        }
        let key_fn: extern "C-unwind" fn(*mut AnyObject, Sel) -> Bool = if key { yes } else { no };
        // SAFETY: 서명이 NSWindow 메서드(BOOL 반환, 인자 없음)와 같다.
        unsafe {
            b.add_method(sel!(canBecomeKeyWindow), key_fn);
            b.add_method(sel!(canBecomeMainWindow), no as extern "C-unwind" fn(*mut AnyObject, Sel) -> Bool);
        }
        b.register()
    })
}

/// 활성화하지 않는 패널로 바꾼다(이미 바꿨으면 그대로).
fn make_panel(w: &AnyObject, key: bool) -> bool {
    let size = w.class().instance_size();
    let cls = panel_class(key, size);
    if std::ptr::eq(w.class(), cls) {
        return true;
    }
    if cls.instance_size() != size {
        log::warn!("panel class size {} != window class size {size}, keeping NSWindow", cls.instance_size());
        return false;
    }
    // SAFETY: NSPanel은 NSWindow와 인스턴스 배치가 같다(필드 추가 없음, 위 설명). 바꾼 뒤 부르는 것은 공개 메서드뿐.
    unsafe {
        AnyObject::set_class(w, cls);
        let mask: usize = msg_send![w, styleMask];
        let _: () = msg_send![w, setStyleMask: mask | 1 << 7]; // NSWindowStyleMaskNonactivatingPanel
        let _: () = msg_send![w, setFloatingPanel: true];
        let _: () = msg_send![w, setBecomesKeyOnlyIfNeeded: false];
        let _: () = msg_send![w, setHidesOnDeactivate: false];
        let _: () = msg_send![w, setWorksWhenModal: true];
    }
    true
}

/// 패널·층·Space 동작·그림자를 역할에 맞춘다.
pub fn configure(ns_window: *mut c_void, role: Role) {
    let Some(w) = window(ns_window) else { return };
    if !make_panel(w, role != Role::Island) {
        log::info!("{role:?} stays a normal window (may switch Spaces over full-screen apps)");
    }
    let level = match role {
        Role::Overlay => OVERLAY_LEVEL,
        Role::Island => ISLAND_LEVEL,
        Role::Popover => POPOVER_LEVEL,
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

/// 팝오버를 메뉴 막대 아이콘 바로 아래에 붙인다(CG 전역 포인트 `icon`). `pad` = 카드 둘레 투명 여백,
/// `gap` = 메뉴 막대 아래와 카드 윗변 사이. 창 크기는 지금 것을 쓰고, 가로는 아이콘 가운데에 맞춰 화면 안으로 가둔다.
/// 맥은 창 크기를 바꿀 때 왼쪽 아래를 고정하므로, 크기가 바뀔 때마다 다시 불러 윗변을 붙든다.
pub fn place_below(ns_window: *mut c_void, space: &Space, icon: RectF, pad: f64, gap: f64) -> bool {
    let (Some(w), Some(mtm)) = (window(ns_window), MainThreadMarker::new()) else { return false };
    let Some(primary_h) = NSScreen::screens(mtm).firstObject().map(|p| p.frame().size.height) else { return false };
    let (cx, cy) = (icon.x + icon.w / 2.0, icon.y + icon.h / 2.0);
    let Some(s) = space.screens.iter().find(|s| cx >= s.pt.x && cx < s.pt.right() && cy >= s.pt.y && cy < s.pt.bottom()).or(space.screens.first()) else { return false };
    // SAFETY: NSWindow 공개 메서드.
    let frame: NSRect = unsafe { msg_send![w, frame] };
    let (fw, _) = (frame.size.width, frame.size.height);
    const EDGE: f64 = 8.0; // 카드와 화면 가장자리 사이
    let lo = s.pt.x + EDGE - pad;
    let hi = (s.pt.right() - EDGE + pad - fw).max(lo);
    let x = (cx - fw / 2.0).clamp(lo, hi);
    let top = icon.bottom() + gap - pad;
    // SAFETY: NSWindow 공개 메서드(Cocoa 좌표: 원점 왼쪽 아래 → 윗변 = 주 화면 높이 − CG y).
    unsafe {
        let _: () = msg_send![w, setFrameTopLeftPoint: NSPoint::new(x, primary_h - top)];
    }
    true
}

/// 활성화하지 않고 **지금 Space** 맨 앞으로 바로(비동기 아님). 전체 화면 앱 위에서도 그 자리에 뜬다.
pub fn order_front(ns_window: *mut c_void) {
    let Some(w) = window(ns_window) else { return };
    // SAFETY: NSWindow 공개 메서드.
    unsafe {
        let _: () = msg_send![w, orderFrontRegardless];
    }
}

fn app() -> Option<objc2::rc::Retained<objc2_app_kit::NSApplication>> {
    MainThreadMarker::new().map(objc2_app_kit::NSApplication::sharedApplication)
}

/// 키 창으로(Esc·Enter·방향키를 받게). 활성화하지 않는 패널이라 앱을 활성화하지 않는다 → Space가 넘어가지 않는다.
pub fn make_key(ns_window: *mut c_void) {
    let Some(w) = window(ns_window) else { return };
    // SAFETY: NSWindow 공개 메서드.
    unsafe {
        let _: () = msg_send![w, makeKeyWindow];
    }
}

/// 모드가 끝나면 포커스를 원래 앱(전체 화면 앱 등)으로 돌려준다.
pub fn deactivate() {
    let Some(app) = app() else { return };
    // SAFETY: NSApplication 공개 메서드.
    unsafe {
        let _: () = msg_send![&*app, deactivate];
    }
}
