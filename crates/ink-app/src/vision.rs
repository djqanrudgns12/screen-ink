//! V 근거 엔진: 순수 계산 `ink-vision`을 `VisionSource`로 잇는다(08 §4.3·§4.7).
//!
//! 앱에서는 모니터마다 배경 스레드에서 정지 이미지를 전처리한다(`new`). 재생 시험은 같은 것을
//! 그 자리에서 끝까지 한다(`blocking`). 커서가 있는 모니터는 커서 주변 1024²를 먼저
//! (첫 강조용), 그다음 전체. 단계가 끝날 때마다 `on_ready`(→ 메인 스레드 `on_vision_ready`)를 부른다.
//! 질의는 메모리에서만 하고 기다리지 않는다 — 아직 준비 전이면 빈 답. 좌표는 가상 화면 물리 px.

use crate::ports::{Frame, VisionSource};
use ink_domain::geometry::{MonitorLayout, PhysPoint, PhysRect};
use ink_domain::recog::{VisionCand, VisionKind};
use std::sync::{Arc, Mutex, MutexGuard};

/// 커서 주변 먼저 전처리할 반폭(08 §4.7 "커서 주변 1024² 먼저").
const ROI_HALF: u32 = 512;
/// 경계 맞춤 허용(±3 px, 08 §4.4-3).
const SNAP_TOL: i32 = 3;
/// 드래그 자석 후보를 모을 반경(물리 px). 자석이 잡는 거리(8 논리 px × 배율)보다 넉넉히.
const EDGE_RADIUS: i32 = 48;

type Ready = Arc<dyn Fn() + Send + Sync>;

struct Slot {
    /// 모니터 경계(가상 화면 물리 px)
    bounds: PhysRect,
    /// 커서 주변만 먼저 한 것(가상 화면 기준 범위와 함께). 전체가 끝나면 버린다.
    roi: Option<(PhysRect, ink_vision::Prepared)>,
    full: Option<ink_vision::Prepared>,
}

impl Slot {
    /// 사각형 `r` 전체를 덮는 전처리(전체 우선).
    fn covering(&self, r: PhysRect) -> Option<&ink_vision::Prepared> {
        if !inside(r, self.bounds) {
            return None;
        }
        if let Some(p) = self.full.as_ref() {
            return Some(p);
        }
        self.roi.as_ref().filter(|(area, _)| inside(r, *area)).map(|(_, p)| p)
    }

    fn at(&self, p: PhysPoint) -> Option<&ink_vision::Prepared> {
        self.covering(PhysRect::new(p.x, p.y, 1, 1))
    }

    fn origin(&self) -> (i32, i32) {
        (self.bounds.x, self.bounds.y)
    }
}

#[derive(Default)]
struct Inner {
    /// 모드마다 올라간다 — 늦게 끝난 지난 전처리는 버린다.
    generation: u64,
    slots: Vec<Slot>,
}

pub struct VisionEngine {
    inner: Arc<Mutex<Inner>>,
    on_ready: Ready,
    threaded: bool,
}

impl VisionEngine {
    /// 배경 스레드에서 전처리하고, 단계가 끝날 때마다 `on_ready`(어댑터가 메인 스레드로 넘김).
    pub fn new(on_ready: impl Fn() + Send + Sync + 'static) -> Self {
        Self { inner: Arc::default(), on_ready: Arc::new(on_ready), threaded: true }
    }

    /// `prepare`가 돌아올 때 모든 모니터 전처리가 끝나 있다(재생 시험·벤치).
    pub fn blocking() -> Self {
        Self { inner: Arc::default(), on_ready: Arc::new(|| {}), threaded: false }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        lock(&self.inner)
    }

    fn slot_at(inner: &Inner, p: PhysPoint) -> Option<&Slot> {
        inner.slots.iter().find(|s| s.bounds.contains(p))
    }
}

