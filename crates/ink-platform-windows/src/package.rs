//! Windows MSIX 패키지 모드 감지·StartupTask·전체 화면 감지·자동 재시작 등록 어댑터(D3 · 11-distribution §5·§7).
//!
//! - `Cargo.toml` 기능(feature)을 늘리지 않고 시스템 DLL(`kernel32.dll`, `shell32.dll`, `combase.dll`)을
//!   동적 조회해 호출한다(네트워크 0 원칙 및 타 작업 흐름과 `Cargo.toml` 충돌 방지).
//! - 비패키지 실행 시에는 `GetCurrentPackageFullName` 한 번(`APPMODEL_ERROR_NO_PACKAGE = 15700`)으로
//!   즉시 반환해 WinRT 초기화를 건너뛴다.

use ink_app::ports::{AutostartControl, PackageRuntime};
use ink_domain::package::{
    autostart_backend, classify_channel, evaluate_startup_task, is_fullscreen_notification_state, AutostartBackend, AutostartNotice, InstallChannel,
    PackageSignature, StartupTaskState, STARTUP_TASK_ID,
};
use std::ffi::c_void;
use std::ptr::null_mut;
use std::time::{Duration, Instant};
use windows::core::{GUID, HRESULT};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};

const ERROR_SUCCESS: i32 = 0;
const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
const RESTART_NO_CRASH: u32 = 1;
const RESTART_NO_HANG: u32 = 2;
const RESTART_NO_REBOOT: u32 = 8;

/// WinRT 인터페이스 IID (`windows 0.62.2` 메타데이터 기준 검증됨).
const IID_IPACKAGE_STATICS: GUID = GUID::from_u128(0x4e534bdf_2960_4878_97a4_9624deb72f2d);
const IID_IPACKAGE4: GUID = GUID::from_u128(0x65aed1ae_b95b_450c_882b_6255187f397e);
const IID_IPACKAGE6: GUID = GUID::from_u128(0x8b1ad942_12d7_4754_ae4e_638cbc0e3a2e);
const IID_ISTARTUP_TASK_STATICS: GUID = GUID::from_u128(0xee5b60bd_a148_41a7_b26e_e8b88a1e62f8);
const IID_IASYNC_INFO: GUID = GUID::from_u128(0x00000036_0000_0000_c000_000000000046);

const RO_INIT_MULTITHREADED: i32 = 1;
const S_OK: i32 = 0;
const S_FALSE: i32 = 1;
const RPC_E_CHANGED_MODE: i32 = -2147417850; // 0x80010106

const ASYNC_STATUS_STARTED: i32 = 0;
const ASYNC_STATUS_COMPLETED: i32 = 1;
const ASYNC_STATUS_CANCELED: i32 = 2;

const WINRT_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

struct LoadedModule {
    handle: HMODULE,
    owned: bool,
}

impl LoadedModule {
    fn existing(name: &str) -> Option<Self> {
        let w = wide(name);
        // SAFETY: NUL 종료 UTF-16 문자열로 이미 로드된 모듈 핸들만 조회한다.
        let handle = unsafe { GetModuleHandleW(w.as_ptr()) };
        if handle.is_null() {
            None
        } else {
            Some(Self { handle, owned: false })
        }
    }

    fn load(name: &str) -> Option<Self> {
        let w = wide(name);
        // SAFETY: 시스템 DLL을 로드하고 Drop에서 해제한다.
        let handle = unsafe { LoadLibraryW(w.as_ptr()) };
        if handle.is_null() {
            None
        } else {
            Some(Self { handle, owned: true })
        }
    }

    fn proc(&self, symbol: &[u8]) -> Option<unsafe extern "system" fn() -> isize> {
        debug_assert!(symbol.ends_with(&[0]));
        // SAFETY: 유효한 모듈 핸들과 NUL 종료 ASCII 심볼 이름을 넘긴다.
        unsafe { GetProcAddress(self.handle, symbol.as_ptr()) }
    }
}

impl Drop for LoadedModule {
    fn drop(&mut self) {
        if self.owned && !self.handle.is_null() {
            // SAFETY: `LoadLibraryW`로 직접 올린 모듈만 해제한다.
            unsafe {
                FreeLibrary(self.handle);
            }
        }
    }
}

