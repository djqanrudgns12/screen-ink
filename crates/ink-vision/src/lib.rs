//! screen-ink V 근거(08 §4.3): 접근성 정보가 없는 앱(WebView2·게임·원격 데스크톱·PDF·한글·웹 조각)을 위해
//! 정지 화면(BGRA 물리 픽셀)에서 사각형 후보·경계 지지율·경계 맞춤·드래그 자석 경계를 순수 Rust로 계산한다.

pub mod diff_rect;
mod flat;
mod ladder;
mod prep;

use ink_domain::geometry::{PhysPoint, PhysRect};
use prep::PrepData;
use std::fmt;

/// 경사(밝기 차이) 문턱값 — 08 §4.3·§7 첫 값.
pub const GRAD_THRESHOLD: u8 = 24;
/// 곧은 경계 조각 최소 길이(물리 px).
pub const MIN_SEGMENT_LEN: i32 = 24;
/// 후보·경계 맞춤 최소 지지율(네 변 화소 중 경계 화소 비율).
pub const MIN_SUPPORT: f32 = 0.6;
/// 평탄 영역 채널별 허용 색차(±6).
pub const FLAT_TOLERANCE: u8 = 6;
/// 평탄 영역 탐색 최대 화소 예산.
pub const FLAT_PIXEL_BUDGET: usize = 300_000;
/// 사다리·후보 합침 IoU 문턱(R2-CAP-4).
pub const LADDER_MERGE_IOU: f64 = 0.90;
/// 앞 창 경계 제외 허용 오차(±2 px, R2-CAP-2).
pub const OCCLUDER_EDGE_TOL: i32 = 2;

/// V 후보 종류.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VKind {
    Box,
    Flat,
}

/// V가 찾은 사각형 후보(원본 이미지 기준 물리 픽셀).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VCandidate {
    pub rect: PhysRect,
    pub kind: VKind,
    pub support: f32,
}

/// 정지 화면 전처리 결과 — 커서 질의(`query`)·지지율(`support`)·경계 맞춤(`snap`)·자석 경계(`edges_near`)를 메모리에서 즉시 제공한다.
pub struct Prepared {
    data: PrepData,
}

impl fmt::Debug for Prepared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Prepared")
            .field("full", &(self.data.full_w, self.data.full_h))
            .field("roi_origin", &(self.data.ox, self.data.oy))
            .field("roi_size", &(self.data.rw, self.data.rh))
            .field("h_segs", &self.data.h_segs.len())
            .field("v_segs", &self.data.v_segs.len())
            .finish()
    }
}

/// 모니터 한 장(BGRA 위→아래, 물리 px) 전체를 전처리한다.
pub fn prepare(bgra: &[u8], w: u32, h: u32) -> Prepared {
    Prepared { data: prep::build_roi(bgra, w, h, 0, 0, w, h) }
}

/// 커서 `(cx, cy)` 주변 `±half` 영역만 먼저 전처리한다(08 §4.7).
/// 반환된 `Prepared`의 모든 메서드 입출력 좌표는 원본 이미지 기준 물리 px다.
pub fn prepare_roi(bgra: &[u8], w: u32, h: u32, cx: u32, cy: u32, half: u32) -> Prepared {
    let x0 = cx.saturating_sub(half).min(w);
    let y0 = cy.saturating_sub(half).min(h);
    let x1 = cx.saturating_add(half).min(w);
    let y1 = cy.saturating_add(half).min(h);
    let rw = x1.saturating_sub(x0);
    let rh = y1.saturating_sub(y0);
    Prepared { data: prep::build_roi(bgra, w, h, x0, y0, rw, rh) }
}

impl Prepared {
    #[inline]
    fn to_local_rect(&self, r: PhysRect) -> PhysRect {
        PhysRect::new(r.x - self.data.ox, r.y - self.data.oy, r.w, r.h)
    }

    #[inline]
    fn to_global_rect(&self, r: PhysRect) -> PhysRect {
        PhysRect::new(r.x + self.data.ox, r.y + self.data.oy, r.w, r.h)
    }