fn lock(m: &Mutex<Inner>) -> MutexGuard<'_, Inner> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn inside(r: PhysRect, area: PhysRect) -> bool {
    r.x >= area.x && r.y >= area.y && r.right() <= area.right() && r.bottom() <= area.bottom()
}

fn shift(r: PhysRect, dx: i32, dy: i32) -> PhysRect {
    PhysRect::new(r.x + dx, r.y + dy, r.w, r.h)
}

/// 아직 같은 모드면 결과를 넣는다. 지난 모드의 것이면 `false`.
fn store(inner: &Mutex<Inner>, generation: u64, index: usize, put: impl FnOnce(&mut Slot)) -> bool {
    let mut g = lock(inner);
    if g.generation != generation {
        return false;
    }
    match g.slots.get_mut(index) {
        Some(s) => {
            put(s);
            true
        }
        None => false,
    }
}

/// 모니터 한 장의 전처리.
struct Job {
    inner: Arc<Mutex<Inner>>,
    ready: Ready,
    generation: u64,
    index: usize,
    bounds: PhysRect,
    frame: Frame,
}

impl Job {
    /// `roi_at` = 커서(이미지 좌표). 모니터가 1024²보다 크면 그 주변을 먼저 한다.
    fn run(self, roi_at: Option<(u32, u32)>) {
        let Job { inner, ready, generation, index, bounds, frame: f } = self;
        let (w, h) = (f.width, f.height);
        if let Some((cx, cy)) = roi_at.filter(|_| w > 2 * ROI_HALF || h > 2 * ROI_HALF) {
            let t = std::time::Instant::now();
            let p = ink_vision::prepare_roi(&f.bgra, w, h, cx, cy, ROI_HALF);
            let (x0, y0) = (cx.saturating_sub(ROI_HALF), cy.saturating_sub(ROI_HALF));
            let (x1, y1) = ((cx + ROI_HALF).min(w), (cy + ROI_HALF).min(h));
            let area = PhysRect::from_edges(bounds.x + x0 as i32, bounds.y + y0 as i32, bounds.x + x1 as i32, bounds.y + y1 as i32);
            log::debug!("vision roi monitor {} {:.1} ms", f.monitor_id, t.elapsed().as_secs_f64() * 1000.0);
            if !store(&inner, generation, index, |s| s.roi = Some((area, p))) {
                return;
            }
            ready();
        }
        let t = std::time::Instant::now();
        let p = ink_vision::prepare_shared(f.bgra, w, h);
        log::debug!("vision full monitor {} {}x{} {:.1} ms", f.monitor_id, w, h, t.elapsed().as_secs_f64() * 1000.0);
        if store(&inner, generation, index, |s| {
            s.full = Some(p);
            s.roi = None;
        }) {
            ready();
        }
    }
}

impl VisionSource for VisionEngine {
    fn prepare(&mut self, frames: &[Frame], layout: &MonitorLayout, cursor: PhysPoint) {
        let mut jobs = Vec::new();
        let generation = {
            let mut g = self.lock();
            g.generation += 1;
            g.slots.clear();
            for f in frames {
                let Some(m) = layout.monitors.iter().find(|m| m.id == f.monitor_id) else { continue };
                if f.bgra.len() < f.width as usize * f.height as usize * 4 {
                    continue;
                }
                // 이미지 크기 기준(배치와 이미지가 어긋나도 이미지 밖은 묻지 않게)
                let bounds = PhysRect::new(m.bounds.x, m.bounds.y, f.width as i32, f.height as i32);
                jobs.push((g.slots.len(), bounds, f.clone()));
                g.slots.push(Slot { bounds, roi: None, full: None });
            }
            g.generation
        };
        for (index, bounds, f) in jobs {
            let job = Job { inner: self.inner.clone(), ready: self.on_ready.clone(), generation, index, bounds, frame: f };
            if !self.threaded {
                job.run(None);
                continue;
            }
            let roi_at = bounds.contains(cursor).then(|| ((cursor.x - bounds.x) as u32, (cursor.y - bounds.y) as u32));
            if let Err(e) = std::thread::Builder::new().name("ink-vision".into()).spawn(move || job.run(roi_at)) {
                log::warn!("vision thread: {e}");
            }
        }
    }

