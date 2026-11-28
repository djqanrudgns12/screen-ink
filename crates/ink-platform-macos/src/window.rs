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

use crate::coords::{Placed, RectF, Space};
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
pub(crate) const ALL_SPACES: usize = 1 << 0 | 1 << 4 | 1 << 6 | 1 << 8;
extern "C" {
    fn CGShieldingWindowLevel() -> i32;
}

/// 캡처·판서·확대 층 = `CGShieldingWindowLevel()`(화면 잡기용 최상층, 커서 바로 아래). 예전 값 1000(화면 보호기 층)은
/// 같은 층에 화면 크기 창을 두는 다른 앱(Claude 앱 등)이 정지 화면(999)을 덮어, 전체 화면 앱 위에서 F1이 캡처로
/// 안 바뀐 것처럼 보였다(2026-11-28 맥 실기·시험 가림 창으로 확인).
pub fn overlay_level() -> isize {
    // SAFETY: 인자 없는 CoreGraphics 조회 함수.
    unsafe { CGShieldingWindowLevel() as isize }
}

/// 섬은 오버레이 위에(커서 층보다는 아래).
pub fn island_level() -> isize {
    overlay_level() + 1
}
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
        prevent_activation(w);
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
        Role::Overlay => overlay_level(),
        Role::Island => island_level(),
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
    let Some(frame) = space.screen_of(bounds).and_then(|s| screen_frame(mtm, s)) else { return false };
    // SAFETY: NSWindow 공개 메서드.
    unsafe {
        let _: () = msg_send![w, setFrame: frame, display: false];
    }
    true
}

/// 계약 물리 px 사각형 그대로 창을 놓는다(핀 창 — 원래 자리 1:1). 혼합 배율에서 Tauri 물리 좌표는 옆 화면으로
/// 어긋나서(4K에서 50pt, FHD에서 190pt, 2026-11-26 실측) 포인트로 바꿔 직접 놓는다. 메인 스레드가 아니면 false.
pub fn place_rect(ns_window: *mut c_void, space: &Space, r: PhysRect) -> bool {
    let (Some(w), Some(mtm)) = (window(ns_window), MainThreadMarker::new()) else { return false };
    let Some(primary_h) = NSScreen::screens(mtm).firstObject().map(|p| p.frame().size.height) else { return false };
    let RectF { x, y, w: pw, h: ph } = space.rect_to_points(r);
    let frame = NSRect::new(NSPoint::new(x, primary_h - (y + ph)), NSSize::new(pw, ph));
    // SAFETY: NSWindow 공개 메서드.
    unsafe {
        let _: () = msg_send![w, setFrame: frame, display: true];
    }
    true
}

/// 지금 창 틀 → 계약 물리 px(핀 휠 확대가 지금 자리를 읽을 때). 메인 스레드가 아니면 None.
pub fn frame_phys(ns_window: *mut c_void, space: &Space) -> Option<PhysRect> {
    let (w, mtm) = (window(ns_window)?, MainThreadMarker::new()?);
    let primary_h = NSScreen::screens(mtm).firstObject().map(|p| p.frame().size.height)?;
    // SAFETY: NSWindow 공개 메서드.
    let f: NSRect = unsafe { msg_send![w, frame] };
    let cg = crate::coords::cocoa_to_cg(RectF::new(f.origin.x, f.origin.y, f.size.width, f.size.height), primary_h);
    Some(space.rect_to_phys(cg))
}

