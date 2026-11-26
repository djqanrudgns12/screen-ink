//! 녹화 순수 규칙(05 §8 `RecordingSession`, 02 §5): 상태 기계 · H.264 짝수 크기 · 비트레이트와 남은 공간 ·
//! 임시 파일 이름과 복구 판정(MP4 상자) · 창 추적 녹화의 크기 맞춤 · 대상(모니터·범위·창) 고르기 · 표기.

use crate::geometry::{MonitorLayout, PhysRect};
use serde::{Deserialize, Serialize};

/// `Idle → Picking → Starting → Recording → Finalizing → Idle`(+ 버리기·실패는 Idle로).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecPhase {
    #[default]
    Idle,
    Picking,
    Starting,
    Recording,
    Finalizing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecEvent {
    /// F3 → 범위 고르기
    Pick,
    /// 범위 확정 → 인코더 시작 부탁
    Start,
    /// 인코더가 시작됨
    Started,
    /// F3·[중지]·잠금·절전·모니터 해제
    Stop,
    /// 마무리 끝(저장됨 또는 실패 — 둘 다 Idle)
    Finalized,
    /// 고르기 취소·버리기·시작 실패
    Cancel,
}

/// 상태 전이. 맞지 않는 사건은 `None`(무시).
pub fn transition(phase: RecPhase, ev: RecEvent) -> Option<RecPhase> {
    use RecEvent::*;
    use RecPhase::*;
    match (phase, ev) {
        (Idle, Pick) => Some(Picking),
        (Picking, Start) => Some(Starting),
        (Starting, Started) => Some(Recording),
        (Starting | Recording, Stop) => Some(Finalizing),
        (Finalizing, Finalized) => Some(Idle),
        (Picking | Starting | Recording, Cancel) => Some(Idle),
        _ => None,
    }
}

/// 무엇을 녹화하나(사용자 결정 #6: 창을 고르면 창 추적).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RecTarget {
    /// 모니터 전체(클릭 한 번)
    Monitor { monitor_id: u32, rect: PhysRect },
    /// 모니터 캡처를 잘라서(짝수 보정한 사각형, 한 모니터 안)
    Region { monitor_id: u32, rect: PhysRect },
    /// 창 추적(WGC CreateForWindow) — 창이 움직여도 따라간다. `rect`는 시작 순간 자리
    Window { handle: u64, rect: PhysRect },
}

impl RecTarget {
    pub fn rect(&self) -> PhysRect {
        match self {
            RecTarget::Monitor { rect, .. } | RecTarget::Region { rect, .. } | RecTarget::Window { rect, .. } => *rect,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            RecTarget::Monitor { .. } => "monitor",
            RecTarget::Region { .. } => "region",
            RecTarget::Window { .. } => "window",
        }
    }

    pub fn monitor_id(&self) -> Option<u32> {
        match self {
            RecTarget::Monitor { monitor_id, .. } | RecTarget::Region { monitor_id, .. } => Some(*monitor_id),
            RecTarget::Window { .. } => None,
        }
    }
}

/// H.264 4:2:0 최소 크기.
pub const MIN_SIDE: i32 = 16;

/// 가로·세로를 짝수로 내린다(최소 16). `bounds`(모니터) 밖으로 나가지 않게 키울 때는 왼쪽 위로 민다.
pub fn even_rect(r: PhysRect, bounds: PhysRect) -> PhysRect {
    let mut w = (r.w & !1).max(MIN_SIDE).min(bounds.w & !1);
    let mut h = (r.h & !1).max(MIN_SIDE).min(bounds.h & !1);
    w = w.max(2);
    h = h.max(2);
    let x = r.x.min(bounds.right() - w).max(bounds.x);
    let y = r.y.min(bounds.bottom() - h).max(bounds.y);
    PhysRect::new(x, y, w, h)
}

/// 크기를 짝수로(창 추적 첫 크기, 영상 크기).
pub fn even_size(w: u32, h: u32) -> (u32, u32) {
    ((w & !1).max(MIN_SIDE as u32), (h & !1).max(MIN_SIDE as u32))
}

