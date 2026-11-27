//! 상자 사다리(08 §4.3): 커서를 담는 중첩 사각형 3~5개(작은 것 → 큰 것)를 찾는다.
//! 표(칸 → 행 → 표), 카드·버튼·입력 칸, 둥근 모서리 상자를 모두 4 ms 안에 처리한다.

use crate::prep::{PrepData, Seg};
use crate::MIN_SUPPORT;
use ink_domain::geometry::{PhysPoint, PhysRect};

/// 방향별로 수집할 최대 곧은 경계 후보 수.
const MAX_DIR_EDGES: usize = 16;
/// 4방향 직접 조합에 쓸 가까운 경계 수.
const MAX_GRID_EDGES: usize = 6;
/// 둥근 모서리 반지름 탐색 범위(px).
const CORNER_SEARCH: i32 = 16;
/// 각 변의 최소 지지율(열린 띠 방지, 둥근 모서리 허용).
const MIN_SIDE_SUPPORT: f32 = 0.35;
/// 사다리에 남길 최대 상자 수.
const MAX_LADDER_BOXES: usize = 5;
/// 사다리·후보 겹침 정리 IoU 문턱(R2-CAP-4).
pub(crate) const DEDUP_IOU: f64 = 0.90;
/// 앞 창 경계 제외 허용 오차(±2 px, R2-CAP-2).
pub(crate) const OCCLUDER_EDGE_TOL: i32 = 2;

/// 가로선 `[x0, x1)`이 앞 창(`occluders`)의 위·아래 모서리 `±tol`에 붙는지 판정한다.
#[inline]
pub(crate) fn h_line_touches_occluders(y: i32, x0: i32, x1: i32, occluders: &[PhysRect], tol: i32) -> bool {
    occluders.iter().any(|o| {
        ((y - o.y).abs() <= tol || (y - o.bottom()).abs() <= tol) && x1 > o.x && x0 < o.right()
    })
}

/// 세로선 `[y0, y1)`이 앞 창(`occluders`)의 왼쪽·오른쪽 모서리 `±tol`에 붙는지 판정한다.
#[inline]
pub(crate) fn v_line_touches_occluders(x: i32, y0: i32, y1: i32, occluders: &[PhysRect], tol: i32) -> bool {
    occluders.iter().any(|o| {
        ((x - o.x).abs() <= tol || (x - o.right()).abs() <= tol) && y1 > o.y && y0 < o.bottom()
    })
}

/// 사각형이 앞 창 영역과 겹치거나 네 변 중 하나가 앞 창 모서리 `±tol`에 붙는지 판정한다(R2-CAP-2).
pub(crate) fn rect_touches_occluders(r: PhysRect, occluders: &[PhysRect], tol: i32) -> bool {
    occluders.iter().any(|o| r.intersect(o).is_some_and(|i| i.area() > 0))
        || h_line_touches_occluders(r.y, r.x, r.right(), occluders, tol)
        || h_line_touches_occluders(r.bottom(), r.x, r.right(), occluders, tol)
        || v_line_touches_occluders(r.x, r.y, r.bottom(), occluders, tol)
        || v_line_touches_occluders(r.right(), r.y, r.bottom(), occluders, tol)
}

#[derive(Clone, Copy)]
struct HEdge {
    y: i32,
    start: i32,
    end: i32,
}

#[derive(Clone, Copy)]
struct VEdge {
    x: i32,
    start: i32,
    end: i32,
}

#[derive(Clone, Copy)]
pub(crate) struct BoxHit {
    pub rect: PhysRect,
    pub support: f32,
    closure: u8,
    h_closed: bool,
}

/// ROI 좌표 사각형의 경계 지지율(전체 둘레 비율 + 네 변 중 최솟값).
#[allow(dead_code)]
#[inline]
pub(crate) fn rect_support_local(data: &PrepData, r: PhysRect) -> (f32, f32) {
    rect_support_local_excluding(data, r, &[])
}

