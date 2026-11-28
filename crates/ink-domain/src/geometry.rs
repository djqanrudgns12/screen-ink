//! 좌표 — 모든 판정·저장은 물리 픽셀(가상 화면 기준)로 하고, 웹뷰에 그릴 때만 모니터별 논리 좌표로 바꾼다.
//! 혼합 배율(100%+150%)에서 물리 → 논리 → 물리 왕복 오차 0이 목표(02 §8 geometry).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhysPoint {
    pub x: i32,
    pub y: i32,
}

/// 물리 픽셀 사각형. `w`·`h`가 0 이하면 빈 사각형.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhysRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl PhysRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_edges(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { x: left, y: top, w: right - left, h: bottom - top }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    pub fn area(&self) -> i64 {
        if self.is_empty() {
            0
        } else {
            self.w as i64 * self.h as i64
        }
    }

    pub fn contains(&self, p: PhysPoint) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    pub fn intersect(&self, o: &PhysRect) -> Option<PhysRect> {
        let r = PhysRect::from_edges(self.x.max(o.x), self.y.max(o.y), self.right().min(o.right()), self.bottom().min(o.bottom()));
        (!r.is_empty()).then_some(r)
    }

    pub fn union(&self, o: &PhysRect) -> PhysRect {
        if self.is_empty() {
            return *o;
        }
        if o.is_empty() {
            return *self;
        }
        PhysRect::from_edges(self.x.min(o.x), self.y.min(o.y), self.right().max(o.right()), self.bottom().max(o.bottom()))
    }

    /// 두 점(드래그 시작·끝)을 감싸는 사각형 — 끝 픽셀 포함.
    pub fn spanning(a: PhysPoint, b: PhysPoint) -> PhysRect {
        PhysRect::from_edges(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x) + 1, a.y.max(b.y) + 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogicalPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: u32,
    /// 가상 화면 안 물리 픽셀 경계
    pub bounds: PhysRect,
    /// 작업 표시줄을 뺀 영역
    pub work: PhysRect,
    /// 배율(1.0 = 100%, 1.5 = 150%)
    pub scale: f64,
    pub primary: bool,
}

impl Monitor {
    /// 작업 영역 위 가운데를 기준으로 삼고, 강조와 겹치면 같은 화면 아래 가운데에 놓는다.
    pub fn island_rect(&self, logical_size: (f64, f64), avoid: Option<PhysRect>) -> PhysRect {
        let work = self.work;
        let px = |logical: f64| (logical * self.scale).round() as i32;
        let w = px(logical_size.0).min(work.w);
        let h = px(logical_size.1).min(work.h);
        let gap = px(12.0).min(work.h - h);
        let mut rect = PhysRect::new(work.x + (work.w - w) / 2, work.y + gap, w, h);
        if avoid.is_some_and(|a| rect.intersect(&a).is_some()) {
            rect.y = work.bottom() - h - gap;
        }
        rect
    }

    /// 이 모니터 오버레이 웹뷰 안의 논리 좌표(모니터 왼쪽 위 = 0,0).
    pub fn to_local_logical(&self, p: PhysPoint) -> LogicalPoint {
        LogicalPoint { x: (p.x - self.bounds.x) as f64 / self.scale, y: (p.y - self.bounds.y) as f64 / self.scale }
    }

    pub fn to_physical(&self, p: LogicalPoint) -> PhysPoint {
        PhysPoint { x: self.bounds.x + (p.x * self.scale).round() as i32, y: self.bounds.y + (p.y * self.scale).round() as i32 }
    }

    pub fn rect_to_local_logical(&self, r: &PhysRect) -> LogicalRect {
        let o = self.to_local_logical(PhysPoint { x: r.x, y: r.y });
        LogicalRect { x: o.x, y: o.y, w: r.w as f64 / self.scale, h: r.h as f64 / self.scale }
    }

    pub fn rect_to_physical(&self, r: &LogicalRect) -> PhysRect {
        let a = self.to_physical(LogicalPoint { x: r.x, y: r.y });
        let b = self.to_physical(LogicalPoint { x: r.x + r.w, y: r.y + r.h });
        PhysRect::from_edges(a.x, a.y, b.x, b.y)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MonitorLayout {
    pub monitors: Vec<Monitor>,
}

impl MonitorLayout {
    pub fn new(monitors: Vec<Monitor>) -> Self {
        Self { monitors }
    }

    pub fn monitor_at(&self, p: PhysPoint) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.bounds.contains(p))
    }

    /// 모니터 사이 틈에 있는 점이면 가장 가까운 모니터.
    pub fn nearest(&self, p: PhysPoint) -> Option<&Monitor> {
        self.monitor_at(p).or_else(|| {
            self.monitors.iter().min_by_key(|m| {
                let dx = (m.bounds.x - p.x).max(0).max(p.x - (m.bounds.right() - 1)) as i64;
                let dy = (m.bounds.y - p.y).max(0).max(p.y - (m.bounds.bottom() - 1)) as i64;
                dx * dx + dy * dy
            })
        })
    }

