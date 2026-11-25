//! 시스템 사건(M11): 디스플레이 변경·잠금·해제·절전·깨어남·테마 → `SystemEvent`.
//! 모두 메인 스레드 실행 고리에서 온다(디스플레이 콜백·알림 센터 기본 큐). 처리기는 메인 스레드로 넘길 일만 한다.
//! 화면 잠김 여부(`secure_desktop`)도 여기.

use block2::RcBlock;
use ink_app::SystemEvent;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidWakeNotification, NSWorkspaceWillSleepNotification};
use objc2_foundation::{NSDictionary, NSDistributedNotificationCenter, NSNotification, NSNotificationCenter, NSNumber, NSString};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::{Arc, OnceLock};

type Callback = Arc<dyn Fn(SystemEvent) + Send + Sync>;

static CALLBACK: OnceLock<Callback> = OnceLock::new();

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayRegisterReconfigurationCallback(cb: extern "C" fn(u32, u32, *mut c_void), user: *mut c_void) -> i32;
    fn CGSessionCopyCurrentDictionary() -> *const c_void;
}

/// `kCGDisplayBeginConfigurationFlag` — 바뀌기 직전 알림(무시하고 끝난 뒤만 받는다).
const BEGIN_CONFIGURATION: u32 = 1;

extern "C" fn on_display(_display: u32, flags: u32, _user: *mut c_void) {
    if flags & BEGIN_CONFIGURATION != 0 {
        return;
    }
    crate::capture::invalidate();
    if let Some(cb) = CALLBACK.get() {
        cb(SystemEvent::DisplayChange);
    }
}

fn observe(center: &NSNotificationCenter, name: &NSString, ev: SystemEvent) {
    let block = RcBlock::new(move |_n: NonNull<NSNotification>| {
        if let Some(cb) = CALLBACK.get() {
            cb(ev);
        }
    });
    // SAFETY: 블록은 'static이고 센터가 복사해 둔다. 관찰자 표는 앱이 끝날 때까지 둔다(되돌리지 않음).
    let token = unsafe { center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block) };
    std::mem::forget(token);
}

/// 사건을 듣기 시작한다(한 번만). 메인 스레드에서 부른다. 이미 듣고 있으면 false.
pub fn listen(cb: impl Fn(SystemEvent) + Send + Sync + 'static) -> bool {
    if CALLBACK.set(Arc::new(cb)).is_err() {
        return false;
    }
    // SAFETY: 콜백은 프로그램이 끝날 때까지 산다(정적 함수, 사용자 포인터 없음).
    let err = unsafe { CGDisplayRegisterReconfigurationCallback(on_display, std::ptr::null_mut()) };
    if err != 0 {
        log::warn!("display reconfiguration callback failed: {err}");
    }
    let ws = NSWorkspace::sharedWorkspace().notificationCenter();
    // SAFETY: AppKit 알림 이름 상수.
    unsafe {
        observe(&ws, NSWorkspaceWillSleepNotification, SystemEvent::Suspend);
        observe(&ws, NSWorkspaceDidWakeNotification, SystemEvent::Resume);
    }
    // 잠금·해제·테마는 분산 알림(문서화된 이름은 없지만 오래 쓰여 온 시스템 알림)
    let dist = NSDistributedNotificationCenter::defaultCenter();
    observe(&dist, &NSString::from_str("com.apple.screenIsLocked"), SystemEvent::Lock);
    observe(&dist, &NSString::from_str("com.apple.screenIsUnlocked"), SystemEvent::Unlock);
    observe(&dist, &NSString::from_str("AppleInterfaceThemeChangedNotification"), SystemEvent::ThemeChange);
    true
}

/// 화면이 잠겨 있는지(`CGSSessionScreenIsLocked`). 단축키를 조용히 무시할 때(R-SYS-3).
pub fn screen_locked() -> bool {
    // SAFETY: 받은 사전은 CF 소유권 규칙(Copy)대로 여기서 놓는다. CFDictionary ↔ NSDictionary는 무료 다리.
    unsafe {
        let d = CGSessionCopyCurrentDictionary();
        let Some(d) = Retained::from_raw(d as *mut NSDictionary<NSString, AnyObject>) else { return false };
        d.objectForKey(&NSString::from_str("CGSSessionScreenIsLocked")).and_then(|o| o.downcast::<NSNumber>().ok()).is_some_and(|n| n.boolValue())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn 지금은_잠겨_있지_않다() {
        // 시험을 돌리는 동안 화면은 켜져 있다
        assert!(!super::screen_locked());
    }
}
