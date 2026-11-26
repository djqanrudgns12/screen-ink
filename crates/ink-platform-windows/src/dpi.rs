//! D12: DPI를 모르는 창(Unaware·System-aware)이 돌려주는 가상화 좌표를 물리 px로(08 §4.2 M, R-CAP-17).
//!
//! 이 프로세스는 Per-Monitor V2(Tauri 매니페스트)라 OS가 주는 창·커서 좌표는 물리 px다. 그런데 MSAA는
//! 대상 앱이 **자기 DPI 문맥**에서 계산한 좌표를 그대로 넘길 수 있다(Qt 5·Blender·LibreOffice — Snipaste #796).
//! UIA는 UIA 핵심이 바꿔 주므로 대상이 아니다(08 D12 "UIA는 정상").
//!
//! 순서:
//! 1. 창의 DPI 문맥(`GetWindowDpiAwarenessContext`)·창 DPI·모니터 DPI로 배율 비를 구한다(`logical_dpi`).
//!    창 DPI = 창이 자기 좌표를 계산하는 DPI(Unaware 96 · System = 그 앱이 가정한 시스템 DPI · Per-Monitor = 모니터 DPI).
//!    System은 `GetDpiFromDpiAwarenessContext`를 먼저 보고, Per-Monitor 문맥에서 그 값이 0인 것은 96으로 치지 않는다
//!    (스파이크 보고서 §7 실측). 값을 모르면(0) 바꾸지 않는다.
//! 2. 비가 1이 아니면, MSAA 뿌리(OBJID_CLIENT) 사각형이 **물리 클라이언트 사각형**과 맞는지 **논리 클라이언트 사각형**과
//!    맞는지 보고 좌표 공간을 판정한다(`classify_space`). oleacc 대리 객체(표준 컨트롤)는 우리 프로세스에서 돌아
//!    이미 물리 px일 수 있어서, 무조건 바꾸면 오히려 틀린다.
//! 3. 논리면 `LogicalToPhysicalPointForPerMonitorDPI`(두 점)로 바꾸고, API가 실패하면 순수 산술로(`scale_unaware_rect`).
//!
//! 안전망: 바꾼 사각형이 창 경계 밖으로 2 px 넘게 나가면 융합(`recog::build_chain`)의 창 경계 자르기·검증이 거른다.

use ink_domain::geometry::{PhysPoint, PhysRect};

/// 창의 DPI 인식(`DPI_AWARENESS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DpiAwareness {
    /// `DPI_AWARENESS_UNAWARE`(0) — 96 DPI로 그리고 OS가 늘린다(좌표 가상화)
    Unaware,
    /// `DPI_AWARENESS_SYSTEM_AWARE`(1) — 시스템 DPI로 그린다(다른 배율 모니터에서 가상화)
    System,
    /// `DPI_AWARENESS_PER_MONITOR_AWARE`(2) — 이미 물리 px
    PerMonitor,
    /// 읽지 못함(`DPI_AWARENESS_INVALID` 등) — 바꾸지 않는다
    Unknown,
}

impl DpiAwareness {
    pub fn from_raw(v: i32) -> DpiAwareness {
        match v {
            0 => DpiAwareness::Unaware,
            1 => DpiAwareness::System,
            2 => DpiAwareness::PerMonitor,
            _ => DpiAwareness::Unknown,
        }
    }

    /// 좌표가 가상화될 수 있는 창(보정 대상).
    pub fn virtualized(self) -> bool {
        matches!(self, DpiAwareness::Unaware | DpiAwareness::System)
    }
}

/// 창이 자기 좌표를 계산하는 DPI. `ctx_dpi` = `GetDpiFromDpiAwarenessContext`(System 문맥에서만 > 0),
/// `window_dpi` = `GetDpiForWindow`. 0 = 모름(96으로 치지 않는다 — `scale_ratio`가 1로 둔다).
pub fn logical_dpi(awareness: DpiAwareness, ctx_dpi: u32, window_dpi: u32) -> u32 {
    match awareness {
        DpiAwareness::Unaware => 96,
        DpiAwareness::System if ctx_dpi > 0 => ctx_dpi,
        _ => window_dpi,
    }
}