/// 앞 창 경계(`occluders` ±2 px)에 붙은 변의 경계 가중치를 0으로 하여 지지율을 잰다(R2-CAP-2).
#[inline]
pub(crate) fn rect_support_local_excluding(data: &PrepData, r: PhysRect, occluders: &[PhysRect]) -> (f32, f32) {
    if r.w <= 0 || r.h <= 0 {
        return (0.0, 0.0);
    }
    let lx0 = r.x;
    let lx1 = r.right();
    let ly0 = r.y;
    let ly1 = r.bottom();

    let top = if h_line_touches_occluders(ly0, lx0, lx1, occluders, OCCLUDER_EDGE_TOL) {
        0
    } else {
        data.h_edge_count(ly0, lx0, lx1)
    };
    let bot = if h_line_touches_occluders(ly1, lx0, lx1, occluders, OCCLUDER_EDGE_TOL) {
        0
    } else {
        data.h_edge_count(ly1, lx0, lx1)
    };
    let left = if v_line_touches_occluders(lx0, ly0, ly1, occluders, OCCLUDER_EDGE_TOL) {
        0
    } else {
        data.v_edge_count(lx0, ly0, ly1)
    };
    let right = if v_line_touches_occluders(lx1, ly0, ly1, occluders, OCCLUDER_EDGE_TOL) {
        0
    } else {
        data.v_edge_count(lx1, ly0, ly1)
    };

    let wf = r.w as f32;
    let hf = r.h as f32;
    let perim = 2.0 * (wf + hf);
    let total = (top + bot + left + right) as f32 / perim;
    let min_side = (top as f32 / wf)
        .min(bot as f32 / wf)
        .min(left as f32 / hf)
        .min(right as f32 / hf);
    (total, min_side)
}

#[inline]
fn seg_covering(segs: &[Seg], coord: i32, slack: i32) -> Option<Seg> {
    let idx = segs.partition_point(|s| (s.end as i32) + slack <= coord);
    let s = *segs.get(idx)?;
    if (s.start as i32) - slack <= coord && coord < (s.end as i32) + slack {
        Some(s)
    } else {
        None
    }
}

/// 1 px 선의 안쪽·바깥쪽 이중 균열(거리 1 px, 구간 겹침 ≥ 80%)을 바깥쪽으로 합친다.
fn push_coalesced_h(out: &mut Vec<HEdge>, cand: HEdge, prefer_outer_smaller: bool) {
    if let Some(last) = out.last_mut() {
        let dist = (last.y - cand.y).abs();
        if dist <= 1 {
            let inter = (last.end.min(cand.end) - last.start.max(cand.start)).max(0);
            let min_len = (last.end - last.start).min(cand.end - cand.start).max(1);
            if inter * 5 >= min_len * 4 {
                // 더 바깥쪽(커서에서 먼 쪽)을 선택
                let replace = if prefer_outer_smaller { cand.y < last.y } else { cand.y > last.y };
                if replace {
                    *last = cand;
                }
                return;
            }
        }
    }
    out.push(cand);
}

fn push_coalesced_v(out: &mut Vec<VEdge>, cand: VEdge, prefer_outer_smaller: bool) {
    if let Some(last) = out.last_mut() {
        let dist = (last.x - cand.x).abs();
        if dist <= 1 {
            let inter = (last.end.min(cand.end) - last.start.max(cand.start)).max(0);
            let min_len = (last.end - last.start).min(cand.end - cand.start).max(1);
            if inter * 5 >= min_len * 4 {
                let replace = if prefer_outer_smaller { cand.x < last.x } else { cand.x > last.x };
                if replace {
                    *last = cand;
                }
                return;
            }
        }
    }
    out.push(cand);
}

