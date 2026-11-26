//! 실시간 확대 중 휠·Esc 가로채기(M13, R-ZOOM-2): `CGEventTap`(손쉬운 사용 필요).
//! 확대 모니터 안에서 Shift 없이 굴리면 먹고 배율로 넘긴다. Shift+휠·모니터 밖은 아래 앱으로 그대로.
//! 마우스 휠(줄 단위)은 한 줄 = 한 칸(격자), 트랙패드(연속)는 40 pt = 한 칸의 소수배(연속 배율).
//! 부호는 서비스 규약(`ZoomInput::Wheel`): 위로 굴림(사건 값 +) = 음수 = 더 확대.
//! 보조 키 없는 Esc도 먹고 끝내기로 넘긴다 — 전역 단축키(Esc)가 다른 앱과 겹쳐 등록되지 않았을 때의 대비.
//! 탭은 전용 스레드의 실행 고리에서 돈다(메인 스레드가 바빠도 휠이 늦지 않게). 권한이 없으면 만들지 못하고 None.

use crate::coords::Space;
use ink_domain::geometry::PhysRect;
use std::ffi::c_void;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

type CFTypeRef = *const c_void;
type Callback = Arc<dyn Fn(TapEvent) + Send + Sync>;

/// 탭이 넘기는 것.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TapEvent {
    /// 마우스 휠 칸(음수 = 위 = 더 확대)
    Notches(i32),
    /// 트랙패드 연속 스크롤(칸의 소수배, 부호 같음)
    Scroll(f64),
    /// 보조 키 없는 Esc
    Escape,
}

/// 트랙패드 연속 스크롤에서 한 칸으로 볼 포인트(판서 휠 문턱과 같음).
const POINTS_PER_NOTCH: f64 = 40.0;
const KEY_DOWN: u32 = 10;
const SCROLL_WHEEL: u32 = 22;
const FIELD_KEYCODE: u32 = 9;
const KEY_ESCAPE: i64 = 53;
/// Shift·Control·Option·Command
const FLAG_MODIFIERS: u64 = 0x0002_0000 | 0x0004_0000 | 0x0008_0000 | 0x0010_0000;
const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_BY_USER: u32 = 0xFFFF_FFFF;
const FIELD_DELTA_AXIS1: u32 = 11;
const FIELD_POINT_DELTA_AXIS1: u32 = 96;
const FIELD_IS_CONTINUOUS: u32 = 88;
const FLAG_SHIFT: u64 = 0x0002_0000;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(tap: u32, place: u32, options: u32, mask: u64, callback: extern "C" fn(CFTypeRef, u32, CFTypeRef, *mut c_void) -> CFTypeRef, user: *mut c_void) -> CFTypeRef;
    fn CGEventTapEnable(tap: CFTypeRef, enable: bool);
    fn CGEventGetIntegerValueField(event: CFTypeRef, field: u32) -> i64;
    fn CGEventGetFlags(event: CFTypeRef) -> u64;
    fn CGEventGetLocation(event: CFTypeRef) -> CGPoint;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopCommonModes: CFTypeRef;
    fn CFMachPortCreateRunLoopSource(alloc: CFTypeRef, port: CFTypeRef, order: isize) -> CFTypeRef;
    fn CFRunLoopGetCurrent() -> CFTypeRef;
    fn CFRunLoopAddSource(rl: CFTypeRef, source: CFTypeRef, mode: CFTypeRef);
    fn CFRunLoopRun();
    fn CFRunLoopStop(rl: CFTypeRef);
    fn CFRelease(cf: CFTypeRef);
}

struct Ctx {
    cb: Callback,
    /// 확대 모니터(물리 px) — 이 안에서만 먹는다
    monitor: PhysRect,
    space: Space,
    tap: CFTypeRef,
}

/// 쥐고 있는 동안 탭이 돈다. 놓으면(Drop) 실행 고리를 멈추고 스레드가 끝난다.
pub struct WheelTap {
    run_loop: usize,
    thread: Option<JoinHandle<()>>,
}

