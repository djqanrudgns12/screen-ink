//! 정지 화면 전처리(08 §4.3):
//! 밝기·경사 → 가로/세로 곧은 경계 조각(CSR 구간 목록) + 행·열 누적합(O(1) 경계 지지율).

use crate::{GRAD_THRESHOLD, MIN_SEGMENT_LEN};
use std::sync::Arc;

/// 곧은 경계 조각 안에서 허용하는 최대 끊김(1~2 px 교차선·안티에일리어싱 틈).
const MAX_SEG_GAP: u16 = 2;
/// 순수 채도(등휘도 색 대비) 경계 문턱 — 밝기 차이가 작아도 선명한 색 버튼 경계를 잡는다.
const CHROMA_EDGE_THRESHOLD: u8 = 40;
/// 1 px 안티에일리어싱 전환(평탄-중간-평탄) 최소 한쪽 단계 크기.
const AA_HALF_MIN: u8 = 10;
/// 안티에일리어싱 양옆 평탄 판정 상한(그림자·그라데이션 제외).
const AA_FLAT_MAX: u8 = 8;
/// 멀티스레드 대역 분할을 켤 최소 화소 수(1080p·4K급).
const PARALLEL_MIN_PIXELS: usize = 1_000_000;

/// 한 행(또는 한 열) 위의 곧은 경계 구간 `[start, end)`(ROI 내부 좌표).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Seg {
    pub start: u16,
    pub end: u16,
}

/// 전처리 결과(ROI 내부 좌표계, `ox`·`oy`로 원본 이미지 좌표와 연결).
pub(crate) struct PrepData {
    pub full_w: u32,
    pub full_h: u32,
    pub ox: i32,
    pub oy: i32,
    pub rw: u32,
    pub rh: u32,
    /// ROI 영역의 BGRA 픽셀(`rw * rh * 4` 바이트) — 평탄 영역 채우기용.
    pub bgra: Arc<Vec<u8>>,
    /// 가로 경계 행별 누적합: `(rh + 1) * (rw + 1)`.
    /// `h_prefix[ly * stride + lx]` = 균열 행 `ly`에서 `[0, lx)`의 가로 경계 화소 수.
    pub h_prefix: Vec<u16>,
    /// 세로 경계 열별 누적합: `(rh + 1) * (rw + 1)`.
    /// `v_prefix[ly * stride + lx]` = 균열 열 `lx`에서 `[0, ly)`의 세로 경계 화소 수.
    pub v_prefix: Vec<u16>,
    /// 행별 가로 곧은 경계 조각(CSR).
    pub h_segs: Vec<Seg>,
    pub h_row_offsets: Vec<u32>,
    /// 조각이 하나 이상 있는 균열 행 `ly`(오름차순).
    pub h_nonempty_ys: Vec<i32>,
    /// 열별 세로 곧은 경계 조각(CSR).
    pub v_segs: Vec<Seg>,
    pub v_col_offsets: Vec<u32>,
    /// 조각이 하나 이상 있는 균열 열 `lx`(오름차순).
    pub v_nonempty_xs: Vec<i32>,
}

impl PrepData {
    pub(crate) fn empty(full_w: u32, full_h: u32) -> Self {
        Self {
            full_w,
            full_h,
            ox: 0,
            oy: 0,
            rw: 0,
            rh: 0,
            bgra: Arc::default(),
            h_prefix: Vec::new(),
            v_prefix: Vec::new(),
            h_segs: Vec::new(),
            h_row_offsets: Vec::new(),
            h_nonempty_ys: Vec::new(),
            v_segs: Vec::new(),
            v_col_offsets: Vec::new(),
            v_nonempty_xs: Vec::new(),
        }
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.rw == 0 || self.rh == 0
    }

    #[inline]
    pub(crate) fn stride(&self) -> usize {
        self.rw as usize + 1
    }

    /// 균열 행 `ly`(ROI 좌표)에서 `[lx0, lx1)`의 가로 경계 화소 수(O(1)).
    #[inline]
    pub(crate) fn h_edge_count(&self, ly: i32, lx0: i32, lx1: i32) -> u32 {
        if self.is_empty() || ly < 0 || ly > self.rh as i32 {
            return 0;
        }
        let a = lx0.clamp(0, self.rw as i32) as usize;
        let b = lx1.clamp(0, self.rw as i32) as usize;
        if b <= a {
            return 0;
        }
        let base = ly as usize * self.stride();
        (self.h_prefix[base + b] - self.h_prefix[base + a]) as u32
    }