// ───────────────────────── Win32 패키지·알림·재시작 API ─────────────────────────

type GetCurrentPackageNameFn = unsafe extern "system" fn(*mut u32, *mut u16) -> i32;

fn query_package_name(symbol: &[u8]) -> Option<String> {
    let k32 = LoadedModule::existing("kernel32.dll")?;
    let proc = k32.proc(symbol)?;
    // SAFETY: `kernel32.dll`의 `GetCurrentPackageFullName`/`GetCurrentPackageFamilyName` 시그니처와 같다.
    let func: GetCurrentPackageNameFn = unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, GetCurrentPackageNameFn>(proc) };

    let mut len: u32 = 0;
    // SAFETY: 길이만 먼저 묻는다.
    let rc = unsafe { func(&mut len, null_mut()) };
    if rc != ERROR_INSUFFICIENT_BUFFER || len == 0 {
        return None;
    }

    let mut buf = vec![0u16; len as usize];
    // SAFETY: OS가 요구한 길이만큼 할당한 버퍼를 넘긴다.
    let rc = unsafe { func(&mut len, buf.as_mut_ptr()) };
    if rc != ERROR_SUCCESS {
        return None;
    }
    let text_len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..text_len]))
}

/// 현재 프로세스가 MSIX 패키지 컨텍스트로 실행 중인지(가장 싼 판정, Win8+).
pub fn is_packaged() -> bool {
    let Some(k32) = LoadedModule::existing("kernel32.dll") else {
        return false;
    };
    let Some(proc) = k32.proc(b"GetCurrentPackageFullName\0") else {
        return false;
    };
    // SAFETY: `GetCurrentPackageFullName` 시그니처와 같다.
    let func: GetCurrentPackageNameFn = unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, GetCurrentPackageNameFn>(proc) };
    let mut len: u32 = 0;
    // SAFETY: 버퍼 없이 길이만 조회한다. 패키지면 `ERROR_INSUFFICIENT_BUFFER`, 아니면 `APPMODEL_ERROR_NO_PACKAGE(15700)`.
    unsafe { func(&mut len, null_mut()) == ERROR_INSUFFICIENT_BUFFER }
}

/// 패키지 모드일 때 현재 패키지의 FullName(`Name_Version_Arch__PublisherId`).
pub fn current_package_full_name() -> Option<String> {
    query_package_name(b"GetCurrentPackageFullName\0")
}

/// 패키지 모드일 때 현재 패키지의 FamilyName(`Name_PublisherId`).
pub fn current_package_family_name() -> Option<String> {
    query_package_name(b"GetCurrentPackageFamilyName\0")
}

/// 현재 실행 중인 설치 채널(`Unpackaged` / `MsixStore` / `MsixDirect`)을 판정한다.
pub fn detect_channel() -> InstallChannel {
    if !is_packaged() {
        return InstallChannel::Unpackaged;
    }
    let info = query_winrt_package_info();
    classify_channel(true, info.app_installer_uri.is_some(), info.signature)
}

/// `.appinstaller`로 설치된 직접 배포판이면 해당 `.appinstaller` 주소를 돌려준다.
pub fn app_installer_uri() -> Option<String> {
    if !is_packaged() {
        return None;
    }
    query_winrt_package_info().app_installer_uri
}

/// 업데이트 작업 스레드가 WinRT 객체를 모두 놓은 뒤 같은 스레드에서 초기화를 해제한다.
pub(crate) fn with_winrt<T>(work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let api = ComBase::load()?;
    let _guard = api.init_thread()?;
    work()
}

/// 발표·전체 화면 앱이 앞에 있는지(`SHQueryUserNotificationState`).
pub fn fullscreen_app_active() -> bool {
    let Some(shell32) = LoadedModule::load("shell32.dll") else {
        return false;
    };
    let Some(proc) = shell32.proc(b"SHQueryUserNotificationState\0") else {
        return false;
    };
    type QueryFn = unsafe extern "system" fn(*mut i32) -> i32;
    // SAFETY: `SHQueryUserNotificationState(QUERY_USER_NOTIFICATION_STATE*) -> HRESULT`.
    let func: QueryFn = unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, QueryFn>(proc) };
    let mut state: i32 = 0;
    // SAFETY: 출력 정수 포인터 하나만 넘긴다.
    let hr = unsafe { func(&mut state) };
    (hr == S_OK && is_fullscreen_notification_state(state)) || foreground_covers_monitor()
}

