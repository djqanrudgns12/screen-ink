//! F2 창 캡처 판정(R-WIN-1·3, spikes-windows §1 추천 구현 5).
//!
//! 사슬: WGC 첫 프레임(150 ms 제한) → (안 오거나 RGB 전부 0) 창 표시 선호도가 걸려 있으면 보호 화면 →
//! 아니면 정지 스냅숏에서 창 사각형 자르기 → 그것도 0이면 보호 화면 안내. 빈 이미지는 클립보드에 넣지 않는다.

use crate::image::Blankness;

/// 첫 프레임을 기다리는 시간(spikes §1: 대부분 7~19 ms, 이상치 0.3~1.5 s → 그때는 대체 경로).
pub const FIRST_FRAME_MS: u64 = 150;

/// 판정 다음에 할 일.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotNext {
    /// 이 그림을 쓴다
    Use,
    /// 정지 스냅숏에서 창 사각형을 자른다
    Fallback,
    /// "이 창은 가져올 수 없어요"
    Protected,
}

/// WGC 결과(`None` = 시간 안에 프레임이 안 왔거나 실패) + 그 창의 `GetWindowDisplayAffinity` 값.
/// 표시 선호도가 걸린 창은 스냅숏을 잘라도 검정(WDA_MONITOR)이거나 뒤 화면(WDA_EXCLUDEFROMCAPTURE)이라 대체하지 않는다.
pub fn after_primary(primary: Option<Blankness>, display_affinity: u32) -> ShotNext {
    match primary {
        Some(Blankness::Content) => ShotNext::Use,
        _ if display_affinity != 0 => ShotNext::Protected,
        _ => ShotNext::Fallback,
    }
}

/// 대체 경로(스냅숏 자르기) 결과.
pub fn after_fallback(b: Blankness) -> ShotNext {
    if b == Blankness::Content {
        ShotNext::Use
    } else {
        ShotNext::Protected
    }
}

/// `DWMWA_WINDOW_CORNER_PREFERENCE` 값(0 기본·1 둥글게 안 함·2 둥글게·3 작게 둥글게)과 최대화 여부 → 모서리 반지름(물리 px).
/// Win11 기본은 8 px(작게 4 px) × 배율, 최대화·스냅 창은 각진다. 값을 못 읽으면(Win10) `None`을 넘겨 0.
pub fn corner_radius(preference: Option<u32>, maximized: bool, scale: f64) -> u32 {
    if maximized {
        return 0;
    }
    let logical = match preference {
        Some(0) | Some(2) => 8.0,
        Some(3) => 4.0,
        _ => 0.0,
    };
    (logical * scale.max(1.0)).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 대체_사슬은_wgc_선호도_스냅숏_순서() {
        assert_eq!(after_primary(Some(Blankness::Content), 0), ShotNext::Use);
        // 보호 창이 아닌데 검정·빈 표면·시간 초과 → 스냅숏 자르기
        assert_eq!(after_primary(Some(Blankness::Black), 0), ShotNext::Fallback);
        assert_eq!(after_primary(Some(Blankness::Unpainted), 0), ShotNext::Fallback);
        assert_eq!(after_primary(None, 0), ShotNext::Fallback);
        // 표시 선호도(WDA_MONITOR=1, WDA_EXCLUDEFROMCAPTURE=0x11) → 대체 없이 안내
        assert_eq!(after_primary(Some(Blankness::Black), 1), ShotNext::Protected);
        assert_eq!(after_primary(None, 0x11), ShotNext::Protected);
        // 선호도가 있어도 내용이 왔으면 쓴다
        assert_eq!(after_primary(Some(Blankness::Content), 1), ShotNext::Use);
        assert_eq!(after_fallback(Blankness::Content), ShotNext::Use);
        assert_eq!(after_fallback(Blankness::Black), ShotNext::Protected);
        assert_eq!(after_fallback(Blankness::Unpainted), ShotNext::Protected);
    }

    #[test]
    fn 모서리_반지름은_선호도와_배율과_최대화로() {
        assert_eq!(corner_radius(Some(0), false, 1.0), 8);
        assert_eq!(corner_radius(Some(2), false, 1.5), 12);
        assert_eq!(corner_radius(Some(3), false, 2.0), 8);
        assert_eq!(corner_radius(Some(1), false, 1.0), 0, "둥글게 안 함");
        assert_eq!(corner_radius(Some(0), true, 1.0), 0, "최대화는 각짐");
        assert_eq!(corner_radius(None, false, 1.0), 0, "Win10");
    }
}
