//! 별도 프로세스에 직접 만든 시험 창만 고정한다. 화면이나 창 제목을 수집하지 않는다.
use super::*;
use std::{
    process::{Child, Command},
    ptr::{null, null_mut},
    time::{Duration, Instant},
};
use windows_sys::Win32::Graphics::Dwm::DwmGetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::UpdateWindow;
use windows_sys::Win32::{Foundation::RECT, Graphics::Dwm::DWMWA_EXTENDED_FRAME_BOUNDS, System::LibraryLoader::GetModuleHandleW, UI::HiDpi::*};

const CASE: &str = "topmost::tests::실제_창_고정과_테두리_추적";
const ROLE: &str = "SCREEN_INK_W4_TEST_ROLE";
const STYLE: BorderStyle = BorderStyle {
    thickness_px: 4,
    color_rgb: 0xff9f0a,
};

struct Started(Child);
impl Drop for Started {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn helper(mode: &str, window: u64) -> Started {
    Started(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CASE, "--ignored", "--nocapture"])
            .env(ROLE, mode)
            .env("SCREEN_INK_W4_TEST_TARGET", window.to_string())
            .spawn()
            .unwrap(),
    )
}
unsafe fn fixture() {
    let class = wide(&format!("ScreenInkW4Test{}", std::process::id()));
    let inst = GetModuleHandleW(null());
    let wc = WNDCLASSW {
        lpfnWndProc: Some(DefWindowProcW),
        hInstance: inst,
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    assert_ne!(RegisterClassW(&wc), 0);
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        wide("슉쇽 창 고정 시험").as_ptr(),
        WS_OVERLAPPEDWINDOW,
        110,
        110,
        400,
        210,
        null_mut(),
        null_mut(),
        inst,
        null(),
    );
    assert!(!hwnd.is_null());
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
    // 처음 표시하는 메시지 처리가 끝난 뒤 사용자 입력을 받을 준비가 된 창을 시험한다.
    PostMessageW(hwnd, WM_APP, 0, 0);
    let mut msg: MSG = std::mem::zeroed();
    while IsWindow(hwnd) != 0 && GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        if msg.message == WM_APP {
            SetPropW(hwnd, wide("ScreenInkTestReady").as_ptr(), 1usize as _);
        }
        if msg.message == WM_APP + 1 {
            SetForegroundWindow(msg.wParam as HWND);
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}
fn wait(reason: &str, mut check: impl FnMut() -> bool) -> Duration {
    let start = Instant::now();
    while !check() {
        assert!(start.elapsed() < Duration::from_secs(4), "{reason}");
        std::thread::yield_now();
    }
    start.elapsed()
}

#[test]
#[ignore = "로그인된 Windows 화면에서 별도 시험 창을 띄운다"]
fn 실제_창_고정과_테두리_추적() {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        match std::env::var(ROLE).as_deref() {
            Ok("창") => {
                fixture();
                return;
            }
            Ok("중단") => {
                let h = std::env::var("SCREEN_INK_W4_TEST_TARGET").unwrap().parse::<u64>().unwrap();
                let mut port = WinWindowPinner::default();
                port.pin(h, STYLE).unwrap();
                std::process::exit(0);
            }
            _ => {}
        }
        let mut child = helper("창", 0);
        let class = wide(&format!("ScreenInkW4Test{}", child.0.id()));
        let mut hwnd = null_mut();
        wait("시험 창 열기", || {
            hwnd = FindWindowW(class.as_ptr(), null());
            !GetPropW(hwnd, wide("ScreenInkTestReady").as_ptr()).is_null()
        });
        let h = hwnd as u64;
        let mut port = WinWindowPinner::default();
        assert_eq!(port.candidate(h).unwrap().pid, child.0.id());
        for round in 1..=100 {
            port.pin(h, STYLE).unwrap_or_else(|e| panic!("{round}번째 고정 실패 {e:?}"));
            assert!(port.candidate(h).unwrap().topmost);
            port.unpin(h).unwrap();
            let w = port.candidate(h).unwrap();
            assert!(!w.topmost && !w.has_our_prop);
        }
        port.pin(h, STYLE).unwrap();
        let border = GetWindow(hwnd, GW_HWNDPREV);
        let mut pid = 0;
        GetWindowThreadProcessId(border, &mut pid);
        assert_eq!(pid, std::process::id());
        assert_ne!(GetWindowLongW(border, GWL_EXSTYLE) as u32 & WS_EX_TRANSPARENT, 0);
        let mut affinity = 0;
        assert_ne!(GetWindowDisplayAffinity(border, &mut affinity), 0);
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE);
        let tracks = || {
            let mut frame: RECT = std::mem::zeroed();
            let mut ring: RECT = std::mem::zeroed();
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS as u32,
                (&mut frame as *mut RECT).cast(),
                size_of::<RECT>() as u32,
            ) >= 0
                && GetWindowRect(border, &mut ring) != 0
                && (frame.left, frame.top, frame.right, frame.bottom) == (ring.left, ring.top, ring.right, ring.bottom)
        };
        // 같은 창 영역을 F1의 GDI 경로로 읽고 픽셀 수만 남긴다.
        let orange_pixels = || {
            windows_sys::Win32::Graphics::Dwm::DwmFlush();
            let mut r: RECT = std::mem::zeroed();
            assert_ne!(GetWindowRect(border, &mut r), 0);
            let bounds = ink_domain::geometry::PhysRect::from_edges(r.left, r.top, r.right, r.bottom);
            crate::screen::capture_monitor(bounds)
                .unwrap()
                .chunks_exact(4)
                .filter(|px| px[..3] == [0x0a, 0x9f, 0xff])
                .count()
        };
        assert_ne!(SetWindowDisplayAffinity(border, WDA_NONE), 0);
        wait("테두리 주황색이 실제로 그려짐", || orange_pixels() > 100);
        assert_ne!(SetWindowDisplayAffinity(border, WDA_EXCLUDEFROMCAPTURE), 0);
        wait("F1 캡처에서 주황 테두리 0픽셀", || orange_pixels() == 0);
        let mut initial: RECT = std::mem::zeroed();
        let mut initial_border: RECT = std::mem::zeroed();
        assert_ne!(GetWindowRect(hwnd, &mut initial), 0);
        assert_ne!(GetWindowRect(border, &mut initial_border), 0);
        let mut worst = Duration::ZERO;
        for i in 0..12 {
            let started = Instant::now();
            assert_ne!(
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    130 + 3 * i,
                    145 + 2 * i,
                    400 + i,
                    220 + i,
                    SWP_NOZORDER | SWP_NOACTIVATE
                ),
                0
            );
            wait("새 위치와 크기에 테두리 도착", || {
                let mut r: RECT = std::mem::zeroed();
                GetWindowRect(border, &mut r) != 0 && tracks()
                    && r.left == 130 + 3 * i + initial_border.left - initial.left
                    && r.top == 145 + 2 * i + initial_border.top - initial.top
                    && r.right == 130 + 3 * i + 400 + i + initial_border.right - initial.right
                    && r.bottom == 145 + 2 * i + 220 + i + initial_border.bottom - initial.bottom
            });
            worst = worst.max(started.elapsed());
        }
        eprintln!("이동·크기 변경 12회 중 가장 긴 테두리 대기 {worst:?}");
        assert!(worst <= Duration::from_millis(16));
        ShowWindow(hwnd, SW_MAXIMIZE);
        wait("최대화 때 테두리 추적", || IsZoomed(hwnd) != 0 && tracks());
        ShowWindow(hwnd, SW_HIDE);
        wait("숨길 때 테두리 숨김", || IsWindowVisible(border) == 0);
        ShowWindow(hwnd, SW_RESTORE);
        wait("다시 열 때 테두리 표시", || IsWindowVisible(border) != 0 && tracks());
        ShowWindow(hwnd, SW_MINIMIZE);
        wait("최소화 때 테두리 숨김", || IsWindowVisible(border) == 0);
        ShowWindow(hwnd, SW_RESTORE);
        wait("복원 때 테두리 표시", || IsWindowVisible(border) != 0 && tracks());
        port.update_border_style(BorderStyle { thickness_px: 0, ..STYLE });
        wait("테두리 끄기", || IsWindowVisible(border) == 0);
        port.update_border_style(STYLE);
        wait("테두리 다시 켜기", || IsWindowVisible(border) != 0);
        let mut second = helper("창", 0);
        let second_class = wide(&format!("ScreenInkW4Test{}", second.0.id()));
        let mut other = null_mut();
        wait("두 번째 시험 창 열기", || {
            other = FindWindowW(second_class.as_ptr(), null());
            !GetPropW(other, wide("ScreenInkTestReady").as_ptr()).is_null()
        });
        PostMessageW(hwnd, WM_APP + 1, other as usize, 0);
        wait("두 번째 창 활성화", || GetForegroundWindow() == other);
        assert!(port.candidate(h).unwrap().topmost);
        assert_eq!(port.foreground().unwrap().pid, second.0.id());
        port.pin(other as u64, STYLE).unwrap();
        port.unpin_all();
        assert!(!port.candidate(other as u64).unwrap().topmost);
        assert!(!port.candidate(h).unwrap().topmost);
        PostMessageW(other, WM_CLOSE, 0, 0);
        let _ = second.0.wait();
        assert_eq!(IsWindow(border), 0);
        SetPropW(hwnd, wide(PROP_POWERTOYS).as_ptr(), 1usize as _);
        assert_eq!(
            decide(&port.candidate(h).unwrap(), std::process::id()),
            TopmostDecision::Skip("PowerToys가 고정한 창이에요")
        );
        assert!(!port.candidate(h).unwrap().topmost);
        RemovePropW(hwnd, wide(PROP_POWERTOYS).as_ptr());
        let mut stopped = helper("중단", h);
        assert!(stopped.0.wait().unwrap().success());
        assert!(port.candidate(h).unwrap().topmost);
        // 실제 사용자 창은 정리하지 않도록 방금 중단한 프로세스의 표시로만 열거한다.
        assert_eq!(sweep(Some(stopped.0.id() as usize)), 1);
        assert!(!port.candidate(h).unwrap().topmost);
        port.pin(h, STYLE).unwrap();
        let ring = GetWindow(hwnd, GW_HWNDPREV);
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
        wait("대상 닫힘", || IsWindow(hwnd) == 0);
        wait("테두리 자동 제거", || IsWindow(ring) == 0);
        assert!(!port.is_alive(h));
        let _ = child.0.wait();
    }
}
