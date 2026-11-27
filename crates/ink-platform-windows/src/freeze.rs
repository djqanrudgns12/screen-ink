//! 정지 화면 창 — 모니터마다 네이티브 팝업 하나에 캡처한 BGRA를 GDI로 바로 그린다.
//!
//! 웹뷰로 이미지를 넘기면(사용자 프로토콜) 1080p 한 장에 ~160 ms가 들어 진입 시간 M1(p50 ≤ 80 ms)을
//! 못 맞춘다. 여기서는 `SetDIBitsToDevice`로 그려(1080p ≈ 수 ms) 정지 화면이 즉시 보인다.
//! 정지 화면은 지금 화면과 같아 눈에 띄지 않으므로 처음부터 **50% 어둡게**(S07) 그리고, 강조 영역만
//! 밝은 원본으로 뚫는다 — 단계 3의 강조 이동도 GDI 사각형 복사 한 번이라 빠르다.
//! 웹뷰 오버레이(투명)는 그 위에서 테두리·꺾쇠·확대경·크기 표만 그린다. 이 창은 입력을 받지 않는다.
//! 창은 반드시 메인(이벤트 루프) 스레드에서 만들고 쓴다.

use ink_domain::geometry::PhysRect;
use std::collections::HashMap;
use std::ptr::{null, null_mut};
use std::sync::{Arc, Mutex, OnceLock};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, InvalidateRect, RedrawWindow, SetDIBitsToDevice, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, PAINTSTRUCT,
    RDW_INVALIDATE, RDW_UPDATENOW,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SetWindowPos, ShowWindow, HTTRANSPARENT, HWND_TOPMOST, MA_NOACTIVATE, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SW_HIDE, WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_NCHITTEST, WM_PAINT, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

struct Painted {
    bright: Arc<Vec<u8>>,
    dim: Vec<u8>,
    w: i32,
    h: i32,
    /// 밝게 뚫을 사각형(모니터 기준 물리 px)
    highlight: Option<PhysRect>,
}

fn frames() -> &'static Mutex<HashMap<isize, Painted>> {
    static F: OnceLock<Mutex<HashMap<isize, Painted>>> = OnceLock::new();
    F.get_or_init(Default::default)
}

/// 50% 어둡게: 채널마다 반(>>1). 픽셀 4바이트를 u32 하나로 — 1080p ≈ 1 ms(최적화 빌드).
pub fn dim_half(bgra: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bgra.len()];
    for (o, i) in out.chunks_exact_mut(4).zip(bgra.chunks_exact(4)) {
        let p = u32::from_le_bytes([i[0], i[1], i[2], i[3]]);
        o.copy_from_slice(&((p >> 1) & 0x7F7F_7F7F).to_le_bytes());
    }
    out
}

fn class_name() -> &'static [u16] {
    static C: OnceLock<Vec<u16>> = OnceLock::new();
    C.get_or_init(|| "ScreenInkFreeze\0".encode_utf16().collect())
}

fn ensure_class() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        // SAFETY: 클래스 이름은 'static 버퍼, 창 프로시저는 아래 함수.
        unsafe {
            let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: GetModuleHandleW(null()), lpszClassName: class_name().as_ptr(), ..std::mem::zeroed() };
            RegisterClassW(&wc);
        }
    });
}

/// `buf`(w×h BGRA)의 사각형 `r`을 같은 자리에 그린다.
unsafe fn blit(dc: HDC, buf: &[u8], w: i32, h: i32, r: PhysRect) {
    let r = match PhysRect::new(0, 0, w, h).intersect(&r) {
        Some(r) => r,
        None => return,
    };
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        biHeight: -r.h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..std::mem::zeroed()
    };
    // 원본 행 너비를 유지하면서 그릴 줄만 위→아래 DIB로 넘긴다. 이 띠 안의 원본 y는 0이다.
    let start = r.y as usize * w as usize * 4;
    SetDIBitsToDevice(dc, r.x, r.y, r.w as u32, r.h as u32, r.x, 0, 0, r.h as u32, buf[start..].as_ptr().cast(), &bmi, DIB_RGB_COLORS);
}

unsafe extern "system" fn wndproc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(h, &mut ps);
            if let Some(p) = frames().lock().unwrap_or_else(|e| e.into_inner()).get(&(h as isize)) {
                let dirty = PhysRect::from_edges(ps.rcPaint.left, ps.rcPaint.top, ps.rcPaint.right, ps.rcPaint.bottom);
                let bright = p.highlight.and_then(|r| r.intersect(&dirty));
                // 첫 모니터 전체 강조처럼 모두 원본으로 덮일 때는 4K 어두운 복사를 생략한다.
                if bright != Some(dirty) {
                    blit(dc, &p.dim, p.w, p.h, dirty);
                }
                if let Some(hl) = bright {
                    blit(dc, &p.bright, p.w, p.h, hl);
                }
            }
            EndPaint(h, &ps);
            0
        }
        WM_ERASEBKGND => 1,
        WM_NCHITTEST => HTTRANSPARENT as LRESULT,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        _ => DefWindowProcW(h, msg, w, l),
    }
}