    /// 균열 열 `lx`(ROI 좌표)에서 `[ly0, ly1)`의 세로 경계 화소 수(O(1)).
    #[inline]
    pub(crate) fn v_edge_count(&self, lx: i32, ly0: i32, ly1: i32) -> u32 {
        if self.is_empty() || lx < 0 || lx > self.rw as i32 {
            return 0;
        }
        let a = ly0.clamp(0, self.rh as i32) as usize;
        let b = ly1.clamp(0, self.rh as i32) as usize;
        if b <= a {
            return 0;
        }
        let stride = self.stride();
        let col = lx as usize;
        (self.v_prefix[b * stride + col] - self.v_prefix[a * stride + col]) as u32
    }

    /// 균열 행 `ly`의 가로 곧은 경계 조각 슬라이스.
    #[inline]
    pub(crate) fn h_row_segs(&self, ly: i32) -> &[Seg] {
        if ly < 0 || ly > self.rh as i32 || self.h_row_offsets.is_empty() {
            return &[];
        }
        let idx = ly as usize;
        let s = self.h_row_offsets[idx] as usize;
        let e = self.h_row_offsets[idx + 1] as usize;
        &self.h_segs[s..e]
    }

    /// 균열 열 `lx`의 세로 곧은 경계 조각 슬라이스.
    #[inline]
    pub(crate) fn v_col_segs(&self, lx: i32) -> &[Seg] {
        if lx < 0 || lx > self.rw as i32 || self.v_col_offsets.is_empty() {
            return &[];
        }
        let idx = lx as usize;
        let s = self.v_col_offsets[idx] as usize;
        let e = self.v_col_offsets[idx + 1] as usize;
        &self.v_segs[s..e]
    }
}

#[derive(Clone, Copy)]
struct RunState {
    start: u16,
    end: u16,
    hits: u16,
}

impl RunState {
    const EMPTY: Self = Self { start: u16::MAX, end: 0, hits: 0 };

    #[inline]
    fn is_empty(self) -> bool {
        self.start == u16::MAX
    }

    #[inline]
    fn is_valid(self) -> bool {
        if self.is_empty() {
            return false;
        }
        let len = (self.end - self.start) as i32;
        len >= MIN_SEGMENT_LEN && (self.hits as u32) * 5 >= (len as u32) * 4
    }
}

#[inline]
fn fill_luma_row(row_bgra: &[u8], out_y: &mut [u8]) {
    for (dst, px) in out_y.iter_mut().zip(row_bgra.chunks_exact(4)) {
        *dst = ((29u32 * px[0] as u32 + 150u32 * px[1] as u32 + 77u32 * px[2] as u32) >> 8) as u8;
    }
}

#[inline]
fn chroma_edge_4(a: [u8; 4], b: [u8; 4]) -> bool {
    a[0].abs_diff(b[0]) >= CHROMA_EDGE_THRESHOLD
        || a[1].abs_diff(b[1]) >= CHROMA_EDGE_THRESHOLD
        || a[2].abs_diff(b[2]) >= CHROMA_EDGE_THRESHOLD
}

#[inline]
fn same_bgr(a: [u8; 4], b: [u8; 4]) -> bool {
    ((u32::from_le_bytes(a) ^ u32::from_le_bytes(b)) & 0x00FF_FFFF) == 0
}

/// 1 px 안티에일리어싱 전환 판정: `prev2 ≈ prev1`, `next1 ≈ next2`, `cur`이 단조 중간값.
#[inline]
fn is_aa_step(prev2: u8, prev1: u8, cur: u8, next1: u8, next2: u8) -> bool {
    let total = prev1.abs_diff(next1);
    if total < GRAD_THRESHOLD {
        return false;
    }
    if cur.abs_diff(next1) < AA_HALF_MIN {
        return false;
    }
    let mono = (prev1 < cur && cur < next1) || (prev1 > cur && cur > next1);
    mono && prev2.abs_diff(prev1) < AA_FLAT_MAX && next1.abs_diff(next2) < AA_FLAT_MAX
}

