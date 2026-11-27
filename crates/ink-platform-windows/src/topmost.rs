//! 다른 프로세스의 창에 고정 표시를 먼저 남겨 중간에 종료돼도 복구할 수 있게 한다.

use crate::screen::{class_name, cloaked};
use crate::topmost_border::Border;
use ink_app::ports::{BorderStyle, PinError, WindowPinner};
use ink_domain::topmost::{decide, CandidateWindow, TopmostDecision, PROP_POWERTOYS, PROP_SCREEN_INK};
use std::collections::BTreeMap;
use std::sync::{Once, OnceLock};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED, HWND, LPARAM},
    System::{
        Console::{SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_C_EVENT},
        Diagnostics::Debug::MessageBeep,
        Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION},
    },
    UI::{
        Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN},
        WindowsAndMessaging::*,
    },
};

pub(super) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub(super) fn our_key() -> &'static [u16] {
    static KEY: OnceLock<Vec<u16>> = OnceLock::new();
    KEY.get_or_init(|| wide(PROP_SCREEN_INK))
}

pub(super) unsafe fn owner(hwnd: HWND) -> usize {
    GetPropW(hwnd, our_key().as_ptr()) as usize
}

unsafe fn exe_name(pid: u32) -> String {
    let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if proc.is_null() {
        return String::new();
    }
    let mut path = vec![0u16; 32768];
    let mut n = path.len() as u32;
    let ok = QueryFullProcessImageNameW(proc, 0, path.as_mut_ptr(), &mut n);
    CloseHandle(proc);
    if ok == 0 {
        return String::new();
    }
    let name = path[..n as usize].rsplit(|c| *c == b'\\' as u16).next().unwrap_or_default();
    String::from_utf16_lossy(name).to_lowercase()
}

unsafe fn snapshot(hwnd: HWND) -> Option<CandidateWindow> {
    let mut pid = 0;
    if hwnd.is_null() || GetWindowThreadProcessId(hwnd, &mut pid) == 0 {
        return None;
    }
    let mut notification = 0;
    Some(CandidateWindow {
        handle: hwnd as u64,
        pid,
        exe: exe_name(pid),
        class: class_name(hwnd),
        visible: IsWindowVisible(hwnd) != 0,
        minimized: IsIconic(hwnd) != 0,
        cloaked: cloaked(hwnd),
        topmost: (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST) != 0,
        has_our_prop: owner(hwnd) != 0,
        has_powertoys_prop: !GetPropW(hwnd, wide(PROP_POWERTOYS).as_ptr()).is_null(),
        fullscreen_d3d_game: SHQueryUserNotificationState(&mut notification) >= 0 && notification == QUNS_RUNNING_D3D_FULL_SCREEN,
    })
}

unsafe fn last_error(hwnd: HWND) -> PinError {
    let code = GetLastError();
    if IsWindow(hwnd) == 0 {
        PinError::Gone
    } else if code == ERROR_ACCESS_DENIED {
        PinError::AccessDenied
    } else {
        PinError::Other(format!("창 속성 변경 오류 {code}"))
    }
}