    fn query(&self, p: PhysPoint) -> Vec<VisionCand> {
        let g = self.lock();
        let Some(slot) = Self::slot_at(&g, p) else { return Vec::new() };
        let Some(prep) = slot.at(p) else { return Vec::new() };
        let (ox, oy) = slot.origin();
        prep.query(p.x - ox, p.y - oy)
            .into_iter()
            .map(|c| VisionCand {
                rect: shift(c.rect, ox, oy),
                kind: match c.kind {
                    ink_vision::VKind::Box => VisionKind::Box,
                    ink_vision::VKind::Flat => VisionKind::Flat,
                },
                support: c.support,
            })
            .collect()
    }

    fn support(&self, r: PhysRect) -> Option<f32> {
        let g = self.lock();
        g.slots.iter().find_map(|s| s.covering(r).map(|p| p.support(shift(r, -s.bounds.x, -s.bounds.y))))
    }

    fn snap(&self, r: PhysRect) -> PhysRect {
        let g = self.lock();
        g.slots
            .iter()
            .find_map(|s| s.covering(r).map(|p| shift(p.snap(shift(r, -s.bounds.x, -s.bounds.y), SNAP_TOL), s.bounds.x, s.bounds.y)))
            .unwrap_or(r)
    }

    fn query_occluded(&self, p: PhysPoint, occluders: &[PhysRect]) -> Vec<VisionCand> {
        let g = self.lock();
        let Some(slot) = Self::slot_at(&g, p) else { return Vec::new() };
        let Some(prep) = slot.at(p) else { return Vec::new() };
        let (ox, oy) = slot.origin();
        let occ: Vec<PhysRect> = occluders.iter().map(|o| shift(*o, -ox, -oy)).collect();
        prep.query_with_occluders(p.x - ox, p.y - oy, &occ)
            .into_iter()
            .map(|c| VisionCand {
                rect: shift(c.rect, ox, oy),
                kind: match c.kind {
                    ink_vision::VKind::Box => VisionKind::Box,
                    ink_vision::VKind::Flat => VisionKind::Flat,
                },
                support: c.support,
            })
            .collect()
    }

    fn support_occluded(&self, r: PhysRect, occluders: &[PhysRect]) -> Option<f32> {
        let g = self.lock();
        g.slots.iter().find_map(|s| {
            s.covering(r).map(|p| {
                let occ: Vec<PhysRect> = occluders.iter().map(|o| shift(*o, -s.bounds.x, -s.bounds.y)).collect();
                p.support_with_occluders(shift(r, -s.bounds.x, -s.bounds.y), &occ)
            })
        })
    }

    fn snap_occluded(&self, r: PhysRect, occluders: &[PhysRect]) -> PhysRect {
        let g = self.lock();
        g.slots
            .iter()
            .find_map(|s| {
                s.covering(r).map(|p| {
                    let occ: Vec<PhysRect> = occluders.iter().map(|o| shift(*o, -s.bounds.x, -s.bounds.y)).collect();
                    shift(p.snap_with_occluders(shift(r, -s.bounds.x, -s.bounds.y), SNAP_TOL, &occ), s.bounds.x, s.bounds.y)
                })
            })
            .unwrap_or(r)
    }

    fn refine_image(&self, r: PhysRect) -> PhysRect {
        let g = self.lock();
        g.slots
            .iter()
            .find_map(|s| s.covering(r).map(|p| shift(p.refine_image(shift(r, -s.bounds.x, -s.bounds.y)), s.bounds.x, s.bounds.y)))
            .unwrap_or(r)
    }