/// 논리 → 물리 배율 비. 보정 대상이 아니거나 값을 모르면 1.
pub fn scale_ratio(awareness: DpiAwareness, window_dpi: u32, monitor_dpi: u32) -> f64 {
    if !awareness.virtualized() || window_dpi == 0 || monitor_dpi == 0 {
        return 1.0;
    }
    monitor_dpi as f64 / window_dpi as f64
}

/// 비가 사실상 1인지(반올림 오차 안).
pub fn is_identity(ratio: f64) -> bool {
    (ratio - 1.0).abs() < 1e-6
}

/// 순수 폴백: 논리 사각형을 `origin`(창이 있는 모니터의 물리 왼쪽 위) 기준으로 `ratio`배.
/// 네 변을 따로(원점에서 떨어진 거리를) 반올림해서 이웃한 사각형끼리 틈·겹침이 생기지 않고,
/// 음수 좌표 모니터에서도 반올림 방향이 같다.
/// [추정] 가상화된 좌표계에서 모니터 원점은 물리와 같고 크기만 줄어든다 — 1순위는 OS API(`to_physical_rect`).
pub fn scale_unaware_rect(rect: PhysRect, origin: PhysPoint, ratio: f64) -> PhysRect {
    let f = |v: i32, o: i32| o + ((v - o) as f64 * ratio).round() as i32;
    PhysRect::from_edges(f(rect.x, origin.x), f(rect.y, origin.y), f(rect.right(), origin.x), f(rect.bottom(), origin.y))
}

/// `scale_unaware_rect`의 반대(물리 → 논리). 좌표 공간 판정에 쓴다.
pub fn unscale_rect(rect: PhysRect, origin: PhysPoint, ratio: f64) -> PhysRect {
    if ratio <= 0.0 {
        return rect;
    }
    scale_unaware_rect(rect, origin, 1.0 / ratio)
}

/// MSAA가 준 좌표가 어느 공간인지.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordSpace {
    /// 이미 물리 px — 그대로 쓴다
    Physical,
    /// 창의 논리(가상화) 좌표 — 물리로 바꾼다
    Logical,
}

/// 변마다의 오차 중 가장 큰 것.
fn edge_error(a: PhysRect, b: PhysRect) -> i32 {
    [(a.x - b.x).abs(), (a.y - b.y).abs(), (a.right() - b.right()).abs(), (a.bottom() - b.bottom()).abs()].into_iter().max().unwrap_or(0)
}

/// 뿌리(OBJID_CLIENT) 사각형을 물리·논리 클라이언트 사각형과 견줘 좌표 공간을 고른다.
/// 비기거나 뿌리가 비었으면 물리(아무것도 바꾸지 않는 쪽이 안전 — 틀려도 V 경계 맞춤·창 자르기가 받친다).
pub fn classify_space(root: PhysRect, client_phys: PhysRect, client_logical: PhysRect) -> CoordSpace {
    if root.is_empty() || client_phys == client_logical {
        return CoordSpace::Physical;
    }
    if edge_error(root, client_logical) < edge_error(root, client_phys) {
        CoordSpace::Logical
    } else {
        CoordSpace::Physical
    }
}

/// 창 하나의 DPI 사정(창마다 한 번 읽어 두고 그 창의 요소 전부에 쓴다).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowDpi {
    pub awareness: DpiAwareness,
    pub window_dpi: u32,
    pub monitor_dpi: u32,
    /// 창이 있는 모니터의 물리 왼쪽 위(순수 폴백의 원점)
    pub monitor_origin: PhysPoint,
    /// 자식 창의 DPI 인식이 다를 수 있음(`DPI_HOSTING_BEHAVIOR_MIXED`) — 뿌리 하나로 판정하면 틀릴 수 있다 [확인 필요]
    pub mixed_hosting: bool,
}