/// 셸이 발표 모드로 분류하지 않은 브라우저 영상·슬라이드 쇼도 확인한다.
fn foreground_covers_monitor() -> bool {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL},
        UI::WindowsAndMessaging::{GetClientRect, GetForegroundWindow, GetShellWindow},
    };
    // 전경 창의 내용 영역만 읽는다. 제목 표시줄이 있는 최대화 창과 바탕 화면은 제외한다.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() || hwnd == GetShellWindow() || matches!(crate::screen::class_name(hwnd).as_str(), "Progman" | "WorkerW") {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let mut client: RECT = std::mem::zeroed();
        let mut origin = POINT { x: 0, y: 0 };
        if GetMonitorInfoW(monitor, &mut info) == 0 || GetClientRect(hwnd, &mut client) == 0 || ClientToScreen(hwnd, &mut origin) == 0 {
            return false;
        }
        let screen = info.rcMonitor;
        origin.x <= screen.left && origin.y <= screen.top && origin.x + client.right >= screen.right && origin.y + client.bottom >= screen.bottom
    }
}

/// 업데이트·패치로 종료된 뒤 Windows가 앱을 다시 켜도록 등록한다(`RegisterApplicationRestart`).
/// 충돌·응답 없음·재부팅 시 자동 재시작은 끄고 업데이트 적용 직후에만 켠다.
pub fn register_application_restart(args: &str) -> Result<(), String> {
    let k32 = LoadedModule::existing("kernel32.dll").ok_or_else(|| "kernel32.dll을 찾을 수 없어요".to_string())?;
    let proc = k32.proc(b"RegisterApplicationRestart\0").ok_or_else(|| "RegisterApplicationRestart가 없어요".to_string())?;
    type RegisterFn = unsafe extern "system" fn(*const u16, u32) -> i32;
    // SAFETY: `RegisterApplicationRestart(PCWSTR, DWORD) -> HRESULT`.
    let func: RegisterFn = unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, RegisterFn>(proc) };
    let w = wide(args);
    let flags = RESTART_NO_CRASH | RESTART_NO_HANG | RESTART_NO_REBOOT;
    // SAFETY: NUL 종료 UTF-16 인자 문자열을 넘긴다.
    let hr = unsafe { func(w.as_ptr(), flags) };
    if hr >= 0 {
        Ok(())
    } else {
        Err(format!("RegisterApplicationRestart 실패: {hr:#010x}"))
    }
}

/// `RegisterApplicationRestart` 등록을 해제한다.
pub fn unregister_application_restart() -> Result<(), String> {
    let k32 = LoadedModule::existing("kernel32.dll").ok_or_else(|| "kernel32.dll을 찾을 수 없어요".to_string())?;
    let proc = k32.proc(b"UnregisterApplicationRestart\0").ok_or_else(|| "UnregisterApplicationRestart가 없어요".to_string())?;
    type UnregisterFn = unsafe extern "system" fn() -> i32;
    // SAFETY: `UnregisterApplicationRestart() -> HRESULT`.
    let func: UnregisterFn = unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, UnregisterFn>(proc) };
    // SAFETY: 인자 없음.
    let hr = unsafe { func() };
    if hr >= 0 {
        Ok(())
    } else {
        Err(format!("UnregisterApplicationRestart 실패: {hr:#010x}"))
    }
}

// ───────────────────────── WinRT COM 최소 래퍼 (`combase.dll`) ─────────────────────────

#[repr(C)]
struct IUnknownVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct IInspectableVtbl {
    base: IUnknownVtbl,
    get_iids: unsafe extern "system" fn(*mut c_void, *mut u32, *mut *mut GUID) -> HRESULT,
    get_runtime_class_name: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    get_trust_level: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
}

