//! 권한(M10): 화면 기록·손쉬운 사용·마이크를 확인하고, 단추 하나로 요청 + 정확한 설정 칸을 연다.
//! 켜는 것은 사용자가 시스템 설정에서 한다(앱이 직접 바꾸지 않는다). 화면 기록은 켠 뒤 앱을 다시 시작해야 반영되고
//! (그 순간 macOS가 "종료 후 다시 열기"를 묻는다), 손쉬운 사용은 바로 반영된다(인수인계 2026-11-25 §2.5-3).

use block2::RcBlock;
use ink_app::MicAccess;
use objc2::runtime::{AnyClass, Bool};
use objc2::msg_send;
use objc2_av_foundation::AVMediaTypeAudio;
use objc2_core_graphics::{CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};
use objc2_foundation::NSInteger;
use std::ffi::c_void;

/// 시스템 설정 › 개인정보 보호 및 보안 › 화면 기록(macOS 13+ 같은 주소).
pub const SCREEN_PANE: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";
/// 시스템 설정 › 개인정보 보호 및 보안 › 손쉬운 사용.
pub const AX_PANE: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
/// 시스템 설정 › 개인정보 보호 및 보안 › 마이크.
pub const MIC_PANE: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone";

type CFTypeRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
    static kAXTrustedCheckOptionPrompt: CFTypeRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    fn CFDictionaryCreate(alloc: CFTypeRef, keys: *const CFTypeRef, values: *const CFTypeRef, n: isize, kcb: *const c_void, vcb: *const c_void) -> CFTypeRef;
    fn CFRelease(cf: CFTypeRef);
}

/// 화면 기록 권한(묻지 않음). 이 프로세스가 시작할 때 값이라, 켠 뒤에는 다시 시작해야 true가 된다.
pub fn screen() -> bool {
    CGPreflightScreenCaptureAccess()
}

/// 손쉬운 사용 권한(묻지 않음, 켜면 바로 true).
pub fn accessibility() -> bool {
    // SAFETY: 인자 없는 공개 함수.
    unsafe { AXIsProcessTrusted() }
}

/// 화면 기록: 처음이면 시스템 요청 창(목록에 앱을 올림) + 설정 칸 열기.
pub fn request_screen() {
    let _ = CGRequestScreenCaptureAccess();
    open_pane(SCREEN_PANE);
}

/// 손쉬운 사용: 시스템 요청 창(목록에 앱을 올림) + 설정 칸 열기.
pub fn request_accessibility() {
    // SAFETY: 키·값 하나짜리 CF 사전을 만들어 넘기고 놓는다(상수 키·값은 CF가 쥔 정적 객체).
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let opts = CFDictionaryCreate(std::ptr::null(), keys.as_ptr(), values.as_ptr(), 1, &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
        if !opts.is_null() {
            let _ = AXIsProcessTrustedWithOptions(opts);
            CFRelease(opts);
        }
    }
    open_pane(AX_PANE);
}

/// 마이크 권한 상태(묻지 않음). `3` = Authorized, `0` = NotDetermined, 그 외 = Denied/Restricted.
pub fn microphone() -> MicAccess {
    let Some(cls) = AnyClass::get(c"AVCaptureDevice") else {
        return MicAccess::Denied;
    };
    // SAFETY: `AVMediaTypeAudio`는 AVFoundation 정적 NSString 상수.
    let status: NSInteger = unsafe {
        let Some(media) = AVMediaTypeAudio else {
            return MicAccess::Denied;
        };
        msg_send![cls, authorizationStatusForMediaType: media]
    };
    match status {
        3 => MicAccess::Granted,
        0 => MicAccess::NotDetermined,
        _ => MicAccess::Denied,
    }
}

/// 마이크 권한 요청(비동기 시스템 대화상자, 기다리지 않는다).
pub fn request_microphone() {
    let Some(cls) = AnyClass::get(c"AVCaptureDevice") else {
        return;
    };
    // SAFETY: `AVMediaTypeAudio`는 AVFoundation 정적 상수, 블록은 'static.
    unsafe {
        let Some(media) = AVMediaTypeAudio else {
            return;
        };
        let block = RcBlock::new(|_granted: Bool| {});
        let _: () = msg_send![cls, requestAccessForMediaType: media, completionHandler: &*block];
    }
}

/// 시스템 설정 › 개인정보 보호 및 보안 › 마이크 칸을 연다.
pub fn open_microphone_settings() {
    open_pane(MIC_PANE);
}

fn open_pane(url: &str) {
    if let Err(e) = std::process::Command::new("/usr/bin/open").arg(url).spawn() {
        log::warn!("open settings pane failed: {e}");
    }
}