fn invalidate(hwnd: isize, r: PhysRect) {
    let rc = RECT { left: r.x, top: r.y, right: r.right(), bottom: r.bottom() };
    // SAFETY: 우리 창의 일부 다시 그리기 요청.
    unsafe { InvalidateRect(hwnd as HWND, &rc, 0) };
}

/// 모니터 id → 정지 화면 창. 메인 스레드 전용(HWND는 정수로만 들고 다닌다).
#[derive(Default)]
pub struct Freeze {
    windows: HashMap<u32, isize>,
}

impl Freeze {
    fn window(&mut self, monitor: u32) -> Option<isize> {
        if let Some(h) = self.windows.get(&monitor) {
            return Some(*h);
        }
        ensure_class();
        // SAFETY: 이 스레드에서 만들고 이 스레드에서만 쓴다.
        let h = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name().as_ptr(),
                class_name().as_ptr(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            )
        };
        if h.is_null() {
            return None;
        }
        self.windows.insert(monitor, h as isize);
        Some(h as isize)
    }

    /// 50% 어둡게 한 정지 화면(+ 처음 강조만 밝게)을 그 모니터 위에 즉시(동기 그리기) 보인다. 성공하면 true.
    /// `highlight`는 모니터 기준 물리 px.
    pub fn show(&mut self, monitor: u32, bounds: PhysRect, bgra: Arc<Vec<u8>>, w: u32, h: u32, highlight: Option<PhysRect>) -> bool {
        let Some(hwnd) = self.window(monitor) else { return false };
        if bgra.len() < (w as usize) * (h as usize) * 4 {
            return false;
        }
        let dim = dim_half(&bgra);
        frames().lock().unwrap_or_else(|e| e.into_inner()).insert(hwnd, Painted { bright: bgra, dim, w: w as i32, h: h as i32, highlight });
        // SAFETY: 우리 창 하나의 위치·표시·다시 그리기.
        unsafe {
            SetWindowPos(hwnd as HWND, HWND_TOPMOST, bounds.x, bounds.y, bounds.w, bounds.h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            RedrawWindow(hwnd as HWND, null(), null_mut(), RDW_INVALIDATE | RDW_UPDATENOW) != 0
        }
    }

    /// 강조(밝게 뚫기)를 옮긴다 — 바뀐 두 사각형만 다시 그린다. `rect`는 모니터 기준 물리 px.
    pub fn set_highlight(&mut self, monitor: u32, rect: Option<PhysRect>) {
        let Some(&hwnd) = self.windows.get(&monitor) else { return };
        let old = {
            let mut f = frames().lock().unwrap_or_else(|e| e.into_inner());
            let Some(p) = f.get_mut(&hwnd) else { return };
            std::mem::replace(&mut p.highlight, rect)
        };
        if old == rect {
            return;
        }
        for r in [old, rect].into_iter().flatten() {
            invalidate(hwnd, r);
        }
        // SAFETY: 우리 창 즉시 그리기.
        unsafe { UpdateWindow(hwnd as HWND) };
    }

    /// 모두 숨기고 그림을 버린다.
    pub fn hide_all(&mut self) {
        let mut f = frames().lock().unwrap_or_else(|e| e.into_inner());
        for h in self.windows.values() {
            // SAFETY: 우리 창 숨기기.
            unsafe { ShowWindow(*h as HWND, SW_HIDE) };
            f.remove(h);
        }
    }

    /// 배치에 없는 모니터의 창을 닫는다.
    pub fn retain(&mut self, monitors: &[u32]) {
        let gone: Vec<u32> = self.windows.keys().filter(|m| !monitors.contains(m)).copied().collect();
        for m in gone {
            if let Some(h) = self.windows.remove(&m) {
                frames().lock().unwrap_or_else(|e| e.into_inner()).remove(&h);
                // SAFETY: 우리 창 닫기.
                unsafe { DestroyWindow(h as HWND) };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 반_어둡게는_채널마다_반() {
        let src = [200u8, 100, 51, 255, 0, 1, 2, 3];
        assert_eq!(dim_half(&src), vec![100, 50, 25, 127, 0, 0, 1, 1]);
        let big = vec![255u8; 1920 * 1080 * 4];
        let t = std::time::Instant::now();
        let d = dim_half(&big);
        eprintln!("dim 1080p in {:?}", t.elapsed());
        assert_eq!(d[0], 127);
    }
}
