//! RB-15(최대화 창·작업 표시줄·바탕 화면)·RB-16(투명 덮개 3종) 자동 시험(08 §5.1, D1·D4, C §2.3 `3a-rb1516`).
//!
//! - **순수 판정**(어느 OS에서나): `top_windows()`가 그 장면에서 읽을 창 목록을 그대로 적어 `filter_windows`·`window_at`이
//!   맞게 고르는지 본다.
//! - **실제 창**(Windows, `#[ignore]`): 이 시험이 Win32 창을 직접 띄우고 `top_windows()` → `filter_windows()`로 같은 것을
//!   확인한다. 화면에 잠깐(1초 안) 창이 뜨고 포커스를 가져갈 수 있어서 메인 세션이 따로 돌린다:
//!   `cargo test -p ink-platform-windows --test rb15_16 -- --ignored --test-threads=1`
//!
//! 창 제목은 쓰지 않는다(빈 제목, R-LOG-1).

use ink_domain::geometry::{Monitor, MonitorLayout, PhysPoint, PhysRect};
use ink_domain::recog::{build_chain, first_level, Evidence, NodeKind};
use ink_domain::snapshot::{filter_windows, CaptureKind, RawWindow, Snapshot};

fn two_monitors() -> MonitorLayout {
    MonitorLayout::new(vec![
        Monitor { id: 0, bounds: PhysRect::new(0, 0, 1920, 1080), work: PhysRect::new(0, 0, 1920, 1040), scale: 1.0, primary: true },
        Monitor { id: 1, bounds: PhysRect::new(1920, 0, 2560, 1440), work: PhysRect::new(1920, 0, 2560, 1392), scale: 1.5, primary: false },
    ])
}

fn win(handle: u64, class: &str, rect: PhysRect) -> RawWindow {
    RawWindow { handle, pid: 1000 + handle as u32, exe: "rbtest.exe".into(), class: class.into(), rect, visible: true, ..Default::default() }
}

fn snap(layout: &MonitorLayout, raw: &[RawWindow]) -> Snapshot {
    Snapshot { id: 1, kind: CaptureKind::Region, layout: layout.clone(), windows: filter_windows(raw, layout, 0, &[]), cursor: PhysPoint { x: 0, y: 0 } }
}

fn center(r: PhysRect) -> PhysPoint {
    PhysPoint { x: r.x + r.w / 2, y: r.y + r.h / 2 }
}

// ───────────── 순수 판정 ─────────────

/// RB-16: 앞에서부터 모니터 전체 맨 위 계층 창(투명도 1) · 투명도 0 계층 창 · 클릭 통과 계층 창 · 시험 창 · 바탕 화면.
fn rb16_scene(layout: &MonitorLayout) -> Vec<RawWindow> {
    let m = layout.monitors[0].bounds;
    let target = PhysRect::new(200, 150, 800, 600);
    vec![
        RawWindow { topmost: true, tool: true, layered_alpha: Some(1), ..win(1, "RbCover", m) },
        RawWindow { topmost: true, tool: true, layered_alpha: Some(0), ..win(2, "RbAlphaZero", PhysRect::new(300, 200, 600, 400)) },
        RawWindow { topmost: true, tool: true, layered_alpha: Some(60), click_through: true, ..win(3, "RbClickThrough", PhysRect::new(250, 180, 700, 500)) },
        win(4, "RbTarget", target),
        RawWindow { exe: "explorer.exe".into(), ..win(5, "Progman", layout.virtual_bounds()) },
    ]
}

