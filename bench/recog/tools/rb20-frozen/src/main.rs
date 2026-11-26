//! RB-20 멈춘 시험 앱(08 §5.1, D17·D19, C §2.3 `3f-frozen`).
//!
//! screen-ink가 이 창 위에서 F1·커서 이동을 해도 화면이 100 ms 넘게 굳지 않는지 보려고, 접근성 질의를 일부러 막는다.
//!
//!   rb20-frozen [--mode hung|infinite-msaa|hang-location|slow-msaa] [--slow-ms 5] [--life 120]
//!
//! - `hung`: 3초마다 메시지 루프를 10초씩 막는다(5초가 지나면 OS가 "응답 없음"으로 본다).
//! - `infinite-msaa`(기본): `WM_GETOBJECT`에 직접 만든 `IAccessible`을 준다 — 자식이 백만 개이고 자식마다 또 백만 개,
//!   `accHitTest`는 돌아오지 않는다(Snipaste #116류).
//! - `hang-location`: `infinite-msaa` + 자식의 `accLocation`도 돌아오지 않는다 → MSAA 일꾼 마감(100 ms)·교체 시험.
//! - `slow-msaa`: `accLocation`마다 `--slow-ms`만큼 잔다 → MSAA 걸음 예산(20 ms) 끊기 시험.
//! - `--life`초가 지나면 스스로 끝난다(멈춘 채 남지 않게, 기본 120초).
//!
//! 이 앱은 일부러 DPI를 모른다(매니페스트·DPI 호출 없음): 배율이 100%가 아닌 모니터에서 `accLocation`이 논리 좌표를
//! 주므로 D12 보정 경로(`dpi::classify_space` → Logical)도 함께 탄다. 자식 사각형 자리(20 px 격자의 16 px 칸)를 회색으로
//! 그려 두어, 근거 보기(`capture.evidence`)에서 보정이 맞는지 눈으로도 보인다. 창 제목 외 글자는 없다.

