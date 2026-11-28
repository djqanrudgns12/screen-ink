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

/// 포커스 변경 없이 물리 픽셀로 계산한 자리에 섬을 보인다.
pub fn show_island(hwnd: isize, target: Option<PhysRect>) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOSIZE, SWP_SHOWWINDOW};
    let Some(r) = target else {
        crate::events::set_window_shown(hwnd, true);
        return;
    };
    // 위치와 크기를 함께 주면 WM_DPICHANGED가 새 물리 크기에 배율을 한 번 더 적용한다.
    // 먼저 이전 크기 그대로 모니터를 건너고, 도착한 뒤 작업 영역에 맞춘다.
    // SAFETY: 두 호출 모두 앱 소유의 섬 창만 옮기며 포커스를 가져오지 않는다.
    unsafe {
        SetWindowPos(hwnd as HWND, HWND_TOPMOST, r.x, r.y, 0, 0, SWP_NOACTIVATE | SWP_NOSIZE);
        SetWindowPos(hwnd as HWND, HWND_TOPMOST, r.x, r.y, r.w, r.h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
}

// ───────────── 창 목록 ─────────────

unsafe extern "system" fn collect_window(h: HWND, data: LPARAM) -> BOOL {
    let list = &mut *(data as *mut Vec<HWND>);
    list.push(h);
    TRUE
}

pub(super) fn class_name(h: HWND) -> String {
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

pub(super) fn frame_bounds(h: HWND) -> Option<RECT> {
    dwm_rect(h).or_else(|| {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        (unsafe { GetWindowRect(h, &mut r) } != 0).then_some(r)
    })
}

pub(super) fn cloaked(h: HWND) -> bool {
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
                let r = if visible { frame_bounds(h) } else { None };
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

// ───────────── 커서 모양(` 키·"커서도 찍기") ─────────────

/// 지금 커서를 검은·흰 바탕에 그려 곧은 알파 BGRA로. 숨은 커서면 None.
pub fn cursor_sprite() -> Option<ink_domain::image::CursorSprite> {
    use windows_sys::Win32::Graphics::Gdi::{GetObjectW, BITMAP};
    use windows_sys::Win32::UI::WindowsAndMessaging::{DrawIconEx, GetCursorInfo, GetIconInfo, CURSORINFO, CURSOR_SHOWING, DI_NORMAL, ICONINFO};
    // SAFETY: 커서 정보 구조체를 채워 받고, 아이콘 비트맵은 다 쓴 뒤 지운다. DIB는 만든 순서 반대로 돌려준다.
    unsafe {
        let mut ci: CURSORINFO = std::mem::zeroed();
        ci.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
        if GetCursorInfo(&mut ci) == 0 || ci.flags & CURSOR_SHOWING == 0 || ci.hCursor.is_null() {
            return None;
        }
        let mut ii: ICONINFO = std::mem::zeroed();
        if GetIconInfo(ci.hCursor, &mut ii) == 0 {
            return None;
        }
        // 크기: 색 비트맵이 있으면 그것, 없으면(흑백 커서) 마스크 높이의 반
        let mut bm: BITMAP = std::mem::zeroed();
        let src = if !ii.hbmColor.is_null() { ii.hbmColor } else { ii.hbmMask };
        GetObjectW(src, std::mem::size_of::<BITMAP>() as i32, &mut bm as *mut _ as *mut c_void);
        let w = bm.bmWidth.max(1);
        let h = if ii.hbmColor.is_null() { (bm.bmHeight / 2).max(1) } else { bm.bmHeight.max(1) };
        let (hx, hy) = (ii.xHotspot as i32, ii.yHotspot as i32);
        if !ii.hbmColor.is_null() {
            DeleteObject(ii.hbmColor);
        }
        if !ii.hbmMask.is_null() {
            DeleteObject(ii.hbmMask);
        }
        let screen = GetDC(null_mut());
        let draw = |fill: u8| -> Option<Vec<u8>> {
            let mem = CreateCompatibleDC(screen);
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB, ..std::mem::zeroed() };
            let mut bits: *mut c_void = null_mut();
            let dib = CreateDIBSection(mem, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if dib.is_null() || bits.is_null() {
                DeleteDC(mem);
                return None;
            }
            let old = SelectObject(mem, dib);
            std::ptr::write_bytes(bits as *mut u8, fill, (w * h * 4) as usize);
            DrawIconEx(mem, 0, 0, ci.hCursor, w, h, 0, null_mut(), DI_NORMAL);
            GdiFlush();
            let px = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize).to_vec();
            SelectObject(mem, old);
            DeleteObject(dib);
            DeleteDC(mem);
            Some(px)
        };
        let (black, white) = (draw(0), draw(255));
        ReleaseDC(null_mut(), screen);
        let bgra = ink_domain::image::alpha_from_two(&black?, &white?);
        Some(ink_domain::image::CursorSprite { x: ci.ptScreenPos.x - hx, y: ci.ptScreenPos.y - hy, w: w as u32, h: h as u32, bgra })
    }
}

// ───────────── H 자식 창 · 커서 · Alt · 지역 시각 ─────────────

unsafe extern "system" fn collect_child(h: HWND, data: LPARAM) -> BOOL {
    let list = &mut *(data as *mut Vec<HWND>);
    if list.len() < 400 {
        list.push(h);
    }
    TRUE
}

/// 보이는 자식 창 사각형(DWM 없이 GetWindowRect — 자식은 물리 px). 다른 앱에 메시지를 보내지 않는다.
pub fn child_rects(parent: u64) -> Vec<PhysRect> {
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumChildWindows;
    let mut handles: Vec<HWND> = Vec::new();
    // SAFETY: 콜백은 이 호출 동안만 `handles`를 쓴다.
    unsafe {
        EnumChildWindows(parent as HWND, Some(collect_child), &mut handles as *mut _ as LPARAM);
    }
    let mut out: Vec<PhysRect> = Vec::new();
    for h in handles {
        // SAFETY: 읽기 전용 질의.
        unsafe {
            if IsWindowVisible(h) == 0 {
                continue;
            }
            let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            if GetWindowRect(h, &mut r) == 0 {
                continue;
            }
            let pr = rect(&r);
            if pr.w >= 8 && pr.h >= 8 && !out.contains(&pr) {
                out.push(pr);
            }
        }
    }
    out
}

pub fn set_cursor(p: PhysPoint) {
    use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;
    // SAFETY: 커서 위치만 바꾼다.
    unsafe { SetCursorPos(p.x, p.y) };
}

pub fn alt_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_MENU};
    // SAFETY: 키 상태 읽기.
    unsafe { (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000) != 0 }
}