unsafe fn change_topmost(hwnd: HWND, on: bool) -> Result<(), PinError> {
    let after = if on { HWND_TOPMOST } else { HWND_NOTOPMOST };
    if SetWindowPos(hwnd, after, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) == 0 {
        return Err(last_error(hwnd));
    }
    // 다른 입력 큐의 창은 성공을 돌려준 뒤에 확장 스타일이 바뀌기도 한다.
    let until = std::time::Instant::now() + std::time::Duration::from_millis(80);
    loop {
        if IsWindow(hwnd) == 0 {
            return Err(PinError::Gone);
        }
        if (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0) == on {
            return Ok(());
        }
        if std::time::Instant::now() >= until {
            return Err(PinError::AccessDenied);
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

pub(super) unsafe fn release(hwnd: HWND, mark: usize) -> Result<(), PinError> {
    if mark == 0 || owner(hwnd) != mark {
        return Err(PinError::Gone);
    }
    change_topmost(hwnd, false)?;
    if RemovePropW(hwnd, our_key().as_ptr()).is_null() {
        return Err(last_error(hwnd));
    }
    Ok(())
}

struct Sweep {
    mark: Option<usize>,
    released: usize,
}

unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
    let sweep = &mut *(data as *mut Sweep);
    let mark = owner(hwnd);
    if mark != 0 && sweep.mark.map_or(true, |m| m == mark) && release(hwnd, mark).is_ok() {
        sweep.released += 1;
    }
    1
}

fn sweep(mark: Option<usize>) -> usize {
    let mut state = Sweep { mark, released: 0 };
    // 열거는 동기 호출이므로 콜백이 끝날 때까지 state가 살아 있다.
    unsafe {
        EnumWindows(Some(visit), (&mut state as *mut Sweep) as LPARAM);
    }
    state.released
}

unsafe extern "system" fn console_exit(event: u32) -> i32 {
    if matches!(event, CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT) {
        sweep(Some(std::process::id() as usize));
    }
    0
}

#[derive(Default)]
pub struct WinWindowPinner {
    borders: BTreeMap<u64, Border>,
}

impl WindowPinner for WinWindowPinner {
    fn foreground(&self) -> Option<CandidateWindow> {
        unsafe { snapshot(GetForegroundWindow()) }
    }
    fn candidate(&self, hwnd: u64) -> Option<CandidateWindow> {
        unsafe { snapshot(hwnd as HWND) }
    }
    fn pin(&mut self, handle: u64, style: BorderStyle) -> Result<(), PinError> {
        let hwnd = handle as HWND;
        let win = self.candidate(handle).ok_or(PinError::Gone)?;
        if decide(&win, std::process::id()) != TopmostDecision::Pin {
            return Err(PinError::Other("고정할 수 없는 창".into()));
        }
        let mark = std::process::id() as usize;
        unsafe {
            if SetPropW(hwnd, our_key().as_ptr(), mark as _) == 0 {
                return Err(last_error(hwnd));
            }
            if let Err(err) = change_topmost(hwnd, true) {
                if owner(hwnd) == mark {
                    RemovePropW(hwnd, our_key().as_ptr());
                }
                return Err(err);
            }
        }
        match Border::start(handle, win.pid, style) {
            Ok(border) => {
                self.borders.insert(handle, border);
                Ok(())
            }
            Err(err) => {
                unsafe {
                    release(hwnd, mark)?;
                }
                Err(err)
            }
        }
    }
    fn unpin(&mut self, handle: u64) -> Result<(), PinError> {
        let result = unsafe { release(handle as HWND, std::process::id() as usize) };
        if result.is_ok() || result == Err(PinError::Gone) {
            self.borders.remove(&handle);
        }
        result
    }
    fn forget_destroyed(&mut self, hwnd: u64) {
        self.borders.remove(&hwnd);
    }
    fn update_border_style(&mut self, style: BorderStyle) {
        for border in self.borders.values() {
            border.set_style(style);
        }
    }
    fn cleanup_orphans(&mut self) -> usize {
        static REGISTER: Once = Once::new();
        REGISTER.call_once(|| unsafe {
            SetConsoleCtrlHandler(Some(console_exit), 1);
        });
        sweep(None)
    }
    fn unpin_all(&mut self) {
        for handle in self.borders.keys().copied().collect::<Vec<_>>() {
            let _ = self.unpin(handle);
        }
    }
    fn is_alive(&self, hwnd: u64) -> bool {
        unsafe { owner(hwnd as HWND) == std::process::id() as usize }
    }
    fn play_pin_sound(&self) {
        unsafe {
            MessageBeep(MB_OK);
        }
    }
}

impl Drop for WinWindowPinner {
    fn drop(&mut self) {
        self.unpin_all();
    }
}

#[cfg(test)]
#[path = "topmost_test.rs"]
mod tests;
