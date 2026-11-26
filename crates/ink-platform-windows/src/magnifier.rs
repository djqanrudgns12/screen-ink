//! 실시간 확대(5.1, R-ZOOM-1·4) — Windows Magnification API 돋보기 어댑터.
//!
//! 스파이크 실측(spikes/REPORT.md §3)에 맞춘 구조:
//! - **돋보기 스레드 하나가 창을 쥔다**: 호스트 창(맨 위·클릭 통과·활성화 안 함) + 자식 `WC_MAGNIFIER`(확대 커서).
//!   Mag* 호출은 창을 만든 그 스레드에서만. 첫 `WC_MAGNIFIER` 생성이 약 0.9 s(이후도 약 145 ms)라
//!   **앱 시작 때 숨겨서 한 번 만들어 두고**(`prewarm`) 확대마다 그 모니터로 옮겨 보이기·숨기기만 한다.
//! - **프레임 박자 = `MagSetWindowSource`**: 한 번이 화면 주사율만큼(60 Hz ≈ 16.6 ms) 막히므로 따로 자지 않고
//!   프레임마다 부른다(안 막히는 환경이면 16 ms로 보정). 서비스는 `update`로 최신 원본·배율만 넘긴다(잠금 하나).
//! - **휠 훅은 따로 도는 스레드**(`WH_MOUSE_LL`, 확대 중에만): 프레임 루프가 막히는 스레드에 훅을 두면 시스템 전체
//!   마우스가 그만큼 늦어진다. 확대 모니터 안 세로 휠만 먹고, **Shift+휠은 아래 앱으로 넘긴다**(사용자 결정 2026-11-26).
//!   저수준 *키보드* 훅은 쓰지 않는다(R-KEY-1).
//! - 자기 창(호스트는 자동, HUD·섬·팝오버는 명시)을 `MagSetWindowFilterList(EXCLUDE)`로 원본에서 뺀다(무한 거울 0).
//! - 서비스가 3초 넘게 갱신하지 않으면(멈춤·교착) 스스로 숨기고 커서를 되돌린다 → 다음 `update`가 `Err`.
//! - **`stop`은 숨긴 창이 화면 합성에서 빠질 때까지(`DwmFlush`, 한 프레임) 기다린 뒤 돌아온다** — 곧바로 GDI로
//!   정지 캡처를 하는 5.3(Alt+1 판서 이어받기)·확대 중 캡처 키에서 확대 화면이 찍히지 않게. 그래서
//!   `Magnifier::wait_hidden`은 기본(할 일 없음) 그대로 둔다(HUD 묶음 `zoom_hud::HudMagnifier`도 그대로 맞다).

