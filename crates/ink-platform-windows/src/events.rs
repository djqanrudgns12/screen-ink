//! 시스템 사건(잠금·절전·디스플레이 변경·테마)과 보안 데스크톱·캡처 제외(R-SYS-3, R-UI-5).
//! 사건은 숨은 최상위 창 하나(전용 스레드 + 메시지 루프)로 받는다 — 방송 메시지는 메시지 전용 창에 오지 않는다.

use ink_app::SystemEvent;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::RemoteDesktop::{WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION};
use windows_sys::Win32::System::StationsAndDesktops::{CloseDesktop, GetUserObjectInformationW, OpenInputDesktop, UOI_NAME};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW, SetWindowDisplayAffinity, TranslateMessage, MSG, WNDCLASSW,
};

const WM_DISPLAYCHANGE: u32 = 0x007E;
const WM_SETTINGCHANGE: u32 = 0x001A;
const WM_DPICHANGED: u32 = 0x02E0;
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
const WM_POWERBROADCAST: u32 = 0x0218;
const WTS_SESSION_LOCK: usize = 0x7;
const WTS_SESSION_UNLOCK: usize = 0x8;
const PBT_APMSUSPEND: usize = 0x4;
const PBT_APMRESUMEAUTOMATIC: usize = 0x12;
const WDA_NONE: u32 = 0x0;
const WDA_EXCLUDEFROMCAPTURE: u32 = 0x11;

type Callback = Box<dyn Fn(SystemEvent) + Send + Sync>;
static CALLBACK: OnceLock<Callback> = OnceLock::new();

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn fire(ev: SystemEvent) {
    if let Some(cb) = CALLBACK.get() {
        cb(ev);
    }
}

unsafe extern "system" fn wndproc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_DISPLAYCHANGE | WM_DPICHANGED => fire(SystemEvent::DisplayChange),
        WM_SETTINGCHANGE => {
            // lParam = 바뀐 영역 이름. 테마면 트레이 색만, 그 밖은 배치 확인(지문이 같으면 서비스가 아무것도 안 함)
            let area = if l != 0 { read_wide(l as *const u16) } else { String::new() };
            fire(if area == "ImmersiveColorSet" { SystemEvent::ThemeChange } else { SystemEvent::DisplayChange });
        }
        WM_WTSSESSION_CHANGE => match w {
            WTS_SESSION_LOCK => fire(SystemEvent::Lock),
            WTS_SESSION_UNLOCK => fire(SystemEvent::Unlock),
            _ => {}
        },
        WM_POWERBROADCAST => {
            match w {
                PBT_APMSUSPEND => fire(SystemEvent::Suspend),
                PBT_APMRESUMEAUTOMATIC => fire(SystemEvent::Resume),
                _ => {}
            }
            return 1;
        }
        _ => {}
    }
    DefWindowProcW(h, msg, w, l)
}