extern "C" fn tap_cb(_proxy: CFTypeRef, kind: u32, event: CFTypeRef, user: *mut c_void) -> CFTypeRef {
    // SAFETY: user는 이 탭의 스레드가 쥔 Ctx(탭이 사는 동안 유효), 이 콜백은 그 스레드에서만 불린다.
    let ctx = unsafe { &mut *(user as *mut Ctx) };
    if kind == TAP_DISABLED_BY_TIMEOUT || kind == TAP_DISABLED_BY_USER {
        // SAFETY: 살아 있는 탭을 다시 켠다.
        unsafe { CGEventTapEnable(ctx.tap, true) };
        return event;
    }
    if kind == KEY_DOWN {
        // SAFETY: 살아 있는 사건의 값 읽기.
        let (code, flags) = unsafe { (CGEventGetIntegerValueField(event, FIELD_KEYCODE), CGEventGetFlags(event)) };
        if code == KEY_ESCAPE && flags & FLAG_MODIFIERS == 0 {
            (ctx.cb)(TapEvent::Escape);
            return std::ptr::null();
        }
        return event;
    }
    if kind != SCROLL_WHEEL {
        return event;
    }
    // SAFETY: 살아 있는 사건의 값 읽기.
    let (shift, at, continuous, lines, points) = unsafe {
        (
            CGEventGetFlags(event) & FLAG_SHIFT != 0,
            CGEventGetLocation(event),
            CGEventGetIntegerValueField(event, FIELD_IS_CONTINUOUS) != 0,
            CGEventGetIntegerValueField(event, FIELD_DELTA_AXIS1),
            CGEventGetIntegerValueField(event, FIELD_POINT_DELTA_AXIS1),
        )
    };
    let inside = ctx.monitor.contains(ctx.space.to_phys(at.x, at.y));
    if shift || !inside {
        return event;
    }
    if let Some(ev) = to_event(continuous, lines, points) {
        (ctx.cb)(ev);
    }
    // 먹는다 — 아래 앱은 이 휠을 받지 않는다(0 값 사건 = 관성 끝 등도 함께)
    std::ptr::null()
}

/// 스크롤 사건 값 → 서비스 입력(위 = 음수). 움직임이 없으면 None.
pub fn to_event(continuous: bool, lines: i64, points: i64) -> Option<TapEvent> {
    if continuous {
        (points != 0).then(|| TapEvent::Scroll(-(points as f64) / POINTS_PER_NOTCH))
    } else {
        (lines != 0).then(|| TapEvent::Notches(-(lines.clamp(-100, 100) as i32)))
    }
}

/// 확대 모니터 `monitor`(물리 px)에서 휠·Esc를 가로챈다. 권한이 없으면 None.
pub fn start(monitor: PhysRect, cb: impl Fn(TapEvent) + Send + Sync + 'static) -> Option<WheelTap> {
    let (tx, rx) = mpsc::channel::<Option<usize>>();
    let cb: Callback = Arc::new(cb);
    let space = crate::space();
    let thread = std::thread::Builder::new()
        .name("ink-zoom-wheel".into())
        .spawn(move || {
            let ctx = Box::into_raw(Box::new(Ctx { cb, monitor, space, tap: std::ptr::null() }));
            // SAFETY: 세션 탭(1)·머리(0)·기본(0 = 고칠 수 있음). 콜백은 이 스레드 실행 고리에서 돈다.
            // 키 사건을 못 받게 막혀 있으면(입력 모니터링) 스크롤만이라도.
            let tap = unsafe {
                let both = CGEventTapCreate(1, 0, 0, 1u64 << SCROLL_WHEEL | 1u64 << KEY_DOWN, tap_cb, ctx as *mut c_void);
                if both.is_null() {
                    CGEventTapCreate(1, 0, 0, 1u64 << SCROLL_WHEEL, tap_cb, ctx as *mut c_void)
                } else {
                    both
                }
            };
            if tap.is_null() {
                log::info!("zoom wheel tap unavailable (accessibility not granted?)");
                // SAFETY: 위에서 만든 Box를 되돌려 놓는다.
                drop(unsafe { Box::from_raw(ctx) });
                let _ = tx.send(None);
                return;
            }
            // SAFETY: 살아 있는 탭·실행 고리. 소스는 실행 고리가 쥔다.
            unsafe {
                (*ctx).tap = tap;
                let src = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
                let rl = CFRunLoopGetCurrent();
                CFRunLoopAddSource(rl, src, kCFRunLoopCommonModes);
                CGEventTapEnable(tap, true);
                let _ = tx.send(Some(rl as usize));
                CFRunLoopRun();
                CGEventTapEnable(tap, false);
                CFRelease(src);
                CFRelease(tap);
                drop(Box::from_raw(ctx));
            }
        })
        .ok()?;
    match rx.recv() {
        Ok(Some(rl)) => Some(WheelTap { run_loop: rl, thread: Some(thread) }),
        _ => {
            let _ = thread.join();
            None
        }
    }
}

impl Drop for WheelTap {
    fn drop(&mut self) {
        // SAFETY: 그 스레드의 실행 고리를 멈춘다(다른 스레드에서 불러도 된다).
        unsafe { CFRunLoopStop(self.run_loop as CFTypeRef) };
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 위로_굴리면_음수_더_확대이고_트랙패드는_40포인트가_한_칸() {
        assert_eq!(to_event(false, 1, 0), Some(TapEvent::Notches(-1)));
        assert_eq!(to_event(false, -3, 0), Some(TapEvent::Notches(3)));
        assert_eq!(to_event(true, 0, 20), Some(TapEvent::Scroll(-0.5)));
        assert_eq!(to_event(true, 0, -80), Some(TapEvent::Scroll(2.0)));
        assert_eq!(to_event(true, 0, 0), None);
        assert_eq!(to_event(false, 0, 7), None);
    }
}