use ink_domain::geometry::{PhysPoint, PhysRect};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows_sys::core::{w, PCWSTR};
use windows_sys::Win32::Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_SHIFT};
use windows_sys::Win32::UI::Magnification::{
    MagInitialize, MagSetWindowFilterList, MagSetWindowSource, MagSetWindowTransform, MagShowSystemCursor, MagUninitialize, MAGTRANSFORM, MS_SHOWMAGNIFIEDCURSOR,
    MW_FILTERMODE_EXCLUDE, WC_MAGNIFIER,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, PeekMessageW, PostMessageW, PostThreadMessageW,
    RegisterClassExW, SetLayeredWindowAttributes, SetWindowPos, SetWindowsHookExW, ShowWindow, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HWND_TOPMOST, LWA_ALPHA,
    MSG, MSLLHOOKSTRUCT, PM_NOREMOVE, PM_REMOVE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, WH_MOUSE_LL, WM_APP, WM_MOUSEWHEEL, WM_QUIT,
    WNDCLASSEXW, WS_CHILD, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

/// 프레임 하나의 목표(≈60 Hz) — `MagSetWindowSource`가 막지 않는 환경에서만 이만큼 쉰다.
const FRAME: Duration = Duration::from_millis(16);
/// 이보다 빨리 돌아오면 "막지 않았다"로 본다.
const NO_BLOCK: Duration = Duration::from_millis(2);
/// 서비스가 이만큼 갱신하지 않으면 스스로 숨긴다.
const WATCHDOG: Duration = Duration::from_secs(3);
/// 첫 진입이 미리 만들기보다 빠르면 기다리는 한도(첫 `WC_MAGNIFIER` ≈ 0.9 s).
const INIT_TIMEOUT: Duration = Duration::from_secs(3);
const CMD_TIMEOUT: Duration = Duration::from_secs(1);
/// 돋보기 스레드 깨우기(명령이 대기열에 있음) · 끝내기.
const WM_WAKE: u32 = WM_APP + 0x51;
const WM_EXIT: u32 = WM_APP + 0x52;
/// 커서가 확대 모니터 안이면 시스템 커서를 숨기고 돋보기 안 확대 커서만 보인다(커서 둘 방지).
/// [확인 필요] 실기에서 확대 커서가 같이 사라지면 false로.
const HIDE_SYSTEM_CURSOR: bool = true;
const CLASS: PCWSTR = w!("ScreenInkMagnifierHost");
/// 휠 한 칸(`WHEEL_DELTA`).
pub const WHEEL_NOTCH: i32 = 120;

// ───────────── 순수 계산(시험은 화면 없이) ─────────────

/// 휠 누적: 고해상도 휠·터치패드는 120보다 작은 값을 여러 번 보낸다. 방향이 바뀌면 누적을 버린다.
/// 돌려주는 것 = (새 누적, 칸 수). 칸 수는 서비스 규약대로 **음수 = 위(더 확대)**.
pub fn wheel_steps(acc: i32, delta: i32) -> (i32, i32) {
    let acc = if acc != 0 && delta != 0 && acc.signum() != delta.signum() { 0 } else { acc };
    let sum = acc.saturating_add(delta);
    let notches = sum / WHEEL_NOTCH;
    (sum - notches * WHEEL_NOTCH, -notches)
}

/// 휠을 배율로 먹을지: 확대 모니터 안 + Shift를 누르지 않음. Shift+휠은 아래 앱으로(사용자 결정).
pub fn eats_wheel(inside_monitor: bool, shift_down: bool) -> bool {
    inside_monitor && !shift_down
}

/// 배율 → 3×3 확대 행렬(이동 없음 — 위치는 원본 사각형이 정한다).
pub fn transform(scale: f64) -> [f32; 9] {
    let s = scale as f32;
    [s, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 1.0]
}

/// 원본에서 뺄 창: 호스트가 맨 앞, 0·중복 없음.
pub fn filter_list(host: isize, exclude: &[isize]) -> Vec<isize> {
    let mut out = vec![host];
    for &h in exclude {
        if h != 0 && !out.contains(&h) {
            out.push(h);
        }
    }
    out
}

pub fn rect(r: PhysRect) -> RECT {
    RECT { left: r.x, top: r.y, right: r.right(), bottom: r.bottom() }
}

// ───────────── 어댑터 ─────────────

#[derive(Clone, Copy)]
struct Latest {
    source: PhysRect,
    scale: f64,
    at: Instant,
}

enum Cmd {
    Show { monitor: PhysRect, exclude: Vec<isize>, reply: mpsc::Sender<Result<(), String>> },
    Hide { reply: mpsc::Sender<()> },
}

struct Shared {
    latest: Mutex<Latest>,
    cmds: Mutex<VecDeque<Cmd>>,
    /// 보이는 중 멈춤 감시가 숨김 — 서비스가 확대를 끝내야 한다
    lost: AtomicBool,
    /// 돋보기 스레드가 끝남(초기화 실패 포함) — 다음 시작 때 다시 만든다
    dead: AtomicBool,
}

impl Shared {
    fn latest(&self) -> Latest {
        *self.latest.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn set_latest(&self, source: PhysRect, scale: f64) {
        *self.latest.lock().unwrap_or_else(|e| e.into_inner()) = Latest { source, scale, at: Instant::now() };
    }

    fn take_cmd(&self) -> Option<Cmd> {
        self.cmds.lock().unwrap_or_else(|e| e.into_inner()).pop_front()
    }
}

/// 미리 만든 돋보기 스레드.
struct Host {
    shared: Arc<Shared>,
    /// 초기화 결과(호스트 HWND) — 처음 한 번 받는다
    init: Option<mpsc::Receiver<Result<isize, String>>>,
    hwnd: Option<isize>,
}

type WheelFn = Arc<dyn Fn(i32) + Send + Sync>;

/// 확대 중에만 도는 휠 훅 스레드.
struct WheelHook {
    thread_id: u32,
    join: Option<JoinHandle<()>>,
}

/// `ink_app::Magnifier` 구현. `on_wheel(dy)`는 휠 훅 스레드에서 불린다 — 막히지 않게 메시지만 넘길 것(잠금 금지).
pub struct WinMagnifier {
    on_wheel: WheelFn,
    exclude: Vec<isize>,
    host: Option<Host>,
    wheel: Option<WheelHook>,
    showing: bool,
}

impl WinMagnifier {
    pub fn new(on_wheel: impl Fn(i32) + Send + Sync + 'static) -> Self {
        WinMagnifier { on_wheel: Arc::new(on_wheel), exclude: Vec::new(), host: None, wheel: None, showing: false }
    }

    /// 돋보기 스레드와 숨긴 창을 지금 만든다(기다리지 않음). 첫 Alt+2가 0.9 s 늦지 않게 앱 시작 때 부른다.
    pub fn prewarm(&mut self) {
        if self.host.as_ref().is_some_and(|h| !h.shared.dead.load(Ordering::SeqCst)) {
            return;
        }
        let shared = Arc::new(Shared {
            latest: Mutex::new(Latest { source: PhysRect::new(0, 0, 1, 1), scale: 1.0, at: Instant::now() }),
            cmds: Mutex::new(VecDeque::new()),
            lost: AtomicBool::new(false),
            dead: AtomicBool::new(false),
        });
        let (init_tx, init_rx) = mpsc::channel();
        let s = shared.clone();
        let spawned = std::thread::Builder::new().name("ink-magnifier".into()).spawn(move || {
            run_host(&s, init_tx);
            s.dead.store(true, Ordering::SeqCst);
        });
        match spawned {
            Ok(_) => self.host = Some(Host { shared, init: Some(init_rx), hwnd: None }),
            Err(e) => log::warn!("magnifier thread: {e}"),
        }
    }

    /// 다음 `start`부터 원본에서 뺄 우리 창(HUD·섬·팝오버 HWND).
    pub fn set_exclude(&mut self, hwnds: Vec<isize>) {
        self.exclude = hwnds;
    }

    /// 돋보기 스레드가 창을 다 만들 때까지(처음 한 번) 기다린다.
    fn ready_host(&mut self) -> Result<(Arc<Shared>, isize), String> {
        self.prewarm();
        let host = self.host.as_mut().ok_or("magnifier thread not started")?;
        if host.hwnd.is_none() {
            let rx = host.init.take().ok_or("magnifier init lost")?;
            match rx.recv_timeout(INIT_TIMEOUT) {
                Ok(Ok(h)) => host.hwnd = Some(h),
                Ok(Err(e)) => {
                    self.host = None;
                    return Err(e);
                }
                Err(_) => {
                    // 아직 만드는 중이면 다음 번에 다시 기다린다
                    host.init = None;
                    self.host = None;
                    return Err("magnifier init timed out".into());
                }
            }
        }
        let host = self.host.as_ref().ok_or("magnifier thread not started")?;
        Ok((host.shared.clone(), host.hwnd.unwrap_or_default()))
    }

    fn send(shared: &Shared, hwnd: isize, cmd: Cmd) -> bool {
        shared.cmds.lock().unwrap_or_else(|e| e.into_inner()).push_back(cmd);
        // SAFETY: 창 핸들 하나에 메시지만 넣는다(없어졌으면 실패할 뿐).
        unsafe { PostMessageW(hwnd as HWND, WM_WAKE, 0, 0) != 0 }
    }

    fn stop_wheel(&mut self) {
        let Some(mut w) = self.wheel.take() else { return };
        // SAFETY: 스레드 메시지 하나.
        unsafe { PostThreadMessageW(w.thread_id, WM_QUIT, 0, 0) };
        if let Some(j) = w.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for WinMagnifier {
    fn drop(&mut self) {
        ink_app::Magnifier::stop(self);
        if let Some(h) = self.host.as_ref().and_then(|h| h.hwnd) {
            // SAFETY: 위와 같음. 기다리지 않는다(앱 종료 중).
            unsafe { PostMessageW(h as HWND, WM_EXIT, 0, 0) };
        }
    }
}

impl ink_app::Magnifier for WinMagnifier {
    fn start(&mut self, monitor: PhysRect, source: PhysRect, scale: f64) -> Result<(), String> {
        self.stop();
        if monitor.is_empty() {
            return Err("empty monitor".into());
        }
        let (shared, hwnd) = self.ready_host()?;
        shared.set_latest(source, scale);
        shared.lost.store(false, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        if !Self::send(&shared, hwnd, Cmd::Show { monitor, exclude: self.exclude.clone(), reply: tx }) {
            self.host = None;
            return Err("magnifier thread gone".into());
        }
        match rx.recv_timeout(CMD_TIMEOUT) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err("magnifier show timed out".into()),
        }
        self.showing = true;
        self.wheel = spawn_wheel(monitor, self.on_wheel.clone());
        if self.wheel.is_none() {
            log::warn!("wheel hook unavailable — wheel zoom off, keys still work");
        }
        Ok(())
    }

    fn update(&mut self, source: PhysRect, scale: f64) -> Result<(), String> {
        let Some(host) = self.host.as_ref().filter(|_| self.showing) else { return Err("magnifier not showing".into()) };
        if host.shared.lost.load(Ordering::SeqCst) || host.shared.dead.load(Ordering::SeqCst) {
            self.stop();
            return Err("magnifier stopped".into());
        }
        host.shared.set_latest(source, scale);
        Ok(())
    }

    fn stop(&mut self) {
        self.stop_wheel();
        if !std::mem::take(&mut self.showing) {
            return;
        }
        let Some((shared, hwnd)) = self.host.as_ref().and_then(|h| h.hwnd.map(|w| (h.shared.clone(), w))) else { return };
        let (tx, rx) = mpsc::channel();
        if !Self::send(&shared, hwnd, Cmd::Hide { reply: tx }) {
            return;
        }
        if rx.recv_timeout(CMD_TIMEOUT).is_err() {
            log::warn!("magnifier hide timed out");
            return;
        }
        wait_composed();
    }
}

/// 숨긴 돋보기가 빠진 화면이 한 번 합성될 때까지(다음 DWM present) 기다린다 — 그 뒤의 GDI 캡처에는 돋보기가 없다.
/// 60 Hz에서 0~16.7 ms. [실기 확인 필요] 학교 PC에서 Alt+2 → Alt+1 판서 배경에 확대 화면이 겹쳐 찍히지 않는지.
fn wait_composed() {
    let t = Instant::now();
    // SAFETY: 인자 없는 동기 호출. 실패(합성 꺼짐 등)는 기다리지 않은 것과 같다.
    let hr = unsafe { DwmFlush() };
    if hr < 0 {
        log::debug!("DwmFlush failed (0x{hr:08X})");
    } else {
        log::debug!("magnifier hidden, composed in {} ms", t.elapsed().as_millis());
    }
}

// ───────────── 휠 훅 스레드 ─────────────

struct HookCtx {
    monitor: PhysRect,
    acc: i32,
    cb: WheelFn,
}

thread_local! {
    static HOOK: RefCell<Option<HookCtx>> = const { RefCell::new(None) };
}

/// 확대 모니터 안의 세로 휠(Shift 없이)만 먹고 배율 칸으로 넘긴다. 나머지는 그대로 흘려보낸다.
unsafe extern "system" fn wheel_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wparam as u32 == WM_MOUSEWHEEL && lparam != 0 {
        // SAFETY: WH_MOUSE_LL의 lParam은 MSLLHOOKSTRUCT를 가리킨다(이 호출 동안 유효).
        let info = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        let delta = (info.mouseData >> 16) as u16 as i16 as i32;
        let pt = PhysPoint { x: info.pt.x, y: info.pt.y };
        // SAFETY: 키 상태 읽기만.
        let shift = unsafe { GetAsyncKeyState(VK_SHIFT as i32) } as u16 & 0x8000 != 0;
        let eaten = HOOK.with(|h| {
            let mut h = h.borrow_mut();
            let Some(ctx) = h.as_mut() else { return false };
            if !eats_wheel(ctx.monitor.contains(pt), shift) {
                ctx.acc = 0;
                return false;
            }
            let (acc, dy) = wheel_steps(ctx.acc, delta);
            ctx.acc = acc;
            for _ in 0..dy.unsigned_abs() {
                (ctx.cb)(dy.signum());
            }
            true
        });
        if eaten {
            return 1;
        }
    }
    // SAFETY: 훅 사슬 다음으로 그대로 넘긴다.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

/// 훅만 도는 스레드(메시지를 곧바로 돌려 마우스가 늦지 않게). 훅을 못 걸면 `None`.
fn spawn_wheel(monitor: PhysRect, cb: WheelFn) -> Option<WheelHook> {
    let (tx, rx) = mpsc::channel::<Option<u32>>();
    let join = std::thread::Builder::new()
        .name("ink-zoom-wheel".into())
        .spawn(move || {
            // SAFETY: 이 스레드의 훅·메시지 대기열만 다룬다. 끝나기 전에 훅을 푼다.
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                // 대기열을 먼저 만든다(그 뒤에 온 WM_QUIT를 놓치지 않게)
                PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                HOOK.with(|h| *h.borrow_mut() = Some(HookCtx { monitor, acc: 0, cb }));
                let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(wheel_hook), GetModuleHandleW(std::ptr::null()), 0);
                if hook.is_null() {
                    log::warn!("SetWindowsHookExW(WH_MOUSE_LL) failed ({})", GetLastError());
                    let _ = tx.send(None);
                    return;
                }
                let _ = tx.send(Some(GetCurrentThreadId()));
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                UnhookWindowsHookEx(hook);
                HOOK.with(|h| *h.borrow_mut() = None);
            }
        })
        .ok()?;
    match rx.recv_timeout(CMD_TIMEOUT) {
        Ok(Some(thread_id)) => Some(WheelHook { thread_id, join: Some(join) }),
        _ => {
            let _ = join.join();
            None
        }
    }
}

// ───────────── 돋보기 스레드 ─────────────

unsafe extern "system" fn host_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: 기본 창 처리.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// 시스템 커서 숨김 상태(바뀔 때만 부른다 — 숨길 때 반드시 되돌린다).
#[derive(Default)]
struct CursorState {
    hidden: bool,
}

impl CursorState {
    unsafe fn sync(&mut self, monitor: PhysRect) {
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: 출력 점 하나.
        let inside = unsafe { GetCursorPos(&mut p) } != 0 && monitor.contains(PhysPoint { x: p.x, y: p.y });
        if inside != self.hidden {
            // SAFETY: 이 프로세스의 확대 런타임 안에서 커서 표시만 바꾼다.
            unsafe { MagShowSystemCursor(if inside { 0 } else { 1 }) };
            self.hidden = inside;
        }
    }

    unsafe fn restore(&mut self) {
        if self.hidden {
            // SAFETY: 위와 같음.
            unsafe { MagShowSystemCursor(1) };
            self.hidden = false;
        }
    }
}

/// 타이머 간격 통계(앱 로그용 — 진짜 화면 fps는 PresentMon으로, bench/m6-zoom.ps1).
struct FrameStats {
    started: Instant,
    last: Option<Instant>,
    frames: u64,
    max_gap: Duration,
}

impl FrameStats {
    fn new() -> Self {
        FrameStats { started: Instant::now(), last: None, frames: 0, max_gap: Duration::ZERO }
    }

    fn frame(&mut self) {
        let now = Instant::now();
        if let Some(l) = self.last {
            self.max_gap = self.max_gap.max(now - l);
        }
        self.last = Some(now);
        self.frames += 1;
    }

    fn line(&self) -> String {
        let secs = self.started.elapsed().as_secs_f64().max(1e-3);
        format!("zoom frames {} in {:.1} s (avg {:.1} fps, max gap {} ms)", self.frames, secs, self.frames as f64 / secs, self.max_gap.as_millis())
    }
}

unsafe fn register_class(inst: HINSTANCE) {
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(host_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: inst,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: CLASS,
        hIconSm: std::ptr::null_mut(),
    };
    // SAFETY: 구조체는 지역 변수, 클래스 이름은 정적 문자열. 두 번째부터는 이미 있다(실패 무시).
    unsafe { RegisterClassExW(&wc) };
}

/// 보이는 동안의 상태.
struct Showing {
    monitor: PhysRect,
    scale: Option<f64>,
    cursor: CursorState,
    stats: FrameStats,
}

/// 돋보기 스레드 본체: 숨긴 창을 만들고 → (명령을 기다림 ⇄ 보이는 동안 프레임 루프) → 끝나면 정리.
fn run_host(shared: &Shared, init: mpsc::Sender<Result<isize, String>>) {
    // SAFETY: 이 스레드가 만든 창만 다루고, 끝나기 전에 없앤다. 포인터는 모두 지역 변수.
    unsafe {
        let t0 = Instant::now();
        // 물리 px로 놓고 원본도 물리 px(혼합 배율). 프로세스가 이미 PMv2여도 해가 없다.
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if MagInitialize() == 0 {
            let _ = init.send(Err(format!("MagInitialize failed ({})", GetLastError())));
            return;
        }
        let inst = GetModuleHandleW(std::ptr::null());
        register_class(inst);
        // 숨긴 채 1×1로 만든다(WS_VISIBLE 없음) — 확대할 때 모니터로 옮긴다
        let host = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!("screen-ink magnifier"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            inst,
            std::ptr::null(),
        );
        if host.is_null() {
            let e = GetLastError();
            MagUninitialize();
            let _ = init.send(Err(format!("magnifier host window failed ({e})")));
            return;
        }
        SetLayeredWindowAttributes(host, 0, 255, LWA_ALPHA);
        let mag = CreateWindowExW(
            0,
            WC_MAGNIFIER,
            w!("screen-ink magnifier view"),
            WS_CHILD | WS_VISIBLE | MS_SHOWMAGNIFIEDCURSOR as u32,
            0,
            0,
            1,
            1,
            host,
            std::ptr::null_mut(),
            inst,
            std::ptr::null(),
        );
        if mag.is_null() {
            let e = GetLastError();
            DestroyWindow(host);
            MagUninitialize();
            let _ = init.send(Err(format!("WC_MAGNIFIER failed ({e})")));
            return;
        }
        log::info!("magnifier prewarmed in {} ms", t0.elapsed().as_millis());
        let _ = init.send(Ok(host as isize));

        let mut showing: Option<Showing> = None;
        let mut msg: MSG = std::mem::zeroed();
        'run: loop {
            // 숨긴 동안은 메시지를 기다리고, 보이는 동안은 쌓인 것만 돌린다
            if showing.is_none() {
                let r = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                if r == 0 || r == -1 {
                    break 'run;
                }
                if !handle(shared, host, mag, &msg, &mut showing) {
                    break 'run;
                }
                continue;
            }
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT || !handle(shared, host, mag, &msg, &mut showing) {
                    break 'run;
                }
            }
            let Some(s) = showing.as_mut() else { continue };
            let l = shared.latest();
            if l.at.elapsed() > WATCHDOG {
                log::warn!("magnifier watchdog: no update for {} ms — hiding", l.at.elapsed().as_millis());
                shared.lost.store(true, Ordering::SeqCst);
                hide(host, &mut showing);
                continue;
            }
            if s.scale != Some(l.scale) {
                let mut t = MAGTRANSFORM { v: transform(l.scale) };
                if MagSetWindowTransform(mag, &mut t) == 0 {
                    log::warn!("MagSetWindowTransform failed ({})", GetLastError());
                }
                s.scale = Some(l.scale);
            }
            // 원본을 다시 비춘다 — 이 호출이 주사율만큼 막혀 프레임 박자가 된다(REPORT §3)
            let t = Instant::now();
            MagSetWindowSource(mag, rect(l.source));
            InvalidateRect(mag, std::ptr::null(), 0);
            if HIDE_SYSTEM_CURSOR {
                s.cursor.sync(s.monitor);
            }
            s.stats.frame();
            let took = t.elapsed();
            if took < NO_BLOCK {
                std::thread::sleep(FRAME - took);
            }
        }

        hide(host, &mut showing);
        DestroyWindow(host);
        MagUninitialize();
    }
}

