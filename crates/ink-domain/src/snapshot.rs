//! 정지 스냅숏(02 §0-1, 08 §4.1-2): 단축키 순간의 모니터 배치 + 창 목록 + (어댑터가 쥔) 모니터별 이미지.
//! 모드가 끝날 때까지 이 값만 본다. 창 제목은 담지 않는다(05 개인정보).
//!
//! 창 목록 거르기는 OS의 클릭 판정과 같은 규칙으로(08 D1·D2): 보이지 않음·최소화·크기 0·cloaked·
//! 응답 없음·자기 창·감지 제외 앱·`WS_EX_TRANSPARENT`·투명도 0 계층 창을 빼고, 최대화 창은 작업 영역으로 자른다.

use crate::geometry::{MonitorLayout, PhysPoint, PhysRect};
use crate::settings::ExcludedApp;
use serde::{Deserialize, Serialize};

/// 어댑터가 읽은 그대로의 최상위 창(Z 순서: 앞 → 뒤).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawWindow {
    pub handle: u64,
    pub pid: u32,
    /// 실행 파일 이름(소문자, 경로 X)
    pub exe: String,
    /// 창 클래스(앱 가족 판정용, 08 §4.6)
    pub class: String,
    /// DWM 확장 프레임 경계(그림자 뺀 실제 경계), 물리 px
    pub rect: PhysRect,
    pub visible: bool,
    pub minimized: bool,
    pub maximized: bool,
    /// 다른 가상 데스크톱·숨은 UWP(`DWMWA_CLOAKED`)
    pub cloaked: bool,
    /// `IsHungAppWindow`
    pub hung: bool,
    /// `WS_EX_TRANSPARENT` — 클릭이 통과하는 덮개
    pub click_through: bool,
    /// 계층 창 투명도(0~255). 계층 창이 아니면 None
    pub layered_alpha: Option<u8>,
    pub topmost: bool,
    /// `WS_EX_TOOLWINDOW`
    pub tool: bool,
}

/// 걸러진 창 — 후보로 쓸 수 있는 것만.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub handle: u64,
    pub pid: u32,
    pub exe: String,
    pub class: String,
    pub rect: PhysRect,
    /// 0 = 맨 앞
    pub z: u32,
    pub topmost: bool,
    /// 감지 제외 목록의 "창 단위만" — 요소 질의를 하지 않는다(R-CAP-15)
    pub window_only: bool,
    /// 모니터를 거의 덮는 맨 위 계층·도구 창 — 첫 강조에서 뺀다(08 D1)
    pub suspect_overlay: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureKind {
    /// F1 범위 캡처
    Region,
    /// F2 창 캡처
    Window,
    /// F3 녹화 범위 고르기
    Record,
}

impl CaptureKind {
    pub fn from_slot(slot: &str) -> Option<Self> {
        match slot {
            "region" => Some(CaptureKind::Region),
            "window" => Some(CaptureKind::Window),
            "record" => Some(CaptureKind::Record),
            _ => None,
        }
    }

    pub fn slot(self) -> &'static str {
        match self {
            CaptureKind::Region => "region",
            CaptureKind::Window => "window",
            CaptureKind::Record => "record",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: u64,
    pub kind: CaptureKind,
    pub layout: MonitorLayout,
    pub windows: Vec<WindowInfo>,
    /// 단축키 순간 커서(물리 px)
    pub cursor: PhysPoint,
}

impl Snapshot {
    /// 그 점을 담는 맨 앞 창. `skip_overlays`면 덮개 의심 창을 건너뛴다(첫 강조용).
    pub fn window_at(&self, p: PhysPoint, skip_overlays: bool) -> Option<&WindowInfo> {
        self.windows.iter().find(|w| w.rect.contains(p) && !(skip_overlays && w.suspect_overlay))
    }

    /// 커서가 있는 모니터 전체(작업 표시줄 포함). 틈에 있으면 가장 가까운 모니터.
    pub fn cursor_monitor(&self) -> Option<PhysRect> {
        self.layout.monitor_at(self.cursor).or_else(|| self.layout.nearest(self.cursor)).map(|m| m.bounds)
    }