#[test]
fn rb16_판정_투명_덮개_3종은_첫_강조에_안_잡힌다() {
    let l = two_monitors();
    let s = snap(&l, &rb16_scene(&l));
    let handles: Vec<u64> = s.windows.iter().map(|w| w.handle).collect();
    assert!(!handles.contains(&2), "투명도 0 계층 창은 후보에서 뺀다: {handles:?}");
    assert!(!handles.contains(&3), "클릭 통과(WS_EX_TRANSPARENT) 창은 후보에서 뺀다: {handles:?}");
    let cover = s.windows.iter().find(|w| w.handle == 1).expect("모니터 덮개는 후보로는 남는다(Tab으로 고를 수 있게)");
    assert!(cover.suspect_overlay, "모니터 전체 맨 위 계층 창 = 덮개 의심");
    // 세 덮개가 모두 겹치는 점에서 첫 강조는 아래의 시험 창
    let p = center(PhysRect::new(300, 200, 600, 400));
    assert_eq!(s.window_at(p, true).map(|w| w.handle), Some(4));
    assert_eq!(s.window_at(p, false).map(|w| w.handle), Some(1), "덮개를 건너뛰지 않으면 덮개가 맨 앞");
}

#[test]
fn rb16_판정_덮개는_다른_모니터의_창_고르기에_끼지_않는다() {
    let l = two_monitors();
    let mut raw = rb16_scene(&l);
    raw.insert(4, win(6, "RbOther", PhysRect::new(2200, 200, 900, 700)));
    let s = snap(&l, &raw);
    assert_eq!(s.window_at(PhysPoint { x: 2500, y: 500 }, true).map(|w| w.handle), Some(6));
}

/// RB-15: 앞에서부터 작업 표시줄 둘 · 첫 모니터 최대화 창 · 둘째 모니터 최대화 창 · 바탕 화면.
fn rb15_scene(layout: &MonitorLayout) -> Vec<RawWindow> {
    vec![
        RawWindow { exe: "explorer.exe".into(), topmost: true, ..win(1, "Shell_TrayWnd", PhysRect::new(0, 1040, 1920, 40)) },
        RawWindow { exe: "explorer.exe".into(), topmost: true, ..win(2, "Shell_SecondaryTrayWnd", PhysRect::new(1920, 1392, 2560, 48)) },
        // 최대화 창은 화면 밖으로 8 px씩 나간다(GetWindowRect·DWM 경계 어느 쪽이든)
        RawWindow { maximized: true, ..win(3, "RbMaxed", PhysRect::new(-8, -8, 1936, 1056)) },
        RawWindow { maximized: true, ..win(4, "RbMaxed2", PhysRect::new(1912, -8, 2576, 1408)) },
        RawWindow { exe: "explorer.exe".into(), ..win(5, "Progman", layout.virtual_bounds()) },
    ]
}

#[test]
fn rb15_판정_최대화_창은_그_모니터의_작업_영역으로() {
    let l = two_monitors();
    let s = snap(&l, &rb15_scene(&l));
    let rect = |h: u64| s.windows.iter().find(|w| w.handle == h).map(|w| w.rect);
    assert_eq!(rect(3), Some(l.monitors[0].work), "첫 모니터 최대화 = 첫 모니터 작업 영역");
    assert_eq!(rect(4), Some(l.monitors[1].work), "둘째 모니터 최대화 = 둘째 모니터 작업 영역(D4: 다른 모니터 기준으로 자르지 않음)");
    // 작업 표시줄 위에서는 작업 표시줄(최대화 창이 그 밑으로 가지 않는다)
    assert_eq!(s.window_at(PhysPoint { x: 960, y: 1060 }, true).map(|w| w.handle), Some(1));
    assert_eq!(s.window_at(PhysPoint { x: 3000, y: 1420 }, true).map(|w| w.handle), Some(2));
    assert_eq!(s.window_at(PhysPoint { x: 960, y: 500 }, true).map(|w| w.handle), Some(3));
}

