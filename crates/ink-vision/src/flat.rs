//! 평탄 영역(08 §4.3): 커서 점 색과 채널별 ±6 이내로 이어진 영역을 30만 화소 예산 안에서 채워 바운딩 박스를 만든다.
//! 4K에서도 1 ms 안에 끝나도록 스캔라인 채우기 + 64×64 타일 비트셋(방문한 타일만 할당)을 쓴다.

use crate::prep::PrepData;
use crate::{ladder, FLAT_PIXEL_BUDGET, FLAT_TOLERANCE, GRAD_THRESHOLD, OCCLUDER_EDGE_TOL};
use ink_domain::geometry::PhysRect;
use std::collections::VecDeque;

const TILE_SHIFT: usize = 6; // 64×64 타일
const TILE_WORDS: usize = (64 * 64) / 64; // 64개의 u64 = 512 바이트

struct TileBitset {
    tiles_x: usize,
    tiles: Vec<Option<Box<[u64; TILE_WORDS]>>>,
}

impl TileBitset {
    fn new(w: usize, h: usize) -> Self {
        let tiles_x = (w + 63) >> TILE_SHIFT;
        let tiles_y = (h + 63) >> TILE_SHIFT;
        Self { tiles_x, tiles: vec![None; tiles_x * tiles_y] }
    }

    #[inline]
    fn tile_and_bit(&self, x: usize, y: usize) -> (usize, usize, u64) {
        let tid = (y >> TILE_SHIFT) * self.tiles_x + (x >> TILE_SHIFT);
        let local = ((y & 63) << 6) | (x & 63);
        (tid, local >> 6, 1u64 << (local & 63))
    }

    #[inline]
    fn is_set(&self, x: usize, y: usize) -> bool {
        let (tid, word, mask) = self.tile_and_bit(x, y);
        match self.tiles[tid].as_deref() {
            Some(bits) => (bits[word] & mask) != 0,
            None => false,
        }
    }

    #[inline]
    fn set(&mut self, x: usize, y: usize) {
        let (tid, word, mask) = self.tile_and_bit(x, y);
        let bits = self.tiles[tid].get_or_insert_with(|| Box::new([0u64; TILE_WORDS]));
        bits[word] |= mask;
    }
}

#[inline]
fn color_matches(row: &[u8], x: usize, b0: u8, g0: u8, r0: u8) -> bool {
    let off = x * 4;
    row[off].abs_diff(b0) <= FLAT_TOLERANCE
        && row[off + 1].abs_diff(g0) <= FLAT_TOLERANCE
        && row[off + 2].abs_diff(r0) <= FLAT_TOLERANCE
}

