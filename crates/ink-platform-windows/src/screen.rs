//! 모니터 배치·창 목록·GDI 정지 캡처(03 §3 `MonitorProbe`·`WindowCatalog`·`ScreenSource`).
//! 프로세스는 Per-Monitor V2(Tauri 매니페스트)라 모든 좌표가 물리 px이다(R-DPI-1).
//! 창 제목은 읽지 않는다 — 실행 파일 이름과 창 클래스만(05 개인정보).

use ink_app::{Frame, MonitorProbe, ScreenSource, WindowCatalog};
use ink_domain::geometry::{Monitor, MonitorLayout, PhysPoint, PhysRect};
use ink_domain::snapshot::RawWindow;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT, TRUE};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EnumDisplayMonitors, GdiFlush, GetDC, GetMonitorInfoW, ReleaseDC, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HDC, HMONITOR, MONITORINFO, SRCCOPY,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetCursorPos, GetLayeredWindowAttributes, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsHungAppWindow, IsIconic,
    IsWindowVisible, IsZoomed, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
};

/// `MONITORINFO.dwFlags`의 주 모니터 표시.
const MONITORINFOF_PRIMARY: u32 = 1;

fn rect(r: &RECT) -> PhysRect {
    PhysRect::from_edges(r.left, r.top, r.right, r.bottom)
}

// ───────────── 모니터 ─────────────

unsafe extern "system" fn collect_monitor(h: HMONITOR, _dc: HDC, _r: *mut RECT, data: LPARAM) -> BOOL {
    let list = &mut *(data as *mut Vec<HMONITOR>);
    list.push(h);
    TRUE
}

/// 모니터 배치. id = 왼쪽→오른쪽·위→아래 순서(오버레이 창 이름 `ink-overlay-<id>`와 같음).
pub fn monitor_layout() -> MonitorLayout {
    let mut handles: Vec<HMONITOR> = Vec::new();
    // SAFETY: 콜백은 이 호출 동안만 `handles`를 쓴다.
    unsafe {
        EnumDisplayMonitors(null_mut(), null(), Some(collect_monitor), &mut handles as *mut _ as LPARAM);
    }
    let mut monitors: Vec<Monitor> = handles
        .into_iter()
        .filter_map(|h| {
            // SAFETY: 구조체 크기를 채워 넘긴다.
            unsafe {
                let mut info: MONITORINFO = std::mem::zeroed();
                info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                if GetMonitorInfoW(h, &mut info) == 0 {
                    return None;
                }
                let (mut dx, mut dy) = (96u32, 96u32);
                let scale = if GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) == 0 { dx as f64 / 96.0 } else { 1.0 };
                Some(Monitor { id: 0, bounds: rect(&info.rcMonitor), work: rect(&info.rcWork), scale, primary: info.dwFlags & MONITORINFOF_PRIMARY != 0 })
            }
        })
        .collect();
    monitors.sort_by_key(|m| (m.bounds.x, m.bounds.y));
    for (i, m) in monitors.iter_mut().enumerate() {
        m.id = i as u32;
    }
    MonitorLayout::new(monitors)
}

pub fn cursor_pos() -> PhysPoint {
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: 출력 하나.
    unsafe { GetCursorPos(&mut p) };
    PhysPoint { x: p.x, y: p.y }
}

// ───────────── 창 목록 ─────────────

unsafe extern "system" fn collect_window(h: HWND, data: LPARAM) -> BOOL {
    let list = &mut *(data as *mut Vec<HWND>);
    list.push(h);
    TRUE
}

fn class_name(h: HWND) -> String {
    let mut buf = [0u16; 128];
    // SAFETY: 버퍼 길이를 함께 넘긴다.
    let n = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn dwm_rect(h: HWND) -> Option<RECT> {
    let mut r: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: RECT 크기를 넘긴다.
    let hr = unsafe { DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS as _, &mut r as *mut _ as *mut c_void, std::mem::size_of::<RECT>() as u32) };
    (hr == 0).then_some(r)
}

fn cloaked(h: HWND) -> bool {
    let mut v: u32 = 0;
    // SAFETY: DWORD 하나.
    let hr = unsafe { DwmGetWindowAttribute(h, DWMWA_CLOAKED as _, &mut v as *mut _ as *mut c_void, 4) };
    hr == 0 && v != 0
}

/// Z 순서(앞 → 뒤) 최상위 창 전부. 거르기는 도메인(`snapshot::filter_windows`)이 한다.
pub fn top_windows() -> Vec<RawWindow> {
    let mut handles: Vec<HWND> = Vec::new();
    // SAFETY: 콜백은 이 호출 동안만 `handles`를 쓴다.
    unsafe {
        EnumWindows(Some(collect_window), &mut handles as *mut _ as LPARAM);
    }
    let exe_of = process_exe_map();
    handles
        .into_iter()
        .map(|h| {
            // SAFETY: 모두 읽기 전용 질의, 창이 그 사이 사라지면 0·false가 돌아올 뿐이다.
            unsafe {
                let visible = IsWindowVisible(h) != 0;
                let ex = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
                let mut pid = 0u32;
                GetWindowThreadProcessId(h, &mut pid);
                let layered_alpha = if ex & WS_EX_LAYERED != 0 {
                    let (mut key, mut alpha, mut flags) = (0u32, 255u8, 0u32);
                    // LWA_ALPHA(2)일 때만 투명도가 뜻이 있다. UpdateLayeredWindow 창은 실패 → 불투명으로 본다
                    if GetLayeredWindowAttributes(h, &mut key, &mut alpha, &mut flags) != 0 && flags & 2 != 0 {
                        Some(alpha)
                    } else {
                        Some(255)
                    }
                } else {
                    None
                };
                let r = if visible {
                    dwm_rect(h).or_else(|| {
                        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                        (GetWindowRect(h, &mut r) != 0).then_some(r)
                    })
                } else {
                    None
                };
                RawWindow {
                    handle: h as usize as u64,
                    pid,
                    exe: exe_of.get(&pid).cloned().unwrap_or_default(),
                    class: if visible { class_name(h) } else { String::new() },
                    rect: r.as_ref().map(rect).unwrap_or_default(),
                    visible,
                    minimized: visible && IsIconic(h) != 0,
                    maximized: visible && IsZoomed(h) != 0,
                    cloaked: visible && cloaked(h),
                    hung: visible && IsHungAppWindow(h) != 0,
                    click_through: ex & WS_EX_TRANSPARENT != 0,
                    layered_alpha,
                    topmost: ex & WS_EX_TOPMOST != 0,
                    tool: ex & WS_EX_TOOLWINDOW != 0,
                }
            }
        })
        .collect()
}