#[test]
fn rb15_판정_전체_화면_창은_작업_표시줄만큼_자르지_않는다() {
    let l = two_monitors();
    let m1 = l.monitors[1].bounds;
    let raw = vec![
        // 둘째 모니터 전체 화면(모니터와 같은 사각형, 최대화 표시가 있어도) — Snipaste #584
        RawWindow { maximized: true, ..win(7, "RbFull", m1) },
        // 최대화 표시 없는 테두리 없는 전체 화면 창(브라우저 F11·영상)
        win(8, "RbBorderless", l.monitors[0].bounds),
    ];
    let s = snap(&l, &raw);
    let rect = |h: u64| s.windows.iter().find(|w| w.handle == h).map(|w| w.rect);
    assert_eq!(rect(7), Some(m1));
    assert_eq!(rect(8), Some(l.monitors[0].bounds));
}

#[test]
fn rb15_판정_바탕_화면은_셸_바탕_창이_받는다() {
    let l = two_monitors();
    let raw = vec![RawWindow { maximized: true, ..win(3, "RbMaxed", PhysRect::new(-8, -8, 1936, 1056)) }, RawWindow { exe: "explorer.exe".into(), ..win(5, "Progman", l.virtual_bounds()) }];
    let s = snap(&l, &raw);
    // 둘째 모니터 빈 곳 = 바탕 화면(셸 바탕 창)
    let w = s.window_at(PhysPoint { x: 2500, y: 700 }, true).expect("바탕 화면 창");
    assert_eq!(w.class, "Progman");
    assert_eq!(w.rect, l.monitors[1].work, "바탕 화면 = 그 모니터 작업 영역");
}

/// 08 §4.2 W "바탕 화면 = 모니터 작업 영역"을 사슬까지 본다. 셸 바탕 창(Progman)은 가상 화면 전체라,
/// `filter_windows`가 모니터마다 작업 영역 후보로 나누지 않으면 여러 모니터에서 첫 강조가 가상 화면 전체가 된다.
#[test]
fn rb15_바탕_화면_첫_강조는_그_모니터_작업_영역() {
    let l = two_monitors();
    let raw = vec![RawWindow { exe: "explorer.exe".into(), ..win(5, "Progman", l.virtual_bounds()) }];
    let s = snap(&l, &raw);
    let p = PhysPoint { x: 2500, y: 700 };
    let m = l.nearest(p).expect("모니터");
    let ev = Evidence {
        point: p,
        window: s.window_at(p, true).map(|w| w.rect),
        children: &[],
        elements: &[],
        vision: &[],
        monitor: m.bounds,
        virtual_bounds: l.virtual_bounds(),
        support: None,
        snap: None,
    };
    let chain = build_chain(&ev);
    let first = first_level(&chain, None);
    assert_eq!(chain[first].rect, m.work, "첫 강조 = 둘째 모니터 작업 영역: {chain:?}");
    assert!(chain.iter().any(|n| n.kind == NodeKind::Monitor && n.rect == m.bounds), "휠 위로 모니터 전체: {chain:?}");
}

// ───────────── 실제 창(메인이 실행) ─────────────