/// 목표 비트레이트(bps): 화소 × 초당 장면 × 0.08 비트(1080p30 ≈ 5 Mbps), 1~16 Mbps.
pub fn bitrate(w: u32, h: u32, fps: u32) -> u32 {
    let bits = w as f64 * h as f64 * fps.max(1) as f64 * 0.08;
    bits.clamp(1_000_000.0, 16_000_000.0) as u32
}

/// 분당 예상 바이트(목표 비트레이트 기준 — 첫 실측으로 바꿀 추정, 02 §5).
pub fn bytes_per_minute(w: u32, h: u32, fps: u32) -> u64 {
    bitrate(w, h, fps) as u64 * 60 / 8
}

/// 남은 공간으로 녹화할 수 있는 분(내림).
pub fn minutes_left(free_bytes: u64, w: u32, h: u32, fps: u32) -> u64 {
    free_bytes / bytes_per_minute(w, h, fps).max(1)
}

/// 시작 전 확인(02 §5): 10분을 못 채우면 그 분 수 — 말풍선 후 그대로 시작한다.
pub const WARN_MINUTES: u64 = 10;

pub fn space_warning(free_bytes: Option<u64>, w: u32, h: u32, fps: u32) -> Option<u64> {
    let m = minutes_left(free_bytes?, w, h, fps);
    (m < WARN_MINUTES).then_some(m)
}

/// 임시 파일 이름(`%TEMP%\shookshok\rec-<id>.mp4`).
pub fn temp_name(id: u64) -> String {
    format!("rec-{id}.mp4")
}

/// 임시 녹화 파일 이름이면 그 id.
pub fn parse_temp_name(name: &str) -> Option<u64> {
    let id = name.strip_prefix("rec-")?.strip_suffix(".mp4")?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    id.parse().ok()
}

/// 끝나지 않은 녹화(다음 시작 때 복구 제안, 05 §8 R).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leftover {
    pub path: String,
    pub bytes: u64,
    /// MP4 마무리(moov)가 있어 재생될 것 같음
    pub playable: bool,
}

/// MP4 상자 머리 16바이트 → (상자 길이, 종류). 길이 0 = 파일 끝까지, 1 = 64비트 길이.
pub fn mp4_box(head: &[u8]) -> Option<(u64, [u8; 4])> {
    if head.len() < 8 {
        return None;
    }
    let size32 = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as u64;
    let kind = [head[4], head[5], head[6], head[7]];
    let size = match size32 {
        1 => {
            if head.len() < 16 {
                return None;
            }
            u64::from_be_bytes([head[8], head[9], head[10], head[11], head[12], head[13], head[14], head[15]])
        }
        n => n,
    };
    Some((size, kind))
}

/// 맨 위 상자 종류들로 재생 가능 여부: `ftyp`로 시작하고 `moov`가 있어야 한다(싱크 라이터는 마무리 때 moov를 쓴다).
pub fn mp4_playable(kinds: &[[u8; 4]]) -> bool {
    kinds.first() == Some(b"ftyp") && kinds.iter().any(|k| k == b"moov")
}

/// 창 추적 중 창 크기가 바뀌면 영상 크기 안에 비율을 지켜 줄여 가운데 둔다(키우지 않는다). 짝수 위치·크기.
pub fn fit_inside(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> PhysRect {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return PhysRect::new(0, 0, 0, 0);
    }
    let s = (dst_w as f64 / src_w as f64).min(dst_h as f64 / src_h as f64).min(1.0);
    let w = ((src_w as f64 * s).round() as u32).min(dst_w) & !1;
    let h = ((src_h as f64 * s).round() as u32).min(dst_h) & !1;
    let x = ((dst_w - w) / 2) & !1;
    let y = ((dst_h - h) / 2) & !1;
    PhysRect::new(x as i32, y as i32, w as i32, h as i32)
}

