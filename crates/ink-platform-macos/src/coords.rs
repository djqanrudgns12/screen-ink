//! 맥 좌표 → 계약 좌표(가상 화면 물리 px, 왼쪽 위 원점, y 아래로)의 **유일한** 변환 자리(R-DPI-1).
//!
//! 맥 좌표는 세 가지다.
//! - Cocoa(`NSScreen.frame`): 포인트, 원점 = 주 화면 **왼쪽 아래**, y 위로
//! - CG 전역(`CGWindowListCopyWindowInfo`·`CGEvent` 위치): 포인트, 원점 = 주 화면 **왼쪽 위**, y 아래로
//! - 물리 px: 포인트 × 그 모니터 배율(`backingScaleFactor`)
//!
//! 모니터마다 배율이 다르면 `포인트 × 배율`을 그대로 쓸 때 물리 배치가 겹치거나 틈이 생긴다
//! (4K@2.0 `0..1920pt` → `0..3840px`, 옆 FHD@1.0 `1920pt` → `1920px` = 겹침). 그래서 주 모니터에서
//! 시작해 **맞닿은 이웃을 맞닿은 채로 민다**(Windows 물리 배치와 같은 성질: 크기는 그 모니터 물리 px,
//! 맞닿은 모서리는 맞닿은 채). 이웃으로 이어지지 않는 배치(틈)나 밀어도 겹치는 배치는 모든 원점을
//! `포인트 × 가장 큰 배율`로 둔다(겹침 없음 보장, 틈은 생김). 결정 기록: 인수인계 §4.
//!
//! 변환 규칙은 [`map`]·[`unmap`] 두 함수뿐이고 나머지는 모두 이것을 부른다.

use ink_domain::geometry::{Monitor, MonitorLayout, PhysPoint, PhysRect};

/// 포인트 사각형(원점 왼쪽 위, CG 전역).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectF {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl RectF {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    fn distance2(&self, x: f64, y: f64) -> f64 {
        let dx = (self.x - x).max(0.0).max(x - self.right());
        let dy = (self.y - y).max(0.0).max(y - self.bottom());
        dx * dx + dy * dy
    }
}

/// Cocoa 사각형(원점 왼쪽 아래) → CG 전역(원점 왼쪽 위). `primary_height` = 주 화면 `frame.size.height`.
pub fn cocoa_to_cg(r: RectF, primary_height: f64) -> RectF {
    RectF { x: r.x, y: primary_height - (r.y + r.h), w: r.w, h: r.h }
}

/// 한 화면의 맥 사실값(CG 전역 포인트).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenInfo {
    /// `CGDirectDisplayID`
    pub display: u32,
    pub frame: RectF,
    /// 메뉴 막대·Dock을 뺀 영역(`visibleFrame`)
    pub visible: RectF,
    pub scale: f64,
    pub primary: bool,
}

/// 물리 배치가 정해진 화면.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub display: u32,
    /// 계약 모니터 id(왼쪽→오른쪽·위→아래, Windows와 같은 규칙)
    pub id: u32,
    pub pt: RectF,
    pub phys: PhysRect,
    pub work: PhysRect,
    pub scale: f64,
    pub primary: bool,
}

/// 화면 배치 하나에 대한 변환기. 배치가 바뀌면 새로 만든다.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Space {
    pub screens: Vec<Placed>,
    /// 맞닿은 채 밀기로 정했으면 true, `포인트 × 가장 큰 배율` 대체 배치면 false
    pub packed: bool,
}

/// 포인트 → 물리 px(그 화면 기준). 변환 규칙 ①.
fn map(s: &Placed, x: f64, y: f64) -> (f64, f64) {
    (s.phys.x as f64 + (x - s.pt.x) * s.scale, s.phys.y as f64 + (y - s.pt.y) * s.scale)
}

/// 물리 px → 포인트(그 화면 기준). 변환 규칙 ②.
fn unmap(s: &Placed, x: f64, y: f64) -> (f64, f64) {
    (s.pt.x + (x - s.phys.x as f64) / s.scale, s.pt.y + (y - s.phys.y as f64) / s.scale)
}