struct BandOut {
    h_segs: Vec<Seg>,
    h_row_counts: Vec<u32>,
    h_nonempty_ys: Vec<i32>,
    /// 대역 안에서 완전히 끝난 유효 세로 조각(경계와 무관).
    v_interior: Vec<(u16, Seg)>,
    /// 대역 시작 경계(`start <= y_start + MAX_SEG_GAP`)에 닿은 첫 조각.
    v_head: Vec<RunState>,
    /// 대역 끝 경계(`end + MAX_SEG_GAP >= y_end`)에 닿은 마지막 조각(`v_head`와 다를 때만).
    v_tail: Vec<RunState>,
}

/// 행 범위 `[y_start, y_end)`를 처리한다.
/// `h_band`: 길이 `(y_end - y_start) * stride` (`h_prefix[y_start..y_end]`).
/// `v_band`: 길이 `(y_end - y_start) * stride` (`v_prefix[(y_start + 1)..=(y_end)]`, 대역 내부 0 기준 누적합).
fn process_band(
    roi_bgra: &[u8],
    rw_u: usize,
    rh_u: usize,
    y_start: usize,
    y_end: usize,
    h_band: &mut [u16],
    v_band: &mut [u16],
) -> BandOut {
    let stride = rw_u + 1;
    let roi_row_bytes = rw_u * 4;
    let band_rows = y_end - y_start;

    let mut h_segs = Vec::with_capacity(256);
    let mut h_row_counts = vec![0u32; band_rows];
    let mut h_nonempty_ys = Vec::with_capacity(band_rows / 4);

    let mut v_interior: Vec<(u16, Seg)> = Vec::with_capacity(256);
    let mut v_head = vec![RunState::EMPTY; rw_u + 1];
    let mut v_tail = vec![RunState::EMPTY; rw_u + 1];
    let mut v_states = vec![RunState::EMPTY; rw_u + 1];
    let mut previous_edges = Vec::new();

    // 5행 슬라이딩 밝기 버퍼(y-2..=y+2 미리 채움)
    let mut luma_ring: [Vec<u8>; 5] = std::array::from_fn(|_| vec![0u8; rw_u]);
    let pre_lo = y_start.saturating_sub(2);
    let pre_hi = (y_start + 2).min(rh_u);
    for init_y in pre_lo..pre_hi {
        fill_luma_row(&roi_bgra[init_y * roi_row_bytes..(init_y + 1) * roi_row_bytes], &mut luma_ring[init_y % 5]);
    }

    let head_cutoff = (y_start as u16).saturating_add(MAX_SEG_GAP);
    let y_end_16 = y_end as u16;

    for local_y in 0..band_rows {
        let ly = y_start + local_y;
        if ly + 2 < rh_u {
            let ny = ly + 2;
            fill_luma_row(&roi_bgra[ny * roi_row_bytes..(ny + 1) * roi_row_bytes], &mut luma_ring[ny % 5]);
        }

        let cur_row = &roi_bgra[ly * roi_row_bytes..(ly + 1) * roi_row_bytes];
        let cur_luma = &luma_ring[ly % 5];

        // 같은 두 줄 사이에는 가로 경계가 없어 0으로 시작한 누적합을 그대로 쓴다.
        let repeated = ly > 0 && cur_row == &roi_bgra[(ly - 1) * roi_row_bytes..ly * roi_row_bytes];
        if ly >= 1 && !repeated {
            let h_start_idx = h_segs.len();
            let prev_row = &roi_bgra[(ly - 1) * roi_row_bytes..ly * roi_row_bytes];
            let prev_luma = &luma_ring[(ly - 1) % 5];
            let can_v_aa = ly >= 2 && ly + 2 < rh_u;
            let (luma_m2, luma_p1, luma_p2) = if can_v_aa {
                (&luma_ring[(ly - 2) % 5][..], &luma_ring[(ly + 1) % 5][..], &luma_ring[(ly + 2) % 5][..])
            } else {
                (&[][..], &[][..], &[][..])
            };

            let h_pref_row = &mut h_band[local_y * stride + 1..(local_y + 1) * stride];
            let mut acc: u16 = 0;
            let mut run = RunState::EMPTY;

            for (lx, ((c_px, p_px), dst_pref)) in cur_row
                .chunks_exact(4)
                .zip(prev_row.chunks_exact(4))
                .zip(h_pref_row.iter_mut())
                .enumerate()
            {
                let a = [c_px[0], c_px[1], c_px[2], c_px[3]];
                let b = [p_px[0], p_px[1], p_px[2], p_px[3]];
                let is_edge = if same_bgr(a, b) {
                    false
                } else {
                    let dy = cur_luma[lx].abs_diff(prev_luma[lx]);
                    if dy >= GRAD_THRESHOLD || chroma_edge_4(a, b) {
                        true
                    } else if can_v_aa && dy >= AA_HALF_MIN {
                        is_aa_step(luma_m2[lx], prev_luma[lx], cur_luma[lx], luma_p1[lx], luma_p2[lx])
                    } else {
                        false
                    }
                };

                if is_edge {
                    acc += 1;
                    let x16 = lx as u16;
                    if run.is_empty() {
                        run = RunState { start: x16, end: x16 + 1, hits: 1 };
                    } else if x16 - run.end <= MAX_SEG_GAP {
                        run.end = x16 + 1;
                        run.hits += 1;
                    } else {
                        if run.is_valid() {
                            h_segs.push(Seg { start: run.start, end: run.end });
                        }
                        run = RunState { start: x16, end: x16 + 1, hits: 1 };
                    }
                }
                *dst_pref = acc;
            }
            if run.is_valid() {
                h_segs.push(Seg { start: run.start, end: run.end });
            }
            let added = (h_segs.len() - h_start_idx) as u32;
            h_row_counts[local_y] = added;
            if added > 0 {
                h_nonempty_ys.push(ly as i32);
            }
        }

        // 2b. 세로 경계(lx >= 1): 대역 첫 행은 0 기준, 이후 행은 직전 행 누적
        let ly16 = ly as u16;
        // 세로 경계 판정에는 현재 줄만 쓰므로 같은 줄이면 앞줄의 경계 열만 한 칸씩 늘린다.
        // 대역 첫 줄에서는 앞 대역의 상태를 빌리지 않는다.
        if repeated && local_y > 0 {
            let (prev, cur) = v_band.split_at_mut(local_y * stride);
            let prefix = &mut cur[..stride];
            prefix.copy_from_slice(&prev[prev.len() - stride..]);
            for &x in &previous_edges {
                let state: &mut RunState = &mut v_states[x];
                state.end = ly16 + 1;
                state.hits += 1;
                prefix[x] += 1;
            }
            continue;
        }
        previous_edges.clear();
        let mut prev_px = [cur_row[0], cur_row[1], cur_row[2], cur_row[3]];
        let mut prev_y = cur_luma[0];

        if local_y == 0 {
            let v_pref_cur = &mut v_band[1..stride - 1];
            for (i, (c_px, dst_v)) in cur_row[4..].chunks_exact(4).zip(v_pref_cur.iter_mut()).enumerate() {
                let lx = i + 1;
                let a = [c_px[0], c_px[1], c_px[2], c_px[3]];
                let cy = cur_luma[lx];
                let is_edge = if same_bgr(a, prev_px) {
                    false
                } else {
                    let dx = cy.abs_diff(prev_y);
                    if dx >= GRAD_THRESHOLD || chroma_edge_4(a, prev_px) {
                        true
                    } else if dx >= AA_HALF_MIN && lx >= 2 && lx + 2 < rw_u {
                        is_aa_step(cur_luma[lx - 2], prev_y, cy, cur_luma[lx + 1], cur_luma[lx + 2])
                    } else {
                        false
                    }
                };
                prev_px = a;
                prev_y = cy;

                if is_edge {
                    previous_edges.push(lx);
                    *dst_v = 1;
                    v_states[lx] = RunState { start: ly16, end: ly16 + 1, hits: 1 };
                } else {
                    *dst_v = 0;
                }
            }
        } else {
            let (v_prev_all, v_cur_all) = v_band.split_at_mut(local_y * stride);
            let v_pref_prev = &v_prev_all[(local_y - 1) * stride + 1..local_y * stride - 1];
            let v_pref_cur = &mut v_cur_all[1..stride - 1];

            for (i, ((c_px, &pv), dst_v)) in cur_row[4..]
                .chunks_exact(4)
                .zip(v_pref_prev.iter())
                .zip(v_pref_cur.iter_mut())
                .enumerate()
            {
                let lx = i + 1;
                let a = [c_px[0], c_px[1], c_px[2], c_px[3]];
                let cy = cur_luma[lx];
                let is_edge = if same_bgr(a, prev_px) {
                    false
                } else {
                    let dx = cy.abs_diff(prev_y);
                    if dx >= GRAD_THRESHOLD || chroma_edge_4(a, prev_px) {
                        true
                    } else if dx >= AA_HALF_MIN && lx >= 2 && lx + 2 < rw_u {
                        is_aa_step(cur_luma[lx - 2], prev_y, cy, cur_luma[lx + 1], cur_luma[lx + 2])
                    } else {
                        false
                    }
                };
                prev_px = a;
                prev_y = cy;

                if is_edge {
                    previous_edges.push(lx);
                    *dst_v = pv + 1;
                    let st = &mut v_states[lx];
                    if st.is_empty() {
                        *st = RunState { start: ly16, end: ly16 + 1, hits: 1 };
                    } else if ly16 - st.end <= MAX_SEG_GAP {
                        st.end = ly16 + 1;
                        st.hits += 1;
                    } else {
                        let finished = *st;
                        if finished.start <= head_cutoff && v_head[lx].is_empty() {
                            v_head[lx] = finished;
                        } else if finished.is_valid() {
                            v_interior.push((lx as u16, Seg { start: finished.start, end: finished.end }));
                        }
                        *st = RunState { start: ly16, end: ly16 + 1, hits: 1 };
                    }
                } else {
                    *dst_v = pv;
                }
            }
        }
    }

    // 대역 끝에 남은 세로 run 분류(head / tail / interior)
    for lx in 1..rw_u {
        let st = v_states[lx];
        if st.is_empty() {
            continue;
        }
        let touches_head = st.start <= head_cutoff && v_head[lx].is_empty();
        let touches_tail = st.end.saturating_add(MAX_SEG_GAP) >= y_end_16;
        if touches_head {
            v_head[lx] = st;
        } else if touches_tail {
            v_tail[lx] = st;
        } else if st.is_valid() {
            v_interior.push((lx as u16, Seg { start: st.start, end: st.end }));
        }
    }

    BandOut { h_segs, h_row_counts, h_nonempty_ys, v_interior, v_head, v_tail }
}