/// 고른 범위 → 녹화 대상. `hover_kind` = 확정 직전 고르기 방식(`Selector::mode_name`: window·full·element·drag),
/// `window` = 그 자리 창(핸들·사각형). 창을 고른 그대로면 창 추적, 모니터 전체면 모니터, 나머지는 범위(한 모니터로 자름).
pub fn target_for(hover_kind: &str, rect: PhysRect, window: Option<(u64, PhysRect)>, layout: &MonitorLayout) -> Option<RecTarget> {
    if hover_kind == "window" {
        if let Some((handle, wr)) = window.filter(|(_, wr)| *wr == rect) {
            return Some(RecTarget::Window { handle, rect: wr });
        }
    }
    let center = crate::geometry::PhysPoint { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 };
    let m = layout.monitor_at(center).or_else(|| layout.nearest(center))?;
    if rect == m.bounds || (hover_kind == "full" && rect.intersect(&m.bounds) == Some(m.bounds)) {
        return Some(RecTarget::Monitor { monitor_id: m.id, rect: m.bounds });
    }
    let clipped = rect.intersect(&m.bounds)?;
    Some(RecTarget::Region { monitor_id: m.id, rect: even_rect(clipped, m.bounds) })
}

/// 경과 표기: `00:42` · `12:05` · `1:02:03`.
pub fn clock_label(ms: u64) -> String {
    let s = ms / 1000;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// 크기 표기: 10 MB 아래는 소수 한 자리(`4.2 MB`), 그 위는 정수(`18 MB`), 1 GB부터 `1.3 GB`.
pub fn size_label(bytes: u64) -> String {
    let mb = bytes as f64 / 1_048_576.0;
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else if mb >= 10.0 {
        format!("{} MB", mb.round() as u64)
    } else {
        format!("{:.1} MB", mb)
    }
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

    #[test]
    fn 녹화_해상도_짝수보정_및_남은공간_예상분_계산() {
        let mon = PhysRect::new(0, 0, 1920, 1080);
        assert_eq!(even_rect(PhysRect::new(1, 1, 1919, 1079), mon), PhysRect::new(1, 1, 1918, 1078));
        // 너무 작으면 16으로 키우되 모니터 안으로
        assert_eq!(even_rect(PhysRect::new(1915, 1075, 5, 5), mon), PhysRect::new(1904, 1064, 16, 16));
        assert_eq!(even_size(1281, 721), (1280, 720));
        // 1080p30 ≈ 5 Mbps → 분당 약 37 MB
        assert_eq!(bitrate(1920, 1080, 30), 4_976_640);
        assert_eq!(bytes_per_minute(1920, 1080, 30), 37_324_800);
        // 60fps는 두 배, 아주 작은 범위도 1 Mbps 바닥
        assert_eq!(bitrate(1920, 1080, 60), 9_953_280);
        assert_eq!(bitrate(100, 100, 30), 1_000_000);
        // 남은 공간: 186.6 MB / 분당 37.3 MB = 5분(내림) → 10분 못 채움 경고
        let free = 186_624_000;
        assert_eq!(minutes_left(free, 1920, 1080, 30), 5);
        assert_eq!(minutes_left(free - 1, 1920, 1080, 30), 4);
        assert_eq!(space_warning(Some(free), 1920, 1080, 30), Some(5));
        assert_eq!(space_warning(Some(50_000_000_000), 1920, 1080, 30), None);
        assert_eq!(space_warning(None, 1920, 1080, 30), None, "모르면 경고하지 않음");
    }

    #[test]
    fn 상태_기계는_정해진_길로만() {
        use RecEvent::*;
        use RecPhase::*;
        let mut p = Idle;
        for (ev, want) in [(Pick, Picking), (Start, Starting), (Started, Recording), (Stop, Finalizing), (Finalized, Idle)] {
            p = transition(p, ev).unwrap_or_else(|| panic!("{p:?} + {ev:?}"));
            assert_eq!(p, want);
        }
        assert_eq!(transition(Idle, Stop), None, "녹화 전 중지는 무시");
        assert_eq!(transition(Recording, Pick), None);
        assert_eq!(transition(Starting, Stop), Some(Finalizing), "시작 1초 안 중지도 마무리");
        assert_eq!(transition(Recording, Cancel), Some(Idle), "버리기");
        assert_eq!(transition(Finalizing, Cancel), None, "마무리 중에는 못 버림");
    }

    #[test]
    fn 임시_파일_이름과_mp4_마무리_판정() {
        assert_eq!(temp_name(1_772_582_707), "rec-1772582707.mp4");
        assert_eq!(parse_temp_name("rec-42.mp4"), Some(42));
        for bad in ["rec-.mp4", "rec-4a.mp4", "rec-42.mkv", "shookshok-42.mp4", "rec-42.mp4.tmp"] {
            assert_eq!(parse_temp_name(bad), None, "{bad}");
        }
        let mut head = [0u8; 16];
        head[..8].copy_from_slice(&[0, 0, 0, 24, b'f', b't', b'y', b'p']);
        assert_eq!(mp4_box(&head), Some((24, *b"ftyp")));
        head[..8].copy_from_slice(&[0, 0, 0, 1, b'm', b'd', b'a', b't']);
        head[8..].copy_from_slice(&5_000_000_000u64.to_be_bytes());
        assert_eq!(mp4_box(&head), Some((5_000_000_000, *b"mdat")), "64비트 길이");
        assert_eq!(mp4_box(&head[..8]), None, "64비트 길이인데 머리가 짧음");
        assert!(mp4_playable(&[*b"ftyp", *b"mdat", *b"moov"]));
        assert!(!mp4_playable(&[*b"ftyp", *b"mdat"]), "강제 종료 — moov 없음");
        assert!(!mp4_playable(&[*b"mdat", *b"moov"]));
    }

    #[test]
    fn 창_크기가_바뀌면_비율을_지켜_줄여_가운데() {
        assert_eq!(fit_inside(1280, 720, 1280, 720), PhysRect::new(0, 0, 1280, 720));
        assert_eq!(fit_inside(1920, 1080, 1280, 720), PhysRect::new(0, 0, 1280, 720));
        assert_eq!(fit_inside(1000, 1000, 1280, 720), PhysRect::new(280, 0, 720, 720));
        assert_eq!(fit_inside(640, 360, 1280, 720), PhysRect::new(320, 180, 640, 360), "작아지면 키우지 않고 가운데");
        assert_eq!(fit_inside(0, 10, 1280, 720).w, 0);
    }

    #[test]
    fn 고른_범위로_녹화_대상() {
        let l = layout();
        let win = Some((7u64, PhysRect::new(100, 100, 1200, 800)));
        assert_eq!(target_for("window", PhysRect::new(100, 100, 1200, 800), win, &l), Some(RecTarget::Window { handle: 7, rect: PhysRect::new(100, 100, 1200, 800) }));
        // 창 단계였어도 사각형이 다르면(손잡이로 바꿈) 범위
        assert!(matches!(target_for("window", PhysRect::new(100, 100, 1201, 800), win, &l), Some(RecTarget::Region { .. })));
        // 모니터 전체 클릭 → 모니터
        assert_eq!(target_for("full", PhysRect::new(1920, 0, 2880, 1800), None, &l), Some(RecTarget::Monitor { monitor_id: 1, rect: PhysRect::new(1920, 0, 2880, 1800) }));
        // 드래그 → 범위, 짝수로, 두 모니터에 걸치면 가운데가 있는 모니터로 자름
        assert_eq!(target_for("drag", PhysRect::new(11, 21, 301, 201), None, &l), Some(RecTarget::Region { monitor_id: 0, rect: PhysRect::new(11, 21, 300, 200) }));
        assert_eq!(target_for("drag", PhysRect::new(1800, 0, 400, 100), None, &l), Some(RecTarget::Region { monitor_id: 1, rect: PhysRect::new(1920, 0, 280, 100) }));
        let t = RecTarget::Region { monitor_id: 0, rect: PhysRect::new(0, 0, 2, 2) };
        assert_eq!((t.kind_name(), t.monitor_id()), ("region", Some(0)));
        let json = serde_json::to_value(RecTarget::Window { handle: 3, rect: PhysRect::new(0, 0, 2, 2) }).unwrap();
        assert_eq!(json["kind"], "window");
        assert_eq!(json["handle"], 3);
    }

    #[test]
    fn 경과와_크기_표기() {
        assert_eq!(clock_label(0), "00:00");
        assert_eq!(clock_label(42_999), "00:42");
        assert_eq!(clock_label(725_000), "12:05");
        assert_eq!(clock_label(3_723_000), "1:02:03");
        assert_eq!(size_label(4_404_019), "4.2 MB");
        assert_eq!(size_label(18 * 1_048_576 + 300_000), "18 MB");
        assert_eq!(size_label(1_395_864_371), "1.3 GB");
    }
}
