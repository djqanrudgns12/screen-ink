//! 실시간 확대(5.1, R-ZOOM-1·4) — Windows Magnification API 돋보기 어댑터.
//!
//! 전용 스레드 하나가 모두 쥔다: 호스트 창(맨 위·클릭 통과·활성화 안 함) + 자식 `WC_MAGNIFIER`(확대 커서 표시),
//! 16 ms 타이머(움직이는 화면을 다시 비춤), 휠 훅(`WH_MOUSE_LL`, 확대 중에만 — 저수준 *키보드* 훅은 쓰지 않는다 R-KEY-1).
//! Mag* 호출은 창을 만든 그 스레드에서만 한다. 서비스는 `update`로 최신 원본·배율만 넘기고(잠금 하나),
//! 스레드가 타이머마다 반영한다. 자기 창(호스트·HUD·섬·팝오버)은 `MagSetWindowFilterList(EXCLUDE)`로 원본에서 뺀다.
//! 서비스가 3초 넘게 갱신하지 않으면(멈춤·교착) 스스로 내리고 훅·커서를 되돌린다(R-ZOOM 위험표).

use ink_domain::geometry::{PhysPoint, PhysRect};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows_sys::core::{w, PCWSTR};
use windows_sys::Win32::Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows_sys::Win32::UI::Magnification::{
    MagInitialize, MagSetWindowFilterList, MagSetWindowSource, MagSetWindowTransform, MagShowSystemCursor, MagUninitialize, MAGTRANSFORM, MS_SHOWMAGNIFIEDCURSOR,
    MW_FILTERMODE_EXCLUDE, WC_MAGNIFIER,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, KillTimer, PostMessageW, RegisterClassExW,
    SetLayeredWindowAttributes, SetTimer, SetWindowPos, SetWindowsHookExW, ShowWindow, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HWND_TOPMOST, LWA_ALPHA, MSG,
    MSLLHOOKSTRUCT, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE, WH_MOUSE_LL, WM_APP, WM_MOUSEWHEEL, WM_TIMER, WNDCLASSEXW, WS_CHILD,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

/// 다시 비추는 간격(≈60 Hz). M6(G6) 목표는 1080p ≥ 30 fps.
const FRAME_MS: u32 = 16;
/// 서비스가 이만큼 갱신하지 않으면 스스로 내린다.
const WATCHDOG: Duration = Duration::from_secs(3);
const START_TIMEOUT: Duration = Duration::from_secs(2);
const STOP_TIMEOUT: Duration = Duration::from_millis(500);
const WM_STOP: u32 = WM_APP + 0x51;
const TIMER_ID: usize = 1;
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

struct Shared {
    latest: Mutex<Latest>,
    /// 스레드가 끝남(요청·멈춤 감시·실패)
    stopped: AtomicBool,
    /// 시작 대기 시간이 지남 — 뜨자마자 내려라
    cancel: AtomicBool,
}

impl Shared {
    fn latest(&self) -> Latest {
        *self.latest.lock().unwrap_or_else(|e| e.into_inner())
    }
}

struct Running {
    shared: Arc<Shared>,
    host: isize,
    thread: Option<JoinHandle<()>>,
    done: mpsc::Receiver<()>,
}

type WheelFn = Arc<dyn Fn(i32) + Send + Sync>;

/// `ink_app::Magnifier` 구현. `on_wheel(dy)`는 훅 스레드에서 불린다 — 막히지 않게 메시지만 넘길 것(잠금 금지).
pub struct WinMagnifier {
    on_wheel: WheelFn,
    exclude: Vec<isize>,
    run: Option<Running>,
}

impl WinMagnifier {
    pub fn new(on_wheel: impl Fn(i32) + Send + Sync + 'static) -> Self {
        WinMagnifier { on_wheel: Arc::new(on_wheel), exclude: Vec::new(), run: None }
    }

    /// 다음 `start`부터 원본에서 뺄 우리 창(HUD·섬·팝오버 HWND).
    pub fn set_exclude(&mut self, hwnds: Vec<isize>) {
        self.exclude = hwnds;
    }

    /// 지금 돋보기 호스트 창(HUD를 그 위로 올릴 때).
    pub fn host(&self) -> Option<isize> {
        self.run.as_ref().map(|r| r.host)
    }
}

impl Drop for WinMagnifier {
    fn drop(&mut self) {
        ink_app::Magnifier::stop(self);
    }
}

struct ThreadArgs {
    monitor: PhysRect,
    exclude: Vec<isize>,
    on_wheel: WheelFn,
    shared: Arc<Shared>,
}

impl ink_app::Magnifier for WinMagnifier {
    fn start(&mut self, monitor: PhysRect, source: PhysRect, scale: f64) -> Result<(), String> {
        self.stop();
        if monitor.is_empty() {
            return Err("empty monitor".into());
        }
        let shared = Arc::new(Shared { latest: Mutex::new(Latest { source, scale, at: Instant::now() }), stopped: AtomicBool::new(false), cancel: AtomicBool::new(false) });
        let (ready_tx, ready_rx) = mpsc::channel::<Result<isize, String>>();
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let args = ThreadArgs { monitor, exclude: self.exclude.clone(), on_wheel: self.on_wheel.clone(), shared: shared.clone() };
        let thread = std::thread::Builder::new()
            .name("ink-magnifier".into())
            .spawn(move || {
                let shared = args.shared.clone();
                run_thread(args, ready_tx);
                shared.stopped.store(true, Ordering::SeqCst);
                let _ = done_tx.send(());
            })
            .map_err(|e| format!("magnifier thread: {e}"))?;
        match ready_rx.recv_timeout(START_TIMEOUT) {
            Ok(Ok(host)) => {
                self.run = Some(Running { shared, host, thread: Some(thread), done: done_rx });
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                // 떼어 둔다: 늦게 뜨면 cancel을 보고 스스로 내린다
                shared.cancel.store(true, Ordering::SeqCst);
                Err("magnifier start timed out".into())
            }
        }
    }

    fn update(&mut self, source: PhysRect, scale: f64) -> Result<(), String> {
        let stopped = match &self.run {
            None => return Err("magnifier not running".into()),
            Some(r) => r.shared.stopped.load(Ordering::SeqCst),
        };
        if stopped {
            self.stop();
            return Err("magnifier stopped".into());
        }
        if let Some(r) = &self.run {
            *r.shared.latest.lock().unwrap_or_else(|e| e.into_inner()) = Latest { source, scale, at: Instant::now() };
        }
        Ok(())
    }

    fn stop(&mut self) {
        let Some(mut run) = self.run.take() else { return };
        // SAFETY: 창 핸들 하나에 메시지만 넣는다(이미 없어졌으면 실패할 뿐).
        unsafe {
            PostMessageW(run.host as HWND, WM_STOP, 0, 0);
        }
        match run.done.recv_timeout(STOP_TIMEOUT) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(t) = run.thread.take() {
                    let _ = t.join();
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => log::warn!("magnifier thread did not stop in {} ms", STOP_TIMEOUT.as_millis()),
        }
    }
}

// ───────────── 돋보기 스레드 ─────────────

struct HookCtx {
    monitor: PhysRect,
    acc: i32,
    cb: WheelFn,
}

thread_local! {
    static HOOK: RefCell<Option<HookCtx>> = const { RefCell::new(None) };
}

/// 확대 모니터 안의 세로 휠만 먹고 배율 칸으로 넘긴다. 다른 모니터·가로 휠은 그대로 흘려보낸다.
unsafe extern "system" fn wheel_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wparam as u32 == WM_MOUSEWHEEL && lparam != 0 {
        // SAFETY: WH_MOUSE_LL의 lParam은 MSLLHOOKSTRUCT를 가리킨다(이 호출 동안 유효).
        let info = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        let delta = (info.mouseData >> 16) as u16 as i16 as i32;
        let pt = PhysPoint { x: info.pt.x, y: info.pt.y };
        let eaten = HOOK.with(|h| {
            let mut h = h.borrow_mut();
            let Some(ctx) = h.as_mut() else { return false };
            if !ctx.monitor.contains(pt) {
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

unsafe extern "system" fn host_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: 기본 창 처리.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// 시스템 커서 숨김 상태(바뀔 때만 부른다 — 끝날 때 반드시 되돌린다).
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

/// 마지막으로 돋보기에 건 값(같으면 다시 부르지 않는다).
#[derive(Default)]
struct Applied {
    source: Option<PhysRect>,
    scale: Option<f64>,
}

unsafe fn apply(mag: HWND, l: &Latest, a: &mut Applied) {
    if a.scale != Some(l.scale) {
        let mut t = MAGTRANSFORM { v: transform(l.scale) };
        // SAFETY: 이 스레드가 만든 돋보기 창, 행렬은 지역 변수.
        if unsafe { MagSetWindowTransform(mag, &mut t) } == 0 {
            log::warn!("MagSetWindowTransform failed ({})", unsafe { GetLastError() });
        }
        a.scale = Some(l.scale);
    }
    if a.source != Some(l.source) {
        // SAFETY: 위와 같음.
        if unsafe { MagSetWindowSource(mag, rect(l.source)) } == 0 {
            log::warn!("MagSetWindowSource failed ({})", unsafe { GetLastError() });
        }
        a.source = Some(l.source);
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
    // 두 번째 확대부터는 이미 있다(실패 무시)
    // SAFETY: 구조체는 지역 변수, 클래스 이름은 정적 문자열.
    unsafe { RegisterClassExW(&wc) };
}

fn run_thread(a: ThreadArgs, ready: mpsc::Sender<Result<isize, String>>) {
    // SAFETY: 이 스레드가 만든 창·훅·타이머만 다루고, 끝나기 전에 모두 없앤다. 포인터는 모두 지역 변수.
    unsafe {
        let t0 = Instant::now();
        // 물리 px로 놓고 원본도 물리 px(혼합 배율). 프로세스가 이미 PMv2여도 해가 없다.
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if MagInitialize() == 0 {
            let _ = ready.send(Err(format!("MagInitialize failed ({})", GetLastError())));
            return;
        }
        let inst = GetModuleHandleW(std::ptr::null());
        register_class(inst);
        let m = a.monitor;
        let host = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!("screen-ink magnifier"),
            WS_POPUP,
            m.x,
            m.y,
            m.w,
            m.h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            inst,
            std::ptr::null(),
        );
        if host.is_null() {
            let e = GetLastError();
            MagUninitialize();
            let _ = ready.send(Err(format!("magnifier host window failed ({e})")));
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
            m.w,
            m.h,
            host,
            std::ptr::null_mut(),
            inst,
            std::ptr::null(),
        );
        if mag.is_null() {
            let e = GetLastError();
            DestroyWindow(host);
            MagUninitialize();
            let _ = ready.send(Err(format!("WC_MAGNIFIER failed ({e})")));
            return;
        }
        // 자기 창 제외(무한 거울 0, R-ZOOM-4)
        let mut filter: Vec<HWND> = filter_list(host as isize, &a.exclude).into_iter().map(|h| h as HWND).collect();
        if MagSetWindowFilterList(mag, MW_FILTERMODE_EXCLUDE, filter.len() as i32, filter.as_mut_ptr()) == 0 {
            log::warn!("MagSetWindowFilterList failed ({})", GetLastError());
        }
        let mut applied = Applied::default();
        apply(mag, &a.shared.latest(), &mut applied);

        HOOK.with(|h| *h.borrow_mut() = Some(HookCtx { monitor: m, acc: 0, cb: a.on_wheel.clone() }));
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(wheel_hook), inst, 0);
        if hook.is_null() {
            log::warn!("wheel hook failed ({}) — wheel zoom off, keys still work", GetLastError());
        }
        let mut cursor = CursorState::default();
        ShowWindow(host, SW_SHOWNOACTIVATE);
        SetWindowPos(host, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
        InvalidateRect(mag, std::ptr::null(), 0);
        UpdateWindow(mag);
        if HIDE_SYSTEM_CURSOR {
            cursor.sync(m);
        }
        SetTimer(host, TIMER_ID, FRAME_MS, None);

        let cancelled = a.shared.cancel.load(Ordering::SeqCst);
        if !cancelled {
            let _ = ready.send(Ok(host as isize));
            log::debug!("magnifier up in {:.1} ms", t0.elapsed().as_secs_f64() * 1000.0);
        }
        let mut stats = FrameStats::new();
        let mut msg: MSG = std::mem::zeroed();
        // 시작 대기 시간이 지났으면 고리를 돌지 않고 바로 정리
        loop {
            if cancelled {
                break;
            }
            let r = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if r == 0 || r == -1 || msg.message == WM_STOP {
                break;
            }
            if msg.message == WM_TIMER && msg.hwnd == host && msg.wParam == TIMER_ID {
                let l = a.shared.latest();
                if l.at.elapsed() > WATCHDOG {
                    log::warn!("magnifier watchdog: no update for {} ms — stopping", l.at.elapsed().as_millis());
                    break;
                }
                apply(mag, &l, &mut applied);
                InvalidateRect(mag, std::ptr::null(), 0);
                if HIDE_SYSTEM_CURSOR {
                    cursor.sync(m);
                }
                stats.frame();
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // 정리: 훅 → 커서 → 타이머·창 → 런타임
        if !hook.is_null() {
            UnhookWindowsHookEx(hook);
        }
        HOOK.with(|h| *h.borrow_mut() = None);
        cursor.restore();
        KillTimer(host, TIMER_ID);
        DestroyWindow(host);
        MagUninitialize();
        log::info!("{}", stats.line());
    }
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
