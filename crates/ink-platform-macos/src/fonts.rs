//! 이 컴퓨터 글꼴 목록(글자 도구 글꼴 고르기 R2-TXT-4).

/// 이 컴퓨터 글꼴 집 이름(`NSFontManager.availableFontFamilies`) — 메인 스레드에서 부른다. 이름은 화면에만 쓴다(로그 금지).
pub fn families() -> Vec<String> {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    // SAFETY: NSFontManager·NSArray·NSString 공개 메서드, 메인 스레드에서 부른다.
    unsafe {
        let fm: *mut AnyObject = msg_send![objc2::class!(NSFontManager), sharedFontManager];
        if fm.is_null() {
            return Vec::new();
        }
        let arr: *mut AnyObject = msg_send![fm, availableFontFamilies];
        if arr.is_null() {
            return Vec::new();
        }
        let n: usize = msg_send![arr, count];
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let s: *mut AnyObject = msg_send![arr, objectAtIndex: i];
            let c: *const std::ffi::c_char = msg_send![s, UTF8String];
            if !c.is_null() {
                out.push(std::ffi::CStr::from_ptr(c).to_string_lossy().into_owned());
            }
        }
        out
    }
}