    /// 진입 순간의 기본 강조(2026-11-25 사용자 확정):
    /// - F1 범위 캡처·F3 녹화 = 커서가 있는 **모니터 전체**(작업 표시줄 포함) → F1 다음 Enter = 그 화면 전체.
    ///   커서를 움직이면 요소·창 인식(단계 3)이 이어받는다.
    /// - F2 창 캡처 = 커서 아래 창(없으면 모니터 전체).
    pub fn default_highlight(&self) -> Option<PhysRect> {
        match self.kind {
            CaptureKind::Region | CaptureKind::Record => self.cursor_monitor(),
            CaptureKind::Window => self.window_at(self.cursor, true).map(|w| w.rect).or_else(|| self.cursor_monitor()),
        }
    }
}

/// 덮개 의심: 모니터 넓이의 이 비율 이상을 덮는 맨 위 계층·도구 창.
const OVERLAY_COVER: f64 = 0.9;

/// 셸 바탕 창(바탕 화면 아이콘·배경). 가상 화면 전체를 덮는 창 하나라 그대로 두면 여러 모니터에서
/// 바탕 화면 강조가 가상 화면 전체가 된다 → 모니터마다 작업 영역 후보로 나눈다(08 §4.2 W "바탕 화면 = 모니터 작업 영역").
pub fn is_shell_desktop(class: &str) -> bool {
    class == "Progman" || class == "WorkerW"
}

/// 창 목록 거르기(02 §1.1 `WindowCatalog::filter` + 08 D1·D4).
pub fn filter_windows(raw: &[RawWindow], layout: &MonitorLayout, own_pid: u32, excluded: &[ExcludedApp]) -> Vec<WindowInfo> {
    let mut out = Vec::new();
    for w in raw {
        if !w.visible || w.minimized || w.cloaked || w.hung || w.rect.is_empty() || w.pid == own_pid {
            continue;
        }
        if w.click_through || w.layered_alpha == Some(0) {
            continue;
        }
        let rule = excluded.iter().find(|e| e.exe.eq_ignore_ascii_case(&w.exe));
        if rule.is_some_and(|e| e.mode == "exclude") {
            continue;
        }
        if is_shell_desktop(&w.class) {
            // 바탕 화면: 이 창이 걸친 모니터마다 그 모니터의 작업 영역 하나씩(같은 핸들)
            for m in layout.monitors.iter().filter(|m| w.rect.intersect(&m.bounds).is_some() && !m.work.is_empty()) {
                out.push(WindowInfo {
                    handle: w.handle,
                    pid: w.pid,
                    exe: w.exe.clone(),
                    class: w.class.clone(),
                    rect: m.work,
                    z: out.len() as u32,
                    topmost: w.topmost,
                    window_only: rule.is_some_and(|e| e.mode == "windowOnly"),
                    suspect_overlay: false,
                });
            }
            continue;
        }
        let mut rect = match layout.clip(&w.rect) {
            Some(r) => r,
            None => continue, // 화면 밖
        };
        let monitor = layout.nearest(PhysPoint { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 });
        // 최대화 창은 화면 밖 8px까지 나간다 → 그 모니터 작업 영역으로(전체 화면 창은 자르지 않음, D4)
        if w.maximized {
            if let Some(m) = monitor {
                if rect != m.bounds {
                    rect = rect.intersect(&m.work).unwrap_or(rect);
                }
            }
        }
        if rect.w < 4 || rect.h < 4 {
            continue;
        }
        let suspect_overlay = (w.topmost && (w.layered_alpha.is_some() || w.tool))
            && monitor.is_some_and(|m| rect.intersect(&m.bounds).map_or(0, |x| x.area()) as f64 >= m.bounds.area() as f64 * OVERLAY_COVER);
        out.push(WindowInfo {
            handle: w.handle,
            pid: w.pid,
            exe: w.exe.clone(),
            class: w.class.clone(),
            rect,
            z: out.len() as u32,
            topmost: w.topmost,
            window_only: rule.is_some_and(|e| e.mode == "windowOnly"),
            suspect_overlay,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Monitor;

    fn layout() -> MonitorLayout {
        MonitorLayout::new(vec![
            Monitor { id: 0, bounds: PhysRect::new(0, 0, 1920, 1080), work: PhysRect::new(0, 0, 1920, 1040), scale: 1.0, primary: true },
            Monitor { id: 1, bounds: PhysRect::new(1920, 0, 2880, 1800), work: PhysRect::new(1920, 0, 2880, 1728), scale: 1.5, primary: false },
        ])
    }

    fn win(handle: u64, rect: PhysRect) -> RawWindow {
        RawWindow { handle, pid: 100 + handle as u32, exe: "app.exe".into(), class: "C".into(), rect, visible: true, ..Default::default() }
    }

    #[test]
    fn 고를_수_없는_창은_뺀다() {
        let r = PhysRect::new(100, 100, 400, 300);
        let raw = vec![
            RawWindow { visible: false, ..win(1, r) },
            RawWindow { minimized: true, ..win(2, r) },
            RawWindow { cloaked: true, ..win(3, r) },
            RawWindow { hung: true, ..win(4, r) },
            RawWindow { click_through: true, ..win(5, r) },
            RawWindow { layered_alpha: Some(0), ..win(6, r) },
            RawWindow { pid: 42, ..win(7, r) },
            win(8, PhysRect::new(100, 100, 0, 300)),
            win(9, PhysRect::new(-5000, -5000, 100, 100)),
            RawWindow { layered_alpha: Some(255), ..win(10, r) },
        ];
        let w = filter_windows(&raw, &layout(), 42, &[]);
        assert_eq!(w.iter().map(|x| x.handle).collect::<Vec<_>>(), vec![10]);
        assert_eq!(w[0].z, 0);
    }

    #[test]
    fn 최대화_창은_작업_영역으로_전체_화면은_그대로() {
        let maxed = RawWindow { maximized: true, ..win(1, PhysRect::new(-8, -8, 1936, 1056)) };
        let full = RawWindow { maximized: true, ..win(2, PhysRect::new(1920, 0, 2880, 1800)) };
        let w = filter_windows(&[maxed, full], &layout(), 0, &[]);
        assert_eq!(w[0].rect, PhysRect::new(0, 0, 1920, 1040));
        assert_eq!(w[1].rect, PhysRect::new(1920, 0, 2880, 1800), "전체 화면 창은 작업 표시줄만큼 자르지 않는다(D4)");
    }

    #[test]
    fn 감지_제외와_창_단위만() {
        let r = PhysRect::new(0, 0, 500, 500);
        let raw = vec![RawWindow { exe: "LiceCap.exe".into(), ..win(1, r) }, RawWindow { exe: "hwp.exe".into(), ..win(2, r) }];
        let ex = vec![
            ExcludedApp { exe: "licecap.exe".into(), mode: "exclude".into(), auto: false },
            ExcludedApp { exe: "hwp.exe".into(), mode: "windowOnly".into(), auto: true },
        ];
        let w = filter_windows(&raw, &layout(), 0, &ex);
        assert_eq!(w.len(), 1);
        assert!(w[0].window_only);
    }

    #[test]
    fn f1_기본_강조는_작업_표시줄까지_모니터_전체_f2는_창() {
        // 최대화 창(작업 영역으로 잘림) 위에서 눌러도 F1은 모니터 전체
        let maxed = RawWindow { maximized: true, ..win(1, PhysRect::new(-8, -8, 1936, 1056)) };
        let windows = filter_windows(&[maxed], &layout(), 0, &[]);
        let mut s = Snapshot { id: 1, kind: CaptureKind::Region, layout: layout(), windows, cursor: PhysPoint { x: 600, y: 500 } };
        assert_eq!(s.default_highlight(), Some(PhysRect::new(0, 0, 1920, 1080)), "작업 표시줄 포함");
        s.kind = CaptureKind::Record;
        assert_eq!(s.default_highlight(), Some(PhysRect::new(0, 0, 1920, 1080)));
        s.kind = CaptureKind::Window;
        assert_eq!(s.default_highlight(), Some(PhysRect::new(0, 0, 1920, 1040)), "F2는 창(작업 영역)");
        // 둘째 모니터(150%)에서 누르면 그 모니터
        s.kind = CaptureKind::Region;
        s.cursor = PhysPoint { x: 2500, y: 900 };
        assert_eq!(s.default_highlight(), Some(PhysRect::new(1920, 0, 2880, 1800)));
    }

    #[test]
    fn 바탕_화면_창은_모니터마다_그_작업_영역() {
        let l = layout();
        let desk = RawWindow { exe: "explorer.exe".into(), ..win(9, PhysRect::new(0, 0, 4800, 1800)) };
        let desk = RawWindow { class: "Progman".into(), ..desk };
        let w = filter_windows(&[desk], &l, 0, &[]);
        assert_eq!(w.iter().map(|x| x.rect).collect::<Vec<_>>(), vec![l.monitors[0].work, l.monitors[1].work]);
        assert!(w.iter().all(|x| x.handle == 9 && !x.suspect_overlay));
        let s = Snapshot { id: 1, kind: CaptureKind::Region, layout: l.clone(), windows: w, cursor: PhysPoint { x: 2500, y: 900 } };
        assert_eq!(s.window_at(PhysPoint { x: 2500, y: 900 }, true).map(|x| x.rect), Some(l.monitors[1].work));
        assert_eq!(s.window_at(PhysPoint { x: 500, y: 1060 }, true), None, "작업 표시줄 띠는 바탕 화면 후보 밖(작업 표시줄 창이 받음)");
        // 한 모니터에만 걸친 WorkerW는 그 모니터만
        let worker = RawWindow { class: "WorkerW".into(), ..win(10, PhysRect::new(0, 0, 1920, 1080)) };
        assert_eq!(filter_windows(&[worker], &l, 0, &[]).iter().map(|x| x.rect).collect::<Vec<_>>(), vec![l.monitors[0].work]);
        assert!(is_shell_desktop("Progman") && is_shell_desktop("WorkerW") && !is_shell_desktop("CabinetWClass"));
    }

    #[test]
    fn 모니터를_덮는_맨_위_투명_창은_덮개_의심() {
        let cover = RawWindow { topmost: true, layered_alpha: Some(1), ..win(1, PhysRect::new(0, 0, 1920, 1080)) };
        let app = win(2, PhysRect::new(200, 200, 800, 600));
        let s = Snapshot {
            id: 1,
            kind: CaptureKind::Region,
            layout: layout(),
            windows: filter_windows(&[cover, app], &layout(), 0, &[]),
            cursor: PhysPoint { x: 300, y: 300 },
        };
        assert!(s.windows[0].suspect_overlay);
        assert_eq!(s.window_at(s.cursor, true).map(|w| w.handle), Some(2), "첫 강조는 덮개 아래 실제 창");
        assert_eq!(s.window_at(s.cursor, false).map(|w| w.handle), Some(1));
    }
}