#[cfg(windows)]
mod real {
    use super::center;
    use ink_domain::geometry::{MonitorLayout, PhysRect};
    use ink_domain::snapshot::{filter_windows, CaptureKind, Snapshot};
    use ink_platform_windows::screen::{monitor_layout, top_windows};
    use std::sync::Once;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, PeekMessageW, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos, ShowWindow, TranslateMessage, HWND_TOPMOST,
        LWA_ALPHA, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOWMAXIMIZED, SW_SHOWNOACTIVATE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
        WS_EX_TRANSPARENT, WS_OVERLAPPEDWINDOW, WS_POPUP,
    };

    const CLASS: &str = "ScreenInkRbTest\0";

    unsafe extern "system" fn proc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        DefWindowProcW(h, msg, w, l)
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    /// 물리 px로 읽도록 Per-Monitor V2(앱과 같게) + 창 종류 등록. 시험 실행 파일은 매니페스트가 없어 기본이 DPI 모름이다.
    fn setup() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            // SAFETY: 프로세스 DPI 문맥은 창을 만들기 전에 한 번. 등록 구조체는 이 호출 동안만 산다.
            unsafe {
                SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                let class = wide(CLASS);
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(proc),
                    hInstance: GetModuleHandleW(std::ptr::null()),
                    // COLOR_WINDOW + 1 — 흰 바탕(내용은 판정과 무관)
                    hbrBackground: 6 as _,
                    lpszClassName: class.as_ptr(),
                    ..std::mem::zeroed()
                };
                RegisterClassW(&wc);
            }
        });
    }

    /// 시험이 끝나면 창을 지운다(실패해도).
    struct Win(HWND);

    impl Drop for Win {
        fn drop(&mut self) {
            // SAFETY: 이 시험이 만든 창.
            unsafe { DestroyWindow(self.0) };
        }
    }

    impl Win {
        fn handle(&self) -> u64 {
            self.0 as usize as u64
        }
    }

    fn create(ex: u32, style: u32, r: PhysRect) -> Win {
        let class = wide(CLASS);
        let title = wide("\0");
        // SAFETY: 등록한 창 종류, 부모·메뉴 없음.
        let h = unsafe { CreateWindowExW(ex, class.as_ptr(), title.as_ptr(), style, r.x, r.y, r.w, r.h, std::ptr::null_mut(), std::ptr::null_mut(), GetModuleHandleW(std::ptr::null()), std::ptr::null()) };
        assert!(!h.is_null(), "CreateWindowExW 실패");
        Win(h)
    }

    /// 투명 계층 창: 투명도 `alpha`, 맨 위로, 활성화하지 않고 보인다.
    fn overlay(ex: u32, r: PhysRect, alpha: u8) -> Win {
        let w = create(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | ex, WS_POPUP, r);
        // SAFETY: 이 시험이 만든 창.
        unsafe {
            SetLayeredWindowAttributes(w.0, 0, alpha, LWA_ALPHA);
            SetWindowPos(w.0, HWND_TOPMOST, r.x, r.y, r.w, r.h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        }
        w
    }

    /// 메시지를 돌려 창이 "응답 없음"(IsHungAppWindow)으로 걸러지지 않게.
    fn pump(ms: u64) {
        let until = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < until {
            // SAFETY: 이 스레드의 메시지만 꺼내 보낸다.
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn snapshot(layout: &MonitorLayout) -> Snapshot {
        // own_pid 0: 이 시험이 만든 창도 후보로 둔다(앱은 자기 창을 뺀다)
        Snapshot { id: 1, kind: CaptureKind::Region, layout: layout.clone(), windows: filter_windows(&top_windows(), layout, 0, &[]), cursor: Default::default() }
    }

    #[test]
    #[ignore = "실제 화면 필요 — 메인이 실행"]
    fn rb16_투명덮개_3종_제외() {
        setup();
        let layout = monitor_layout();
        let m = layout.primary().expect("주 모니터").clone();
        let target_rect = PhysRect::new(m.work.x + 200, m.work.y + 150, 800, 600);
        let target = create(WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW, target_rect);
        // SAFETY: 이 시험이 만든 창.
        unsafe {
            SetWindowPos(target.0, HWND_TOPMOST, target_rect.x, target_rect.y, target_rect.w, target_rect.h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            ShowWindow(target.0, SW_SHOWNOACTIVATE);
        }
        let inner = PhysRect::new(target_rect.x + 100, target_rect.y + 100, 600, 400);
        // ① 클릭 통과(WS_EX_TRANSPARENT) 반투명 ② 투명도 0 ③ 모니터 전체 투명도 1 — 뒤에 만든 것이 앞
        let click_through = overlay(WS_EX_TRANSPARENT, inner, 60);
        let alpha_zero = overlay(0, inner, 0);
        let cover = overlay(0, m.bounds, 1);
        pump(300);

        let s = snapshot(&layout);
        let ids: Vec<u64> = s.windows.iter().map(|w| w.handle).collect();
        let p = center(inner);
        assert!(ids.contains(&target.handle()), "시험 창이 창 목록에 있어야 한다(응답 없음·가려짐 아님)");
        assert!(!ids.contains(&click_through.handle()), "클릭 통과 덮개 제외");
        assert!(!ids.contains(&alpha_zero.handle()), "투명도 0 덮개 제외");
        let c = s.windows.iter().find(|w| w.handle == cover.handle()).expect("모니터 덮개는 후보로 남음");
        assert!(c.suspect_overlay, "모니터 전체 투명도 1 = 덮개 의심");
        let picked = s.window_at(p, true).map(|w| (w.handle, w.exe.clone(), w.class.clone()));
        assert_eq!(picked.as_ref().map(|x| x.0), Some(target.handle()), "첫 강조 창 = 덮개 아래 시험 창, 실제: {picked:?}");
    }

    #[test]
    #[ignore = "실제 화면 필요 — 메인이 실행"]
    fn rb15_최대화_작업영역_전체화면_모니터() {
        setup();
        let layout = monitor_layout();
        let m = layout.primary().expect("주 모니터").clone();

        // ① 최대화 창 = 작업 영역, 작업 표시줄은 그 위에 따로
        let maxed = create(0, WS_OVERLAPPEDWINDOW, PhysRect::new(m.work.x + 100, m.work.y + 100, 640, 480));
        // SAFETY: 이 시험이 만든 창. 최대화는 활성화를 피할 수 없다(잠깐 포커스를 가져감).
        unsafe { ShowWindow(maxed.0, SW_SHOWMAXIMIZED) };
        pump(400);
        let s = snapshot(&layout);
        let got = s.windows.iter().find(|w| w.handle == maxed.handle()).map(|w| w.rect);
        assert_eq!(got, Some(m.work), "최대화 창 = 그 모니터 작업 영역");
        if m.work != m.bounds {
            // 작업 표시줄이 있는 쪽 띠의 가운데
            let band = if m.work.bottom() < m.bounds.bottom() {
                PhysRect::from_edges(m.bounds.x, m.work.bottom(), m.bounds.right(), m.bounds.bottom())
            } else if m.work.y > m.bounds.y {
                PhysRect::from_edges(m.bounds.x, m.bounds.y, m.bounds.right(), m.work.y)
            } else if m.work.x > m.bounds.x {
                PhysRect::from_edges(m.bounds.x, m.bounds.y, m.work.x, m.bounds.bottom())
            } else {
                PhysRect::from_edges(m.work.right(), m.bounds.y, m.bounds.right(), m.bounds.bottom())
            };
            let at = s.window_at(center(band), true).map(|w| w.class.clone());
            assert_eq!(at.as_deref(), Some("Shell_TrayWnd"), "작업 표시줄 위 = 작업 표시줄");
        }
        drop(maxed);

        // ② 테두리 없는 전체 화면 창 = 모니터 그대로(작업 표시줄만큼 자르지 않음)
        let full = create(0, WS_POPUP, m.bounds);
        // SAFETY: 이 시험이 만든 창.
        unsafe { ShowWindow(full.0, SW_SHOWNOACTIVATE) };
        pump(300);
        let s = snapshot(&layout);
        let got = s.windows.iter().find(|w| w.handle == full.handle()).map(|w| w.rect);
        assert_eq!(got, Some(m.bounds), "전체 화면 창 = 모니터 전체(D4)");
        drop(full);

        // ③ 바탕 화면: 셸 바탕 창(Progman·WorkerW)이 모니터마다 그 작업 영역 후보로 들어간다
        let s = snapshot(&layout);
        let desks: Vec<PhysRect> = s.windows.iter().filter(|w| w.class == "Progman" || w.class == "WorkerW").map(|w| w.rect).collect();
        assert!(desks.contains(&m.work), "주 모니터 바탕 화면 후보 = 작업 영역 {:?}: {desks:?}", m.work);
        assert!(desks.iter().all(|d| layout.monitors.iter().any(|x| x.work == *d)), "바탕 화면 후보는 모두 어느 모니터의 작업 영역: {desks:?}");
    }
}
