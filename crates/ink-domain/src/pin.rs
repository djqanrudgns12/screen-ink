//! 캡처 핀(R-CAP-29) 순수 계산 — 휠 확대 단계·커서 기준 창 자리·투명도.

use crate::geometry::PhysRect;

/// 확대 단계(%) — 10~100은 10씩, 100~400은 25씩(Snipaste·이미지 보기 습관).
pub const ZOOM_LEVELS: [u32; 22] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 375, 400];
/// 투명도(%) 단계: 20~100, 10씩.
pub const OPACITY_MIN: u32 = 20;
pub const OPACITY_MAX: u32 = 100;

/// 휠 한 칸: `dy < 0`(위) = 크게, `dy > 0`(아래) = 작게. 단계 밖 값이면 가장 가까운 단계부터.
pub fn next_zoom(zoom: u32, dy: i32) -> u32 {
    if dy == 0 {
        return zoom;
    }
    let i = ZOOM_LEVELS.iter().position(|&z| z >= zoom).unwrap_or(ZOOM_LEVELS.len() - 1);
    let on_level = ZOOM_LEVELS[i] == zoom;
    let j = match (dy < 0, on_level) {
        (true, true) => (i + 1).min(ZOOM_LEVELS.len() - 1),
        (true, false) => i,
        // 아래: 단계 위든 사이든 한 단계 아래로
        (false, _) => i.saturating_sub(1),
    };
    ZOOM_LEVELS[j]
}

/// 새 배율의 창 자리: 원래 그림(`base_w`×`base_h`)에 배율을 곱하고, 창 안 비율 `anchor`(0~1, 커서 자리)를 화면에서 그대로 둔다.
pub fn zoom_rect(cur: PhysRect, base_w: u32, base_h: u32, zoom: u32, anchor: (f64, f64)) -> PhysRect {
    let w = ((base_w as f64 * zoom as f64 / 100.0).round() as i32).max(1);
    let h = ((base_h as f64 * zoom as f64 / 100.0).round() as i32).max(1);
    let (ax, ay) = (anchor.0.clamp(0.0, 1.0), anchor.1.clamp(0.0, 1.0));
    let px = cur.x as f64 + ax * cur.w as f64;
    let py = cur.y as f64 + ay * cur.h as f64;
    PhysRect::new((px - ax * w as f64).round() as i32, (py - ay * h as f64).round() as i32, w, h)
}

/// Ctrl+휠 한 칸: 위 = 진하게(+10), 아래 = 옅게(-10).
pub fn next_opacity(opacity: u32, dy: i32) -> u32 {
    let o = opacity.clamp(OPACITY_MIN, OPACITY_MAX) as i32;
    let step = if dy < 0 { 10 } else if dy > 0 { -10 } else { 0 };
    (o + step).clamp(OPACITY_MIN as i32, OPACITY_MAX as i32) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 확대_단계는_끝에서_멈춘다() {
        assert_eq!(next_zoom(100, -1), 125);
        assert_eq!(next_zoom(100, 1), 90);
        assert_eq!(next_zoom(400, -1), 400);
        assert_eq!(next_zoom(10, 1), 10);
        assert_eq!(next_zoom(110, -1), 125, "단계 밖 값 → 위 단계");
        assert_eq!(next_zoom(110, 1), 100, "단계 밖 값 → 아래 단계");
        assert_eq!(next_zoom(100, 0), 100);
    }

    #[test]
    fn 커서_자리를_그대로_두고_커진다() {
        let cur = PhysRect::new(100, 100, 400, 200);
        // 커서가 창 왼쪽 위 → 왼쪽 위 고정
        assert_eq!(zoom_rect(cur, 400, 200, 200, (0.0, 0.0)), PhysRect::new(100, 100, 800, 400));
        // 가운데 → 가운데 고정
        assert_eq!(zoom_rect(cur, 400, 200, 200, (0.5, 0.5)), PhysRect::new(-100, 0, 800, 400));
        // 50%로 줄이면 오른쪽 아래 기준
        assert_eq!(zoom_rect(cur, 400, 200, 50, (1.0, 1.0)), PhysRect::new(300, 200, 200, 100));
        // 아주 작은 그림도 1 px 아래로 안 감
        assert_eq!(zoom_rect(PhysRect::new(0, 0, 3, 3), 3, 3, 10, (0.0, 0.0)).w, 1);
    }

    #[test]
    fn 투명도는_20에서_100() {
        assert_eq!(next_opacity(100, 1), 90);
        assert_eq!(next_opacity(20, 1), 20);
        assert_eq!(next_opacity(90, -1), 100);
        assert_eq!(next_opacity(100, -1), 100);
        assert_eq!(next_opacity(5, 0), 20, "범위 밖 값은 안으로");
    }
}
