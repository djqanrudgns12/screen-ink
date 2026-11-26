//! Windows 어댑터. 판정은 하지 않고 사실만 읽는다.
//! - 실행 중인 프로세스: 실행 파일 이름만(창 제목·경로 X, 05 개인정보 규칙)
//! - PowerToys: 저수준 훅이라 `RegisterHotKey` 시험으로 겹침이 안 보인다 → 모듈 설정 파일을 **읽기만** 해서 키를 안다(02 §6.2-2)

pub mod powertoys;
#[cfg(windows)]
pub mod clipboard;
#[cfg(windows)]
pub mod events;
#[cfg(windows)]
pub mod uia;
#[cfg(windows)]
pub mod freeze;
#[cfg(windows)]
pub mod package;
#[cfg(windows)]
pub mod screen;
#[cfg(windows)]
pub mod magnifier;
// 3b-dpi·3b-msaa(D12·R-CAP-4): 순수 판정은 어느 OS에서나 시험하고, Win32 부분은 모듈 안에서 cfg(windows)
pub mod dpi;
pub mod msaa;

use ink_app::SystemProbe;
use ink_domain::shortcuts::LiveBinding;
use std::path::PathBuf;

/// `SystemProbe` 구현. PowerToys 설정 폴더는 바꿔 끼울 수 있다(시험).
pub struct WindowsProbe {
    powertoys_dir: Option<PathBuf>,
}

impl Default for WindowsProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsProbe {
    pub fn new() -> Self {
        let dir = std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Microsoft").join("PowerToys"));
        Self { powertoys_dir: dir }
    }
}

impl SystemProbe for WindowsProbe {
    fn running_apps(&self) -> Vec<String> {
        process_names()
    }

    fn live_bindings(&self) -> Option<Vec<LiveBinding>> {
        if !process_names().iter().any(|n| n == "powertoys.exe") {
            return Some(Vec::new());
        }
        powertoys::read_bindings(self.powertoys_dir.as_ref()?)
    }

    fn secure_desktop(&self) -> bool {
        #[cfg(windows)]
        {
            events::secure_desktop_active()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

/// 실행 중인 프로세스 실행 파일 이름(소문자, 중복 제거).
#[cfg(windows)]
pub fn process_names() -> Vec<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};

    let mut out = std::collections::BTreeSet::new();
    // SAFETY: 스냅숏 핸들은 이 함수 안에서만 쓰고 닫는다. 구조체 크기는 API 규약대로 채운다.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return Vec::new();
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut entry);
        while ok != 0 {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            out.insert(String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase());
            ok = Process32NextW(snap, &mut entry);
        }
        CloseHandle(snap);
    }
    out.into_iter().collect()
}

#[cfg(not(windows))]
pub fn process_names() -> Vec<String> {
    Vec::new()
}

/// 작업 표시줄이 밝은 테마인지(트레이 아이콘 색 고르기). 읽을 수 없으면 false(Windows 11 기본 = 어두움).
#[cfg(windows)]
pub fn taskbar_is_light() -> bool {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0".encode_utf16().collect();
    let name: Vec<u16> = "SystemUsesLightTheme\0".encode_utf16().collect();
    let mut value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: 출력 버퍼는 DWORD 하나, 크기를 함께 넘긴다.
    let rc = unsafe { RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), &mut value as *mut u32 as *mut _, &mut size) };
    rc == 0 && value == 1
}

#[cfg(not(windows))]
pub fn taskbar_is_light() -> bool {
    false
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn 프로세스_목록에_자기_자신이_있다() {
        let names = super::process_names();
        let me = std::env::current_exe().unwrap().file_name().unwrap().to_string_lossy().to_lowercase();
        assert!(names.contains(&me), "{me} not in {} names", names.len());
    }
}