impl Placed {
    /// 이 화면 기준 물리 px 사각형(왼쪽 위 원점) → 이 화면 안 포인트(왼쪽 위 원점). 정지 화면 강조 자리.
    pub fn local_to_pt(&self, r: PhysRect) -> RectF {
        let (ox, oy) = (self.phys.x as f64, self.phys.y as f64);
        let (x0, y0) = unmap(self, ox + r.x as f64, oy + r.y as f64);
        let (x1, y1) = unmap(self, ox + r.right() as f64, oy + r.bottom() as f64);
        RectF::new(x0 - self.pt.x, y0 - self.pt.y, x1 - x0, y1 - y0)
    }
}

/// `w × h` 판에서 `hole`만 비우고 둘레를 덮는 사각형 4개(위·아래·왼쪽·오른쪽, 왼쪽 위 원점).
/// 구멍이 없거나 판 밖이면 첫 칸이 판 전체, 나머지는 빈 칸.
pub fn around(w: f64, h: f64, hole: Option<RectF>) -> [RectF; 4] {
    let empty = RectF::default();
    let hole = hole.and_then(|r| {
        let (x0, y0) = (r.x.clamp(0.0, w), r.y.clamp(0.0, h));
        let (x1, y1) = (r.right().clamp(0.0, w), r.bottom().clamp(0.0, h));
        (x1 > x0 && y1 > y0).then(|| RectF::new(x0, y0, x1 - x0, y1 - y0))
    });
    let Some(r) = hole else { return [RectF::new(0.0, 0.0, w, h), empty, empty, empty] };
    [
        RectF::new(0.0, 0.0, w, r.y),
        RectF::new(0.0, r.bottom(), w, h - r.bottom()),
        RectF::new(0.0, r.y, r.x, r.h),
        RectF::new(r.right(), r.y, w - r.right(), r.h),
    ]
}

const TOL: f64 = 0.5;

fn spans_overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> bool {
    a0 < b1 - TOL && b0 < a1 - TOL
}

fn phys_size(s: &ScreenInfo) -> (i32, i32) {
    ((s.frame.w * s.scale).round() as i32, (s.frame.h * s.scale).round() as i32)
}

/// 이미 놓인 `m` 옆에 맞닿은 `n`의 물리 원점. 맞닿지 않았으면 None.
fn place_next_to(m: &Placed, n: &ScreenInfo) -> Option<(i32, i32)> {
    let (nw, nh) = phys_size(n);
    let (a, b) = (m.pt, n.frame);
    // 맞닿은 모서리를 따라 어긋난 만큼: 안쪽에 들어간 모서리를 가진 화면의 배율로 잰다
    let along = |m0: f64, n0: f64, m_phys0: i32| -> i32 {
        if n0 >= m0 {
            m_phys0 + ((n0 - m0) * m.scale).round() as i32
        } else {
            m_phys0 - ((m0 - n0) * n.scale).round() as i32
        }
    };
    if spans_overlap(a.y, a.bottom(), b.y, b.bottom()) {
        if (b.x - a.right()).abs() < TOL {
            return Some((m.phys.right(), along(a.y, b.y, m.phys.y)));
        }
        if (b.right() - a.x).abs() < TOL {
            return Some((m.phys.x - nw, along(a.y, b.y, m.phys.y)));
        }
    }
    if spans_overlap(a.x, a.right(), b.x, b.right()) {
        if (b.y - a.bottom()).abs() < TOL {
            return Some((along(a.x, b.x, m.phys.x), m.phys.bottom()));
        }
        if (b.bottom() - a.y).abs() < TOL {
            return Some((along(a.x, b.x, m.phys.x), m.phys.y - nh));
        }
    }
    None
}