    /// 커서 `(x, y)`에서 V 후보(상자 사다리 작은 것→큰 것 + 평탄 영역)를 찾는다.
    /// 지지율 `< 0.6`이거나 커서를 담지 않는 후보는 버린다.
    pub fn query(&self, x: i32, y: i32) -> Vec<VCandidate> {
        self.query_with_occluders(x, y, &[])
    }

    /// 앞 창(`occluders`)의 경계 ±2 px에 붙은 V 선·후보를 버리고 사다리 IoU ≥ 0.90 겹침을 합쳐 찾는다(R2-CAP-2·4).
    pub fn query_with_occluders(&self, x: i32, y: i32, occluders: &[PhysRect]) -> Vec<VCandidate> {
        if self.data.is_empty() {
            return Vec::new();
        }
        let lx = x - self.data.ox;
        let ly = y - self.data.oy;
        if lx < 0 || ly < 0 || lx >= self.data.rw as i32 || ly >= self.data.rh as i32 {
            return Vec::new();
        }

        let local_occ: Vec<PhysRect> = occluders.iter().map(|&o| self.to_local_rect(o)).collect();
        let boxes = ladder::find_box_ladder_excluding(&self.data, lx, ly, &local_occ);
        let mut out: Vec<VCandidate> = Vec::with_capacity(boxes.len() + 1);
        let cursor = PhysPoint { x, y };

        for b in boxes {
            let g = self.to_global_rect(b.rect);
            if g.contains(cursor) && b.support >= MIN_SUPPORT {
                out.push(VCandidate { rect: g, kind: VKind::Box, support: b.support });
            }
        }

        if let Some(flat_local) = flat::find_flat_region(&self.data, lx, ly) {
            if local_occ.is_empty() || !ladder::rect_touches_occluders(flat_local, &local_occ, OCCLUDER_EDGE_TOL) {
                let g = self.to_global_rect(flat_local);
                let sup = self.support_with_occluders(g, occluders);
                if g.contains(cursor) && sup >= MIN_SUPPORT {
                    let dup = out.iter().any(|c| rect_iou(c.rect, g) >= LADDER_MERGE_IOU);
                    if !dup {
                        out.push(VCandidate { rect: g, kind: VKind::Flat, support: sup });
                    }
                }
            }
        }

        out.sort_by(|a, b| {
            a.rect
                .area()
                .cmp(&b.rect.area())
                .then_with(|| (a.kind == VKind::Flat).cmp(&(b.kind == VKind::Flat)))
        });

        let mut deduped: Vec<VCandidate> = Vec::with_capacity(out.len());
        for cand in out {
            if let Some(existing) = deduped.iter_mut().find(|c| rect_iou(c.rect, cand.rect) >= LADDER_MERGE_IOU) {
                let better = (cand.kind == VKind::Box && existing.kind == VKind::Flat)
                    || (cand.kind == existing.kind && cand.support > existing.support + 0.01);
                if better {
                    *existing = cand;
                }
            } else {
                deduped.push(cand);
            }
        }
        deduped
    }

    /// 네 변 화소 중 경계 화소 비율(누적합으로 O(1)).
    pub fn support(&self, r: PhysRect) -> f32 {
        self.support_with_occluders(r, &[])
    }

    /// 앞 창 경계(±2 px)에 붙은 변은 경계 가중치를 0으로 하여 지지율을 잰다(R2-CAP-2).
    pub fn support_with_occluders(&self, r: PhysRect, occluders: &[PhysRect]) -> f32 {
        if self.data.is_empty() || r.w <= 0 || r.h <= 0 {
            return 0.0;
        }
        let local = self.to_local_rect(r);
        let local_occ: Vec<PhysRect> = occluders.iter().map(|&o| self.to_local_rect(o)).collect();
        let (sup, _) = ladder::rect_support_local_excluding(&self.data, local, &local_occ);
        sup
    }

