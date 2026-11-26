//! 실시간 확대(Alt+2, 02 §4 · R-ZOOM-1·2) 순수 계산 — 휠 단계 · 커서 고정점 · 부드러운 추적(0.35 + 10% 죽은 영역) ·
//! 원본 사각형 모니터 가두기 · 위치 지도·후광 자리 · 판서 초기 변환(5.3 `Alt+1` 이어받기).
//!
//! 좌표는 모두 가상 화면 물리 px. 돋보기는 원본 사각형(`source`)을 배율만큼 키워 모니터 전체를 덮는다:
//! `화면 점 = monitor.xy + (원본 점 − source.xy) × scale`.

use crate::geometry::{LogicalPoint, PhysPoint, PhysRect};
use crate::mode::Mode;
use serde::{Deserialize, Serialize};

/// 휠 배율 단계(R-ZOOM-2).
pub const LIVE_ZOOM_STEPS: &[f64] = &[1.25, 1.5, 2.0, 2.5, 3.0, 4.0, 6.0, 8.0];
/// 들어갈 때 배율(F14 섬 `×2.0`).
pub const LIVE_ZOOM_START: f64 = 2.0;
/// 60 Hz 한 프레임에 남은 거리의 몇 배를 따라가나(02 §4).
pub const FOLLOW_ALPHA: f64 = 0.35;
/// 죽은 영역 = 원본 폭·높이의 10%(중심 ±5%) — 작은 손떨림에 화면이 흔들리지 않게.
pub const DEAD_ZONE: f64 = 0.10;
/// 커서가 원본 가장자리에서 이만큼(원본 크기 비율) 안쪽에 남게 — 빨리 움직여도 확대 화면에서 커서를 잃지 않게.
pub const CURSOR_KEEP: f64 = 0.02;
/// 틱이 늦어도 한 번에 이 이상은 따라가지 않는다(멈췄다 깨어난 뒤 튀지 않게).
const MAX_DT: f64 = 0.25;
const EPS: f64 = 1e-9;

/// 전역 칸(Alt+2).
pub const ZOOM_SLOT: &str = "zoom";
/// 확대 화면 안 키(R-KEY-6) — 확대 중에만 잠시 전역으로 잡는다. **끝내기(Esc) 하나뿐**:
/// 배율은 휠로만 바꾸고, 방향키는 아래 앱(PPT 넘기기 등)으로 가야 한다(2026-11-26 사용자 결정 — z-in·z-out 칸 삭제).
pub const ZOOM_EXIT_SLOT: &str = "z-exit";
pub const ZOOM_KEYS: [&str; 1] = [ZOOM_EXIT_SLOT];

/// 판서 칸(Alt+1). 확대 중이면 끝내지 않고 그 순간 확대 화면·배율 그대로 정지 판서로 이어받는다(5.3, R-ZOOM-3).
pub const DRAW_SLOT: &str = "draw";

/// 확대 중 누르면 확대를 끝내고 그 키를 원래대로 처리하는 칸(다른 모드를 여는 키, 03 §3).
/// 핀·타이머·창 항상 위처럼 화면을 바꾸지 않는 키는 확대를 그대로 둔다. 판서(Alt+1)는 이어받기(`ToDraw`).
const ENDS_ZOOM: [&str; 9] = ["region", "window", "record", "board", "last", "full", "active", "delay", "restore"];

/// [판서로](3e-handoff) 초기 확대 여백: 고른 범위가 화면의 88% 안에 꽉 차게(B §2.3).
pub const FIT_MARGIN: f64 = 0.88;
/// 판서 확대 범위(A §2.1 `view.s` 1~8, 판서 엔진 `clampView`와 같음).
pub const DRAW_MIN_ZOOM: f64 = 1.0;
pub const DRAW_MAX_ZOOM: f64 = 8.0;

/// `zoom.follow` 설정(05 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FollowMode {
    /// 0.35 감쇠 + 10% 죽은 영역
    Smooth,
    /// 중심 = 커서(죽은 영역 없음)
    Instant,
}

impl FollowMode {
    pub fn from_setting(v: &str) -> Self {
        if v == "instant" {
            FollowMode::Instant
        } else {
            FollowMode::Smooth
        }
    }
}

/// 위치 지도용 0..1 사각형(모니터 기준).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NormRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// 지금 확대 상태. 중심은 실수(부드러운 추적 보간), 원본은 돋보기에 넘길 정수 사각형.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveZoomView {
    pub monitor_id: u32,
    pub monitor_bounds: PhysRect,
    /// 배율(1.25 ~ 8.0)
    pub scale: f64,
    /// 돋보기가 비추는 원본 사각형(가상 화면 물리 px, 모니터 안에 가둠)
    pub source: PhysRect,
    pub center_x: f64,
    pub center_y: f64,
}