    pub fn primary(&self) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.primary).or_else(|| self.monitors.first())
    }

    /// 가상 화면 전체(F1은 이 범위를 덮는다).
    pub fn virtual_bounds(&self) -> PhysRect {
        self.monitors.iter().fold(PhysRect::default(), |acc, m| acc.union(&m.bounds))
    }

    /// 모니터 경계를 넘는 사각형을 모니터별 조각으로 자른다.
    pub fn split(&self, r: &PhysRect) -> Vec<(u32, PhysRect)> {
        self.monitors.iter().filter_map(|m| m.bounds.intersect(r).map(|x| (m.id, x))).collect()
    }

    /// 화면 밖으로 나간 부분을 잘라 낸다(모니터가 없는 틈 포함).
    pub fn clip(&self, r: &PhysRect) -> Option<PhysRect> {
        self.split(r).into_iter().map(|(_, x)| x).reduce(|a, b| a.union(&b))
    }

    /// 모니터 구성 지문 — 범위 기록이 같은 구성에서만 다시 쓰이게(05 §4). 순서와 무관.
    pub fn fingerprint(&self) -> String {
        let mut parts: Vec<String> = self
            .monitors
            .iter()
            .map(|m| format!("{},{},{},{}@{}", m.bounds.x, m.bounds.y, m.bounds.w, m.bounds.h, (m.scale * 100.0).round() as i64))
            .collect();
        parts.sort();
        parts.join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 왼쪽 = 150% 노트북(2880x1800), 오른쪽 = 100% 모니터(1920x1080), 위로 어긋남.
    fn mixed() -> MonitorLayout {
        MonitorLayout::new(vec![
            Monitor { id: 1, bounds: PhysRect::new(0, 0, 2880, 1800), work: PhysRect::new(0, 0, 2880, 1728), scale: 1.5, primary: true },
            Monitor { id: 2, bounds: PhysRect::new(2880, -200, 1920, 1080), work: PhysRect::new(2880, -200, 1920, 1040), scale: 1.0, primary: false },
        ])
    }

    #[test]
    fn 섬은_상단과_겹칠_때만_작업_영역_아래로_간다() {
        let work = PhysRect::new(0, 0, 3840, 2064);
        let m = Monitor { id: 0, bounds: work, work, scale: 2.0, primary: true };
        let place = |avoid| m.island_rect((560.0, 88.0), avoid);
        let top = PhysRect::new(1360, 24, 1120, 176);
        let bottom = PhysRect::new(1360, 1864, 1120, 176);
        assert_eq!(place(None), top);
        assert_eq!(place(Some(PhysRect::new(1280, 65, 820, 320))), bottom);
        assert_eq!(place(Some(work)), bottom);
        assert_eq!(place(Some(PhysRect::new(1280, 200, 820, 320))), top);
        assert_eq!(place(Some(PhysRect::new(-820, 65, 820, 320))), top);
    }

    #[test]
    fn 혼합_배율_왕복_오차_0() {
        let l = mixed();
        for m in &l.monitors {
            for (dx, dy) in [(0, 0), (1, 1), (7, 13), (m.bounds.w - 1, m.bounds.h - 1), (1001, 333)] {
                let p = PhysPoint { x: m.bounds.x + dx, y: m.bounds.y + dy };
                assert_eq!(m.to_physical(m.to_local_logical(p)), p, "monitor {} {:?}", m.id, p);
            }
            let r = PhysRect::new(m.bounds.x + 3, m.bounds.y + 5, 301, 157);
            assert_eq!(m.rect_to_physical(&m.rect_to_local_logical(&r)), r);
        }
    }

    #[test]
    fn 음수_좌표_모니터와_가장_가까운_모니터() {
        let l = mixed();
        assert_eq!(l.monitor_at(PhysPoint { x: 3000, y: -100 }).map(|m| m.id), Some(2));
        // 오른쪽 모니터 아래 틈(880..1800 사이 y, x>2880)
        assert_eq!(l.monitor_at(PhysPoint { x: 3000, y: 1500 }), None);
        assert_eq!(l.nearest(PhysPoint { x: 3000, y: 1500 }).map(|m| m.id), Some(1));
        assert_eq!(l.virtual_bounds(), PhysRect::from_edges(0, -200, 4800, 1800));
    }

    #[test]
    fn 경계를_넘는_사각형_자르기() {
        let l = mixed();
        let r = PhysRect::new(2800, 800, 200, 200); // 오른쪽 모니터 아래쪽 틈까지 걸침
        let parts = l.split(&r);
        assert_eq!(parts, vec![(1, PhysRect::new(2800, 800, 80, 200)), (2, PhysRect::new(2880, 800, 120, 80))]);
        assert_eq!(l.clip(&PhysRect::new(-50, -50, 100, 100)), Some(PhysRect::new(0, 0, 50, 50)));
    }

    #[test]
    fn 지문은_순서와_무관() {
        let mut l = mixed();
        let a = l.fingerprint();
        l.monitors.reverse();
        assert_eq!(a, l.fingerprint());
        l.monitors[0].scale = 1.25;
        assert_ne!(a, l.fingerprint());
    }

    #[test]
    fn 드래그_두_점은_끝_픽셀을_포함() {
        let r = PhysRect::spanning(PhysPoint { x: 10, y: 20 }, PhysPoint { x: 5, y: 8 });
        assert_eq!(r, PhysRect::new(5, 8, 6, 13));
    }
}