#[repr(C)]
struct IPackageStaticsVtbl {
    base: IInspectableVtbl,
    current: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct IPackage4Vtbl {
    base: IInspectableVtbl,
    signature_kind: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
}

#[repr(C)]
struct IPackage6Vtbl {
    base: IInspectableVtbl,
    get_app_installer_info: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct IAppInstallerInfoVtbl {
    base: IInspectableVtbl,
    uri: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct IUriRuntimeClassVtbl {
    base: IInspectableVtbl,
    absolute_uri: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct IStartupTaskStaticsVtbl {
    base: IInspectableVtbl,
    get_for_current_package_async: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    get_async: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct IStartupTaskVtbl {
    base: IInspectableVtbl,
    request_enable_async: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    disable: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    state: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
}

#[repr(C)]
struct IAsyncInfoVtbl {
    base: IInspectableVtbl,
    id: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
    status: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
    error_code: unsafe extern "system" fn(*mut c_void, *mut HRESULT) -> HRESULT,
    cancel: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    close: unsafe extern "system" fn(*mut c_void) -> HRESULT,
}

#[repr(C)]
struct IAsyncOperationPtrVtbl {
    base: IInspectableVtbl,
    set_completed: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT,
    completed: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    get_results: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct IAsyncOperationI32Vtbl {
    base: IInspectableVtbl,
    set_completed: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT,
    completed: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    get_results: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
}

struct ComPtr(*mut c_void);

impl ComPtr {
    fn from_raw(raw: *mut c_void) -> Option<Self> {
        if raw.is_null() {
            None
        } else {
            Some(Self(raw))
        }
    }

    fn as_ptr(&self) -> *mut c_void {
        self.0
    }

    unsafe fn vtbl<T>(&self) -> &T {
        &*(*(self.0 as *const *const T))
    }

    fn query(&self, iid: &GUID) -> Result<ComPtr, HRESULT> {
        let mut out: *mut c_void = null_mut();
        // SAFETY: `self.0`은 유효한 COM 객체 포인터이며 `IUnknown::QueryInterface`는 슬롯 0이다.
        let hr = unsafe { (self.vtbl::<IUnknownVtbl>().query_interface)(self.0, iid, &mut out) };
        if hr.is_ok() {
            ComPtr::from_raw(out).ok_or(HRESULT(-2147467261)) // E_POINTER
        } else {
            Err(hr)
        }
    }
}

impl Drop for ComPtr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: 소유한 COM 참조 하나를 해제한다.
            unsafe {
                (self.vtbl::<IUnknownVtbl>().release)(self.0);
            }
        }
    }
}

type RoInitializeFn = unsafe extern "system" fn(i32) -> i32;
type RoUninitializeFn = unsafe extern "system" fn();
type RoGetActivationFactoryFn = unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT;
type WindowsCreateStringFn = unsafe extern "system" fn(*const u16, u32, *mut *mut c_void) -> HRESULT;
type WindowsDeleteStringFn = unsafe extern "system" fn(*mut c_void) -> HRESULT;
type WindowsGetStringRawBufferFn = unsafe extern "system" fn(*mut c_void, *mut u32) -> *const u16;

struct ComBase {
    _module: LoadedModule,
    ro_init: RoInitializeFn,
    ro_uninit: RoUninitializeFn,
    ro_factory: RoGetActivationFactoryFn,
    create_string: WindowsCreateStringFn,
    delete_string: WindowsDeleteStringFn,
    raw_buffer: WindowsGetStringRawBufferFn,
}

struct RoGuard<'a> {
    api: &'a ComBase,
    should_uninit: bool,
}

impl Drop for RoGuard<'_> {
    fn drop(&mut self) {
        if self.should_uninit {
            // SAFETY: 이 스레드에서 `RoInitialize`가 성공한 경우에만 짝을 맞춰 호출한다.
            unsafe {
                (self.api.ro_uninit)();
            }
        }
    }
}

struct HString<'a> {
    api: &'a ComBase,
    raw: *mut c_void,
}