fn clamp_scale(s: f64) -> f64 {
    let lo = LIVE_ZOOM_STEPS[0];
    let hi = LIVE_ZOOM_STEPS[LIVE_ZOOM_STEPS.len() - 1];
    if s.is_finite() {
        s.clamp(lo, hi)
    } else {
        LIVE_ZOOM_START
    }
}

/// 실수 구간 가두기(구간이 뒤집히면 가운데).
fn clamp_f(v: f64, lo: f64, hi: f64) -> f64 {
    if lo > hi {
        (lo + hi) / 2.0
    } else {
        v.clamp(lo, hi)
    }
}

impl LiveZoomView {
    /// 커서 자리를 중심으로 시작(모니터 밖으로 나가지 않게 가둔다).
    pub fn new(monitor_id: u32, monitor_bounds: PhysRect, scale: f64, cursor: PhysPoint) -> Self {
        let mut v = LiveZoomView { monitor_id, monitor_bounds, scale: clamp_scale(scale), source: PhysRect::default(), center_x: cursor.x as f64, center_y: cursor.y as f64 };
        v.settle();
        v
    }

    /// 원본 크기(실수) = 모니터 / 배율.
    pub fn source_size(&self) -> (f64, f64) {
        (self.monitor_bounds.w.max(1) as f64 / self.scale, self.monitor_bounds.h.max(1) as f64 / self.scale)
    }

    /// 중심을 모니터 안으로 가두고 정수 원본 사각형을 다시 만든다.
    fn settle(&mut self) {
        let m = self.monitor_bounds;
        let (wf, hf) = self.source_size();
        self.center_x = clamp_f(self.center_x, m.x as f64 + wf / 2.0, m.right() as f64 - wf / 2.0);
        self.center_y = clamp_f(self.center_y, m.y as f64 + hf / 2.0, m.bottom() as f64 - hf / 2.0);
        // 올림 — 원본 × 배율이 모니터보다 작으면 확대 화면 오른쪽·아래에 빈 줄이 생긴다(1366 / 2.5 등)
        let w = ((wf - 1e-6).ceil() as i32).clamp(1, m.w.max(1));
        let h = ((hf - 1e-6).ceil() as i32).clamp(1, m.h.max(1));
        let x = ((self.center_x - w as f64 / 2.0).round() as i32).clamp(m.x, (m.right() - w).max(m.x));
        let y = ((self.center_y - h as f64 / 2.0).round() as i32).clamp(m.y, (m.bottom() - h).max(m.y));
        self.source = PhysRect::new(x, y, w, h);
    }
}

/// 휠·키 한 칸: `dy < 0` = 더 확대(위), `dy > 0` = 덜 확대(아래). 끝 단계에서는 멈춘다.
/// 단계에 없는 배율이면 그 방향의 다음 단계로 간다(판서 단계표에도 같은 함수를 쓸 수 있다).
pub fn next_zoom_step(current: f64, dy: i32, steps: &[f64]) -> f64 {
    let (Some(&first), Some(&last)) = (steps.first(), steps.last()) else { return current };
    let mut s = current;
    for _ in 0..dy.unsigned_abs().min(steps.len() as u32) {
        s = if dy < 0 { steps.iter().copied().find(|&x| x > s + EPS).unwrap_or(last) } else { steps.iter().rev().copied().find(|&x| x < s - EPS).unwrap_or(first) };
    }
    s
}

/// 죽은 영역을 넘은 양(안이면 0): `d − clamp(d, −half, half)`.
fn beyond(d: f64, half: f64) -> f64 {
    d - d.clamp(-half, half)
}