/// 지역 시각(이름 규칙 {yyyy}…{ss}).
pub fn local_time() -> ink_domain::naming::NameContext {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: 출력 구조체 하나.
    let t = unsafe {
        let mut t = std::mem::zeroed();
        GetLocalTime(&mut t);
        t
    };
    ink_domain::naming::NameContext { year: t.wYear, month: t.wMonth as u8, day: t.wDay as u8, hour: t.wHour as u8, minute: t.wMinute as u8, second: t.wSecond as u8, app: None, n: 0 }
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

    fn cursor_sprite(&mut self) -> Option<ink_domain::image::CursorSprite> {
        cursor_sprite()
    }
}

impl WindowCatalog for WindowsScreen {
    fn windows(&self) -> Vec<RawWindow> {
        top_windows()
    }

    fn children(&self, window: u64) -> Vec<PhysRect> {
        child_rects(window)
    }
}

impl MonitorProbe for WindowsScreen {
    fn layout(&self) -> MonitorLayout {
        monitor_layout()
    }

    fn cursor(&self) -> PhysPoint {
        cursor_pos()
    }

    fn set_cursor(&self, p: PhysPoint) {
        set_cursor(p)
    }

    fn alt_down(&self) -> bool {
        alt_down()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 섬은_도착_모니터의_배율과_작업_영역을_쓴다() {
        for (work, scale, want) in [
            (PhysRect::new(0, 0, 3840, 2064), 2.0, PhysRect::new(1360, 24, 1120, 176)),
            (PhysRect::new(3840, 0, 1920, 1032), 1.0, PhysRect::new(4520, 12, 560, 88)),
            (PhysRect::new(-1600, -160, 1600, 960), 1.25, PhysRect::new(-1150, -145, 700, 110)),
            (PhysRect::new(80, 40, 500, 90), 1.5, PhysRect::new(80, 40, 500, 90)),
        ] {
            let m = Monitor { id: 0, bounds: work, work, scale, primary: false };
            assert_eq!(m.island_rect((560.0, 88.0), None), want);
        }
    }

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