/// 화면 하나를 덮는 창 틀(Cocoa 좌표: 원점 = 주 화면 왼쪽 아래).
pub(crate) fn screen_frame(mtm: MainThreadMarker, s: &Placed) -> Option<NSRect> {
    let primary_h = NSScreen::screens(mtm).firstObject().map(|p| p.frame().size.height)?;
    let RectF { x, y, w, h } = s.pt;
    Some(NSRect::new(NSPoint::new(x, primary_h - (y + h)), NSSize::new(w, h)))
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

/// 화살표 커서로 되돌린다. 웹뷰의 `cursor: none`은 맥 WebKit에서 "투명 커서로 바꾸기"라, 창을 숨겨도
/// 다른 앱이 커서를 다시 정할 때까지 커서가 안 보인다(판서 뒤 커서 사라짐, 2026-11-26 사용자 발견).
/// 우리 창이 아직 커서 아래에 있을 때(숨기기 전) 부른다 — 커서는 그 아래 창의 앱이 정한다.
pub fn reset_cursor() {
    if MainThreadMarker::new().is_none() {
        return;
    }
    // SAFETY: NSCursor 공개 클래스 메서드·인스턴스 메서드, 메인 스레드.
    unsafe {
        let c: *mut AnyObject = msg_send![objc2::class!(NSCursor), arrowCursor];
        if !c.is_null() {
            let _: () = msg_send![c, set];
        }
    }
}

/// 만든 뒤에 `NonactivatingPanel`을 켜면 창 서버의 "활성화 막기" 표시가 갱신되지 않는 경우가 있다 → 키 창이 될 때
/// 앱이 활성화되며 설정 창이 있는 다른 Space로 넘어간다(전체 화면 앱 위 트레이 팝오버가 안 보이는 원인 후보,
/// R2-BUG-TRAY-2). 비공개 `_setPreventsActivation:`이 있으면 그 표시를 바로 갱신한다(없으면 아무 일도 없다).
fn prevent_activation(w: &AnyObject) {
    let sel = sel!(_setPreventsActivation:);
    // SAFETY: 응답하는지 먼저 묻고, BOOL 하나를 받는 메서드로만 부른다.
    unsafe {
        let ok: Bool = msg_send![w, respondsToSelector: sel];
        if ok.as_bool() {
            let _: () = msg_send![w, _setPreventsActivation: Bool::YES];
        }
    }
}

/// 그 창이 지금 Space에 있는지(`isOnActiveSpace`). 창이 없으면 false.
pub fn on_active_space(ns_window: *mut c_void) -> bool {
    let Some(w) = window(ns_window) else { return false };
    // SAFETY: NSWindow 공개 메서드.
    let b: Bool = unsafe { msg_send![w, isOnActiveSpace] };
    b.as_bool()
}

/// 그 창이 키 창인지.
pub fn is_key(ns_window: *mut c_void) -> bool {
    let Some(w) = window(ns_window) else { return false };
    // SAFETY: NSWindow 공개 메서드.
    let b: Bool = unsafe { msg_send![w, isKeyWindow] };
    b.as_bool()
}

/// 그 창의 층(`level`).
pub fn level(ns_window: *mut c_void) -> isize {
    let Some(w) = window(ns_window) else { return 0 };
    // SAFETY: NSWindow 공개 메서드(NSInteger).
    unsafe { msg_send![w, level] }
}

/// 우리 앱이 지금 활성 앱인지(메인 스레드가 아니면 false).
pub fn app_active() -> bool {
    let Some(app) = app() else { return false };
    // SAFETY: NSApplication 공개 메서드.
    let b: Bool = unsafe { msg_send![&*app, isActive] };
    b.as_bool()
}

/// 글자 입력 진단(R2-TXT-0 맥 한글 조합 버그 조사): 키 창·첫 응답자·지금 입력 문맥의 주인·입력 소스.
/// 반환 = (키 창 포인터, 한 줄 설명). 창 제목·입력한 글자는 담지 않는다.
pub fn input_state() -> (usize, String) {
    let Some(app) = app() else { return (0, "not main thread".into()) };
    let class_of = |o: *mut AnyObject| -> String {
        if o.is_null() {
            return "nil".into();
        }
        // SAFETY: 살아 있는 객체의 클래스 이름 읽기.
        unsafe { (*o).class().name().to_string_lossy().into_owned() }
    };
    // SAFETY: NSApplication·NSWindow·NSTextInputContext 공개 메서드, 메인 스레드.
    unsafe {
        let active: Bool = msg_send![&*app, isActive];
        let key: *mut AnyObject = msg_send![&*app, keyWindow];
        let fr: *mut AnyObject = if key.is_null() { std::ptr::null_mut() } else { msg_send![key, firstResponder] };
        let ctx: *mut AnyObject = msg_send![objc2::class!(NSTextInputContext), currentInputContext];
        let client: *mut AnyObject = if ctx.is_null() { std::ptr::null_mut() } else { msg_send![ctx, client] };
        let src: *mut AnyObject = if ctx.is_null() { std::ptr::null_mut() } else { msg_send![ctx, selectedKeyboardInputSource] };
        let src_name = if src.is_null() {
            "nil".to_string()
        } else {
            let c: *const std::ffi::c_char = msg_send![src, UTF8String];
            if c.is_null() { "?".into() } else { std::ffi::CStr::from_ptr(c).to_string_lossy().into_owned() }
        };
        let fr_ctx = input_context_of(fr);
        (
            key as usize,
            format!(
                "active {}, key {} ({}), first responder {}, current ctx client {}, responder ctx is current {}, source {src_name}",
                active.as_bool(),
                class_of(key),
                !key.is_null(),
                class_of(fr),
                class_of(client),
                !fr_ctx.is_null() && fr_ctx == ctx
            ),
        )
    }
}

/// 응답자의 입력 문맥(`-[NSView inputContext]`). 응답하지 않는 객체(창 자신 등)·nil이면 nil.
fn input_context_of(o: *mut AnyObject) -> *mut AnyObject {
    if o.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: 응답하는지 먼저 묻고, 인자 없는 객체 반환 메서드로만 부른다.
    unsafe {
        let ok: Bool = msg_send![o, respondsToSelector: sel!(inputContext)];
        if ok.as_bool() {
            msg_send![o, inputContext]
        } else {
            std::ptr::null_mut()
        }
    }
}

/// 글 상자가 초점을 받을 때(R2-TXT-0 맥 한글 조합): 그 창을 키 창으로 두고, 첫 응답자(웹뷰)의 입력 문맥이
/// 지금 입력 문맥(`currentInputContext`)이 아니면 지금 것을 내리고 웹뷰 것을 올린다.
/// 맥 입력기(2벌식 등)는 지금 입력 문맥으로만 조합 글을 보낸다 — 활성화하지 않는 패널에서는 앱 활성화가 없어
/// 지금 문맥이 갱신되지 않고 다른 창(이전 키 창)의 것으로 남을 수 있다 → 한글 키가 페이지에 오지 않고, 조합이 필요 없는
/// 숫자만 들어간다(M16 기록: 실행마다 다르고 한 실행 안에서는 같음 = 처음 키 창이 된 순서에 달림).
/// 반환 = 바로잡았으면 Some(바로잡기 전 상태), 이미 맞았거나 할 수 없으면 None.
pub fn ensure_text_input(ns_window: *mut c_void) -> Option<String> {
    MainThreadMarker::new()?;
    let w = window(ns_window)?;
    // SAFETY: NSWindow·NSTextInputContext 공개 메서드, 메인 스레드.
    unsafe {
        // 그사이 숨겨진 창(Esc·모드 끝)이면 아무것도 하지 않는다 — 지금 키 창(설정 창 등)의 입력을 빼앗지 않게
        let visible: Bool = msg_send![w, isVisible];
        if !visible.as_bool() {
            return None;
        }
        let was_key: Bool = msg_send![w, isKeyWindow];
        if !was_key.as_bool() {
            let _: () = msg_send![w, makeKeyWindow];
        }
        let fr: *mut AnyObject = msg_send![w, firstResponder];
        let fr_ctx = input_context_of(fr);
        if fr_ctx.is_null() {
            return None;
        }
        let cur: *mut AnyObject = msg_send![objc2::class!(NSTextInputContext), currentInputContext];
        if cur == fr_ctx && was_key.as_bool() {
            return None;
        }
        if !cur.is_null() && cur != fr_ctx {
            let _: () = msg_send![cur, deactivate];
        }
        let _: () = msg_send![fr_ctx, activate];
        Some(format!("was key {}, responder ctx was current {}, current ctx nil {}", was_key.as_bool(), cur == fr_ctx, cur.is_null()))
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