    /// 각 변을 `±tol` 안의 곧은 경계(그 변 지지율 `≥ 0.6`)로 붙인다(경계 맞춤 08 §4.4-3).
    pub fn snap(&self, r: PhysRect, tol: i32) -> PhysRect {
        self.snap_with_occluders(r, tol, &[])
    }

    /// 앞 창 경계(`occluders` ±2 px)에는 붙이지 않고 `±tol` 안의 곧은 경계로 맞춘다(R2-CAP-2).
    pub fn snap_with_occluders(&self, r: PhysRect, tol: i32, occluders: &[PhysRect]) -> PhysRect {
        if self.data.is_empty() || r.w <= 0 || r.h <= 0 || tol <= 0 {
            return r;
        }
        let local = self.to_local_rect(r);
        let local_occ: Vec<PhysRect> = occluders.iter().map(|&o| self.to_local_rect(o)).collect();
        let lx0 = local.x;
        let lx1 = local.right();
        let ly0 = local.y;
        let ly1 = local.bottom();

        let snap_axis = |orig: i32, span0: i32, span1: i32, horizontal: bool, is_min_edge: bool| -> i32 {
            let span = (span1 - span0).max(0);
            if span <= 0 {
                return orig;
            }
            let max_coord = if horizontal { self.data.rh } else { self.data.rw } as i32;
            let lo = (orig - tol).max(0);
            let hi = (orig + tol).min(max_coord);
            let mut best = orig;
            let mut best_sup = -1.0f32;
            let mut best_dist = i32::MAX;
            for c in lo..=hi {
                let touches_occ = !local_occ.is_empty()
                    && if horizontal {
                        ladder::h_line_touches_occluders(c, span0, span1, &local_occ, OCCLUDER_EDGE_TOL)
                    } else {
                        ladder::v_line_touches_occluders(c, span0, span1, &local_occ, OCCLUDER_EDGE_TOL)
                    };
                if touches_occ {
                    continue;
                }
                if !local_occ.is_empty() {
                    let expanded = if is_min_edge && c < orig {
                        Some(if horizontal {
                            PhysRect::from_edges(span0, c, span1, orig)
                        } else {
                            PhysRect::from_edges(c, span0, orig, span1)
                        })
                    } else if !is_min_edge && c > orig {
                        Some(if horizontal {
                            PhysRect::from_edges(span0, orig, span1, c)
                        } else {
                            PhysRect::from_edges(orig, span0, c, span1)
                        })
                    } else {
                        None
                    };
                    if expanded.is_some_and(|strip| rect_overlaps_occluders(strip, &local_occ)) {
                        continue;
                    }
                }
                let hits = if horizontal {
                    self.data.h_edge_count(c, span0, span1)
                } else {
                    self.data.v_edge_count(c, span0, span1)
                };
                let sup = hits as f32 / span as f32;
                if sup < MIN_SUPPORT {
                    continue;
                }
                let seg_ok = span < MIN_SEGMENT_LEN
                    || if horizontal {
                        has_h_seg_overlap(&self.data, c, span0, span1)
                    } else {
                        has_v_seg_overlap(&self.data, c, span0, span1)
                    };
                if !seg_ok {
                    continue;
                }
                let dist = (c - orig).abs();
                let better = sup > best_sup + 0.001 || ((sup - best_sup).abs() <= 0.001 && dist < best_dist);
                if better {
                    best_sup = sup;
                    best_dist = dist;
                    best = c;
                }
            }
            best
        };

        let mut nt = snap_axis(ly0, lx0, lx1, true, true);
        let mut nb = snap_axis(ly1, lx0, lx1, true, false);
        if nb <= nt {
            nt = ly0;
            nb = ly1;
        }
        let mut nl = snap_axis(lx0, nt, nb, false, true);
        let mut nr = snap_axis(lx1, nt, nb, false, false);
        if nr <= nl {
            nl = lx0;
            nr = lx1;
        }
        // 가로 변이 세로 보정 뒤 더 정확히 붙을 수 있으면 1회 다듬기
        if nl != lx0 || nr != lx1 {
            let rt = snap_axis(ly0, nl, nr, true, true);
            let rb = snap_axis(ly1, nl, nr, true, false);
            if rb > rt {
                nt = rt;
                nb = rb;
            }
        }

        self.to_global_rect(PhysRect::from_edges(nl, nt, nr, nb))
    }