fn collect_h_above(data: &PrepData, x: i32, y: i32, limit: usize, slack: i32, occluders: &[PhysRect]) -> Vec<HEdge> {
    let ys = &data.h_nonempty_ys;
    let pos = ys.partition_point(|&hy| hy <= y);
    let mut out = Vec::with_capacity(limit);
    for &hy in ys[..pos].iter().rev() {
        if let Some(seg) = seg_covering(data.h_row_segs(hy), x, slack) {
            if h_line_touches_occluders(hy, seg.start as i32, seg.end as i32, occluders, OCCLUDER_EDGE_TOL) {
                continue;
            }
            push_coalesced_h(&mut out, HEdge { y: hy, start: seg.start as i32, end: seg.end as i32 }, true);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn collect_h_below(data: &PrepData, x: i32, y: i32, limit: usize, slack: i32, occluders: &[PhysRect]) -> Vec<HEdge> {
    let ys = &data.h_nonempty_ys;
    let pos = ys.partition_point(|&hy| hy <= y);
    let mut out = Vec::with_capacity(limit);
    for &hy in &ys[pos..] {
        if let Some(seg) = seg_covering(data.h_row_segs(hy), x, slack) {
            if h_line_touches_occluders(hy, seg.start as i32, seg.end as i32, occluders, OCCLUDER_EDGE_TOL) {
                continue;
            }
            push_coalesced_h(&mut out, HEdge { y: hy, start: seg.start as i32, end: seg.end as i32 }, false);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn collect_v_left(data: &PrepData, x: i32, y: i32, limit: usize, slack: i32, occluders: &[PhysRect]) -> Vec<VEdge> {
    let xs = &data.v_nonempty_xs;
    let pos = xs.partition_point(|&vx| vx <= x);
    let mut out = Vec::with_capacity(limit);
    for &vx in xs[..pos].iter().rev() {
        if let Some(seg) = seg_covering(data.v_col_segs(vx), y, slack) {
            if v_line_touches_occluders(vx, seg.start as i32, seg.end as i32, occluders, OCCLUDER_EDGE_TOL) {
                continue;
            }
            push_coalesced_v(&mut out, VEdge { x: vx, start: seg.start as i32, end: seg.end as i32 }, true);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn collect_v_right(data: &PrepData, x: i32, y: i32, limit: usize, slack: i32, occluders: &[PhysRect]) -> Vec<VEdge> {
    let xs = &data.v_nonempty_xs;
    let pos = xs.partition_point(|&vx| vx <= x);
    let mut out = Vec::with_capacity(limit);
    for &vx in &xs[pos..] {
        if let Some(seg) = seg_covering(data.v_col_segs(vx), y, slack) {
            if v_line_touches_occluders(vx, seg.start as i32, seg.end as i32, occluders, OCCLUDER_EDGE_TOL) {
                continue;
            }
            push_coalesced_v(&mut out, VEdge { x: vx, start: seg.start as i32, end: seg.end as i32 }, false);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

/// `[lo, hi]` 범위에서 `ly0..ly1` 구간 세로 경계 화소가 가장 많은 열을 찾는다(동점이면 바깥쪽, 앞 창 경계 제외).
fn best_v_in_range(
    data: &PrepData,
    lo: i32,
    hi: i32,
    ly0: i32,
    ly1: i32,
    prefer_smaller: bool,
    occluders: &[PhysRect],
) -> Option<(i32, u32)> {
    let a = lo.max(0);
    let b = hi.min(data.rw as i32);
    if a > b || ly1 <= ly0 {
        return None;
    }
    let mut best_x = a;
    let mut best_c = 0u32;
    for lx in a..=b {
        if v_line_touches_occluders(lx, ly0, ly1, occluders, OCCLUDER_EDGE_TOL) {
            continue;
        }
        let c = data.v_edge_count(lx, ly0, ly1);
        let better = c > best_c || (c == best_c && c > 0 && (if prefer_smaller { lx < best_x } else { lx > best_x }));
        if better {
            best_c = c;
            best_x = lx;
        }
    }
    (best_c > 0).then_some((best_x, best_c))
}

/// `[lo, hi]` 범위에서 `lx0..lx1` 구간 가로 경계 화소가 가장 많은 행을 찾는다(동점이면 바깥쪽, 앞 창 경계 제외).
fn best_h_in_range(
    data: &PrepData,
    lo: i32,
    hi: i32,
    lx0: i32,
    lx1: i32,
    prefer_smaller: bool,
    occluders: &[PhysRect],
) -> Option<(i32, u32)> {
    let a = lo.max(0);
    let b = hi.min(data.rh as i32);
    if a > b || lx1 <= lx0 {
        return None;
    }
    let mut best_y = a;
    let mut best_c = 0u32;
    for ly in a..=b {
        if h_line_touches_occluders(ly, lx0, lx1, occluders, OCCLUDER_EDGE_TOL) {
            continue;
        }
        let c = data.h_edge_count(ly, lx0, lx1);
        let better = c > best_c || (c == best_c && c > 0 && (if prefer_smaller { ly < best_y } else { ly > best_y }));
        if better {
            best_c = c;
            best_y = ly;
        }
    }
    (best_c > 0).then_some((best_y, best_c))
}

/// 사각형 네 변이 바깥으로 이어지지 않고 꼭짓점에서 닫히는 정도(0..=4)와 가로 닫힘 여부.
fn closure_score(data: &PrepData, r: PhysRect) -> (u8, bool) {
    let lx0 = r.x;
    let lx1 = r.right();
    let ly0 = r.y;
    let ly1 = r.bottom();
    let probe = 12;
    let gap = 2;

    let h_closed_at = |ly: i32| -> bool {
        let left_ext = data.h_edge_count(ly, lx0 - gap - probe, lx0 - gap);
        let right_ext = data.h_edge_count(ly, lx1 + gap, lx1 + gap + probe);
        left_ext < 6 && right_ext < 6
    };
    let v_closed_at = |lx: i32| -> bool {
        let top_ext = data.v_edge_count(lx, ly0 - gap - probe, ly0 - gap);
        let bot_ext = data.v_edge_count(lx, ly1 + gap, ly1 + gap + probe);
        top_ext < 6 && bot_ext < 6
    };

    let t_c = h_closed_at(ly0);
    let b_c = h_closed_at(ly1);
    let l_c = v_closed_at(lx0);
    let r_c = v_closed_at(lx1);
    let score = (t_c as u8) + (b_c as u8) + (l_c as u8) + (r_c as u8);
    (score, t_c && b_c)
}

fn iou_rect(a: PhysRect, b: PhysRect) -> f64 {
    let inter = a.intersect(&b).map_or(0, |r| r.area()) as f64;
    let uni = (a.area() + b.area()) as f64 - inter;
    if uni <= 0.0 {
        0.0
    } else {
        inter / uni
    }
}

fn near_same(a: PhysRect, b: PhysRect) -> bool {
    (a.x - b.x).abs() <= 2 && (a.y - b.y).abs() <= 2 && (a.right() - b.right()).abs() <= 2 && (a.bottom() - b.bottom()).abs() <= 2
}

fn encloses(outer: PhysRect, inner: PhysRect) -> bool {
    outer.x <= inner.x + 2
        && outer.y <= inner.y + 2
        && outer.right() >= inner.right() - 2
        && outer.bottom() >= inner.bottom() - 2
        && outer.area() > inner.area()
}

/// 사다리 후보 목록에서 `IoU ≥ DEDUP_IOU(0.90)` 또는 ±2 px 겹침을 합쳐 중복을 없앤다(R2-CAP-4).
pub(crate) fn dedup_hits(hits: Vec<BoxHit>) -> Vec<BoxHit> {
    let mut out: Vec<BoxHit> = Vec::with_capacity(hits.len());
    for h in hits {
        if let Some(existing) = out.iter_mut().find(|c| near_same(c.rect, h.rect) || iou_rect(c.rect, h.rect) >= DEDUP_IOU) {
            let better = h.closure > existing.closure
                || (h.closure == existing.closure && h.support > existing.support + 0.01)
                || (h.closure == existing.closure
                    && (h.support - existing.support).abs() <= 0.01
                    && h.rect.area() > existing.rect.area());
            if better {
                *existing = h;
            }
        } else {
            out.push(h);
        }
    }
    out.sort_by_key(|c| c.rect.area());
    out
}

/// ROI 좌표 `(lx, ly)`에서 상자 사다리(작은 것 → 큰 것, 최대 5개)를 찾는다.
#[allow(dead_code)]
pub(crate) fn find_box_ladder(data: &PrepData, lx: i32, ly: i32) -> Vec<BoxHit> {
    find_box_ladder_excluding(data, lx, ly, &[])
}

/// 앞 창 경계(`occluders` ±2 px)를 제외하고 상자 사다리를 찾는다(R2-CAP-2·4).
pub(crate) fn find_box_ladder_excluding(
    data: &PrepData,
    lx: i32,
    ly: i32,
    occluders: &[PhysRect],
) -> Vec<BoxHit> {
    if data.is_empty() || lx < 0 || ly < 0 || lx >= data.rw as i32 || ly >= data.rh as i32 {
        return Vec::new();
    }
    let pt = PhysPoint { x: lx, y: ly };

    let mut tops = collect_h_above(data, lx, ly, MAX_DIR_EDGES, 0, occluders);
    let mut bots = collect_h_below(data, lx, ly, MAX_DIR_EDGES, 0, occluders);
    let mut lefts = collect_v_left(data, lx, ly, MAX_DIR_EDGES, 0, occluders);
    let mut rights = collect_v_right(data, lx, ly, MAX_DIR_EDGES, 0, occluders);

    // 둥근 모서리 근처 보강: 커서가 모서리 굴곡 바로 옆에 있을 때 반대 축 중앙에서 경계를 보충
    if let (Some(l), Some(r)) = (lefts.first().copied(), rights.first().copied()) {
        let mid_x = (l.x + r.x) / 2;
        if mid_x != lx {
            for e in collect_h_above(data, mid_x, ly, 4, 0, occluders) {
                if e.start - CORNER_SEARCH <= lx && lx < e.end + CORNER_SEARCH && !tops.iter().any(|t| (t.y - e.y).abs() <= 1) {
                    tops.push(e);
                }
            }
            for e in collect_h_below(data, mid_x, ly, 4, 0, occluders) {
                if e.start - CORNER_SEARCH <= lx && lx < e.end + CORNER_SEARCH && !bots.iter().any(|b| (b.y - e.y).abs() <= 1) {
                    bots.push(e);
                }
            }
            tops.sort_by_key(|e| ly - e.y);
            bots.sort_by_key(|e| e.y - ly);
        }
    }
    if let (Some(t), Some(b)) = (tops.first().copied(), bots.first().copied()) {
        let mid_y = (t.y + b.y) / 2;
        if mid_y != ly {
            for e in collect_v_left(data, lx, mid_y, 4, 0, occluders) {
                if e.start - CORNER_SEARCH <= ly && ly < e.end + CORNER_SEARCH && !lefts.iter().any(|l| (l.x - e.x).abs() <= 1) {
                    lefts.push(e);
                }
            }
            for e in collect_v_right(data, lx, mid_y, 4, 0, occluders) {
                if e.start - CORNER_SEARCH <= ly && ly < e.end + CORNER_SEARCH && !rights.iter().any(|r| (r.x - e.x).abs() <= 1) {
                    rights.push(e);
                }
            }
            lefts.sort_by_key(|e| lx - e.x);
            rights.sort_by_key(|e| e.x - lx);
        }
    }

    let mut raw: Vec<BoxHit> = Vec::with_capacity(64);
    let mut try_rect = |rect: PhysRect| {
        if rect.w < 4 || rect.h < 4 || rect.area() < 16 || !rect.contains(pt) {
            return;
        }
        if !occluders.is_empty() && rect_touches_occluders(rect, occluders, OCCLUDER_EDGE_TOL) {
            return;
        }
        let (sup, min_side) = rect_support_local_excluding(data, rect, occluders);
        if sup < MIN_SUPPORT || min_side < MIN_SIDE_SUPPORT {
            return;
        }
        let (closure, h_closed) = closure_score(data, rect);
        if let Some(existing) = raw.iter_mut().find(|c| near_same(c.rect, rect) || iou_rect(c.rect, rect) >= DEDUP_IOU) {
            let better = closure > existing.closure
                || (closure == existing.closure && sup > existing.support + 0.01)
                || (closure == existing.closure && (sup - existing.support).abs() <= 0.01 && rect.area() > existing.rect.area());
            if better {
                *existing = BoxHit { rect, support: sup, closure, h_closed };
            }
        } else {
            raw.push(BoxHit { rect, support: sup, closure, h_closed });
        }
    };

    // 1. 가로 쌍(t, b) 끝점 기반 제안 — 행·둥근 상자·형제가 많은 카드/도구 막대 + 표 전체까지 한 번에 포착
    let h_lim = tops.len().min(MAX_DIR_EDGES);
    let b_lim = bots.len().min(MAX_DIR_EDGES);
    for t in &tops[..h_lim] {
        for b in &bots[..b_lim] {
            let h = b.y - t.y;
            if h < 4 {
                continue;
            }
            let x0 = t.start.max(b.start);
            let x1 = t.end.min(b.end);
            if x1 - x0 < 4 || lx < x0 - CORNER_SEARCH || lx >= x1 + CORNER_SEARCH {
                continue;
            }
            let left_best = best_v_in_range(data, x0 - CORNER_SEARCH, x0 + 2, t.y, b.y, true, occluders);
            let right_best = best_v_in_range(data, x1 - 2, x1 + CORNER_SEARCH, t.y, b.y, false, occluders);
            if let (Some((xl, _)), Some((xr, _))) = (left_best, right_best) {
                try_rect(PhysRect::from_edges(xl, t.y, xr, b.y));
                // 표 전체 포착: 행의 양 끝 세로 경계(xl, xr)가 위·아래로 이어진 전체 높이 확인
                let mid_y = (t.y + b.y) / 2;
                if let (Some(vl), Some(vr)) = (seg_covering(data.v_col_segs(xl), mid_y, CORNER_SEARCH), seg_covering(data.v_col_segs(xr), mid_y, CORNER_SEARCH)) {
                    let ty0 = (vl.start as i32).max(vr.start as i32);
                    let ty1 = (vl.end as i32).min(vr.end as i32);
                    if let (Some((yt, _)), Some((yb, _))) = (
                        best_h_in_range(data, ty0 - CORNER_SEARCH, ty0 + 2, xl, xr, true, occluders),
                        best_h_in_range(data, ty1 - 2, ty1 + CORNER_SEARCH, xl, xr, false, occluders),
                    ) {
                        try_rect(PhysRect::from_edges(xl, yt, xr, yb));
                    }
                }
            }
        }
    }

    // 2. 세로 쌍(l, r) 끝점 기반 제안 — 열·세로 사이드바·세로 카드 + 표 전체 포착
    let l_lim = lefts.len().min(MAX_DIR_EDGES);
    let r_lim = rights.len().min(MAX_DIR_EDGES);
    for l in &lefts[..l_lim] {
        for r in &rights[..r_lim] {
            let w = r.x - l.x;
            if w < 4 {
                continue;
            }
            let y0 = l.start.max(r.start);
            let y1 = l.end.min(r.end);
            if y1 - y0 < 4 || ly < y0 - CORNER_SEARCH || ly >= y1 + CORNER_SEARCH {
                continue;
            }
            let top_best = best_h_in_range(data, y0 - CORNER_SEARCH, y0 + 2, l.x, r.x, true, occluders);
            let bot_best = best_h_in_range(data, y1 - 2, y1 + CORNER_SEARCH, l.x, r.x, false, occluders);
            if let (Some((yt, _)), Some((yb, _))) = (top_best, bot_best) {
                try_rect(PhysRect::from_edges(l.x, yt, r.x, yb));
                let mid_x = (l.x + r.x) / 2;
                if let (Some(ht), Some(hb)) = (seg_covering(data.h_row_segs(yt), mid_x, CORNER_SEARCH), seg_covering(data.h_row_segs(yb), mid_x, CORNER_SEARCH)) {
                    let tx0 = (ht.start as i32).max(hb.start as i32);
                    let tx1 = (ht.end as i32).min(hb.end as i32);
                    if let (Some((xl, _)), Some((xr, _))) = (
                        best_v_in_range(data, tx0 - CORNER_SEARCH, tx0 + 2, yt, yb, true, occluders),
                        best_v_in_range(data, tx1 - 2, tx1 + CORNER_SEARCH, yt, yb, false, occluders),
                    ) {
                        try_rect(PhysRect::from_edges(xl, yt, xr, yb));
                    }
                }
            }
        }
    }

    // 3. 4방향 직접 조합 — 표 칸·분할 창처럼 네 변이 모두 더 길게 뻗은 내부 상자 포착
    let gt = tops.len().min(MAX_GRID_EDGES);
    let gb = bots.len().min(MAX_GRID_EDGES);
    let gl = lefts.len().min(MAX_GRID_EDGES);
    let gr = rights.len().min(MAX_GRID_EDGES);
    let mut valid_l: Vec<i32> = Vec::with_capacity(gl);
    let mut valid_r: Vec<i32> = Vec::with_capacity(gr);

    for t in &tops[..gt] {
        for b in &bots[..gb] {
            let h = b.y - t.y;
            if h < 4 {
                continue;
            }
            let min_v_hits = ((h as f32) * MIN_SIDE_SUPPORT) as u32;
            valid_l.clear();
            for l in &lefts[..gl] {
                if data.v_edge_count(l.x, t.y, b.y) >= min_v_hits {
                    valid_l.push(l.x);
                }
            }
            valid_r.clear();
            for r in &rights[..gr] {
                if data.v_edge_count(r.x, t.y, b.y) >= min_v_hits {
                    valid_r.push(r.x);
                }
            }
            for &xl in &valid_l {
                for &xr in &valid_r {
                    try_rect(PhysRect::from_edges(xl, t.y, xr, b.y));
                }
            }
        }
    }

    if raw.is_empty() {
        return Vec::new();
    }

    // 4. 엄격히 중첩되는 사다리(작은 것 → 큰 것, 최대 5개) 선택
    raw.sort_by_key(|c| c.rect.area());

    let mut ladder: Vec<BoxHit> = Vec::with_capacity(MAX_LADDER_BOXES);
    // 첫 단계: 가장 작은 유효 상자(칸·버튼·입력 칸)
    ladder.push(raw[0]);

    while ladder.len() < MAX_LADDER_BOXES {
        let cur = ladder[ladder.len() - 1];
        let enclosing: Vec<BoxHit> = raw
            .iter()
            .copied()
            .filter(|c| encloses(c.rect, cur.rect) && !near_same(c.rect, cur.rect) && iou_rect(c.rect, cur.rect) < DEDUP_IOU)
            .collect();
        if enclosing.is_empty() {
            break;
        }

        // 닫힌 구조(closure >= 2: 표 행·표·카드·대화 상자)를 임의 부분 격자(closure 0~1)보다 우선한다.
        // 같은 닫힘 수준에서는 행 정렬(h_closed)과 작은 면적(한 겹씩 확장)을 우선한다.
        let structured: Vec<BoxHit> = enclosing.iter().copied().filter(|c| c.closure >= 2).collect();
        let pool = if structured.is_empty() { &enclosing } else { &structured };

        let next = if !structured.is_empty() {
            // 직전 상자와 같은 높이의 가로 닫힌 행(표 칸 → 행)이 있으면 최우선
            let same_row = pool.iter().copied().find(|c| {
                c.h_closed && (c.rect.y - cur.rect.y).abs() <= 2 && (c.rect.bottom() - cur.rect.bottom()).abs() <= 2
            });
            same_row.unwrap_or_else(|| {
                // 그 외에는 닫힘이 높은 것 중 가장 작은 다음 껍질(단, 훨씬 작은 closure>=2 껍질이 있으면 먼저)
                pool.iter()
                    .copied()
                    .min_by(|a, b| {
                        a.rect
                            .area()
                            .cmp(&b.rect.area())
                            .then_with(|| b.closure.cmp(&a.closure))
                            .then_with(|| b.h_closed.cmp(&a.h_closed))
                    })
                    .unwrap()
            })
        } else {
            pool.iter()
                .copied()
                .min_by(|a, b| b.closure.cmp(&a.closure).then_with(|| a.rect.area().cmp(&b.rect.area())))
                .unwrap()
        };

        ladder.push(next);
    }

    dedup_hits(ladder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 사다리_겹침_정리는_iou_090_이상_중복을_합친다() {
        // 300×200 vs 288×188 → IoU ≈ 0.9024 >= 0.90
        let outer = BoxHit {
            rect: PhysRect::new(100, 100, 300, 200),
            support: 0.95,
            closure: 4,
            h_closed: true,
        };
        let inner = BoxHit {
            rect: PhysRect::new(106, 106, 288, 188),
            support: 0.85,
            closure: 3,
            h_closed: true,
        };
        let distinct = BoxHit {
            rect: PhysRect::new(140, 130, 160, 100),
            support: 0.90,
            closure: 4,
            h_closed: true,
        };
        let deduped = dedup_hits(vec![distinct, inner, outer]);
        assert_eq!(deduped.len(), 2, "0.9024 겹침은 하나로 합친다: {:?}", deduped.iter().map(|h| h.rect).collect::<Vec<_>>());
        assert_eq!(deduped[0].rect, distinct.rect);
        assert_eq!(deduped[1].rect, outer.rect);
        assert!(iou_rect(deduped[0].rect, deduped[1].rect) < DEDUP_IOU);
    }

    #[test]
    fn 앞_창_경계_판별은_2px_이내만_걸린다() {
        let occ = [PhysRect::new(0, 0, 800, 160)];
        assert!(h_line_touches_occluders(160, 100, 500, &occ, OCCLUDER_EDGE_TOL));
        assert!(h_line_touches_occluders(162, 100, 500, &occ, OCCLUDER_EDGE_TOL));
        assert!(!h_line_touches_occluders(163, 100, 500, &occ, OCCLUDER_EDGE_TOL));
        assert!(!h_line_touches_occluders(160, 850, 950, &occ, OCCLUDER_EDGE_TOL));
    }
}
