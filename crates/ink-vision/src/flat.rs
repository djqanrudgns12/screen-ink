//! 평탄 영역(08 §4.3): 커서 점 색과 채널별 ±6 이내로 이어진 영역을 30만 화소 예산 안에서 채워 바운딩 박스를 만든다.
//! 4K에서도 1 ms 안에 끝나도록 스캔라인 채우기 + 64×64 타일 비트셋(방문한 타일만 할당)을 쓴다.

use crate::prep::PrepData;
use crate::{FLAT_PIXEL_BUDGET, FLAT_TOLERANCE};
use ink_domain::geometry::PhysRect;

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