    /// 이미지 요소(`r`, 가상 화면 물리 px) 안에서 여백·그림자를 걷어내고 실제 그림 가장자리를 다시 잰다(R2-CAP-3).
    pub fn refine_image(&self, r: PhysRect) -> PhysRect {
        self.refine_image_with_occluders(r, &[])
    }

    /// 앞 창(`occluders`)에 가려 잘린 변은 여백 찾기를 건너뛰고 나머지 변의 여백·그림자를 다시 잰다(R2-CAP-2·3).
    pub fn refine_image_with_occluders(&self, r: PhysRect, occluders: &[PhysRect]) -> PhysRect {
        if self.data.is_empty() || r.w <= 0 || r.h <= 0 {
            return r;
        }
        let local = self.to_local_rect(r);
        let local_occ: Vec<PhysRect> = occluders.iter().map(|&o| self.to_local_rect(o)).collect();
        let refined = flat::refine_image_bounds_with_occluders(&self.data, local, &local_occ);
        self.to_global_rect(refined)
    }

    /// `(x, y)`에서 반경 `radius` 안의 곧은 세로 경계 `x`들과 가로 경계 `y`들(정렬·중복 제거, 드래그 자석용).
    pub fn edges_near(&self, x: i32, y: i32, radius: i32) -> (Vec<i32>, Vec<i32>) {
        if self.data.is_empty() || radius < 0 {
            return (Vec::new(), Vec::new());
        }
        let lx = x - self.data.ox;
        let ly = y - self.data.oy;
        let x_lo = lx - radius;
        let x_hi = lx + radius;
        let y_lo = ly - radius;
        let y_hi = ly + radius;

        let mut xs = Vec::new();
        let vxs = &self.data.v_nonempty_xs;
        let v_start = vxs.partition_point(|&vx| vx < x_lo);
        for &vx in &vxs[v_start..] {
            if vx > x_hi {
                break;
            }
            if has_v_seg_overlap(&self.data, vx, y_lo, y_hi + 1) {
                xs.push(vx + self.data.ox);
            }
        }

        let mut ys = Vec::new();
        let hys = &self.data.h_nonempty_ys;
        let h_start = hys.partition_point(|&hy| hy < y_lo);
        for &hy in &hys[h_start..] {
            if hy > y_hi {
                break;
            }
            if has_h_seg_overlap(&self.data, hy, x_lo, x_hi + 1) {
                ys.push(hy + self.data.oy);
            }
        }

        (xs, ys)
    }
}

fn has_h_seg_overlap(data: &PrepData, ly: i32, lx0: i32, lx1: i32) -> bool {
    let segs = data.h_row_segs(ly);
    let idx = segs.partition_point(|s| (s.end as i32) <= lx0);
    segs.get(idx).is_some_and(|s| (s.start as i32) < lx1)
}

fn has_v_seg_overlap(data: &PrepData, lx: i32, ly0: i32, ly1: i32) -> bool {
    let segs = data.v_col_segs(lx);
    let idx = segs.partition_point(|s| (s.end as i32) <= ly0);
    segs.get(idx).is_some_and(|s| (s.start as i32) < ly1)
}

fn rect_iou(a: PhysRect, b: PhysRect) -> f64 {
    let inter = a.intersect(&b).map_or(0, |r| r.area()) as f64;
    let uni = (a.area() + b.area()) as f64 - inter;
    if uni <= 0.0 {
        0.0
    } else {
        inter / uni
    }
}

#[inline]
fn rect_overlaps_occluders(r: PhysRect, occluders: &[PhysRect]) -> bool {
    !r.is_empty() && occluders.iter().any(|o| r.intersect(o).is_some_and(|i| i.area() > 0))
}

#[cfg(test)]
mod tests;