    fn edges_near(&self, p: PhysPoint) -> (Vec<i32>, Vec<i32>) {
        let g = self.lock();
        let Some(slot) = Self::slot_at(&g, p) else { return (Vec::new(), Vec::new()) };
        let Some(prep) = slot.at(p) else { return (Vec::new(), Vec::new()) };
        let (ox, oy) = slot.origin();
        let (xs, ys) = prep.edges_near(p.x - ox, p.y - oy, EDGE_RADIUS);
        (xs.into_iter().map(|x| x + ox).collect(), ys.into_iter().map(|y| y + oy).collect())
    }

    fn clear(&mut self) {
        let mut g = self.lock();
        g.generation += 1;
        g.slots.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ink_domain::geometry::Monitor;
    use std::sync::mpsc;
    use std::time::Duration;

    /// 흰 바탕에 회색으로 채운 상자 하나(변마다 경계가 한 줄).
    fn frame(id: u32, w: u32, h: u32, boxed: PhysRect) -> Frame {
        let mut px = vec![255u8; (w * h * 4) as usize];
        for y in boxed.y..boxed.bottom() {
            for x in boxed.x..boxed.right() {
                let o = ((y as u32 * w + x as u32) * 4) as usize;
                px[o..o + 3].copy_from_slice(&[180, 180, 180]);
            }
        }
        Frame { monitor_id: id, width: w, height: h, bgra: Arc::new(px) }
    }

    fn layout() -> MonitorLayout {
        let m = |id, x| Monitor { id, bounds: PhysRect::new(x, 0, 400, 300), work: PhysRect::new(x, 0, 400, 280), scale: 1.0, primary: id == 1 };
        MonitorLayout { monitors: vec![m(1, 0), m(2, 400)] }
    }

    fn engine() -> (VisionEngine, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        (
            VisionEngine::new(move || {
                let _ = tx.lock().unwrap().send(());
            }),
            rx,
        )
    }

    #[test]
    fn 둘째_모니터_상자도_가상_화면_좌표로_찾는다() {
        let (mut v, rx) = engine();
        let frames = vec![frame(1, 400, 300, PhysRect::new(10, 10, 50, 40)), frame(2, 400, 300, PhysRect::new(100, 80, 160, 120))];
        let p = PhysPoint { x: 400 + 180, y: 140 };
        assert!(v.query(p).is_empty(), "준비 전엔 기다리지 않고 빈 답");
        v.prepare(&frames, &layout(), p);
        for _ in 0..2 {
            rx.recv_timeout(Duration::from_secs(10)).expect("전처리 끝 알림");
        }
        let cands = v.query(p);
        let want = PhysRect::new(500, 80, 160, 120);
        assert!(cands.iter().any(|c| c.rect == want), "{cands:?}");
        let sup = v.support(want).expect("준비된 모니터 안");
        assert!(sup > 0.9, "{sup}");
        assert_eq!(v.snap(PhysRect::new(502, 81, 157, 118)), want, "±3 px 안의 경계로 붙음");
        assert!(v.support(PhysRect::new(390, 10, 20, 20)).is_none(), "두 모니터에 걸친 사각형은 모름");
        let (xs, ys) = v.edges_near(PhysPoint { x: 505, y: 85 });
        assert!(xs.contains(&500) && ys.contains(&80), "{xs:?} {ys:?}");
    }

    #[test]
    fn 모드가_끝나면_늦게_온_전처리는_버린다() {
        let (mut v, rx) = engine();
        let frames = vec![frame(1, 400, 300, PhysRect::new(10, 10, 50, 40))];
        let p = PhysPoint { x: 20, y: 20 };
        v.prepare(&frames, &layout(), p);
        v.clear();
        assert!(v.query(p).is_empty());
        // 늦게 끝난 스레드가 있어도 알림이 없거나, 있어도 결과는 비어 있다
        let _ = rx.recv_timeout(Duration::from_millis(500));
        assert!(v.query(p).is_empty());
        assert!(v.support(PhysRect::new(0, 0, 100, 100)).is_none());
    }
}