/// 메시지 하나. `false` = 스레드를 끝낸다.
unsafe fn handle(shared: &Shared, host: HWND, mag: HWND, msg: &MSG, showing: &mut Option<Showing>) -> bool {
    match msg.message {
        WM_EXIT => false,
        WM_WAKE => {
            while let Some(cmd) = shared.take_cmd() {
                match cmd {
                    // SAFETY: 이 스레드가 만든 창.
                    Cmd::Show { monitor, exclude, reply } => {
                        let r = unsafe { show(shared, host, mag, monitor, &exclude, showing) };
                        let _ = reply.send(r);
                    }
                    Cmd::Hide { reply } => {
                        // SAFETY: 위와 같음.
                        unsafe { hide(host, showing) };
                        let _ = reply.send(());
                    }
                }
            }
            true
        }
        _ => {
            // SAFETY: 이 스레드 대기열의 메시지.
            unsafe {
                TranslateMessage(msg);
                DispatchMessageW(msg);
            }
            true
        }
    }
}

/// 그 모니터로 옮기고 → 제외 목록·배율·원본 → 보이고 맨 위로. 첫 그림까지 한 프레임.
unsafe fn show(shared: &Shared, host: HWND, mag: HWND, m: PhysRect, exclude: &[isize], showing: &mut Option<Showing>) -> Result<(), String> {
    // SAFETY: 이 스레드가 만든 창, 버퍼는 지역 변수.
    unsafe {
        hide(host, showing);
        if SetWindowPos(host, HWND_TOPMOST, m.x, m.y, m.w, m.h, SWP_NOACTIVATE) == 0 {
            return Err(format!("magnifier move failed ({})", GetLastError()));
        }
        SetWindowPos(mag, std::ptr::null_mut(), 0, 0, m.w, m.h, SWP_NOACTIVATE | SWP_NOZORDER);
        // 자기 창 제외(무한 거울 0, R-ZOOM-4). 호스트는 자동으로 빠지지만 함께 넣어 둔다.
        let mut filter: Vec<HWND> = filter_list(host as isize, exclude).into_iter().map(|h| h as HWND).collect();
        if MagSetWindowFilterList(mag, MW_FILTERMODE_EXCLUDE, filter.len() as i32, filter.as_mut_ptr()) == 0 {
            log::warn!("MagSetWindowFilterList failed ({})", GetLastError());
        }
        let l = shared.latest();
        let mut t = MAGTRANSFORM { v: transform(l.scale) };
        MagSetWindowTransform(mag, &mut t);
        MagSetWindowSource(mag, rect(l.source));
        ShowWindow(host, SW_SHOWNOACTIVATE);
        SetWindowPos(host, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOSIZE | SWP_NOMOVE);
        InvalidateRect(mag, std::ptr::null(), 0);
        UpdateWindow(mag);
        let mut s = Showing { monitor: m, scale: Some(l.scale), cursor: CursorState::default(), stats: FrameStats::new() };
        if HIDE_SYSTEM_CURSOR {
            s.cursor.sync(m);
        }
        *showing = Some(s);
        Ok(())
    }
}

