//! 모드 상태 기계(03 §3): 한 번에 한 모드. 모드 중 같은 단축키 = 끝, 다른 모드 키 = 지금 모드를 끝내고 전환.
//! F2를 400 ms 안에 두 번 = 활성 창 캡처(02 §6.3).

use crate::snapshot::CaptureKind;
use serde::{Deserialize, Serialize};

pub const DOUBLE_PRESS_MS: u64 = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Mode {
    Idle,
    Capture { session: u64, kind: CaptureKind, started_ms: u64 },
    /// 판서(Alt+1, 정지 화면 위)·칠판(Alt+3, `board`) — 커서 모니터 하나(A §2.5)
    Draw { session: u64, board: bool, monitor_id: u32, started_ms: u64 },
    /// 실시간 확대(Alt+2, 단계 5) — 커서 모니터 하나. 판정은 `zoom::on_hotkey`가 먼저 한다.
    LiveZoom { session: u64, monitor_id: u32, started_ms: u64 },
}

/// 판서 전역 칸: `draw` = Alt+1 · `board` = Alt+3 · `restore` = Alt+Shift+1(직전 판서 바로 열기, R-DRAW-2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawKind {
    Draw,
    Board,
    Restore,
}

impl DrawKind {
    pub fn from_slot(slot: &str) -> Option<Self> {
        match slot {
            "draw" => Some(DrawKind::Draw),
            "board" => Some(DrawKind::Board),
            "restore" => Some(DrawKind::Restore),
            _ => None,
        }
    }

