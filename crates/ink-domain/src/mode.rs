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
    /// 녹화 중이고 다른 모드가 없음(05 §8). 녹화는 판서·확대·캡처와 겹칠 수 있어 이 값은 "쉬는 중 + 녹화"를 뜻한다
    Recording { started_ms: u64 },
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
}

/// 단축키 칸 + 지금 모드 → 할 일. `now_ms`는 단조 시계.
pub fn on_hotkey(mode: &Mode, slot: &str, now_ms: u64) -> ModeCommand {
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
        Mode::Capture { .. } => ModeCommand::Switch(k),
        // 녹화 위에서: F3 = 녹화 끝, F1·F2 = 캡처 시작(녹화는 계속)
        Mode::Recording { .. } if k == CaptureKind::Record => ModeCommand::End,
        Mode::Recording { .. } => ModeCommand::Start(k),
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
        assert_eq!(on_hotkey(&m, "draw", 100), ModeCommand::Ignore);
    }

    #[test]
    fn f2_두_번은_활성_창() {
        let m = Mode::Capture { session: 1, kind: CaptureKind::Window, started_ms: 1_000 };
        assert_eq!(on_hotkey(&m, "window", 1_300), ModeCommand::ActiveWindow);
        assert_eq!(on_hotkey(&m, "window", 1_500), ModeCommand::End);
    }

    #[test]
    fn 녹화_중에는_f3가_끝_다른_캡처는_시작() {
        let m = Mode::Recording { started_ms: 0 };
        assert_eq!(on_hotkey(&m, "record", 100), ModeCommand::End);
        assert_eq!(on_hotkey(&m, "region", 100), ModeCommand::Start(CaptureKind::Region));
        assert_eq!(on_hotkey(&m, "window", 100), ModeCommand::Start(CaptureKind::Window));
        let json = serde_json::to_value(m).unwrap();
        assert_eq!(json, serde_json::json!({"mode": "recording", "startedMs": 0}));
    }
}