unsafe fn hide(host: HWND, showing: &mut Option<Showing>) {
    let Some(mut s) = showing.take() else { return };
    // SAFETY: 이 스레드가 만든 창.
    unsafe {
        ShowWindow(host, SW_HIDE);
        s.cursor.restore();
    }
    log::info!("{}", s.stats.line());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 휠_누적_120마다_한_칸_위는_음수() {
        assert_eq!(wheel_steps(0, 120), (0, -1));
        assert_eq!(wheel_steps(0, -120), (0, 1));
        assert_eq!(wheel_steps(0, 240), (0, -2));
        // 고해상도 휠: 40씩 세 번 = 한 칸
        let (a, d) = wheel_steps(0, 40);
        assert_eq!((a, d), (40, 0));
        let (a, d) = wheel_steps(a, 40);
        assert_eq!((a, d), (80, 0));
        assert_eq!(wheel_steps(a, 40), (0, -1));
        // 방향이 바뀌면 누적을 버린다
        assert_eq!(wheel_steps(100, -30), (-30, 0));
        assert_eq!(wheel_steps(0, 0), (0, 0));
    }

    #[test]
    fn 시프트_휠과_다른_모니터_휠은_넘긴다() {
        assert!(eats_wheel(true, false));
        assert!(!eats_wheel(true, true), "Shift+휠은 아래 앱으로(사용자 결정)");
        assert!(!eats_wheel(false, false));
        assert!(!eats_wheel(false, true));
    }

    #[test]
    fn 확대_행렬과_원본_사각형() {
        assert_eq!(transform(2.5), [2.5, 0.0, 0.0, 0.0, 2.5, 0.0, 0.0, 0.0, 1.0]);
        let r = rect(PhysRect::new(-1920, -200, 640, 360));
        assert_eq!((r.left, r.top, r.right, r.bottom), (-1920, -200, -1280, 160));
    }

    #[test]
    fn 제외_목록은_호스트가_맨_앞이고_중복_없음() {
        assert_eq!(filter_list(7, &[3, 0, 7, 3, 9]), vec![7, 3, 9]);
        assert_eq!(filter_list(7, &[]), vec![7]);
    }
}