fn placed(s: &ScreenInfo, x: i32, y: i32) -> Placed {
    let (w, h) = phys_size(s);
    Placed { display: s.display, id: 0, pt: s.frame, phys: PhysRect::new(x, y, w, h), work: PhysRect::default(), scale: s.scale, primary: s.primary }
}

fn any_overlap(list: &[Placed]) -> bool {
    list.iter().enumerate().any(|(i, a)| list[i + 1..].iter().any(|b| a.phys.intersect(&b.phys).is_some_and(|r| r.w > 0 && r.h > 0)))
}

impl Space {
    pub fn new(screens: &[ScreenInfo]) -> Self {
        let mut screens: Vec<ScreenInfo> = screens.iter().copied().filter(|s| s.frame.w > 0.0 && s.frame.h > 0.0 && s.scale > 0.0).collect();
        if screens.is_empty() {
            return Self::default();
        }
        // 주 화면 먼저, 그다음 왼쪽 위부터(결정적인 순서)
        screens.sort_by(|a, b| b.primary.cmp(&a.primary).then(a.frame.x.total_cmp(&b.frame.x)).then(a.frame.y.total_cmp(&b.frame.y)));
        let first = &screens[0];
        let mut out = vec![placed(first, (first.frame.x * first.scale).round() as i32, (first.frame.y * first.scale).round() as i32)];
        let mut rest: Vec<ScreenInfo> = screens[1..].to_vec();
        while !rest.is_empty() {
            let found = rest.iter().enumerate().find_map(|(i, n)| out.iter().find_map(|m| place_next_to(m, n)).map(|xy| (i, xy)));
            let Some((i, (x, y))) = found else { break };
            let n = rest.remove(i);
            out.push(placed(&n, x, y));
        }
        let mut packed = true;
        if !rest.is_empty() || any_overlap(&out) {
            let s_max = screens.iter().map(|s| s.scale).fold(1.0, f64::max);
            out = screens.iter().map(|s| placed(s, (s.frame.x * s_max).round() as i32, (s.frame.y * s_max).round() as i32)).collect();
            packed = false;
        }
        let order = out_order(&screens, &out);
        for (p, s) in out.iter_mut().zip(order) {
            let v = s.visible;
            let (x0, y0) = map(p, v.x, v.y);
            let (x1, y1) = map(p, v.right(), v.bottom());
            let work = PhysRect::from_edges(x0.round() as i32, y0.round() as i32, x1.round() as i32, y1.round() as i32);
            p.work = work.intersect(&p.phys).unwrap_or(p.phys);
        }
        out.sort_by_key(|p| (p.phys.x, p.phys.y));
        for (i, p) in out.iter_mut().enumerate() {
            p.id = i as u32;
        }
        Self { screens: out, packed }
    }

    /// 계약 모니터 배치.
    pub fn layout(&self) -> MonitorLayout {
        MonitorLayout::new(self.screens.iter().map(|p| Monitor { id: p.id, bounds: p.phys, work: p.work, scale: p.scale, primary: p.primary }).collect())
    }

    fn screen_at_pt(&self, x: f64, y: f64) -> Option<&Placed> {
        self.screens.iter().find(|s| s.pt.contains(x, y)).or_else(|| self.screens.iter().min_by(|a, b| a.pt.distance2(x, y).total_cmp(&b.pt.distance2(x, y))))
    }

    fn screen_at_phys(&self, p: PhysPoint) -> Option<&Placed> {
        self.screens.iter().find(|s| s.phys.contains(p)).or_else(|| {
            let d = |s: &Placed| {
                let dx = (s.phys.x - p.x).max(0).max(p.x - (s.phys.right() - 1)) as i64;
                let dy = (s.phys.y - p.y).max(0).max(p.y - (s.phys.bottom() - 1)) as i64;
                dx * dx + dy * dy
            };
            self.screens.iter().min_by_key(|s| d(s))
        })
    }

