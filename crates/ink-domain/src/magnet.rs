//! 모서리 자석(02 §1.4, S09): 끌어당김 8 논리 px × 배율, 붙은 뒤 12 px 넘게 움직여야 떨어진다(히스테리시스).
//! 후보 경계 = 정지 순간 모든 창·요소 사각형의 네 변 + 모니터 변 (+ V 곧은 경계). Alt를 누르는 동안 끔.

use crate::geometry::{PhysPoint, PhysRect};
use serde::{Deserialize, Serialize};

pub const GRAB_LOGICAL: f64 = 8.0;
pub const RELEASE_LOGICAL: f64 = 12.0;

/// 세로 경계 x들과 가로 경계 y들(정렬·중복 없음).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Edges {
    pub xs: Vec<i32>,
    pub ys: Vec<i32>,
}

impl Edges {
    pub fn from_rects<'a>(rects: impl IntoIterator<Item = &'a PhysRect>) -> Edges {
        let mut e = Edges::default();
        for r in rects {
            e.xs.extend([r.x, r.right()]);
            e.ys.extend([r.y, r.bottom()]);
        }
        e.normalize();
        e
    }

    pub fn extend(&mut self, xs: impl IntoIterator<Item = i32>, ys: impl IntoIterator<Item = i32>) {
        self.xs.extend(xs);
        self.ys.extend(ys);
        self.normalize();
    }

    fn normalize(&mut self) {
        self.xs.sort_unstable();
        self.xs.dedup();
        self.ys.sort_unstable();
        self.ys.dedup();
    }
}

fn nearest(sorted: &[i32], v: i32, within: i32) -> Option<i32> {
    let i = sorted.partition_point(|&x| x < v);
    [i.checked_sub(1), Some(i)]
        .into_iter()
        .flatten()
        .filter_map(|j| sorted.get(j).copied())
        .filter(|x| (x - v).abs() <= within)
        .min_by_key(|x| (x - v).abs())
}

/// 지금 붙어 있는 경계(축마다).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapState {
    pub x: Option<i32>,
    pub y: Option<i32>,
}

/// 점 하나를 경계에 붙인다. `scale` = 그 모니터 배율.
pub fn snap_point(p: PhysPoint, edges: &Edges, scale: f64, state: &mut SnapState) -> PhysPoint {
    let grab = (GRAB_LOGICAL * scale).round() as i32;
    let release = (RELEASE_LOGICAL * scale).round() as i32;
    let axis = |v: i32, list: &[i32], held: &mut Option<i32>| -> i32 {
        if let Some(h) = *held {
            if (v - h).abs() <= release {
                return h;
            }
            *held = None;
        }
        match nearest(list, v, grab) {
            Some(n) => {
                *held = Some(n);
                n
            }
            None => v,
        }
    };
    PhysPoint { x: axis(p.x, &edges.xs, &mut state.x), y: axis(p.y, &edges.ys, &mut state.y) }
}

/// 두 점(경계 좌표) → 사각형(끝 포함 아님, 최소 1×1).
pub fn rect_between(a: PhysPoint, b: PhysPoint) -> PhysRect {
    let r = PhysRect::from_edges(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y));
    PhysRect::new(r.x, r.y, r.w.max(1), r.h.max(1))
}

/// 붙은 변을 따라 그릴 1 px 안내선(물리 px) — (x1, y1, x2, y2).
pub fn guides(rect: &PhysRect, state: &SnapState) -> Vec<(i32, i32, i32, i32)> {
    let mut out = Vec::new();
    if let Some(x) = state.x {
        out.push((x, rect.y, x, rect.bottom()));
    }
    if let Some(y) = state.y {
        out.push((rect.x, y, rect.right(), y));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 가까우면_붙고_멀리_가야_떨어진다() {
        let e = Edges::from_rects(&[PhysRect::new(100, 100, 400, 300)]);
        let mut s = SnapState::default();
        assert_eq!(snap_point(PhysPoint { x: 106, y: 250 }, &e, 1.0, &mut s), PhysPoint { x: 100, y: 250 });
        assert_eq!(s.x, Some(100));
        // 11 px — 아직 붙어 있음(히스테리시스)
        assert_eq!(snap_point(PhysPoint { x: 111, y: 250 }, &e, 1.0, &mut s).x, 100);
        // 13 px — 떨어짐
        assert_eq!(snap_point(PhysPoint { x: 113, y: 250 }, &e, 1.0, &mut s).x, 113);
        assert_eq!(s.x, None);
    }

    #[test]
    fn 배율만큼_끌어당김이_커진다() {
        let e = Edges::from_rects(&[PhysRect::new(100, 100, 400, 300)]);
        let mut s = SnapState::default();
        assert_eq!(snap_point(PhysPoint { x: 111, y: 0 }, &e, 1.5, &mut s).x, 100, "8 × 1.5 = 12");
        let mut s = SnapState::default();
        assert_eq!(snap_point(PhysPoint { x: 111, y: 0 }, &e, 1.0, &mut s).x, 111);
    }

    #[test]
    fn 두_점_사각형과_안내선() {
        let r = rect_between(PhysPoint { x: 500, y: 400 }, PhysPoint { x: 100, y: 100 });
        assert_eq!(r, PhysRect::new(100, 100, 400, 300));
        assert_eq!(rect_between(PhysPoint { x: 5, y: 5 }, PhysPoint { x: 5, y: 5 }).w, 1);
        let g = guides(&r, &SnapState { x: Some(100), y: None });
        assert_eq!(g, vec![(100, 100, 100, 400)]);
    }
}