/// ROI 내부 좌표 `(lx, ly)`에서 평탄 영역 바운딩 박스(ROI 좌표)를 찾는다.
/// 예산을 초과하면 영역이 잘린 것이므로 `None`을 돌려준다.
pub(crate) fn find_flat_region(data: &PrepData, lx: i32, ly: i32) -> Option<PhysRect> {
    if data.is_empty() || lx < 0 || ly < 0 || lx >= data.rw as i32 || ly >= data.rh as i32 {
        return None;
    }
    let w = data.rw as usize;
    let h = data.rh as usize;
    let row_bytes = w * 4;
    let sx = lx as usize;
    let sy = ly as usize;

    let seed_off = sy * row_bytes + sx * 4;
    let b0 = data.bgra[seed_off];
    let g0 = data.bgra[seed_off + 1];
    let r0 = data.bgra[seed_off + 2];

    let mut visited = TileBitset::new(w, h);
    let mut stack: Vec<(usize, usize)> = Vec::with_capacity(256);
    stack.push((sx, sy));

    let mut min_x = sx;
    let mut max_x = sx;
    let mut min_y = sy;
    let mut max_y = sy;
    let mut filled: usize = 0;

    while let Some((px, py)) = stack.pop() {
        if visited.is_set(px, py) {
            continue;
        }
        let row = &data.bgra[py * row_bytes..(py + 1) * row_bytes];
        if !color_matches(row, px, b0, g0, r0) {
            continue;
        }

        // 왼쪽 끝까지 확장
        let mut x_left = px;
        while x_left > 0 && !visited.is_set(x_left - 1, py) && color_matches(row, x_left - 1, b0, g0, r0) {
            x_left -= 1;
        }
        // 오른쪽 끝까지 확장
        let mut x_right = px;
        while x_right + 1 < w && !visited.is_set(x_right + 1, py) && color_matches(row, x_right + 1, b0, g0, r0) {
            x_right += 1;
        }

        let span_len = x_right - x_left + 1;
        filled += span_len;
        if filled > FLAT_PIXEL_BUDGET {
            return None;
        }

        for x in x_left..=x_right {
            visited.set(x, py);
        }

        if x_left < min_x {
            min_x = x_left;
        }
        if x_right > max_x {
            max_x = x_right;
        }
        if py < min_y {
            min_y = py;
        }
        if py > max_y {
            max_y = py;
        }

        // 위·아래 행의 연속 구간 시작점만 스택에 넣기
        if py > 0 {
            let up_row = &data.bgra[(py - 1) * row_bytes..py * row_bytes];
            let mut in_run = false;
            for x in x_left..=x_right {
                let ok = !visited.is_set(x, py - 1) && color_matches(up_row, x, b0, g0, r0);
                if ok && !in_run {
                    stack.push((x, py - 1));
                    in_run = true;
                } else if !ok {
                    in_run = false;
                }
            }
        }
        if py + 1 < h {
            let dn_row = &data.bgra[(py + 1) * row_bytes..(py + 2) * row_bytes];
            let mut in_run = false;
            for x in x_left..=x_right {
                let ok = !visited.is_set(x, py + 1) && color_matches(dn_row, x, b0, g0, r0);
                if ok && !in_run {
                    stack.push((x, py + 1));
                    in_run = true;
                } else if !ok {
                    in_run = false;
                }
            }
        }
    }

    let rect = PhysRect::from_edges(min_x as i32, min_y as i32, (max_x + 1) as i32, (max_y + 1) as i32);
    if rect.w < 4 || rect.h < 4 || rect.area() < 16 {
        return None;
    }
    Some(rect)
}

// ─────────────── R2-CAP-3: 이미지 요소 실제 그림 가장자리 재측정 ───────────────

const MIN_REFINE_SIDE: i32 = 16;
const MIN_REFINED_SIDE: i32 = 8;
const SIDE_FLAT_RATIO: f32 = 0.85;
const OUTSIDE_CONFIRM_RATIO: f32 = 0.50;
const MAX_SHADOW_SPREAD: u8 = 24;
const SHADOW_MAX_ENTRY_DROP: i32 = 9;
const SHADOW_MAX_LUMA_DROP: i32 = 18;
const SHADOW_MAX_CH_DROP: u8 = 20;
const SHADOW_MIN_SCALE: f32 = 0.45;
const SHADOW_CHROMA_TOL: f32 = 8.0;
const MIN_REMAINING_AREA_RATIO: f64 = 0.15;

#[inline]
fn px_bgr(bgra: &[u8], w: usize, x: usize, y: usize) -> [u8; 3] {
    let off = (y * w + x) * 4;
    [bgra[off], bgra[off + 1], bgra[off + 2]]
}

#[inline]
fn bgr_matches(a: [u8; 3], b: [u8; 3]) -> bool {
    a[0].abs_diff(b[0]) <= FLAT_TOLERANCE
        && a[1].abs_diff(b[1]) <= FLAT_TOLERANCE
        && a[2].abs_diff(b[2]) <= FLAT_TOLERANCE
}

#[inline]
fn bgr_luma(c: [u8; 3]) -> u8 {
    ((29u32 * c[0] as u32 + 150u32 * c[1] as u32 + 77u32 * c[2] as u32) >> 8) as u8
}