/// 프레임 하나 따라가기(02 §4). 커서가 이 모니터 밖이면 그대로 둔다.
/// `Smooth`: 축마다 커서가 중심 ±5%(죽은 영역)를 **넘은 만큼만** `× α`로 따라간다 — 커서가 죽은 영역 가장자리에
/// 오도록 부드럽게 수렴하고, 경계에서 멈췄다 한꺼번에 튀지 않는다. α는 60 Hz에서 0.35와 같은 프레임 독립 감쇠.
pub fn step_follow(view: &mut LiveZoomView, cursor: PhysPoint, mode: FollowMode, dt_sec: f64) {
    if !view.monitor_bounds.contains(cursor) {
        return;
    }
    let (w, h) = view.source_size();
    let (cx, cy) = (cursor.x as f64, cursor.y as f64);
    match mode {
        FollowMode::Instant => {
            view.center_x = cx;
            view.center_y = cy;
        }
        FollowMode::Smooth => {
            let ex = beyond(cx - view.center_x, w * DEAD_ZONE / 2.0);
            let ey = beyond(cy - view.center_y, h * DEAD_ZONE / 2.0);
            if ex != 0.0 || ey != 0.0 {
                let dt = if dt_sec.is_finite() { dt_sec.clamp(0.0, MAX_DT) } else { 0.0 };
                let a = 1.0 - (1.0 - FOLLOW_ALPHA).powf(dt * 60.0);
                view.center_x += ex * a;
                view.center_y += ey * a;
            }
        }
    }
    // 커서를 원본 안에 남긴다(확대 화면 밖으로 나간 커서는 보이지 않는다)
    let (kx, ky) = (w / 2.0 - w * CURSOR_KEEP, h / 2.0 - h * CURSOR_KEEP);
    view.center_x = clamp_f(view.center_x, cx - kx, cx + kx);
    view.center_y = clamp_f(view.center_y, cy - ky, cy + ky);
    view.settle();
}

/// 배율을 바꾸되 커서 아래 점이 화면에서 제자리에 있게(고정점). 커서가 확대 화면 밖이면 중심을 고정한다.
pub fn zoom_to(view: &mut LiveZoomView, cursor: PhysPoint, scale: f64) {
    let next = clamp_scale(scale);
    if (next - view.scale).abs() < EPS {
        return;
    }
    let (w, h) = view.source_size();
    let (left, top) = (view.center_x - w / 2.0, view.center_y - h / 2.0);
    let (fx, fy) = if view.monitor_bounds.contains(cursor) && view.source.contains(cursor) { (cursor.x as f64, cursor.y as f64) } else { (view.center_x, view.center_y) };
    // 그 점의 화면 자리(모니터 왼쪽 위 기준 물리 px)
    let (px, py) = ((fx - left) * view.scale, (fy - top) * view.scale);
    view.scale = next;
    let (w2, h2) = view.source_size();
    view.center_x = fx - px / next + w2 / 2.0;
    view.center_y = fy - py / next + h2 / 2.0;
    view.settle();
}

/// 원본 점이 확대 화면에서 놓이는 자리(가상 화면 물리 px, 실수).
pub fn screen_of(view: &LiveZoomView, p: PhysPoint) -> (f64, f64) {
    let m = view.monitor_bounds;
    (m.x as f64 + (p.x - view.source.x) as f64 * view.scale, m.y as f64 + (p.y - view.source.y) as f64 * view.scale)
}

/// 확대 화면 위 커서(그 모니터 HUD 창 안 CSS px) — 44 px 주황 후광 자리. 커서가 원본 밖이면 `None`.
pub fn cursor_css(view: &LiveZoomView, cursor: PhysPoint, monitor_scale: f64) -> Option<LogicalPoint> {
    if !view.source.contains(cursor) {
        return None;
    }
    let ms = if monitor_scale > 0.0 { monitor_scale } else { 1.0 };
    // 커서 끝 화소의 가운데
    Some(LogicalPoint {
        x: ((cursor.x - view.source.x) as f64 + 0.5) * view.scale / ms,
        y: ((cursor.y - view.source.y) as f64 + 0.5) * view.scale / ms,
    })
}

/// 위치 지도(MiniMap) 사각형 = 원본 / 모니터.
pub fn viewport(view: &LiveZoomView) -> NormRect {
    let m = view.monitor_bounds;
    let (mw, mh) = (m.w.max(1) as f64, m.h.max(1) as f64);
    NormRect { x: (view.source.x - m.x) as f64 / mw, y: (view.source.y - m.y) as f64 / mh, w: view.source.w as f64 / mw, h: view.source.h as f64 / mh }
}

/// 5.3 `Alt+1` 이어받기: 판서 초기 변환 `(s, tx, ty)`(A 시안 `DrawViewTransform`).
/// 판서 문서 = 그 모니터 논리 px, `화면 CSS = 문서 × s + t`. 확대 화면 왼쪽 위가 판서 `viewToDoc(0, 0)`이 된다.
pub fn to_draw_transform(view: &LiveZoomView, monitor_scale: f64) -> (f64, f64, f64) {
    let ms = if monitor_scale > 0.0 { monitor_scale } else { 1.0 };
    let m = view.monitor_bounds;
    let s = view.scale;
    (s, -((view.source.x - m.x) as f64) / ms * s, -((view.source.y - m.y) as f64) / ms * s)
}

/// 판서 엔진 `clampAxis`와 같은 가두기: 내용이 화면보다 크면 `[화면 − 내용, 0]`, 작으면 `[0, 남는 폭]`.
fn clamp_axis(t: f64, view_len: f64, content_len: f64) -> f64 {
    let free = view_len - content_len;
    if free <= 0.0 {
        t.clamp(free, 0.0)
    } else {
        t.clamp(0.0, free)
    }
}