    /// CG 전역 포인트 한 점 → 물리 px(그 점이 있는 화면 기준, 틈이면 가장 가까운 화면).
    pub fn to_phys(&self, x: f64, y: f64) -> PhysPoint {
        match self.screen_at_pt(x, y) {
            Some(s) => {
                let (px, py) = map(s, x, y);
                PhysPoint { x: px.floor() as i32, y: py.floor() as i32 }
            }
            None => PhysPoint { x: x.floor() as i32, y: y.floor() as i32 },
        }
    }

    /// CG 전역 포인트 사각형(창) → 물리 px. 가운데가 있는 화면의 배율로 잰다(창은 그 화면에 그려진 크기).
    pub fn rect_to_phys(&self, r: RectF) -> PhysRect {
        let (cx, cy) = r.center();
        match self.screen_at_pt(cx, cy) {
            Some(s) => {
                let (x0, y0) = map(s, r.x, r.y);
                let (x1, y1) = map(s, r.right(), r.bottom());
                PhysRect::from_edges(x0.round() as i32, y0.round() as i32, x1.round() as i32, y1.round() as i32)
            }
            None => PhysRect::from_edges(r.x.round() as i32, r.y.round() as i32, r.right().round() as i32, r.bottom().round() as i32),
        }
    }

    /// 물리 px 사각형 → CG 전역 포인트(핀 창처럼 임의 사각형을 포인트로 놓을 때). [`Self::rect_to_phys`]의 반대 —
    /// 가운데가 있는 화면의 배율로 잰다(혼합 배율에서 Tauri 물리 좌표로 놓으면 옆 화면으로 어긋난다, M16).
    pub fn rect_to_points(&self, r: PhysRect) -> RectF {
        let center = PhysPoint { x: r.x + r.w / 2, y: r.y + r.h / 2 };
        match self.screen_at_phys(center) {
            Some(s) => {
                let (x0, y0) = unmap(s, r.x as f64, r.y as f64);
                let (x1, y1) = unmap(s, r.right() as f64, r.bottom() as f64);
                RectF::new(x0, y0, x1 - x0, y1 - y0)
            }
            None => RectF::new(r.x as f64, r.y as f64, r.w as f64, r.h as f64),
        }
    }

    /// 물리 px 한 점 → CG 전역 포인트(커서 옮기기 등).
    pub fn to_points(&self, p: PhysPoint) -> (f64, f64) {
        match self.screen_at_phys(p) {
            Some(s) => unmap(s, p.x as f64, p.y as f64),
            None => (p.x as f64, p.y as f64),
        }
    }

    /// 계약 모니터 경계 → 그 화면(오버레이 창을 포인트로 놓을 때).
    pub fn screen_of(&self, bounds: PhysRect) -> Option<&Placed> {
        self.screens.iter().find(|s| s.phys == bounds)
    }

    /// "포인트 × 그 화면 배율"로 받은 사각형(tray-icon·tao 물리 좌표) → CG 전역 포인트.
    /// 혼합 배율에서는 숫자만으로 정해지지 않는다(3000 = 배율 2의 1500pt도, 배율 1의 3000pt도 됨).
    /// `hint` = 그 사각형 안에 있던 점(클릭 순간 커서, 포인트)이 들어가는 쪽을 고르고, 없으면 그 화면 안에 드는 쪽.
    pub fn unscale_rect(&self, x: f64, y: f64, w: f64, h: f64, hint: Option<(f64, f64)>) -> RectF {
        let cands: Vec<(RectF, &Placed)> = self.screens.iter().map(|s| (RectF::new(x / s.scale, y / s.scale, w / s.scale, h / s.scale), s)).collect();
        let near = |r: &RectF, (hx, hy): (f64, f64)| hx >= r.x - 2.0 && hx <= r.right() + 2.0 && hy >= r.y - 2.0 && hy <= r.bottom() + 2.0;
        if let Some(hint) = hint {
            if let Some((r, _)) = cands.iter().find(|(r, s)| near(r, hint) && s.pt.contains(r.center().0, r.center().1)) {
                return *r;
            }
        }
        cands.iter().find(|(r, s)| s.pt.contains(r.center().0, r.center().1)).map(|(r, _)| *r).unwrap_or(RectF::new(x, y, w, h))
    }