/// 부드러운 드롭 그림자 픽셀 판정: 바탕색을 어둡게 곱한 무채색 감쇠 범위만 허용한다.
#[inline]
fn is_shadow_pixel(q: [u8; 3], bg: [u8; 3]) -> bool {
    if q[0] > bg[0].saturating_add(FLAT_TOLERANCE)
        || q[1] > bg[1].saturating_add(FLAT_TOLERANCE)
        || q[2] > bg[2].saturating_add(FLAT_TOLERANCE)
    {
        return false;
    }
    let luma_bg = bgr_luma(bg);
    if luma_bg < 12 {
        return false;
    }
    let luma_q = bgr_luma(q);
    let scale = luma_q as f32 / luma_bg as f32;
    if !(SHADOW_MIN_SCALE..=1.02).contains(&scale) {
        return false;
    }
    (q[0] as f32 - bg[0] as f32 * scale).abs() <= SHADOW_CHROMA_TOL
        && (q[1] as f32 - bg[1] as f32 * scale).abs() <= SHADOW_CHROMA_TOL
        && (q[2] as f32 - bg[2] as f32 * scale).abs() <= SHADOW_CHROMA_TOL
}

/// 그림자 전이 단조성(밝기 역전 금지 + 급경사에서 정지). `max_drop`으로 진입/내부 스텝 낙차를 제어한다.
#[inline]
fn shadow_step_ok(p: [u8; 3], q: [u8; 3], max_drop: i32) -> bool {
    let lp = bgr_luma(p) as i32;
    let lq = bgr_luma(q) as i32;
    // 밝아지면(그림자 넘어 그림 밝은 가장자리로 진입) 차단
    if lq > lp + 2 {
        return false;
    }
    let drop = lp - lq;
    if drop > max_drop || drop >= GRAD_THRESHOLD as i32 {
        return false;
    }
    p[0].abs_diff(q[0]) <= SHADOW_MAX_CH_DROP
        && p[1].abs_diff(q[1]) <= SHADOW_MAX_CH_DROP
        && p[2].abs_diff(q[2]) <= SHADOW_MAX_CH_DROP
}

/// 변 위의 색들을 샘플링해 대표 색과 일치 비율(>= 0.85) 및 양 끝 모서리 일치 여부를 확인한다.
/// 가려 잘린 인접 변 쪽 끝(`check_start`/`check_end == false`)은 모서리 검사를 건너뛴다.
fn dominant_side_bg(samples: &[[u8; 3]], check_start: bool, check_end: bool) -> Option<[u8; 3]> {
    if samples.len() < 4 {
        return None;
    }
    let step = (samples.len() / 32).max(1);
    let mut best_c = samples[0];
    let mut best_votes = 0usize;
    for i in (0..samples.len()).step_by(step) {
        let cand = samples[i];
        let votes = samples.iter().step_by(step).filter(|&&s| bgr_matches(s, cand)).count();
        if votes > best_votes {
            best_votes = votes;
            best_c = cand;
        }
    }
    let matched = samples.iter().filter(|&&s| bgr_matches(s, best_c)).count();
    if (matched as f32 / samples.len() as f32) < SIDE_FLAT_RATIO {
        return None;
    }
    // 여백 없는 둥근 모서리 그림 오탐 방지: 양 끝 모서리 구간(3 px)에도 대표 색이 닿아야 여백 변으로 인정
    let end_span = samples.len().min(3);
    let start_slice = &samples[..end_span];
    let end_slice = &samples[samples.len() - end_span..];
    let start_ok = !check_start || start_slice.iter().any(|&s| bgr_matches(s, best_c));
    let end_ok = !check_end || end_slice.iter().any(|&s| bgr_matches(s, best_c));
    if start_ok && end_ok {
        return Some(best_c);
    }
    // 한쪽 끝은 바탕과 일치하고 다른 한쪽 끝만 앞 창 그림자 감쇠 범위에 놓인 경우 허용
    if check_start && check_end {
        let start_bg = start_slice.iter().any(|&s| bgr_matches(s, best_c));
        let end_bg = end_slice.iter().any(|&s| bgr_matches(s, best_c));
        let start_shadow = start_slice.iter().any(|&s| is_shadow_pixel(s, best_c));
        let end_shadow = end_slice.iter().any(|&s| is_shadow_pixel(s, best_c));
        if (start_bg && end_shadow) || (end_bg && start_shadow) {
            return Some(best_c);
        }
    }
    None
}