    pub fn slot(self) -> &'static str {
        match self {
            DrawKind::Draw => "draw",
            DrawKind::Board => "board",
            DrawKind::Restore => "restore",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeCommand {
    Start(CaptureKind),
    /// 같은 키 한 번 더 = 끝
    End,
    /// 지금 모드를 끝내고 다른 캡처로
    Switch(CaptureKind),
    /// F2 두 번 = 커서 아래가 아니라 활성 창
    ActiveWindow,
    /// 캡처 모드와 무관한 키
    Ignore,
    /// 쉬다가 판서·칠판·직전 판서 열기
    StartDraw(DrawKind),
    /// 지금 모드(캡처·다른 판서)를 끝내고 판서·칠판으로
    SwitchDraw(DrawKind),
    /// 판서 중 직전 판서 키 = 그 자리에서 복원(화면 안 Ctrl+Shift+R과 같음)
    DrawRestore,
}

/// 단축키 칸 + 지금 모드 → 할 일. `now_ms`는 단조 시계.
pub fn on_hotkey(mode: &Mode, slot: &str, now_ms: u64) -> ModeCommand {
    if let Some(d) = DrawKind::from_slot(slot) {
        return match *mode {
            Mode::Idle => ModeCommand::StartDraw(d),
            Mode::Draw { board: false, .. } if d == DrawKind::Restore => ModeCommand::DrawRestore,
            Mode::Draw { board, .. } if d != DrawKind::Restore && (d == DrawKind::Board) == board => ModeCommand::End,
            _ => ModeCommand::SwitchDraw(d),
        };
    }
    let Some(k) = CaptureKind::from_slot(slot) else { return ModeCommand::Ignore };
    match *mode {
        Mode::Idle => ModeCommand::Start(k),
        Mode::Capture { kind, started_ms, .. } if kind == k => {
            if k == CaptureKind::Window && now_ms.saturating_sub(started_ms) <= DOUBLE_PRESS_MS {
                ModeCommand::ActiveWindow
            } else {
                ModeCommand::End
            }
        }
        Mode::Capture { .. } | Mode::Draw { .. } | Mode::LiveZoom { .. } => ModeCommand::Switch(k),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 쉬다가_누르면_시작_같은_키는_끝_다른_키는_전환() {
        assert_eq!(on_hotkey(&Mode::Idle, "region", 0), ModeCommand::Start(CaptureKind::Region));
        let m = Mode::Capture { session: 1, kind: CaptureKind::Region, started_ms: 0 };
        assert_eq!(on_hotkey(&m, "region", 100), ModeCommand::End);
        assert_eq!(on_hotkey(&m, "window", 100), ModeCommand::Switch(CaptureKind::Window));
        // 캡처 중 판서 키 = 캡처를 끝내고 판서로(03 §3)
        assert_eq!(on_hotkey(&m, "draw", 100), ModeCommand::SwitchDraw(DrawKind::Draw));
        assert_eq!(on_hotkey(&m, "zoom", 100), ModeCommand::Ignore);
    }

    #[test]
    fn 확대_중_캡처_키는_전환이고_모양은_live_zoom() {
        let m = Mode::LiveZoom { session: 3, monitor_id: 1, started_ms: 0 };
        assert_eq!(on_hotkey(&m, "region", 100), ModeCommand::Switch(CaptureKind::Region));
        let v = serde_json::to_value(m).unwrap();
        assert_eq!(v, serde_json::json!({ "mode": "liveZoom", "session": 3, "monitorId": 1, "startedMs": 0 }));
    }

    #[test]
    fn f2_두_번은_활성_창() {
        let m = Mode::Capture { session: 1, kind: CaptureKind::Window, started_ms: 1_000 };
        assert_eq!(on_hotkey(&m, "window", 1_300), ModeCommand::ActiveWindow);
        assert_eq!(on_hotkey(&m, "window", 1_500), ModeCommand::End);
    }

    #[test]
    fn 판서_칸은_시작_같은_키는_끝_다른_키는_전환() {
        assert_eq!(on_hotkey(&Mode::Idle, "draw", 0), ModeCommand::StartDraw(DrawKind::Draw));
        assert_eq!(on_hotkey(&Mode::Idle, "board", 0), ModeCommand::StartDraw(DrawKind::Board));
        assert_eq!(on_hotkey(&Mode::Idle, "restore", 0), ModeCommand::StartDraw(DrawKind::Restore));

        let draw = Mode::Draw { session: 2, board: false, monitor_id: 0, started_ms: 0 };
        assert_eq!(on_hotkey(&draw, "draw", 50), ModeCommand::End, "Alt+1 중 Alt+1 = 끝(보관)");
        assert_eq!(on_hotkey(&draw, "board", 50), ModeCommand::SwitchDraw(DrawKind::Board));
        assert_eq!(on_hotkey(&draw, "restore", 50), ModeCommand::DrawRestore, "판서 중 Alt+Shift+1 = 그 자리 복원");
        assert_eq!(on_hotkey(&draw, "region", 50), ModeCommand::Switch(CaptureKind::Region));
        // 판서 중 F2는 두 번 누르기가 아니라 전환
        assert_eq!(on_hotkey(&draw, "window", 50), ModeCommand::Switch(CaptureKind::Window));

        let board = Mode::Draw { session: 3, board: true, monitor_id: 1, started_ms: 0 };
        assert_eq!(on_hotkey(&board, "board", 50), ModeCommand::End);
        assert_eq!(on_hotkey(&board, "draw", 50), ModeCommand::SwitchDraw(DrawKind::Draw));
        assert_eq!(on_hotkey(&board, "restore", 50), ModeCommand::SwitchDraw(DrawKind::Restore));
        assert_eq!(DrawKind::from_slot("restore").map(DrawKind::slot), Some("restore"));
        assert_eq!(DrawKind::from_slot("zoom"), None);
    }

    #[test]
    fn 판서_모드는_ui로_camel_case() {
        let m = Mode::Draw { session: 4, board: true, monitor_id: 1, started_ms: 9 };
        assert_eq!(serde_json::to_value(m).unwrap(), serde_json::json!({ "mode": "draw", "session": 4, "board": true, "monitorId": 1, "startedMs": 9 }));
    }
}