/// ROI `[x0, x0+rw) × [y0, y0+rh)` 전처리 실행(큰 화면은 자동으로 멀티스레드 대역 분할).
pub(crate) fn build_roi(bgra: &[u8], w: u32, h: u32, x0: u32, y0: u32, rw: u32, rh: u32) -> PrepData {
    build_roi_with_threads(bgra, (w, h), (x0, y0), (rw, rh), band_threads(rw, rh))
}

fn band_threads(rw: u32, rh: u32) -> usize {
    if (rw as usize).saturating_mul(rh as usize) >= PARALLEL_MIN_PIXELS && rh >= 256 {
        std::thread::available_parallelism().map_or(1, |n| n.get()).min(4)
    } else {
        1
    }
}

pub(crate) fn build_shared(bgra: Arc<Vec<u8>>, w: u32, h: u32) -> PrepData {
    if !valid_pixels(&bgra, (w, h), (w, h)) {
        return PrepData::empty(w, h);
    }
    analyze_pixels(bgra, (w, h), (0, 0), (w, h), band_threads(w, h))
}

fn valid_pixels(bgra: &[u8], (w, h): (u32, u32), (rw, rh): (u32, u32)) -> bool {
    let max_side = u16::MAX as u32 - 2;
    (1..=max_side).contains(&rw) && (1..=max_side).contains(&rh)
        && (w as usize).checked_mul(h as usize).and_then(|n| n.checked_mul(4)).is_some_and(|n| n <= bgra.len())
}