/// 3e-handoff `[판서로]`: 방금 고른 범위(가상 화면 물리 px)가 그 모니터 판서 화면에 꽉 차게 하는 초기 변환 `(s, tx, ty)`.
/// 판서 문서·화면 = 그 모니터 논리 px(`화면 CSS = 문서 × s + t`). 범위는 모니터 안으로 자른다(여러 모니터에 걸치면
/// 이 모니터 몫만). 배율 = 화면 88%에 맞춤을 0.01 단위로 내림(범위가 여백 밖으로 나가지 않게) → 1~8로 가둠.
/// 범위 가운데를 화면 가운데에 놓은 뒤 문서 밖이 보이지 않게 가둔다. 범위가 모니터와 안 겹치면 1×.
pub fn fit_rect_view(monitor_bounds: PhysRect, monitor_scale: f64, rect: PhysRect) -> (f64, f64, f64) {
    let ms = if monitor_scale.is_finite() && monitor_scale > 0.0 { monitor_scale } else { 1.0 };
    let m = monitor_bounds;
    let Some(r) = rect.intersect(&m).filter(|r| r.w > 0 && r.h > 0) else { return (DRAW_MIN_ZOOM, 0.0, 0.0) };
    let (doc_w, doc_h) = (m.w.max(1) as f64 / ms, m.h.max(1) as f64 / ms);
    let (sel_w, sel_h) = (r.w as f64 / ms, r.h as f64 / ms);
    let fit = (FIT_MARGIN * doc_w / sel_w).min(FIT_MARGIN * doc_h / sel_h);
    let s = ((fit * 100.0 + EPS).floor() / 100.0).clamp(DRAW_MIN_ZOOM, DRAW_MAX_ZOOM);
    let (cx, cy) = ((r.x - m.x) as f64 / ms + sel_w / 2.0, (r.y - m.y) as f64 / ms + sel_h / 2.0);
    let tx = clamp_axis(doc_w / 2.0 - cx * s, doc_w, doc_w * s);
    let ty = clamp_axis(doc_h / 2.0 - cy * s, doc_h, doc_h * s);
    (s, tx, ty)
}

/// 섬·배지 글자: `×2.0`, `×1.25`.
pub fn format_scale(s: f64) -> String {
    let hundredths = (s * 100.0).round() as i64;
    if hundredths % 10 == 0 {
        format!("×{:.1}", hundredths as f64 / 100.0)
    } else {
        format!("×{:.2}", hundredths as f64 / 100.0)
    }
}

/// 위치 지도 이름("모니터 1") — 배치 순서. 창 제목·장치 이름은 쓰지 않는다.
pub fn monitor_label(index: usize) -> String {
    format!("모니터 {}", index + 1)
}

/// 단축키 칸 + 지금 모드 → 확대가 맡을 일. `None` = 확대와 무관(원래 처리로).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomCommand {
    /// 쉬다가 Alt+2
    Start,
    /// 캡처 고르기 중 Alt+2 = 캡처를 끝내고 확대
    SwitchFromCapture,
    /// 확대 중 Alt+2 · `z-exit`
    End,
    /// 확대 중 다른 모드 키 — 확대를 끝내고 그 키를 원래대로 처리
    EndThenPass,
    /// 확대 중 Alt+1 — 그 순간 확대 화면·배율 그대로 정지 판서로 이어받기(5.3, R-ZOOM-3)
    ToDraw,
}