impl WindowDpi {
    /// 보정하지 않는 값(Per-Monitor 창·읽기 실패).
    pub const fn identity() -> WindowDpi {
        WindowDpi { awareness: DpiAwareness::PerMonitor, window_dpi: 96, monitor_dpi: 96, monitor_origin: PhysPoint { x: 0, y: 0 }, mixed_hosting: false }
    }

    pub fn ratio(&self) -> f64 {
        scale_ratio(self.awareness, self.window_dpi, self.monitor_dpi)
    }

    /// 이 창의 좌표가 가상화될 수 있는지(비가 1이 아님).
    pub fn virtualized(&self) -> bool {
        !is_identity(self.ratio())
    }
}

#[cfg(windows)]
mod win {
    use super::{logical_dpi, scale_unaware_rect, unscale_rect, DpiAwareness, WindowDpi};
    use ink_domain::geometry::{PhysPoint, PhysRect};
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows_sys::Win32::UI::HiDpi::{
        GetAwarenessFromDpiAwarenessContext, GetDpiForMonitor, GetDpiForWindow, GetDpiFromDpiAwarenessContext, GetWindowDpiAwarenessContext, GetWindowDpiHostingBehavior,
        LogicalToPhysicalPointForPerMonitorDPI, PhysicalToLogicalPointForPerMonitorDPI, DPI_HOSTING_BEHAVIOR_MIXED, MDT_EFFECTIVE_DPI,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

    impl WindowDpi {
        /// 다른 프로세스 창도 읽을 수 있는 질의만 쓴다(메시지를 보내지 않음 — 멈춘 창에서도 막히지 않는다).
        pub fn of(hwnd: u64) -> WindowDpi {
            let h = hwnd as HWND;
            // SAFETY: 모두 창 핸들 읽기 전용 질의. 창이 사라졌으면 0·NULL이 돌아올 뿐이다.
            unsafe {
                let ctx = GetWindowDpiAwarenessContext(h);
                if ctx.is_null() {
                    return WindowDpi::identity();
                }
                let awareness = DpiAwareness::from_raw(GetAwarenessFromDpiAwarenessContext(ctx));
                let window_dpi = logical_dpi(awareness, GetDpiFromDpiAwarenessContext(ctx), GetDpiForWindow(h));
                let mon = MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST);
                let (mut dx, mut dy) = (96u32, 96u32);
                let monitor_dpi = if !mon.is_null() && GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) == 0 { dx } else { window_dpi };
                let mut info: MONITORINFO = std::mem::zeroed();
                info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                let monitor_origin = if !mon.is_null() && GetMonitorInfoW(mon, &mut info) != 0 { PhysPoint { x: info.rcMonitor.left, y: info.rcMonitor.top } } else { PhysPoint::default() };
                WindowDpi { awareness, window_dpi, monitor_dpi, monitor_origin, mixed_hosting: GetWindowDpiHostingBehavior(h) == DPI_HOSTING_BEHAVIOR_MIXED }
            }
        }
    }

    fn convert(hwnd: u64, rect: PhysRect, f: unsafe extern "system" fn(HWND, *mut POINT) -> i32) -> Option<PhysRect> {
        let mut a = POINT { x: rect.x, y: rect.y };
        let mut b = POINT { x: rect.right(), y: rect.bottom() };
        // SAFETY: 점 두 개를 제자리에서 바꾼다. 점이 창 밖이면 FALSE(→ 순수 폴백).
        let ok = unsafe { f(hwnd as HWND, &mut a) != 0 && f(hwnd as HWND, &mut b) != 0 };
        ok.then(|| PhysRect::from_edges(a.x, a.y, b.x, b.y))
    }

    /// 논리(가상화) 사각형 → 물리. 1순위 OS API, 실패하면 순수 산술. 보정 대상이 아니면 그대로.
    pub fn to_physical_rect(hwnd: u64, rect: PhysRect, dpi: &WindowDpi) -> PhysRect {
        if !dpi.virtualized() {
            return rect;
        }
        convert(hwnd, rect, LogicalToPhysicalPointForPerMonitorDPI).unwrap_or_else(|| scale_unaware_rect(rect, dpi.monitor_origin, dpi.ratio()))
    }

    /// 물리 사각형 → 그 창의 논리 좌표(좌표 공간 판정용).
    pub fn to_logical_rect(hwnd: u64, rect: PhysRect, dpi: &WindowDpi) -> PhysRect {
        if !dpi.virtualized() {
            return rect;
        }
        convert(hwnd, rect, PhysicalToLogicalPointForPerMonitorDPI).unwrap_or_else(|| unscale_rect(rect, dpi.monitor_origin, dpi.ratio()))
    }

    /// 클라이언트 영역(화면 좌표, 물리 px — 우리 프로세스가 Per-Monitor V2일 때).
    pub fn client_rect_physical(hwnd: u64) -> Option<PhysRect> {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: 출력 구조체 하나씩, 읽기 전용 질의.
        let ok = unsafe { GetClientRect(hwnd as HWND, &mut r) != 0 && ClientToScreen(hwnd as HWND, &mut p) != 0 };
        ok.then(|| PhysRect::new(p.x, p.y, r.right - r.left, r.bottom - r.top))
    }
}