#[cfg(not(windows))]
fn main() {
    eprintln!("Windows 전용 시험 앱이에요");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() {
    win::run();
}

#[cfg(windows)]
mod win {
    use std::ptr::{null, null_mut};
    use std::sync::atomic::{AtomicIsize, AtomicU64, AtomicU8, Ordering};
    use std::time::Duration;
    use windows::core::{implement, Interface, Result, BSTR, GUID, PCWSTR};
    use windows::Win32::Foundation::{E_INVALIDARG, E_NOTIMPL, WPARAM};
    use windows::Win32::System::Com::{CoInitializeEx, IDispatch, IDispatch_Impl, ITypeInfo, COINIT_APARTMENTTHREADED, DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO};
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::{IAccessible, IAccessible_Impl, LresultFromObject};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{BeginPaint, ClientToScreen, EndPaint, FillRect, GetStockObject, GRAY_BRUSH, PAINTSTRUCT};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, PostQuitMessage, RegisterClassW, SetTimer, ShowWindow, TranslateMessage, CW_USEDEFAULT, MSG, SW_SHOW,
        WM_DESTROY, WM_GETOBJECT, WM_PAINT, WM_TIMER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    };

    const MODE_HUNG: u8 = 0;
    const MODE_INFINITE: u8 = 1;
    const MODE_HANG_LOCATION: u8 = 2;
    const MODE_SLOW: u8 = 3;

    static MODE: AtomicU8 = AtomicU8::new(MODE_INFINITE);
    static SLOW_MS: AtomicU64 = AtomicU64::new(5);
    static MAIN: AtomicIsize = AtomicIsize::new(0);

    /// `OBJID_CLIENT`
    const OBJID_CLIENT: i32 = -4;
    /// 자식 수(끝없음 흉내)
    const CHILDREN: i32 = 1_000_000;
    const ROLE_CLIENT: i32 = 0x0a;
    const ROLE_PUSHBUTTON: i32 = 0x2b;

    fn mode() -> u8 {
        MODE.load(Ordering::SeqCst)
    }

    /// 돌아오지 않는다(`--life`가 지나면 감시 스레드가 프로세스를 끝냄).
    fn block_forever() -> ! {
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// 창 클라이언트 영역(화면 좌표, 이 앱의 DPI 문맥 = 논리).
    fn client_screen() -> (i32, i32, i32, i32) {
        let h = MAIN.load(Ordering::SeqCst) as HWND;
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: 이 앱의 창, 출력 구조체만.
        unsafe {
            GetClientRect(h, &mut r);
            ClientToScreen(h, &mut p);
        }
        (p.x, p.y, r.right - r.left, r.bottom - r.top)
    }

    /// 끝없는 접근성 나무의 한 마디. 뿌리 = depth 0.
    #[implement(IAccessible)]
    struct Acc {
        depth: u32,
        index: i32,
    }

    fn child_id(v: &VARIANT) -> i32 {
        i32::try_from(v).unwrap_or(0)
    }

    impl IDispatch_Impl for Acc_Impl {
        fn GetTypeInfoCount(&self) -> Result<u32> {
            Ok(0)
        }
        fn GetTypeInfo(&self, _itinfo: u32, _lcid: u32) -> Result<ITypeInfo> {
            Err(E_NOTIMPL.into())
        }
        fn GetIDsOfNames(&self, _riid: *const GUID, _names: *const PCWSTR, _count: u32, _lcid: u32, _ids: *mut i32) -> Result<()> {
            Err(E_NOTIMPL.into())
        }
        fn Invoke(&self, _id: i32, _riid: *const GUID, _lcid: u32, _flags: DISPATCH_FLAGS, _params: *const DISPPARAMS, _result: *mut VARIANT, _excep: *mut EXCEPINFO, _argerr: *mut u32) -> Result<()> {
            Err(E_NOTIMPL.into())
        }
    }

    impl IAccessible_Impl for Acc_Impl {
        fn accParent(&self) -> Result<IDispatch> {
            Err(E_NOTIMPL.into())
        }
        fn accChildCount(&self) -> Result<i32> {
            Ok(CHILDREN)
        }
        fn get_accChild(&self, varchild: &VARIANT) -> Result<IDispatch> {
            let id = child_id(varchild);
            if id <= 0 {
                return Err(E_INVALIDARG.into());
            }
            // 자식마다 새 객체 — 그 아래로도 끝없이
            let child: IAccessible = Acc { depth: self.depth + 1, index: id }.into();
            child.cast()
        }
        fn get_accName(&self, _varchild: &VARIANT) -> Result<BSTR> {
            Ok(BSTR::new())
        }
        fn get_accValue(&self, _varchild: &VARIANT) -> Result<BSTR> {
            Err(E_NOTIMPL.into())
        }
        fn get_accDescription(&self, _varchild: &VARIANT) -> Result<BSTR> {
            Err(E_NOTIMPL.into())
        }
        fn get_accRole(&self, varchild: &VARIANT) -> Result<VARIANT> {
            let root = self.depth == 0 && child_id(varchild) == 0;
            Ok(VARIANT::from(if root { ROLE_CLIENT } else { ROLE_PUSHBUTTON }))
        }
        fn get_accState(&self, _varchild: &VARIANT) -> Result<VARIANT> {
            Ok(VARIANT::from(0i32))
        }
        fn get_accHelp(&self, _varchild: &VARIANT) -> Result<BSTR> {
            Err(E_NOTIMPL.into())
        }
        fn get_accHelpTopic(&self, _file: *mut BSTR, _varchild: &VARIANT) -> Result<i32> {
            Err(E_NOTIMPL.into())
        }
        fn get_accKeyboardShortcut(&self, _varchild: &VARIANT) -> Result<BSTR> {
            Err(E_NOTIMPL.into())
        }
        fn accFocus(&self) -> Result<VARIANT> {
            Ok(VARIANT::default())
        }
        fn accSelection(&self) -> Result<VARIANT> {
            Ok(VARIANT::default())
        }
        fn get_accDefaultAction(&self, _varchild: &VARIANT) -> Result<BSTR> {
            Err(E_NOTIMPL.into())
        }
        fn accSelect(&self, _flags: i32, _varchild: &VARIANT) -> Result<()> {
            Err(E_NOTIMPL.into())
        }
        fn accLocation(&self, pxleft: *mut i32, pytop: *mut i32, pcxwidth: *mut i32, pcyheight: *mut i32, varchild: &VARIANT) -> Result<()> {
            let id = child_id(varchild);
            let root = self.depth == 0 && id == 0;
            match mode() {
                MODE_HANG_LOCATION if !root => block_forever(),
                MODE_SLOW => std::thread::sleep(Duration::from_millis(SLOW_MS.load(Ordering::SeqCst))),
                _ => {}
            }
            let (cx, cy, cw, ch) = client_screen();
            let (x, y, w, h) = if root {
                (cx, cy, cw, ch)
            } else {
                // 자식은 클라이언트 안 20 px 격자에 16 px 칸(깊이마다 조금씩 어긋나게)
                let k = (self.index.max(0) + id.max(0)) as i64 + self.depth as i64 * 7;
                let cols = (cw / 20).max(1) as i64;
                let rows = (ch / 20).max(1) as i64;
                (cx + ((k % cols) * 20) as i32 + 2, cy + (((k / cols) % rows) * 20) as i32 + 2, 16, 16)
            };
            if pxleft.is_null() || pytop.is_null() || pcxwidth.is_null() || pcyheight.is_null() {
                return Err(E_INVALIDARG.into());
            }
            // SAFETY: 부르는 쪽이 준 출력 포인터 네 개(위에서 null 확인).
            unsafe {
                *pxleft = x;
                *pytop = y;
                *pcxwidth = w;
                *pcyheight = h;
            }
            Ok(())
        }
        fn accNavigate(&self, _navdir: i32, _varstart: &VARIANT) -> Result<VARIANT> {
            Err(E_NOTIMPL.into())
        }
        fn accHitTest(&self, _x: i32, _y: i32) -> Result<VARIANT> {
            // 돌아오지 않는 accHitTest(D17)
            block_forever()
        }
        fn accDoDefaultAction(&self, _varchild: &VARIANT) -> Result<()> {
            Err(E_NOTIMPL.into())
        }
        fn put_accName(&self, _varchild: &VARIANT, _name: &BSTR) -> Result<()> {
            Err(E_NOTIMPL.into())
        }
        fn put_accValue(&self, _varchild: &VARIANT, _value: &BSTR) -> Result<()> {
            Err(E_NOTIMPL.into())
        }
    }

    unsafe extern "system" fn proc(h: HWND, msg: u32, w: usize, l: LPARAM) -> LRESULT {
        match msg {
            WM_GETOBJECT if mode() != MODE_HUNG && l as i32 == OBJID_CLIENT => {
                let acc: IAccessible = Acc { depth: 0, index: 0 }.into();
                LresultFromObject(&IAccessible::IID, WPARAM(w), &acc).0
            }
            WM_PAINT => {
                // accLocation이 내놓는 자식 자리(20 px 격자의 16 px 칸) — 이 앱의 논리 좌표
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let dc = BeginPaint(h, &mut ps);
                let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetClientRect(h, &mut r);
                let brush = GetStockObject(GRAY_BRUSH);
                for gy in 0..(r.bottom / 20).max(1) {
                    for gx in 0..(r.right / 20).max(1) {
                        let cell = RECT { left: gx * 20 + 2, top: gy * 20 + 2, right: gx * 20 + 18, bottom: gy * 20 + 18 };
                        FillRect(dc, &cell, brush as _);
                    }
                }
                EndPaint(h, &ps);
                0
            }
            WM_TIMER if mode() == MODE_HUNG => {
                // 메시지 루프를 10초 막는다
                std::thread::sleep(Duration::from_secs(10));
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(h, msg, w, l),
        }
    }

    pub fn run() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let arg = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
        let m = match arg("--mode").as_deref() {
            Some("hung") => MODE_HUNG,
            Some("hang-location") => MODE_HANG_LOCATION,
            Some("slow-msaa") => MODE_SLOW,
            Some("infinite-msaa") | None => MODE_INFINITE,
            Some(other) => {
                eprintln!("모르는 모드: {other} (hung | infinite-msaa | hang-location | slow-msaa)");
                std::process::exit(2);
            }
        };
        MODE.store(m, Ordering::SeqCst);
        if let Some(ms) = arg("--slow-ms").and_then(|v| v.parse().ok()) {
            SLOW_MS.store(ms, Ordering::SeqCst);
        }
        let life: u64 = arg("--life").and_then(|v| v.parse().ok()).unwrap_or(120);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(life));
            std::process::exit(0);
        });
        println!("rb20-frozen: 모드 {}, {life}초 뒤 끝남", arg("--mode").unwrap_or_else(|| "infinite-msaa".into()));

        // SAFETY: UI 스레드 = STA(들어오는 COM 호출이 이 메시지 루프로 온다). 등록 구조체·문자열은 호출 동안 산다.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let class = wide("ScreenInkRb20");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: GetModuleHandleW(null()),
                // COLOR_WINDOW + 1
                hbrBackground: 6 as _,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            let title = wide("RB-20");
            let main = CreateWindowExW(0, class.as_ptr(), title.as_ptr(), WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, 640, 480, null_mut(), null_mut(), GetModuleHandleW(null()), null());
            if main.is_null() {
                eprintln!("창을 만들지 못했어요");
                std::process::exit(1);
            }
            MAIN.store(main as isize, Ordering::SeqCst);
            ShowWindow(main, SW_SHOW);
            if m == MODE_HUNG {
                SetTimer(main, 1, 3000, None);
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