unsafe fn read_wide(p: *const u16) -> String {
    let mut n = 0;
    while n < 256 && *p.add(n) != 0 {
        n += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
}

/// 사건 듣기를 시작한다(한 번만). `cb`는 메시지 스레드에서 불리므로 빨리 돌아와야 한다.
pub fn listen(cb: impl Fn(SystemEvent) + Send + Sync + 'static) -> bool {
    if CALLBACK.set(Box::new(cb)).is_err() {
        return false;
    }
    std::thread::Builder::new()
        .name("screen-ink-system-events".into())
        .spawn(|| {
            // SAFETY: 창 클래스·창·메시지 루프는 이 스레드 안에서만 쓴다.
            unsafe {
                let inst = GetModuleHandleW(null());
                let class = wide("ScreenInkSystemEvents");
                let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: inst, lpszClassName: class.as_ptr(), ..std::mem::zeroed() };
                RegisterClassW(&wc);
                // 보이지 않는 최상위 창(WS_OVERLAPPED, 크기 0) — 방송 메시지를 받으려면 최상위여야 한다
                let hwnd = CreateWindowExW(0, class.as_ptr(), class.as_ptr(), 0, 0, 0, 0, 0, null_mut(), null_mut(), inst, null());
                if hwnd.is_null() {
                    log::warn!("system event window failed");
                    return;
                }
                WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        })
        .is_ok()
}

/// UAC·잠금 화면 같은 보안 데스크톱이 입력을 쥐고 있는지. 입력 데스크톱을 못 열거나 이름이 Default가 아니면 true.
pub fn secure_desktop_active() -> bool {
    // SAFETY: 데스크톱 핸들은 열고 바로 닫는다. 이름 버퍼 크기를 함께 넘긴다.
    unsafe {
        let desk = OpenInputDesktop(0, 0, 0x0001 /* DESKTOP_READOBJECTS */);
        if desk.is_null() {
            return true;
        }
        let mut name = [0u16; 64];
        let mut needed = 0u32;
        let ok = GetUserObjectInformationW(desk, UOI_NAME, name.as_mut_ptr().cast(), (name.len() * 2) as u32, &mut needed) != 0;
        CloseDesktop(desk);
        if !ok {
            return false;
        }
        let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        !String::from_utf16_lossy(&name[..len]).eq_ignore_ascii_case("Default")
    }
}

/// 오버레이 창을 Win32로 직접 보이기·숨기기. Tauri의 숨기기는 WebView2 컨트롤러 표시 상태까지 꺼서
/// 렌더러를 멈추고, 보일 때 깨우는 데 창마다 수십 ms가 든다. 여기서는 창만 숨겨 웹뷰는 계속 '보이는 중'으로 두어,
/// 숨은 동안에도 다음 정지 이미지를 받아 그려 둘 수 있게 한다(진입 시간 M1).
pub fn set_window_shown(hwnd: isize, show: bool) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE};
    // SAFETY: 창 핸들 하나의 표시 상태만 바꾼다.
    unsafe {
        if show {
            SetWindowPos(hwnd as HWND, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW) != 0
        } else {
            ShowWindow(hwnd as HWND, SW_HIDE) != 0
        }
    }
}

/// 키 입력을 받도록 앞으로(단축키를 받은 프로세스라 전경 전환이 허용된다).
pub fn focus_window(hwnd: isize) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
    // SAFETY: 창 핸들 하나.
    unsafe { SetForegroundWindow(hwnd as HWND) != 0 }
}

/// 창 캡처를 열기 직전 전경에 있던 외부 창. 창 제목은 읽지 않는다.
pub struct ForegroundWindow {
    hwnd: isize,
    process: u32,
    thread: u32,
}

impl ForegroundWindow {
    pub fn current_external() -> Option<Self> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
        let mut process = 0;
        // SAFETY: 전경 창의 핸들과 스레드·프로세스 번호를 읽기만 한다.
        let (hwnd, thread) = unsafe {
            let hwnd = GetForegroundWindow();
            (hwnd, GetWindowThreadProcessId(hwnd, &mut process))
        };
        (thread != 0 && process != 0 && process != std::process::id()).then_some(Self { hwnd: hwnd as isize, process, thread })
    }

    /// 캡처 창이 여전히 입력을 받는 경우에만 창을 돌려준다. 사용자가 그 사이 다른 창을 열었다면 그대로 둔다.
    pub fn restore_from(self, capture_windows: impl IntoIterator<Item = isize>) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible};
        // SAFETY: 현재 전경 창과 보관했던 창의 표시 상태·소유자를 읽는다.
        unsafe {
            let active = GetForegroundWindow() as isize;
            if !capture_windows.into_iter().any(|hwnd| hwnd == active) {
                return;
            }
            let hwnd = self.hwnd as HWND;
            let mut process = 0;
            if GetWindowThreadProcessId(hwnd, &mut process) != self.thread
                || process != self.process
                || IsWindowVisible(hwnd) == 0
                || IsIconic(hwnd) != 0
                || crate::screen::cloaked(hwnd)
            {
                return;
            }
        }
        focus_window(self.hwnd);
    }
}

/// 창을 화면 캡처·녹화에서 뺀다(Windows 10 2004+). 섬·팝오버·녹화 테두리·고정 테두리에 쓴다.
pub fn exclude_from_capture(hwnd: isize, exclude: bool) -> bool {
    // SAFETY: 창 핸들 하나에 표시 속성만 바꾼다.
    unsafe { SetWindowDisplayAffinity(hwnd as HWND, if exclude { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE }) != 0 }
}

#[cfg(test)]
mod tests {
    #[test]
    fn 지금은_보안_화면이_아니다() {
        assert!(!super::secure_desktop_active());
    }
}