#[cfg(windows)]
pub use win::{client_rect_physical, to_logical_rect, to_physical_rect};

#[cfg(test)]
mod tests {
    use super::*;

    const O: PhysPoint = PhysPoint { x: 0, y: 0 };

    #[test]
    fn dpi_unaware_150퍼센트_좌표_물리변환() {
        // C §4 합격 기준: 논리 (100,100,200,100) → 물리 (150,150,300,150) 오차 0 px
        let r = scale_ratio(DpiAwareness::Unaware, 96, 144);
        assert_eq!(r, 1.5);
        assert_eq!(scale_unaware_rect(PhysRect::new(100, 100, 200, 100), O, r), PhysRect::new(150, 150, 300, 150));
    }

    #[test]
    fn dpi_인식별_배율비() {
        // Per-Monitor 창은 이미 물리 px
        assert_eq!(scale_ratio(DpiAwareness::PerMonitor, 144, 144), 1.0);
        assert_eq!(scale_ratio(DpiAwareness::PerMonitor, 96, 144), 1.0, "Per-Monitor는 창 DPI와 상관없이 1");
        // System-aware: 시스템 100%(96) 앱을 150% 모니터에 → 1.5, 시스템 150% 앱을 100% 모니터에 → 2/3
        assert_eq!(scale_ratio(DpiAwareness::System, 96, 144), 1.5);
        assert!((scale_ratio(DpiAwareness::System, 144, 96) - 2.0 / 3.0).abs() < 1e-9);
        // System-aware 창이 시스템 DPI와 같은 모니터에 있으면 가상화 없음
        assert!(is_identity(scale_ratio(DpiAwareness::System, 120, 120)));
        // 모르는 값은 바꾸지 않는다
        assert_eq!(scale_ratio(DpiAwareness::Unknown, 96, 192), 1.0);
        assert_eq!(scale_ratio(DpiAwareness::Unaware, 0, 144), 1.0);
        assert_eq!(DpiAwareness::from_raw(-1), DpiAwareness::Unknown);
        assert_eq!(DpiAwareness::from_raw(0), DpiAwareness::Unaware);
        assert!(WindowDpi { awareness: DpiAwareness::Unaware, window_dpi: 96, monitor_dpi: 144, monitor_origin: O, mixed_hosting: false }.virtualized());
        assert!(!WindowDpi::identity().virtualized());
    }