/// 이미지 요소(`local_rect`, ROI 좌표) 안에서 여백·부드러운 그림자를 걷어내어 실제 그림 가장자리를 다시 잰다.
/// 여백 없는 사진(예: 상단 하늘)이나 너무 작은 영역은 원래 사각형을 그대로 돌려준다.
#[allow(dead_code)]
pub(crate) fn refine_image_bounds(data: &PrepData, local_rect: PhysRect) -> PhysRect {
    refine_image_bounds_with_occluders(data, local_rect, &[])
}

/// 앞 창(`occluders`, ROI 좌표)에 가려 잘린 변은 여백 찾기를 건너뛰고 나머지 변의 여백·그림자를 걷어낸다(R2-CAP-2·3).
pub(crate) fn refine_image_bounds_with_occluders(
    data: &PrepData,
    local_rect: PhysRect,
    occluders: &[PhysRect],
) -> PhysRect {
    if data.is_empty() {
        return local_rect;
    }
    let Some(clipped) = local_rect.intersect(&PhysRect::new(0, 0, data.rw as i32, data.rh as i32)) else {
        return local_rect;
    };
    if clipped.w < MIN_REFINE_SIDE || clipped.h < MIN_REFINE_SIDE {
        return local_rect;
    }

    let w = data.rw as usize;
    let h = data.rh as usize;
    let lx0 = clipped.x;
    let ly0 = clipped.y;
    let lx1 = clipped.right();
    let ly1 = clipped.bottom();
    let rx0 = lx0 as usize;
    let ry0 = ly0 as usize;
    let rx1 = lx1 as usize;
    let ry1 = ly1 as usize;
    let bw = rx1 - rx0;
    let bh = ry1 - ry0;

    // 4변(0=위, 1=아래, 2=왼쪽, 3=오른쪽)의 앞 창 가림 여부 및 경계 ±2 px 그림자·테두리 오염 마진 계산
    let occluded = if occluders.is_empty() {
        [false; 4]
    } else {
        [
            ladder::h_line_touches_occluders(ly0, lx0, lx1, occluders, OCCLUDER_EDGE_TOL),
            ladder::h_line_touches_occluders(ly1, lx0, lx1, occluders, OCCLUDER_EDGE_TOL),
            ladder::v_line_touches_occluders(lx0, ly0, ly1, occluders, OCCLUDER_EDGE_TOL),
            ladder::v_line_touches_occluders(lx1, ly0, ly1, occluders, OCCLUDER_EDGE_TOL),
        ]
    };
    let mut margin = [0usize; 4];
    for o in occluders {
        if o.right() > lx0 && o.x < lx1 {
            if occluded[0] && o.y < ly0 && (ly0 - o.bottom()).abs() <= OCCLUDER_EDGE_TOL {
                let m = (o.bottom() + OCCLUDER_EDGE_TOL - ly0).clamp(0, OCCLUDER_EDGE_TOL) as usize;
                margin[0] = margin[0].max(m);
            }
            if occluded[1] && o.bottom() > ly1 && (o.y - ly1).abs() <= OCCLUDER_EDGE_TOL {
                let m = (ly1 - (o.y - OCCLUDER_EDGE_TOL)).clamp(0, OCCLUDER_EDGE_TOL) as usize;
                margin[1] = margin[1].max(m);
            }
        }
        if o.bottom() > ly0 && o.y < ly1 {
            if occluded[2] && o.x < lx0 && (lx0 - o.right()).abs() <= OCCLUDER_EDGE_TOL {
                let m = (o.right() + OCCLUDER_EDGE_TOL - lx0).clamp(0, OCCLUDER_EDGE_TOL) as usize;
                margin[2] = margin[2].max(m);
            }
            if occluded[3] && o.right() > lx1 && (o.x - lx1).abs() <= OCCLUDER_EDGE_TOL {
                let m = (lx1 - (o.x - OCCLUDER_EDGE_TOL)).clamp(0, OCCLUDER_EDGE_TOL) as usize;
                margin[3] = margin[3].max(m);
            }
        }
    }

    let x_start = margin[2];
    let x_end = bw.saturating_sub(margin[3]);
    let y_start = margin[0];
    let y_end = bh.saturating_sub(margin[1]);
    if x_end <= x_start + 4 || y_end <= y_start + 4 {
        return local_rect;
    }

    // 가려지지 않은 변만 샘플링(가려 잘린 변은 여백 후보·시드에서 제외)
    let top_s: Option<Vec<[u8; 3]>> = (!occluded[0])
        .then(|| (rx0 + x_start..rx0 + x_end).map(|x| px_bgr(&data.bgra, w, x, ry0 + y_start)).collect());
    let bot_s: Option<Vec<[u8; 3]>> = (!occluded[1])
        .then(|| (rx0 + x_start..rx0 + x_end).map(|x| px_bgr(&data.bgra, w, x, ry0 + y_end - 1)).collect());
    let left_s: Option<Vec<[u8; 3]>> = (!occluded[2])
        .then(|| (ry0 + y_start..ry0 + y_end).map(|y| px_bgr(&data.bgra, w, rx0 + x_start, y)).collect());
    let right_s: Option<Vec<[u8; 3]>> = (!occluded[3])
        .then(|| (ry0 + y_start..ry0 + y_end).map(|y| px_bgr(&data.bgra, w, rx0 + x_end - 1, y)).collect());

    let side_bgs = [
        top_s.as_deref().and_then(|s| dominant_side_bg(s, !occluded[2], !occluded[3])),
        bot_s.as_deref().and_then(|s| dominant_side_bg(s, !occluded[2], !occluded[3])),
        left_s.as_deref().and_then(|s| dominant_side_bg(s, !occluded[0], !occluded[1])),
        right_s.as_deref().and_then(|s| dominant_side_bg(s, !occluded[0], !occluded[1])),
    ];

    // 외부 배경 일치 또는 다변 일치 확인(한쪽 끝만 단색인 하늘 사진 오탐 방지)
    let span_w = (x_end - x_start) as f32;
    let span_h = (y_end - y_start) as f32;
    let mut confirmed = [false; 4];
    for (i, side_opt) in side_bgs.iter().enumerate() {
        let Some(bg) = *side_opt else { continue };
        let matches_other = side_bgs
            .iter()
            .enumerate()
            .any(|(j, ob)| j != i && ob.is_some_and(|c| bgr_matches(bg, c)));
        if matches_other {
            confirmed[i] = true;
            continue;
        }
        let outside_match = match i {
            0 if !occluded[0] && ry0 > 0 => {
                let cnt = (rx0 + x_start..rx0 + x_end)
                    .filter(|&x| bgr_matches(px_bgr(&data.bgra, w, x, ry0 - 1), bg))
                    .count();
                cnt as f32 / span_w >= OUTSIDE_CONFIRM_RATIO
            }
            1 if !occluded[1] && ry1 < h => {
                let cnt = (rx0 + x_start..rx0 + x_end)
                    .filter(|&x| bgr_matches(px_bgr(&data.bgra, w, x, ry1), bg))
                    .count();
                cnt as f32 / span_w >= OUTSIDE_CONFIRM_RATIO
            }
            2 if !occluded[2] && rx0 > 0 => {
                let cnt = (ry0 + y_start..ry0 + y_end)
                    .filter(|&y| bgr_matches(px_bgr(&data.bgra, w, rx0 - 1, y), bg))
                    .count();
                cnt as f32 / span_h >= OUTSIDE_CONFIRM_RATIO
            }
            3 if !occluded[3] && rx1 < w => {
                let cnt = (ry0 + y_start..ry0 + y_end)
                    .filter(|&y| bgr_matches(px_bgr(&data.bgra, w, rx1, y), bg))
                    .count();
                cnt as f32 / span_h >= OUTSIDE_CONFIRM_RATIO
            }
            _ => false,
        };
        confirmed[i] = outside_match;
    }

    if !confirmed.iter().any(|&c| c) {
        return local_rect;
    }

    // 확정 배경색 목록 수집(중복 제거)
    let mut bgs: Vec<[u8; 3]> = Vec::with_capacity(4);
    for (i, &ok) in confirmed.iter().enumerate() {
        if ok {
            if let Some(bg) = side_bgs[i] {
                if !bgs.iter().any(|&b| bgr_matches(b, bg)) {
                    bgs.push(bg);
                }
            }
        }
    }
    if bgs.is_empty() {
        return local_rect;
    }

    let total_cells = bw * bh;
    let mut visited = vec![0u64; total_cells.div_ceil(64)];
    let is_vis = |v: &[u64], lx: usize, ly: usize| -> bool {
        let idx = ly * bw + lx;
        (v[idx >> 6] & (1u64 << (idx & 63))) != 0
    };
    let set_vis = |v: &mut [u64], lx: usize, ly: usize| {
        let idx = ly * bw + lx;
        v[idx >> 6] |= 1u64 << (idx & 63);
    };

    let mut bg_q: VecDeque<(usize, usize, u8)> = VecDeque::with_capacity(bw + bh);
    let mut shadow_q: VecDeque<(usize, usize, u8, u8)> = VecDeque::new();
    let mut filled = 0usize;

    let mut seed_px = |lx: usize, ly: usize, bg_idx: usize| {
        if is_vis(&visited, lx, ly) {
            return;
        }
        let px = px_bgr(&data.bgra, w, rx0 + lx, ry0 + ly);
        if bgr_matches(px, bgs[bg_idx]) {
            set_vis(&mut visited, lx, ly);
            bg_q.push_back((lx, ly, bg_idx as u8));
            filled += 1;
        }
    };

    for (side_idx, &ok) in confirmed.iter().enumerate() {
        if !ok {
            continue;
        }
        let side_bg = side_bgs[side_idx].unwrap();
        let bg_idx = bgs.iter().position(|&b| bgr_matches(b, side_bg)).unwrap_or(0);
        match side_idx {
            0 => (x_start..x_end).for_each(|lx| seed_px(lx, y_start, bg_idx)),
            1 => (x_start..x_end).for_each(|lx| seed_px(lx, y_end - 1, bg_idx)),
            2 => (y_start..y_end).for_each(|ly| seed_px(x_start, ly, bg_idx)),
            _ => (y_start..y_end).for_each(|ly| seed_px(x_end - 1, ly, bg_idx)),
        }
    }

    // 1단계: 평탄 여백 확장 + 접한 부드러운 그림자 시작점 수집
    while let Some((lx, ly, bg_i)) = bg_q.pop_front() {
        let bg = bgs[bg_i as usize];
        let p_px = px_bgr(&data.bgra, w, rx0 + lx, ry0 + ly);
        let neighbors = [
            (lx > x_start).then(|| (lx - 1, ly)),
            (lx + 1 < x_end).then(|| (lx + 1, ly)),
            (ly > y_start).then(|| (lx, ly - 1)),
            (ly + 1 < y_end).then(|| (lx, ly + 1)),
        ];
        for n in neighbors.into_iter().flatten() {
            if is_vis(&visited, n.0, n.1) {
                continue;
            }
            let q_px = px_bgr(&data.bgra, w, rx0 + n.0, ry0 + n.1);
            if bgr_matches(q_px, bg) {
                set_vis(&mut visited, n.0, n.1);
                bg_q.push_back((n.0, n.1, bg_i));
                filled += 1;
                if filled > FLAT_PIXEL_BUDGET {
                    return local_rect;
                }
            } else if is_shadow_pixel(q_px, bg) && shadow_step_ok(p_px, q_px, SHADOW_MAX_ENTRY_DROP) {
                set_vis(&mut visited, n.0, n.1);
                shadow_q.push_back((n.0, n.1, 1, bg_i));
                filled += 1;
                if filled > FLAT_PIXEL_BUDGET {
                    return local_rect;
                }
            }
        }
    }

    // 2단계: 부드러운 그림자 감쇠 영역 확장(여백 색 재진입 금지)
    while let Some((lx, ly, d, bg_i)) = shadow_q.pop_front() {
        if d >= MAX_SHADOW_SPREAD {
            continue;
        }
        let bg = bgs[bg_i as usize];
        let p_px = px_bgr(&data.bgra, w, rx0 + lx, ry0 + ly);
        let neighbors = [
            (lx > x_start).then(|| (lx - 1, ly)),
            (lx + 1 < x_end).then(|| (lx + 1, ly)),
            (ly > y_start).then(|| (lx, ly - 1)),
            (ly + 1 < y_end).then(|| (lx, ly + 1)),
        ];
        for n in neighbors.into_iter().flatten() {
            if is_vis(&visited, n.0, n.1) {
                continue;
            }
            let q_px = px_bgr(&data.bgra, w, rx0 + n.0, ry0 + n.1);
            if bgr_matches(q_px, bg) {
                continue;
            }
            if is_shadow_pixel(q_px, bg) && shadow_step_ok(p_px, q_px, SHADOW_MAX_LUMA_DROP) {
                set_vis(&mut visited, n.0, n.1);
                shadow_q.push_back((n.0, n.1, d + 1, bg_i));
                filled += 1;
                if filled > FLAT_PIXEL_BUDGET {
                    return local_rect;
                }
            }
        }
    }

    // 미방문(전경 그림) 픽셀의 최소 바운딩 박스 계산(마진 제외한 내부 영역 기준)
    let mut min_x = x_end;
    let mut max_x = x_start;
    let mut min_y = y_end;
    let mut max_y = y_start;
    let mut any_fg = false;
    for ly in y_start..y_end {
        for lx in x_start..x_end {
            if !is_vis(&visited, lx, ly) {
                any_fg = true;
                min_x = min_x.min(lx);
                max_x = max_x.max(lx);
                min_y = min_y.min(ly);
                max_y = max_y.max(ly);
            }
        }
    }
    if !any_fg {
        return local_rect;
    }

    // 앞 창 경계 ±2 px 마진 때문에 안쪽으로 밀린 전경 변은 원래 잘린 경계로 복원
    if occluded[0] && min_y == y_start {
        min_y = 0;
    }
    if occluded[1] && max_y + 1 == y_end {
        max_y = bh - 1;
    }
    if occluded[2] && min_x == x_start {
        min_x = 0;
    }
    if occluded[3] && max_x + 1 == x_end {
        max_x = bw - 1;
    }

    let refined = PhysRect::from_edges(
        (rx0 + min_x) as i32,
        (ry0 + min_y) as i32,
        (rx0 + max_x + 1) as i32,
        (ry0 + max_y + 1) as i32,
    );
    if refined.w < MIN_REFINED_SIDE
        || refined.h < MIN_REFINED_SIDE
        || (refined.area() as f64) < (clipped.area() as f64) * MIN_REMAINING_AREA_RATIO
    {
        return local_rect;
    }
    refined
}