impl<'a> HString<'a> {
    fn new(api: &'a ComBase, s: &str) -> Result<Self, String> {
        let utf16: Vec<u16> = s.encode_utf16().collect();
        let mut raw: *mut c_void = null_mut();
        // SAFETY: UTF-16 슬라이스와 길이를 넘겨 새 HSTRING을 만든다.
        let hr = unsafe { (api.create_string)(utf16.as_ptr(), utf16.len() as u32, &mut raw) };
        if hr.is_ok() {
            Ok(Self { api, raw })
        } else {
            Err(format!("WindowsCreateString 실패: {:#010x}", hr.0 as u32))
        }
    }

    fn from_raw(api: &'a ComBase, raw: *mut c_void) -> Self {
        Self { api, raw }
    }

    fn to_rust_string(&self) -> Option<String> {
        if self.raw.is_null() {
            return None;
        }
        let mut len: u32 = 0;
        // SAFETY: 유효한 HSTRING 핸들에서 읽기 전용 버퍼 포인터와 길이를 읽는다.
        let ptr = unsafe { (self.api.raw_buffer)(self.raw, &mut len) };
        if ptr.is_null() {
            return None;
        }
        // SAFETY: `WindowsGetStringRawBuffer`가 돌려준 버퍼는 `len` u16 단위 동안 유효하다.
        let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
        Some(String::from_utf16_lossy(slice))
    }
}

impl Drop for HString<'_> {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: 소유한 HSTRING을 해제한다.
            unsafe {
                let _ = (self.api.delete_string)(self.raw);
            }
        }
    }
}

impl ComBase {
    fn load() -> Result<Self, String> {
        let module = LoadedModule::load("combase.dll").ok_or_else(|| "combase.dll을 로드하지 못했어요".to_string())?;
        macro_rules! sym {
            ($name:literal, $ty:ty) => {{
                let p = module.proc($name).ok_or_else(|| format!("combase 심볼 누락"))?;
                // SAFETY: `combase.dll` WinRT 공용 C ABI 함수 시그니처와 일치한다.
                unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, $ty>(p) }
            }};
        }
        Ok(Self {
            ro_init: sym!(b"RoInitialize\0", RoInitializeFn),
            ro_uninit: sym!(b"RoUninitialize\0", RoUninitializeFn),
            ro_factory: sym!(b"RoGetActivationFactory\0", RoGetActivationFactoryFn),
            create_string: sym!(b"WindowsCreateString\0", WindowsCreateStringFn),
            delete_string: sym!(b"WindowsDeleteString\0", WindowsDeleteStringFn),
            raw_buffer: sym!(b"WindowsGetStringRawBuffer\0", WindowsGetStringRawBufferFn),
            _module: module,
        })
    }

    fn init_thread(&self) -> Result<RoGuard<'_>, String> {
        // SAFETY: 현재 스레드에서 WinRT 초기화를 시도한다. 이미 STA로 초기화된 스레드(`RPC_E_CHANGED_MODE`)는 그대로 쓴다.
        let hr = unsafe { (self.ro_init)(RO_INIT_MULTITHREADED) };
        match hr {
            S_OK | S_FALSE => Ok(RoGuard { api: self, should_uninit: true }),
            RPC_E_CHANGED_MODE => Ok(RoGuard { api: self, should_uninit: false }),
            other => Err(format!("RoInitialize 실패: {:#010x}", other as u32)),
        }
    }

    fn activation_factory(&self, class_name: &str, iid: &GUID) -> Result<ComPtr, String> {
        let h = HString::new(self, class_name)?;
        let mut out: *mut c_void = null_mut();
        // SAFETY: 유효한 클래스 이름 HSTRING과 인터페이스 IID로 WinRT 팩토리를 요청한다.
        let hr = unsafe { (self.ro_factory)(h.raw, iid, &mut out) };
        if hr.is_ok() {
            ComPtr::from_raw(out).ok_or_else(|| format!("{class_name} 팩토리가 비어 있어요"))
        } else {
            Err(format!("RoGetActivationFactory({class_name}) 실패: {:#010x}", hr.0 as u32))
        }
    }
}