    #[test]
    fn dpi_pm_문맥의_0은_96으로_치지_않는다() {
        // 스파이크 §7: GetDpiFromDpiAwarenessContext는 Per-Monitor 문맥에서 0
        assert_eq!(logical_dpi(DpiAwareness::PerMonitor, 0, 144), 144);
        assert_eq!(scale_ratio(DpiAwareness::PerMonitor, logical_dpi(DpiAwareness::PerMonitor, 0, 144), 144), 1.0);
        // System: 앱이 가정한 시스템 DPI(문맥 값)를 먼저, 없으면 GetDpiForWindow
        assert_eq!(logical_dpi(DpiAwareness::System, 120, 96), 120);
        assert_eq!(logical_dpi(DpiAwareness::System, 0, 120), 120);
        // 둘 다 모르면 0 → 바꾸지 않는다(96으로 쳐서 1.5배로 늘리지 않음)
        assert_eq!(scale_ratio(DpiAwareness::System, logical_dpi(DpiAwareness::System, 0, 0), 144), 1.0);
        // Unaware는 언제나 96
        assert_eq!(logical_dpi(DpiAwareness::Unaware, 0, 0), 96);
    }

    #[test]
    fn dpi_둘째_모니터_원점_기준으로_늘린다() {
        // 둘째 모니터(물리 x 1920부터, 200%)의 Unaware 창: 원점은 그대로, 거리만 2배
        let origin = PhysPoint { x: 1920, y: 0 };
        let logical = PhysRect::new(1920 + 50, 40, 100, 30);
        assert_eq!(scale_unaware_rect(logical, origin, 2.0), PhysRect::new(1920 + 100, 80, 200, 60));
        // 음수 좌표 모니터(주 모니터 왼쪽)
        let left = PhysPoint { x: -2560, y: 0 };
        assert_eq!(scale_unaware_rect(PhysRect::new(-2560 + 10, 10, 20, 20), left, 1.25), PhysRect::new(-2560 + 13, 13, 25, 25));
    }

    #[test]
    fn dpi_네_변을_따로_반올림해_이웃_칸이_맞붙는다() {
        // 125%: 너비 3짜리 칸 셋이 이어져 있으면 물리에서도 틈·겹침 없이 이어진다
        let cells: Vec<PhysRect> = (0..3).map(|i| scale_unaware_rect(PhysRect::new(i * 3, 0, 3, 3), O, 1.25)).collect();
        assert_eq!(cells[0].right(), cells[1].x);
        assert_eq!(cells[1].right(), cells[2].x);
        assert_eq!(cells[2].right(), 11, "9 × 1.25 = 11.25 → 11");
    }

    #[test]
    fn dpi_왕복하면_제자리() {
        for ratio in [1.25, 1.5, 1.75, 2.0] {
            let origin = PhysPoint { x: 1920, y: -200 };
            let logical = PhysRect::new(2000, 100, 400, 300);
            let phys = scale_unaware_rect(logical, origin, ratio);
            assert_eq!(unscale_rect(phys, origin, ratio), logical, "ratio {ratio}");
        }
    }

    #[test]
    fn dpi_뿌리_사각형으로_좌표_공간_판정() {
        let phys = PhysRect::new(150, 150, 1200, 900);
        let logical = unscale_rect(phys, O, 1.5); // (100,100,800,600)
        // 앱이 자기(논리) 좌표로 답함 → 바꿔야 함
        assert_eq!(classify_space(PhysRect::new(100, 100, 800, 600), phys, logical), CoordSpace::Logical);
        // oleacc 대리 객체가 우리 프로세스에서 물리로 답함 → 그대로
        assert_eq!(classify_space(PhysRect::new(150, 150, 1200, 900), phys, logical), CoordSpace::Physical);
        // 1 px 흔들림은 가까운 쪽
        assert_eq!(classify_space(PhysRect::new(101, 100, 799, 601), phys, logical), CoordSpace::Logical);
        // 뿌리가 비었거나 두 공간이 같으면(배율 100%) 바꾸지 않는다
        assert_eq!(classify_space(PhysRect::default(), phys, logical), CoordSpace::Physical);
        assert_eq!(classify_space(PhysRect::new(1, 1, 2, 2), phys, phys), CoordSpace::Physical);
    }
}