pub(crate) fn build_roi_with_threads(
    bgra: &[u8],
    full: (u32, u32),
    origin: (u32, u32),
    roi: (u32, u32),
    requested_threads: usize,
) -> PrepData {
    let (w, h) = full;
    let (x0, y0) = origin;
    let (rw, rh) = roi;
    if !valid_pixels(bgra, full, roi) {
        return PrepData::empty(w, h);
    }

    let rw_u = rw as usize;
    let rh_u = rh as usize;
    let roi_row_bytes = rw_u * 4;

    // 1. ROI 픽셀 복사
    let roi_bgra = if x0 == 0 && y0 == 0 && rw == w && rh == h {
        bgra[..roi_row_bytes * rh_u].to_vec()
    } else {
        let mut buf = Vec::with_capacity(roi_row_bytes * rh_u);
        let full_row_bytes = w as usize * 4;
        let x_byte = x0 as usize * 4;
        for ly in 0..rh_u {
            let src_start = (y0 as usize + ly) * full_row_bytes + x_byte;
            buf.extend_from_slice(&bgra[src_start..src_start + roi_row_bytes]);
        }
        buf
    };

    analyze_pixels(Arc::new(roi_bgra), full, origin, roi, requested_threads)
}

fn analyze_pixels(
    roi_bgra: Arc<Vec<u8>>,
    (w, h): (u32, u32),
    (x0, y0): (u32, u32),
    (rw, rh): (u32, u32),
    requested_threads: usize,
) -> PrepData {
    let (rw_u, rh_u) = (rw as usize, rh as usize);
    let stride = rw_u + 1;
    let prefix_len = stride * (rh_u + 1);
    let mut h_prefix = vec![0u16; prefix_len];
    let mut v_prefix = vec![0u16; prefix_len];

    let n_bands = requested_threads.clamp(1, (rh_u / 32).max(1));
    let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(n_bands);
    for b in 0..n_bands {
        let ys = (b * rh_u) / n_bands;
        let ye = ((b + 1) * rh_u) / n_bands;
        if ye > ys {
            ranges.push((ys, ye));
        }
    }

    // 2. 대역별 병렬(또는 단일 스레드) 전처리
    let bands: Vec<BandOut> = if ranges.len() <= 1 {
        let h_slice = &mut h_prefix[..rh_u * stride];
        let v_slice = &mut v_prefix[stride..(rh_u + 1) * stride];
        vec![process_band(&roi_bgra, rw_u, rh_u, 0, rh_u, h_slice, v_slice)]
    } else {
        let mut h_rest = &mut h_prefix[..rh_u * stride];
        let mut v_rest = &mut v_prefix[stride..(rh_u + 1) * stride];
        let mut h_slices = Vec::with_capacity(ranges.len());
        let mut v_slices = Vec::with_capacity(ranges.len());
        for &(ys, ye) in &ranges {
            let len = (ye - ys) * stride;
            let (hl, hr) = h_rest.split_at_mut(len);
            let (vl, vr) = v_rest.split_at_mut(len);
            h_slices.push(hl);
            v_slices.push(vl);
            h_rest = hr;
            v_rest = vr;
        }
        std::thread::scope(|s| {
            let handles: Vec<_> = ranges
                .iter()
                .zip(h_slices)
                .zip(v_slices)
                .map(|((&(ys, ye), hs), vs)| {
                    let bgra_ref = &roi_bgra;
                    s.spawn(move || process_band(bgra_ref, rw_u, rh_u, ys, ye, hs, vs))
                })
                .collect();
            handles.into_iter().map(|h| h.join().expect("ink-vision band worker")).collect()
        })
    };

    // 3. 멀티 대역일 때 v_prefix 누적합 오프셋 보정(병렬)
    if ranges.len() > 1 {
        let mut bases: Vec<Vec<u16>> = Vec::with_capacity(ranges.len());
        bases.push(vec![0u16; stride]);
        for k in 1..ranges.len() {
            let prev_end = ranges[k - 1].1;
            let prev_last_row = &v_prefix[prev_end * stride..(prev_end + 1) * stride];
            let mut next_base = vec![0u16; stride];
            let prev_base = &bases[k - 1];
            for x in 0..stride {
                next_base[x] = prev_base[x] + prev_last_row[x];
            }
            bases.push(next_base);
        }
        let first_end = ranges[0].1;
        let mut v_rest = &mut v_prefix[(first_end + 1) * stride..(rh_u + 1) * stride];
        let mut tail_slices = Vec::with_capacity(ranges.len() - 1);
        for &(ys, ye) in &ranges[1..] {
            let len = (ye - ys) * stride;
            let (vl, vr) = v_rest.split_at_mut(len);
            tail_slices.push(vl);
            v_rest = vr;
        }
        std::thread::scope(|s| {
            for (slice, base) in tail_slices.into_iter().zip(bases.iter().skip(1)) {
                s.spawn(move || {
                    for row in slice.chunks_exact_mut(stride) {
                        for (dst, &b) in row.iter_mut().zip(base.iter()) {
                            *dst += b;
                        }
                    }
                });
            }
        });
    }

    // 4. 가로 조각 CSR 결합
    let total_h: usize = bands.iter().map(|b| b.h_segs.len()).sum();
    let mut h_segs = Vec::with_capacity(total_h);
    let mut h_row_offsets = vec![0u32; rh_u + 2];
    let mut h_nonempty_ys = Vec::with_capacity(bands.iter().map(|b| b.h_nonempty_ys.len()).sum());
    let mut h_cursor: u32 = 0;
    for (band, &(ys, _)) in bands.iter().zip(&ranges) {
        h_segs.extend_from_slice(&band.h_segs);
        h_nonempty_ys.extend_from_slice(&band.h_nonempty_ys);
        for (local_y, &cnt) in band.h_row_counts.iter().enumerate() {
            h_row_offsets[ys + local_y] = h_cursor;
            h_cursor += cnt;
        }
    }
    h_row_offsets[rh_u] = h_cursor;
    h_row_offsets[rh_u + 1] = h_cursor;

    // 5. 세로 조각 경계 접합 + 열별 CSR 정렬(열 안에서는 start 오름차순 보장)
    let mut v_boundary_valid: Vec<(u16, Seg)> = Vec::new();
    for lx in 1..rw_u {
        let mut carry = RunState::EMPTY;
        for band in &bands {
            for run in [band.v_head[lx], band.v_tail[lx]] {
                if run.is_empty() {
                    continue;
                }
                if carry.is_empty() {
                    carry = run;
                } else if run.start.saturating_sub(carry.end) <= MAX_SEG_GAP {
                    carry.end = run.end;
                    carry.hits += run.hits;
                } else {
                    if carry.is_valid() {
                        v_boundary_valid.push((lx as u16, Seg { start: carry.start, end: carry.end }));
                    }
                    carry = run;
                }
            }
        }
        if carry.is_valid() {
            v_boundary_valid.push((lx as u16, Seg { start: carry.start, end: carry.end }));
        }
    }

    let mut v_counts = vec![0u32; rw_u + 1];
    for band in &bands {
        for &(lx, _) in &band.v_interior {
            v_counts[lx as usize] += 1;
        }
    }
    for &(lx, _) in &v_boundary_valid {
        v_counts[lx as usize] += 1;
    }

    let mut v_col_offsets = vec![0u32; rw_u + 2];
    let mut v_nonempty_xs: Vec<i32> = Vec::with_capacity(rw_u / 4);
    let mut running: u32 = 0;
    for lx in 0..=rw_u {
        v_col_offsets[lx] = running;
        let c = v_counts[lx];
        if c > 0 {
            v_nonempty_xs.push(lx as i32);
            running += c;
        }
    }
    v_col_offsets[rw_u + 1] = running;

    let mut v_segs = vec![Seg { start: 0, end: 0 }; running as usize];
    let mut write_pos = v_col_offsets[..=rw_u].to_vec();
    for band in &bands {
        for &(lx, seg) in &band.v_interior {
            let p = &mut write_pos[lx as usize];
            v_segs[*p as usize] = seg;
            *p += 1;
        }
    }
    for (lx, seg) in v_boundary_valid {
        let p = &mut write_pos[lx as usize];
        v_segs[*p as usize] = seg;
        *p += 1;
    }
    // 경계 조각이 interior 조각과 섞였을 수 있는 열만 정렬(열당 조각 수가 아주 작음)
    if ranges.len() > 1 {
        for &lx in &v_nonempty_xs {
            let s = v_col_offsets[lx as usize] as usize;
            let e = v_col_offsets[lx as usize + 1] as usize;
            if e - s > 1 {
                v_segs[s..e].sort_unstable_by_key(|seg| seg.start);
            }
        }
    }

    PrepData {
        full_w: w,
        full_h: h,
        ox: x0 as i32,
        oy: y0 as i32,
        rw,
        rh,
        bgra: roi_bgra,
        h_prefix,
        v_prefix,
        h_segs,
        h_row_offsets,
        h_nonempty_ys,
        v_segs,
        v_col_offsets,
        v_nonempty_xs,
    }
}