fn wait_async(op: &ComPtr) -> Result<(), String> {
    let info = op.query(&IID_IASYNC_INFO).map_err(|hr| format!("IAsyncInfo 질의 실패: {:#010x}", hr.0 as u32))?;
    let start = Instant::now();
    loop {
        let mut status: i32 = ASYNC_STATUS_STARTED;
        // SAFETY: `info`는 `IAsyncInfo` 인터페이스 포인터다.
        let hr = unsafe { (info.vtbl::<IAsyncInfoVtbl>().status)(info.as_ptr(), &mut status) };
        if hr.is_err() {
            return Err(format!("IAsyncInfo::Status 실패: {:#010x}", hr.0 as u32));
        }
        match status {
            ASYNC_STATUS_STARTED => {
                if start.elapsed() >= WINRT_WAIT_TIMEOUT {
                    // SAFETY: 시간 초과 시 비동기 작업 취소를 요청한다.
                    unsafe {
                        let _ = (info.vtbl::<IAsyncInfoVtbl>().cancel)(info.as_ptr());
                    }
                    return Err("WinRT 비동기 호출 시간 초과".into());
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            ASYNC_STATUS_COMPLETED => return Ok(()),
            ASYNC_STATUS_CANCELED => return Err("WinRT 비동기 호출이 취소됐어요".into()),
            _ => {
                let mut err_hr = HRESULT(0);
                // SAFETY: 오류 코드를 조회한다.
                unsafe {
                    let _ = (info.vtbl::<IAsyncInfoVtbl>().error_code)(info.as_ptr(), &mut err_hr);
                }
                return Err(format!("WinRT 비동기 호출 오류: {:#010x}", err_hr.0 as u32));
            }
        }
    }
}

struct WinRtPackageInfo {
    signature: PackageSignature,
    app_installer_uri: Option<String>,
}

fn query_winrt_package_info() -> WinRtPackageInfo {
    let mut out = WinRtPackageInfo {
        signature: PackageSignature::Unknown,
        app_installer_uri: None,
    };
    let Ok(com) = ComBase::load() else {
        return out;
    };
    let Ok(_guard) = com.init_thread() else {
        return out;
    };
    let Ok(statics) = com.activation_factory("Windows.ApplicationModel.Package", &IID_IPACKAGE_STATICS) else {
        return out;
    };
    let mut pkg_raw: *mut c_void = null_mut();
    // SAFETY: `statics`는 `IPackageStatics` 포인터다.
    let hr = unsafe { (statics.vtbl::<IPackageStaticsVtbl>().current)(statics.as_ptr(), &mut pkg_raw) };
    let Some(pkg) = hr.is_ok().then(|| ComPtr::from_raw(pkg_raw)).flatten() else {
        return out;
    };

    if let Ok(pkg4) = pkg.query(&IID_IPACKAGE4) {
        let mut kind: i32 = -1;
        // SAFETY: `pkg4`는 `IPackage4` 포인터다.
        if unsafe { (pkg4.vtbl::<IPackage4Vtbl>().signature_kind)(pkg4.as_ptr(), &mut kind) }.is_ok() {
            out.signature = PackageSignature::from_winrt(kind);
        }
    }

    if let Ok(pkg6) = pkg.query(&IID_IPACKAGE6) {
        let mut info_raw: *mut c_void = null_mut();
        // SAFETY: `pkg6`는 `IPackage6` 포인터다. AppInstaller가 없으면 `info_raw`는 null이다.
        let hr = unsafe { (pkg6.vtbl::<IPackage6Vtbl>().get_app_installer_info)(pkg6.as_ptr(), &mut info_raw) };
        if hr.is_ok() {
            if let Some(info) = ComPtr::from_raw(info_raw) {
                let mut uri_raw: *mut c_void = null_mut();
                // SAFETY: `info`는 `IAppInstallerInfo` 포인터다.
                let hr = unsafe { (info.vtbl::<IAppInstallerInfoVtbl>().uri)(info.as_ptr(), &mut uri_raw) };
                if hr.is_ok() {
                    if let Some(uri_obj) = ComPtr::from_raw(uri_raw) {
                        let mut hraw: *mut c_void = null_mut();
                        // SAFETY: `uri_obj`는 `IUriRuntimeClass` 포인터다.
                        if unsafe { (uri_obj.vtbl::<IUriRuntimeClassVtbl>().absolute_uri)(uri_obj.as_ptr(), &mut hraw) }.is_ok() {
                            let hstr = HString::from_raw(&com, hraw);
                            out.app_installer_uri = hstr.to_rust_string();
                        }
                    }
                }
            }
        }
    }
    out
}

// ───────────────────────── StartupTask API (`Windows.ApplicationModel.StartupTask`) ─────────────────────────

fn get_startup_task(com: &ComBase, task_id: &str) -> Result<ComPtr, String> {
    let statics = com.activation_factory("Windows.ApplicationModel.StartupTask", &IID_ISTARTUP_TASK_STATICS)?;
    let id = HString::new(com, task_id)?;
    let mut op_raw: *mut c_void = null_mut();
    // SAFETY: `statics`는 `IStartupTaskStatics` 포인터이며 `GetAsync`는 슬롯 7이다.
    let hr = unsafe { (statics.vtbl::<IStartupTaskStaticsVtbl>().get_async)(statics.as_ptr(), id.raw, &mut op_raw) };
    if hr.is_err() {
        return Err(format!("StartupTask::GetAsync 실패: {:#010x}", hr.0 as u32));
    }
    let op = ComPtr::from_raw(op_raw).ok_or_else(|| "StartupTask::GetAsync 작업이 비어 있어요".to_string())?;
    wait_async(&op)?;

    let mut task_raw: *mut c_void = null_mut();
    // SAFETY: 완료된 `IAsyncOperation<StartupTask>`에서 결과를 꺼낸다.
    let hr = unsafe { (op.vtbl::<IAsyncOperationPtrVtbl>().get_results)(op.as_ptr(), &mut task_raw) };
    if hr.is_err() {
        return Err(format!("StartupTask::GetAsync 결과 오류: {:#010x}", hr.0 as u32));
    }
    ComPtr::from_raw(task_raw).ok_or_else(|| "StartupTask 객체가 비어 있어요".to_string())
}

fn read_task_state(task: &ComPtr) -> Result<StartupTaskState, String> {
    let mut raw: i32 = -1;
    // SAFETY: `task`는 `IStartupTask` 포인터다.
    let hr = unsafe { (task.vtbl::<IStartupTaskVtbl>().state)(task.as_ptr(), &mut raw) };
    if hr.is_err() {
        return Err(format!("StartupTask::State 실패: {:#010x}", hr.0 as u32));
    }
    StartupTaskState::from_winrt(raw).ok_or_else(|| format!("알 수 없는 StartupTaskState: {raw}"))
}

/// 패키지 모드에서 `StartupTask` 현재 상태를 조회한다. 비패키지에서는 즉시 오류를 돌려준다.
pub fn startup_task_state(task_id: &str) -> Result<StartupTaskState, String> {
    if !is_packaged() {
        return Err("패키지 모드에서만 StartupTask를 쓸 수 있어요".into());
    }
    let com = ComBase::load()?;
    let _guard = com.init_thread()?;
    let task = get_startup_task(&com, task_id)?;
    read_task_state(&task)
}

/// 패키지 모드에서 `StartupTask`를 켜거나 끈 뒤 결과 상태를 돌려준다.
pub fn set_startup_task_enabled(task_id: &str, enabled: bool) -> Result<StartupTaskState, String> {
    if !is_packaged() {
        return Err("패키지 모드에서만 StartupTask를 쓸 수 있어요".into());
    }
    let com = ComBase::load()?;
    let _guard = com.init_thread()?;
    let task = get_startup_task(&com, task_id)?;
    if enabled {
        let mut op_raw: *mut c_void = null_mut();
        // SAFETY: `task`는 `IStartupTask` 포인터다.
        let hr = unsafe { (task.vtbl::<IStartupTaskVtbl>().request_enable_async)(task.as_ptr(), &mut op_raw) };
        if hr.is_err() {
            return Err(format!("StartupTask::RequestEnableAsync 실패: {:#010x}", hr.0 as u32));
        }
        let op = ComPtr::from_raw(op_raw).ok_or_else(|| "RequestEnableAsync 작업이 비어 있어요".to_string())?;
        wait_async(&op)?;
        let mut raw_state: i32 = -1;
        // SAFETY: 완료된 `IAsyncOperation<StartupTaskState>`에서 정수 상태를 꺼낸다.
        let hr = unsafe { (op.vtbl::<IAsyncOperationI32Vtbl>().get_results)(op.as_ptr(), &mut raw_state) };
        if hr.is_err() {
            return Err(format!("RequestEnableAsync 결과 오류: {:#010x}", hr.0 as u32));
        }
        StartupTaskState::from_winrt(raw_state).ok_or_else(|| format!("알 수 없는 StartupTaskState: {raw_state}"))
    } else {
        // SAFETY: `task`는 `IStartupTask` 포인터다.
        let hr = unsafe { (task.vtbl::<IStartupTaskVtbl>().disable)(task.as_ptr()) };
        if hr.is_err() {
            return Err(format!("StartupTask::Disable 실패: {:#010x}", hr.0 as u32));
        }
        read_task_state(&task)
    }
}

// ───────────────────────── 포트 구현 (`ink-app::ports`) ─────────────────────────

/// Windows `PackageRuntime` 어댑터.
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsPackageRuntime;

impl PackageRuntime for WindowsPackageRuntime {
    fn channel(&self) -> InstallChannel {
        detect_channel()
    }

    fn package_full_name(&self) -> Option<String> {
        current_package_full_name()
    }

    fn app_installer_uri(&self) -> Option<String> {
        app_installer_uri()
    }

    fn fullscreen_app_active(&self) -> bool {
        fullscreen_app_active()
    }

    fn register_restart(&self, args: &str) -> Result<(), String> {
        register_application_restart(args)
    }
}

/// MSIX 패키지 모드용 `AutostartControl` 어댑터(`Windows.ApplicationModel.StartupTask`).
#[derive(Clone, Copy, Debug)]
pub struct WindowsStartupTaskAutostart {
    task_id: &'static str,
}

impl Default for WindowsStartupTaskAutostart {
    fn default() -> Self {
        Self::new(STARTUP_TASK_ID)
    }
}

impl WindowsStartupTaskAutostart {
    pub const fn new(task_id: &'static str) -> Self {
        Self { task_id }
    }
}

impl AutostartControl for WindowsStartupTaskAutostart {
    fn backend(&self) -> AutostartBackend {
        autostart_backend(detect_channel())
    }

    fn is_enabled(&self) -> Result<bool, String> {
        Ok(startup_task_state(self.task_id)?.is_enabled())
    }

    fn set_enabled(&mut self, enabled: bool) -> Result<AutostartNotice, String> {
        let state = set_startup_task_enabled(self.task_id, enabled)?;
        Ok(evaluate_startup_task(enabled, state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 비패키지_환경에서_패키지_아님으로_판정한다() {
        assert!(!is_packaged());
        assert_eq!(current_package_full_name(), None);
        assert_eq!(current_package_family_name(), None);
        assert_eq!(app_installer_uri(), None);
        assert_eq!(detect_channel(), InstallChannel::Unpackaged);

        let rt = WindowsPackageRuntime;
        assert_eq!(rt.channel(), InstallChannel::Unpackaged);
        assert_eq!(rt.package_full_name(), None);
        assert_eq!(rt.app_installer_uri(), None);
    }

    #[test]
    fn 비패키지_환경에서_startup_task는_오류를_돌려준다() {
        assert!(startup_task_state(STARTUP_TASK_ID).is_err());
        let mut ctrl = WindowsStartupTaskAutostart::default();
        assert_eq!(ctrl.backend(), AutostartBackend::RegistryRun);
        assert!(ctrl.is_enabled().is_err());
        assert!(ctrl.set_enabled(true).is_err());
    }

    #[test]
    fn 재시작_등록과_해제가_동작한다() {
        register_application_restart("--restarted-after-update").expect("RegisterApplicationRestart");
        unregister_application_restart().expect("UnregisterApplicationRestart");
    }

    #[test]
    fn 전체화면_알림_상태_조회가_정상_반환한다() {
        // 읽기 전용 OS 질의가 예외 없이 bool을 돌려준다.
        let _active = fullscreen_app_active();
    }
}