/// 모드를 빠짐없이 나열하지 않는다(다른 흐름이 모드를 더해도 이 판정은 그대로 — 모르는 모드에서는 `None`).
pub fn on_hotkey(mode: &Mode, slot: &str) -> Option<ZoomCommand> {
    let zooming = matches!(mode, Mode::LiveZoom { .. });
    if slot == ZOOM_SLOT {
        return if zooming {
            Some(ZoomCommand::End)
        } else if matches!(mode, Mode::Idle) {
            Some(ZoomCommand::Start)
        } else if matches!(mode, Mode::Capture { .. }) {
            Some(ZoomCommand::SwitchFromCapture)
        } else {
            None
        };
    }
    if !zooming {
        return None;
    }
    match slot {
        ZOOM_EXIT_SLOT => Some(ZoomCommand::End),
        DRAW_SLOT => Some(ZoomCommand::ToDraw),
        s if ENDS_ZOOM.contains(&s) => Some(ZoomCommand::EndThenPass),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::CaptureKind;

    const FHD: PhysRect = PhysRect::new(0, 0, 1920, 1080);

    fn p(x: i32, y: i32) -> PhysPoint {
        PhysPoint { x, y }
    }

    #[test]
    fn 휠_단계는_끝에서_멈추고_단계_밖_값은_다음_단계로() {
        let s = LIVE_ZOOM_STEPS;
        assert_eq!(next_zoom_step(2.0, -1, s), 2.5);
        assert_eq!(next_zoom_step(2.0, 1, s), 1.5);
        assert_eq!(next_zoom_step(8.0, -1, s), 8.0);
        assert_eq!(next_zoom_step(1.25, 1, s), 1.25);
        assert_eq!(next_zoom_step(1.75, -1, s), 2.0);
        assert_eq!(next_zoom_step(1.75, 1, s), 1.5);
        assert_eq!(next_zoom_step(2.0, -3, s), 4.0);
        assert_eq!(next_zoom_step(2.0, 0, s), 2.0);
        assert_eq!(next_zoom_step(3.0, 1, &[]), 3.0);
    }

    #[test]
    fn 시작은_커서_중심이고_원본은_모니터_안() {
        let v = LiveZoomView::new(0, FHD, 2.0, p(960, 540));
        assert_eq!(v.source, PhysRect::new(480, 270, 960, 540));
        // 왼쪽 위 구석 커서 → 원본은 모니터 왼쪽 위에 붙는다
        let v = LiveZoomView::new(0, FHD, 2.0, p(5, 5));
        assert_eq!(v.source, PhysRect::new(0, 0, 960, 540));
        assert_eq!((v.center_x, v.center_y), (480.0, 270.0));
        // 이상한 배율은 단계 범위로
        assert_eq!(LiveZoomView::new(0, FHD, 50.0, p(0, 0)).scale, 8.0);
        assert_eq!(LiveZoomView::new(0, FHD, f64::NAN, p(0, 0)).scale, LIVE_ZOOM_START);
        // 나누어떨어지지 않는 모니터: 원본 × 배율 ≥ 모니터(빈 줄 없음)
        let odd = PhysRect::new(0, 0, 1366, 768);
        for s in LIVE_ZOOM_STEPS {
            let v = LiveZoomView::new(0, odd, *s, p(683, 384));
            assert!(v.source.w as f64 * s >= 1366.0 && v.source.h as f64 * s >= 768.0, "{s}: {:?}", v.source);
            assert!(v.source.right() <= 1366 && v.source.bottom() <= 768);
        }
    }

    #[test]
    fn 커서_죽은영역_10퍼센트_미이동_및_부드러운_추적() {
        let mut v = LiveZoomView::new(0, FHD, 2.0, p(960, 540));
        let before = v;
        // 원본 960×540 → 죽은 영역 ±48 × ±27
        step_follow(&mut v, p(960 + 47, 540 - 26), FollowMode::Smooth, 1.0 / 60.0);
        assert_eq!(v, before, "죽은 영역 안 손떨림에는 움직이지 않는다");
        // 바깥 → 한 프레임에 죽은 영역을 넘은 만큼(200 − 48)의 0.35, 넘지 않은 축은 그대로
        step_follow(&mut v, p(960 + 200, 540 + 10), FollowMode::Smooth, 1.0 / 60.0);
        assert!((v.center_x - (960.0 + 152.0 * 0.35)).abs() < 1e-9, "{}", v.center_x);
        assert_eq!(v.center_y, 540.0);
        // 프레임 독립: 30 Hz 한 번 = 60 Hz 두 번
        let mut a = before;
        let mut b = before;
        step_follow(&mut a, p(1400, 540), FollowMode::Smooth, 1.0 / 30.0);
        step_follow(&mut b, p(1400, 540), FollowMode::Smooth, 1.0 / 60.0);
        step_follow(&mut b, p(1400, 540), FollowMode::Smooth, 1.0 / 60.0);
        assert!((a.center_x - b.center_x).abs() < 1e-6, "{} vs {}", a.center_x, b.center_x);
        // 오래 따라가면 커서가 죽은 영역 가장자리(중심 + 48)에 오도록 수렴한다
        let mut c = before;
        for _ in 0..120 {
            step_follow(&mut c, p(1300, 540), FollowMode::Smooth, 1.0 / 60.0);
        }
        assert!((c.center_x - (1300.0 - 48.0)).abs() < 1e-6, "{}", c.center_x);
    }

    #[test]
    fn 죽은영역_경계에서_멈췄다_튀지_않는다() {
        // 경계에 붙은 뒤 커서가 2 px 더 나가면 중심도 그만큼의 0.35만 — 한꺼번에 뛰지 않는다
        let mut v = LiveZoomView::new(0, FHD, 2.0, p(960, 540));
        for _ in 0..200 {
            step_follow(&mut v, p(1300, 540), FollowMode::Smooth, 1.0 / 60.0);
        }
        let mut prev = v.center_x;
        for x in [1302, 1304, 1306, 1305, 1307] {
            step_follow(&mut v, p(x, 540), FollowMode::Smooth, 1.0 / 60.0);
            let moved = v.center_x - prev;
            let want = 0.35 * (x as f64 - prev - 48.0).max(0.0);
            assert!((moved - want).abs() < 1e-9 && moved < 2.0, "{x}: {moved} vs {want}");
            prev = v.center_x;
        }
        // 안쪽으로 돌아오면(죽은 영역 안) 멈춘다
        let still = v;
        step_follow(&mut v, p(1290, 540), FollowMode::Smooth, 1.0 / 60.0);
        assert_eq!(v, still);
    }

    #[test]
    fn 바로_따라가기는_죽은영역_없이_중심이_커서() {
        let mut v = LiveZoomView::new(0, FHD, 2.0, p(960, 540));
        step_follow(&mut v, p(970, 545), FollowMode::Instant, 1.0 / 60.0);
        assert_eq!((v.center_x, v.center_y), (970.0, 545.0));
        assert_eq!(FollowMode::from_setting("instant"), FollowMode::Instant);
        assert_eq!(FollowMode::from_setting("smooth"), FollowMode::Smooth);
        assert_eq!(FollowMode::from_setting("??"), FollowMode::Smooth);
    }

    #[test]
    fn 빠른_커서도_원본_안에_남는다() {
        let mut v = LiveZoomView::new(0, FHD, 4.0, p(960, 540));
        // 원본 480×270. 한 프레임에 커서가 600 px 뛰어도 원본 안
        step_follow(&mut v, p(1560, 540), FollowMode::Smooth, 1.0 / 60.0);
        assert!(v.source.contains(p(1560, 540)), "{:?}", v.source);
        let keep = (480.0 * CURSOR_KEEP) as i32;
        assert!(v.source.right() - 1560 >= keep, "가장자리에서 {keep} px 안쪽");
    }

    #[test]
    fn 원본사각형_모니터_경계_가두기_및_음수좌표() {
        // 왼쪽 음수 모니터(150%가 아니어도 좌표만 음수)
        let left = PhysRect::new(-1920, -200, 1920, 1080);
        let mut v = LiveZoomView::new(7, left, 3.0, p(-1915, -195));
        assert_eq!(v.source.x, -1920);
        assert_eq!(v.source.y, -200);
        for target in [p(-1, 879), p(-1920, -200), p(-960, 340), p(-1, -200)] {
            for _ in 0..90 {
                step_follow(&mut v, target, FollowMode::Smooth, 1.0 / 60.0);
                assert!(v.source.x >= left.x && v.source.right() <= left.right(), "{:?}", v.source);
                assert!(v.source.y >= left.y && v.source.bottom() <= left.bottom(), "{:?}", v.source);
            }
        }
        // 커서가 다른 모니터로 가면 그대로 둔다
        let before = v;
        step_follow(&mut v, p(100, 100), FollowMode::Smooth, 1.0 / 60.0);
        assert_eq!(v, before);
    }

    #[test]
    fn 휠_확대는_커서_아래_점이_제자리() {
        let mut v = LiveZoomView::new(0, FHD, 2.0, p(960, 540));
        let c = p(1100, 600);
        let before = screen_of(&v, c);
        for s in [2.5, 3.0, 4.0, 6.0, 8.0, 6.0, 3.0, 1.5, 1.25] {
            zoom_to(&mut v, c, s);
            let after = screen_of(&v, c);
            assert!((after.0 - before.0).abs() <= s && (after.1 - before.1).abs() <= s, "{s}: {before:?} → {after:?}");
            assert_eq!(v.scale, s);
        }
        // 가장자리에서는 가두기가 이긴다(원본은 모니터 밖으로 안 나감)
        let mut e = LiveZoomView::new(0, FHD, 4.0, p(10, 10));
        zoom_to(&mut e, p(10, 10), 1.25);
        assert_eq!((e.source.x, e.source.y), (0, 0));
        assert_eq!((e.source.w, e.source.h), (1536, 864));
    }

    #[test]
    fn 실시간확대_판서_초기변환_좌표일치() {
        // 150% 모니터(물리 2880×1800, 논리 1920×1200), 가상 화면 오른쪽
        let m = PhysRect::new(1920, 0, 2880, 1800);
        let v = LiveZoomView::new(1, m, 2.5, p(3000, 900));
        let (s, tx, ty) = to_draw_transform(&v, 1.5);
        assert_eq!(s, 2.5);
        // viewToDoc(0, 0) = 원본 왼쪽 위(문서 = 모니터 논리 px)
        let doc = ((0.0 - tx) / s, (0.0 - ty) / s);
        let want = ((v.source.x - m.x) as f64 / 1.5, (v.source.y - m.y) as f64 / 1.5);
        assert!((doc.0 - want.0).abs() <= 0.5 && (doc.1 - want.1).abs() <= 0.5, "{doc:?} vs {want:?}");
        // 화면 오른쪽 아래(논리 1920×1200)도 원본 오른쪽 아래와 맞는다
        let br = ((1920.0 - tx) / s, (1200.0 - ty) / s);
        let want_br = ((v.source.right() - m.x) as f64 / 1.5, (v.source.bottom() - m.y) as f64 / 1.5);
        assert!((br.0 - want_br.0).abs() <= 0.5 && (br.1 - want_br.1).abs() <= 0.5, "{br:?} vs {want_br:?}");
        // 판서 가두기 범위(tx ∈ [vw − docW·s, 0]) 안
        assert!(tx <= 0.0 && tx >= 1920.0 - 1920.0 * s - 1e-9);
        assert!(ty <= 0.0 && ty >= 1200.0 - 1200.0 * s - 1e-9);
    }

    #[test]
    fn 후광_자리와_위치_지도() {
        let m = PhysRect::new(1920, 0, 2880, 1800);
        let v = LiveZoomView::new(1, m, 2.0, p(1920 + 1440, 900));
        assert_eq!(v.source, PhysRect::new(1920 + 720, 450, 1440, 900));
        let c = cursor_css(&v, p(1920 + 1440, 900), 1.5).unwrap();
        // 화면 물리 = (720 + 0.5) × 2 = 1441 → CSS 960.67
        assert!((c.x - 1441.0 / 1.5).abs() < 1e-9 && (c.y - 901.0 / 1.5).abs() < 1e-9, "{c:?}");
        assert_eq!(cursor_css(&v, p(1920, 0), 1.5), None, "원본 밖이면 후광 없음");
        let n = viewport(&v);
        assert_eq!(n, NormRect { x: 0.25, y: 0.25, w: 0.5, h: 0.5 });
    }

    #[test]
    fn 배율_글자와_모니터_이름() {
        assert_eq!(format_scale(2.0), "×2.0");
        assert_eq!(format_scale(1.25), "×1.25");
        assert_eq!(format_scale(1.5), "×1.5");
        assert_eq!(format_scale(8.0), "×8.0");
        assert_eq!(monitor_label(0), "모니터 1");
    }

    #[test]
    fn 단축키_판정_시작_끝_전환_확대안키() {
        let idle = Mode::Idle;
        let cap = Mode::Capture { session: 1, kind: CaptureKind::Region, started_ms: 0 };
        let zoom = Mode::LiveZoom { session: 2, monitor_id: 0, started_ms: 0 };
        assert_eq!(on_hotkey(&idle, "zoom"), Some(ZoomCommand::Start));
        assert_eq!(on_hotkey(&cap, "zoom"), Some(ZoomCommand::SwitchFromCapture));
        assert_eq!(on_hotkey(&zoom, "zoom"), Some(ZoomCommand::End));
        assert_eq!(on_hotkey(&zoom, "z-exit"), Some(ZoomCommand::End));
        assert_eq!(ZOOM_KEYS, ["z-exit"], "확대 중 잡는 키는 Esc(끝) 하나 — 방향키는 아래 앱으로");
        // 확대 밖에서는 확대 안 키가 아무 일도 안 한다, 없어진 칸(z-in·z-out)은 언제나 무관
        assert_eq!(on_hotkey(&idle, "z-exit"), None);
        assert_eq!(on_hotkey(&zoom, "z-in"), None);
        assert_eq!(on_hotkey(&zoom, "z-out"), None);
        // 다른 모드 키는 확대를 끝내고 넘긴다, 핀·타이머는 확대를 그대로 둔다
        for s in ["region", "window", "record", "board", "last", "full", "active", "delay", "restore"] {
            assert_eq!(on_hotkey(&zoom, s), Some(ZoomCommand::EndThenPass), "{s}");
            assert_eq!(on_hotkey(&idle, s), None, "{s}");
        }
        for s in ["pin", "pinToggle", "timer", "topmost", "pause"] {
            assert_eq!(on_hotkey(&zoom, s), None, "{s}");
        }
        // 5.3: 확대 중 Alt+1은 끝내지 않고 이어받기, 확대 밖에서는 판서 흐름이 맡는다
        assert_eq!(on_hotkey(&zoom, DRAW_SLOT), Some(ZoomCommand::ToDraw));
        assert_eq!(on_hotkey(&idle, DRAW_SLOT), None);
        assert_eq!(on_hotkey(&cap, DRAW_SLOT), None);
    }

    /// 판서 화면 좌표(CSS) → 문서(모니터 논리 px).
    fn view_to_doc(x: f64, y: f64, (s, tx, ty): (f64, f64, f64)) -> (f64, f64) {
        ((x - tx) / s, (y - ty) / s)
    }

    #[test]
    fn 범위_이어받기_초기확대_계산() {
        // 400×200 범위를 1920×1080 가운데에: 화면 88%에 맞춤 = min(4.224, 4.752) → 4.22(0.01 내림)
        let (s, tx, ty) = fit_rect_view(FHD, 1.0, PhysRect::new(760, 440, 400, 200));
        assert_eq!(s, 4.22);
        // 범위 가운데가 화면 가운데
        let c = view_to_doc(960.0, 540.0, (s, tx, ty));
        assert!((c.0 - 960.0).abs() < 1e-9 && (c.1 - 540.0).abs() < 1e-9, "{c:?}");
        // 범위 전체가 화면 88% 안(가로가 꽉 참)
        let left = 760.0 * s + tx;
        let right = 1160.0 * s + tx;
        assert!(left >= 1920.0 * 0.06 - 1e-6 && right <= 1920.0 * 0.94 + 1e-6, "{left}..{right}");
        assert!(right - left >= 1920.0 * 0.87, "꽉 차게: {}", right - left);

        // 아주 작은 범위는 8배에서 멈추고, 모니터 대부분이면 1×(이동 없음)
        assert_eq!(fit_rect_view(FHD, 1.0, PhysRect::new(100, 100, 20, 10)).0, DRAW_MAX_ZOOM);
        assert_eq!(fit_rect_view(FHD, 1.0, PhysRect::new(0, 0, 1900, 1000)), (1.0, 0.0, 0.0));
        // 안 겹치거나 빈 범위 = 1×
        assert_eq!(fit_rect_view(FHD, 1.0, PhysRect::new(5000, 0, 100, 100)), (1.0, 0.0, 0.0));
        assert_eq!(fit_rect_view(FHD, 1.0, PhysRect::new(10, 10, 0, 50)), (1.0, 0.0, 0.0));
    }

    #[test]
    fn 범위_이어받기는_모니터_가장자리와_배율_모니터에서도_문서_안() {
        // 왼쪽 위 구석 범위 — 가운데로 못 옮기면 가둔다(문서 밖이 보이지 않음)
        let v = fit_rect_view(FHD, 1.0, PhysRect::new(0, 0, 300, 200));
        assert!(v.0 > 1.0);
        assert_eq!((v.1, v.2), (0.0, 0.0), "왼쪽 위에 붙는다");
        let tl = view_to_doc(0.0, 0.0, v);
        assert!(tl.0.abs() < 1e-9 && tl.1.abs() < 1e-9);

        // 150% 모니터(가상 화면 오른쪽, 물리 2880×1800 = 논리 1920×1200)의 오른쪽 아래 범위
        let m = PhysRect::new(1920, 0, 2880, 1800);
        let r = PhysRect::new(1920 + 2400, 1500, 480, 300);
        let v = fit_rect_view(m, 1.5, r);
        // 문서 크기 1920×1200, 범위 논리 320×200 → min(5.28, 5.28) = 5.28
        assert_eq!(v.0, 5.28);
        assert!(v.1 <= 0.0 && v.1 >= 1920.0 - 1920.0 * v.0 - 1e-9, "{v:?}");
        assert!(v.2 <= 0.0 && v.2 >= 1200.0 - 1200.0 * v.0 - 1e-9, "{v:?}");
        // 범위가 화면 안에 다 보인다(오른쪽 아래로 가둠)
        let (l, t) = ((2400.0 / 1.5) * v.0 + v.1, (1500.0 / 1.5) * v.0 + v.2);
        let (rr, b) = ((2880.0 / 1.5) * v.0 + v.1, (1800.0 / 1.5) * v.0 + v.2);
        assert!(l >= -1e-6 && t >= -1e-6 && rr <= 1920.0 + 1e-6 && b <= 1200.0 + 1e-6, "{l},{t} → {rr},{b}");

        // 두 모니터에 걸친 범위는 이 모니터 몫만으로 맞춘다
        let across = fit_rect_view(FHD, 1.0, PhysRect::new(1720, 440, 400, 200));
        let only = fit_rect_view(FHD, 1.0, PhysRect::new(1720, 440, 200, 200));
        assert_eq!(across, only);
    }
}