    pub fn screen_by_display(&self, display: u32) -> Option<&Placed> {
        self.screens.iter().find(|s| s.display == display)
    }
}

/// `out`의 각 원소에 대응하는 원래 화면 정보(display id로 짝).
fn out_order(screens: &[ScreenInfo], out: &[Placed]) -> Vec<ScreenInfo> {
    out.iter().map(|p| *screens.iter().find(|s| s.display == p.display).expect("same set")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scr(display: u32, x: f64, y: f64, w: f64, h: f64, scale: f64, primary: bool) -> ScreenInfo {
        ScreenInfo { display, frame: RectF::new(x, y, w, h), visible: RectF::new(x, y, w, h), scale, primary }
    }

    fn assert_no_overlap(s: &Space) {
        assert!(!any_overlap(&s.screens), "겹침: {:?}", s.screens);
        assert_eq!(s.screens.iter().filter(|p| p.primary).count(), 1);
    }

    #[test]
    fn 레티나_한_대() {
        let mut one = scr(1, 0.0, 0.0, 1440.0, 900.0, 2.0, true);
        one.visible = RectF::new(0.0, 33.0, 1440.0, 867.0 - 70.0); // 메뉴 막대 33pt, Dock 70pt
        let s = Space::new(&[one]);
        let l = s.layout();
        assert_eq!(l.monitors[0].bounds, PhysRect::new(0, 0, 2880, 1800));
        assert_eq!(l.monitors[0].work, PhysRect::new(0, 66, 2880, 1594));
        assert_eq!(s.to_phys(10.25, 20.5), PhysPoint { x: 20, y: 41 });
        assert!(s.packed);
    }

    #[test]
    fn 배율이_다른_두_대_나란히_겹치지_않게_민다() {
        // 이 맥미니: 4K(배율 2, 주) + 오른쪽 FHD(배율 1), 둘 다 1920x1080pt
        let s = Space::new(&[scr(3, 0.0, 0.0, 1920.0, 1080.0, 2.0, true), scr(1, 1920.0, 0.0, 1920.0, 1080.0, 1.0, false)]);
        assert!(s.packed);
        assert_no_overlap(&s);
        let l = s.layout();
        assert_eq!(l.monitors[0].bounds, PhysRect::new(0, 0, 3840, 2160));
        assert_eq!(l.monitors[1].bounds, PhysRect::new(3840, 0, 1920, 1080));
        assert_eq!((l.monitors[0].id, l.monitors[1].id), (0, 1));
        // 오른쪽 화면 안의 점은 그 화면 배율로
        assert_eq!(s.to_phys(1930.0, 5.0), PhysPoint { x: 3850, y: 5 });
        assert_eq!(s.to_phys(10.0, 5.0), PhysPoint { x: 20, y: 10 });
        // 왕복
        assert_eq!(s.to_points(PhysPoint { x: 3850, y: 5 }), (1930.0, 5.0));
        assert_eq!(s.to_points(PhysPoint { x: 20, y: 10 }), (10.0, 5.0));
        // 오른쪽 화면에 걸친 창은 가운데 화면 배율로 잰다
        assert_eq!(s.rect_to_phys(RectF::new(2000.0, 100.0, 400.0, 300.0)), PhysRect::new(3920, 100, 400, 300));
        assert_eq!(s.rect_to_phys(RectF::new(100.0, 100.0, 400.0, 300.0)), PhysRect::new(200, 200, 800, 600));
        // 반대(핀 창 놓기): 4K 범위 700,350 400x250pt, FHD 범위 2300,300 400x250pt — 2026-11-26 실측 어긋남 자리
        assert_eq!(s.rect_to_points(PhysRect::new(1400, 700, 800, 500)), RectF::new(700.0, 350.0, 400.0, 250.0));
        assert_eq!(s.rect_to_points(PhysRect::new(4220, 300, 400, 250)), RectF::new(2300.0, 300.0, 400.0, 250.0));
        for r in [PhysRect::new(3920, 100, 400, 300), PhysRect::new(200, 200, 800, 600)] {
            assert_eq!(s.rect_to_phys(s.rect_to_points(r)), r, "왕복");
        }
    }

    #[test]
    fn 왼쪽_아래로_어긋난_이웃은_주_화면_배율로_어긋남을_잰다() {
        let s = Space::new(&[scr(1, 0.0, 0.0, 1920.0, 1080.0, 2.0, true), scr(2, -1920.0, 200.0, 1920.0, 1080.0, 1.0, false)]);
        assert!(s.packed);
        assert_no_overlap(&s);
        let left = s.screen_by_display(2).unwrap();
        assert_eq!(left.phys, PhysRect::new(-1920, 400, 1920, 1080));
        assert_eq!(left.id, 0, "왼쪽이 0번");
        assert_eq!(s.to_phys(-1.0, 200.0), PhysPoint { x: -1, y: 400 });
    }

    #[test]
    fn 위쪽_이웃이_왼쪽으로_삐져나오면_그_화면_배율로_잰다() {
        let s = Space::new(&[scr(1, 0.0, 0.0, 1920.0, 1080.0, 2.0, true), scr(2, -500.0, -1080.0, 1920.0, 1080.0, 1.0, false)]);
        assert!(s.packed);
        assert_no_overlap(&s);
        assert_eq!(s.screen_by_display(2).unwrap().phys, PhysRect::new(-500, -1080, 1920, 1080));
    }

    #[test]
    fn 세_대_가로_줄() {
        let s = Space::new(&[
            scr(1, 0.0, 0.0, 1512.0, 982.0, 2.0, true),
            scr(2, 1512.0, -200.0, 2560.0, 1440.0, 1.0, false),
            scr(3, 4072.0, -200.0, 1920.0, 1080.0, 2.0, false),
        ]);
        assert!(s.packed);
        assert_no_overlap(&s);
        let b2 = s.screen_by_display(2).unwrap().phys;
        let b3 = s.screen_by_display(3).unwrap().phys;
        assert_eq!(b2.x, 3024);
        assert_eq!(b3.x, b2.right(), "맞닿은 채");
        assert_eq!(b3.y, b2.y);
    }

    #[test]
    fn 떨어진_화면은_가장_큰_배율로_대체_배치() {
        let s = Space::new(&[scr(1, 0.0, 0.0, 1920.0, 1080.0, 2.0, true), scr(2, 2500.0, 0.0, 1920.0, 1080.0, 1.0, false)]);
        assert!(!s.packed);
        assert_no_overlap(&s);
        assert_eq!(s.screen_by_display(2).unwrap().phys, PhysRect::new(5000, 0, 1920, 1080));
        // 틈에 있는 점은 가장 가까운 화면(주 화면, 280pt) 기준으로 잰다
        assert_eq!(s.to_phys(2200.0, 10.0), PhysPoint { x: 4400, y: 20 });
    }

    #[test]
    fn 배율을_곱해_온_트레이_사각형을_포인트로() {
        let s = Space::new(&[scr(3, 0.0, 0.0, 1920.0, 1080.0, 2.0, true), scr(1, 1920.0, 0.0, 1920.0, 1080.0, 1.0, false)]);
        // 주 화면(배율 2) 메뉴 막대의 아이콘: 1200pt,0 30x30pt → tray-icon은 2400,0 60x60을 준다
        assert_eq!(s.unscale_rect(2400.0, 0.0, 60.0, 60.0, Some((1210.0, 12.0))), RectF::new(1200.0, 0.0, 30.0, 30.0));
        // 오른쪽 화면(배율 1)의 아이콘 3000pt: 숫자만 보면 주 화면 1500pt(÷2)와 같다 → 클릭한 커서 위치로 가린다
        assert_eq!(s.unscale_rect(3000.0, 0.0, 30.0, 30.0, Some((3010.0, 10.0))), RectF::new(3000.0, 0.0, 30.0, 30.0));
        assert_eq!(s.unscale_rect(3000.0, 0.0, 60.0, 60.0, Some((1510.0, 10.0))), RectF::new(1500.0, 0.0, 30.0, 30.0));
        // 힌트가 없으면 그 화면 안에 드는 첫 쪽
        assert_eq!(s.unscale_rect(2400.0, 0.0, 60.0, 60.0, None), RectF::new(1200.0, 0.0, 30.0, 30.0));
    }

    #[test]
    fn cocoa_좌표_뒤집기() {
        // 주 화면 높이 1080pt, 메뉴 막대 30pt → visibleFrame(0,0,1920,1050)은 CG에서 y 30부터
        assert_eq!(cocoa_to_cg(RectF::new(0.0, 0.0, 1920.0, 1050.0), 1080.0), RectF::new(0.0, 30.0, 1920.0, 1050.0));
        // 주 화면 위에 놓인 화면(Cocoa y = 1080)은 CG에서 음수
        assert_eq!(cocoa_to_cg(RectF::new(0.0, 1080.0, 1920.0, 1080.0), 1080.0), RectF::new(0.0, -1080.0, 1920.0, 1080.0));
    }

    #[test]
    fn 화면_안_물리_px를_포인트로() {
        let s = Space::new(&[scr(3, 0.0, 0.0, 1920.0, 1080.0, 2.0, true), scr(1, 1920.0, 0.0, 1920.0, 1080.0, 1.0, false)]);
        // 레티나 화면(배율 2): 화면 기준 200,100 400x300 px → 100,50 200x150 pt
        assert_eq!(s.screens[0].local_to_pt(PhysRect::new(200, 100, 400, 300)), RectF::new(100.0, 50.0, 200.0, 150.0));
        // 오른쪽 FHD(배율 1, 물리 원점 3840): 화면 기준 그대로
        assert_eq!(s.screens[1].local_to_pt(PhysRect::new(10, 20, 30, 40)), RectF::new(10.0, 20.0, 30.0, 40.0));
    }

    #[test]
    fn 강조_둘레_네_칸() {
        let [t, b, l, r] = around(100.0, 80.0, Some(RectF::new(10.0, 20.0, 30.0, 40.0)));
        assert_eq!(t, RectF::new(0.0, 0.0, 100.0, 20.0));
        assert_eq!(b, RectF::new(0.0, 60.0, 100.0, 20.0));
        assert_eq!(l, RectF::new(0.0, 20.0, 10.0, 40.0));
        assert_eq!(r, RectF::new(40.0, 20.0, 60.0, 40.0));
        // 네 칸 + 구멍 = 판 전체(겹침 없음)
        let area: f64 = [t, b, l, r].iter().map(|q| q.w * q.h).sum::<f64>() + 30.0 * 40.0;
        assert_eq!(area, 100.0 * 80.0);
        // 판 밖으로 나간 강조는 판 안으로 자른다
        assert_eq!(around(100.0, 80.0, Some(RectF::new(-10.0, -10.0, 30.0, 30.0)))[3], RectF::new(20.0, 0.0, 80.0, 20.0));
        // 강조 없음·판 밖 = 전체 어둡게
        let full = [RectF::new(0.0, 0.0, 100.0, 80.0), RectF::default(), RectF::default(), RectF::default()];
        assert_eq!(around(100.0, 80.0, None), full);
        assert_eq!(around(100.0, 80.0, Some(RectF::new(200.0, 0.0, 10.0, 10.0))), full);
    }

    #[test]
    fn 빈_목록과_잘못된_값() {
        assert!(Space::new(&[]).screens.is_empty());
        assert!(Space::new(&[scr(1, 0.0, 0.0, 0.0, 0.0, 2.0, true)]).screens.is_empty());
        assert_eq!(Space::default().to_phys(3.7, 4.2), PhysPoint { x: 3, y: 4 });
    }
}