/// pid → 실행 파일 이름(소문자).
pub fn process_exe_map() -> HashMap<u32, String> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    let mut out = HashMap::new();
    // SAFETY: 스냅숏 핸들은 여기서만 쓰고 닫는다.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut e);
        while ok != 0 {
            let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
            out.insert(e.th32ProcessID, String::from_utf16_lossy(&e.szExeFile[..len]).to_lowercase());
            ok = Process32NextW(snap, &mut e);
        }
        CloseHandle(snap);
    }
    out
}

// ───────────── 정지 캡처(GDI BitBlt) ─────────────

/// 모니터 한 장을 BGRA(위→아래, 헤더 없음)로. CAPTUREBLT = 계층 창(메뉴·툴팁 그림자)까지 찍는다.
pub fn capture_monitor(bounds: PhysRect) -> Result<Vec<u8>, String> {
    let (w, h) = (bounds.w, bounds.h);
    if w <= 0 || h <= 0 {
        return Err("empty monitor".into());
    }
    // SAFETY: GDI 객체는 만든 순서의 반대로 모두 돌려준다. 비트 포인터는 DIB가 살아 있는 동안만 읽는다.
    unsafe {
        let screen = GetDC(null_mut());
        if screen.is_null() {
            return Err("GetDC failed".into());
        }
        let mem = CreateCompatibleDC(screen);
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut bits: *mut c_void = null_mut();
        let dib = CreateDIBSection(mem, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        let result = if dib.is_null() || bits.is_null() {
            Err("CreateDIBSection failed".to_string())
        } else {
            let old = SelectObject(mem, dib);
            let ok = BitBlt(mem, 0, 0, w, h, screen, bounds.x, bounds.y, SRCCOPY | CAPTUREBLT) != 0;
            GdiFlush();
            let r = if ok {
                let px = std::slice::from_raw_parts(bits as *const u8, (w as usize) * (h as usize) * 4);
                Ok(px.to_vec())
            } else {
                Err("BitBlt failed".to_string())
            };
            SelectObject(mem, old);
            DeleteObject(dib);
            r
        };
        DeleteDC(mem);
        ReleaseDC(null_mut(), screen);
        result
    }
}

/// `ScreenSource` + `WindowCatalog` + `MonitorProbe` 한 몸.
#[derive(Default)]
pub struct WindowsScreen;

impl ScreenSource for WindowsScreen {
    fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String> {
        // 모니터가 둘 이상이면 나란히 찍는다(M1 캡처 단계 ≤ 55 ms, PRD 6절 개선 ①)
        let jobs: Vec<_> = layout.monitors.iter().map(|m| (m.id, m.bounds)).collect();
        if jobs.len() == 1 {
            let (id, b) = jobs[0];
            return Ok(vec![Frame { monitor_id: id, width: b.w as u32, height: b.h as u32, bgra: std::sync::Arc::new(capture_monitor(b)?) }]);
        }
        let handles: Vec<_> = jobs.into_iter().map(|(id, b)| std::thread::spawn(move || (id, b, capture_monitor(b)))).collect();
        let mut out = Vec::new();
        for h in handles {
            let (id, b, r) = h.join().map_err(|_| "capture thread panicked".to_string())?;
            out.push(Frame { monitor_id: id, width: b.w as u32, height: b.h as u32, bgra: std::sync::Arc::new(r?) });
        }
        Ok(out)
    }
}

impl WindowCatalog for WindowsScreen {
    fn windows(&self) -> Vec<RawWindow> {
        top_windows()
    }
}

impl MonitorProbe for WindowsScreen {
    fn layout(&self) -> MonitorLayout {
        monitor_layout()
    }

    fn cursor(&self) -> PhysPoint {
        cursor_pos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 이_pc의_모니터와_창과_캡처() {
        let l = monitor_layout();
        assert!(!l.monitors.is_empty());
        assert_eq!(l.monitors.iter().filter(|m| m.primary).count(), 1);
        let ws = top_windows();
        assert!(!ws.is_empty());
        assert!(ws.iter().all(|w| !w.exe.contains('\\')), "경로 없이 이름만");
        let m = &l.monitors[0];
        let t = std::time::Instant::now();
        let bgra = capture_monitor(m.bounds).expect("GDI 캡처");
        eprintln!("capture {}x{} in {:?}", m.bounds.w, m.bounds.h, t.elapsed());
        assert_eq!(bgra.len(), (m.bounds.w * m.bounds.h * 4) as usize);
    }
}
