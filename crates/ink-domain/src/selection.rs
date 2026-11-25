//! F1 고르기 상태 기계(04 S07~S09, 02 §1.3~1.5) — 순수. 사슬·경계·기록은 `Context`가 준다.
//!
//! - 진입 = 커서 모니터 전체(작업 표시줄 포함). 커서를 3 px 넘게 움직이면 인식 사슬이 이어받는다.
//! - 휠 ↑ 부모 / ↓ 자식 · Tab 창 ↔ 요소 · 클릭 = 확정(S08) · 드래그 = 직접(자석) · 더블클릭·Enter·Ctrl+C = 복사
//! - 확정 뒤 방향키 1 px 이동 · Ctrl+방향 늘리기 · Shift+방향 줄이기 · Ctrl+S 저장
//! - 우클릭 = 범위 다시 정하기 → 한 번 더 = 취소 · Esc = 취소 · Ctrl+A 모니터 → 한 번 더 전체 · R 직전 · , . 지난 범위

use crate::geometry::{PhysPoint, PhysRect};
use crate::magnet::{guides, rect_between, snap_point, Edges, SnapState};
use crate::recog::{first_level, window_index, LevelIntent, Node, NodeKind};
use serde::{Deserialize, Serialize};

/// 진입 뒤 이만큼 움직여야 인식이 시작된다(손떨림·키 누를 때의 미세 이동).
const FRESH_SLOP: i32 = 3;
/// 누른 채 이만큼 움직이면 드래그.
const DRAG_SLOP: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Button {
    Left,
    Right,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    /// KeyboardEvent.key
    pub key: String,
    /// KeyboardEvent.code — 한글 입력 중에도 자리로 판단
    pub code: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Move(PhysPoint),
    /// -1 = 위(넓히기·부모), 1 = 아래(좁히기·자식)
    Wheel(i32),
    Down(PhysPoint, Button),
    Up(PhysPoint, Button),
    DoubleClick(PhysPoint),
    Key(Key),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// 바뀐 것 없음
    None,
    /// 범위·표시가 바뀜
    Redraw,
    Copy(PhysRect),
    Save { rect: PhysRect, quick: bool },
    Cancel,
    /// WASD — OS 커서를 1 px 옮긴다(어댑터)
    SetCursor(PhysPoint),
    /// 한 줄 안내(섬)
    Notice(&'static str),
}

/// 고르기에 필요한 바깥 사정.
pub trait Context {
    /// 커서 점의 사슬(작은 것 → 큰 것)과 그 점의 창 식별자. `elements=false`면 창·모니터만(Tab).
    fn chain_at(&mut self, p: PhysPoint, elements: bool) -> (Vec<Node>, Option<u64>);
    /// 드래그 자석 후보 경계.
    fn edges(&mut self, around: PhysPoint) -> Edges;
    fn monitor_at(&self, p: PhysPoint) -> PhysRect;
    fn virtual_bounds(&self) -> PhysRect;
    fn scale_at(&self, p: PhysPoint) -> f64;
    /// 지금 모니터 구성에서 쓸 수 있는 지난 범위(최신이 앞).
    fn history(&self) -> Vec<PhysRect>;
    /// Alt를 누르고 있는지(자석 끔).
    fn alt_down(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Hover,
    Drag,
    Confirmed,
}

/// 자석 안내선 한 줄(물리 px).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guide {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

/// UI로 나가는 모양(CaptureView의 고르기 부분).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionView {
    pub phase: Phase,
    pub rect: Option<PhysRect>,
    /// 큰 것 → 작은 것(창부터, 창보다 위를 고르면 그 단계부터)
    pub path: Vec<String>,
    /// path 안 지금 단계(없으면 -1)
    pub level: i32,
    pub guides: Vec<Guide>,
    pub magnet: bool,
    pub cursor_included: bool,
    pub help: bool,
}

#[derive(Clone, Debug)]
enum State {
    Hover { chain: Vec<Node>, level: usize, window: Option<u64>, fresh: bool },
    Drag { anchor: PhysPoint, cur: PhysPoint, snap: SnapState, edges: Edges },
    Confirmed { rect: PhysRect },
}

pub struct Selector {
    state: State,
    cursor: PhysPoint,
    entry: PhysPoint,
    pressed: Option<PhysPoint>,
    elements: bool,
    intent: Option<LevelIntent>,
    cursor_included: bool,
    help: bool,
    magnet: bool,
    history_at: Option<usize>,
}

fn fresh_chain(monitor: PhysRect, virt: PhysRect) -> Vec<Node> {
    let n = |rect, kind| Node { rect, kind, score: 0.0, sources: 0, hidden_container: false };
    if monitor == virt {
        vec![n(monitor, NodeKind::Monitor)]
    } else {
        vec![n(monitor, NodeKind::Monitor), n(virt, NodeKind::Virtual)]
    }
}

impl Selector {
    /// 진입: 커서 모니터 전체가 강조(사용자 확정 2026-11-25).
    pub fn new(cursor: PhysPoint, ctx: &dyn Context) -> Selector {
        Selector {
            state: State::Hover { chain: fresh_chain(ctx.monitor_at(cursor), ctx.virtual_bounds()), level: 0, window: None, fresh: true },
            cursor,
            entry: cursor,
            pressed: None,
            elements: true,
            intent: None,
            cursor_included: false,
            help: false,
            magnet: true,
            history_at: None,
        }
    }

    /// 지금 범위.
    pub fn rect(&self) -> Option<PhysRect> {
        match &self.state {
            State::Hover { chain, level, .. } => chain.get(*level).map(|n| n.rect),
            State::Drag { anchor, cur, .. } => Some(rect_between(*anchor, *cur)),
            State::Confirmed { rect } => Some(*rect),
        }
    }

    /// 기록에 남길 고른 방식.
    pub fn mode_name(&self) -> &'static str {
        match &self.state {
            State::Hover { chain, level, .. } => match chain.get(*level).map(|n| n.kind) {
                Some(NodeKind::Window) => "window",
                Some(NodeKind::Monitor | NodeKind::Virtual) => "full",
                _ => "element",
            },
            State::Drag { .. } | State::Confirmed { .. } => "drag",
        }
    }

    pub fn cursor_included(&self) -> bool {
        self.cursor_included
    }

    pub fn view(&self) -> SelectionView {
        let (phase, path, level, guides_v) = match &self.state {
            State::Hover { chain, level, .. } => {
                // 창부터 아래만(창보다 위를 고르면 그 단계부터) — 큰 것 → 작은 것
                let top = window_index(chain).unwrap_or(0).max(*level);
                let path: Vec<String> = chain[..=top.min(chain.len().saturating_sub(1))].iter().rev().map(|n| n.kind.label().to_string()).collect();
                let idx = top as i32 - *level as i32;
                (Phase::Hover, path, idx, Vec::new())
            }
            State::Drag { anchor, cur, snap, .. } => {
                let g = guides(&rect_between(*anchor, *cur), snap).into_iter().map(|(x1, y1, x2, y2)| Guide { x1, y1, x2, y2 }).collect();
                (Phase::Drag, Vec::new(), -1, g)
            }
            State::Confirmed { .. } => (Phase::Confirmed, Vec::new(), -1, Vec::new()),
        };
        SelectionView { phase, rect: self.rect(), path, level, guides: guides_v, magnet: self.magnet, cursor_included: self.cursor_included, help: self.help }
    }

    /// 근거가 늦게 도착했을 때(UIA 색인·V 전처리) — 강조 중이고 이미 움직였으면 사슬을 다시 만든다.
    pub fn refresh(&mut self, ctx: &mut dyn Context) -> Effect {
        match self.state {
            State::Hover { fresh: false, .. } => {
                let before = self.rect();
                self.rehover(ctx);
                if self.rect() != before {
                    Effect::Redraw
                } else {
                    Effect::None
                }
            }
            _ => Effect::None,
        }
    }

    fn rehover(&mut self, ctx: &mut dyn Context) {
        let (chain, window) = ctx.chain_at(self.cursor, self.elements);
        if let State::Hover { window: old, .. } = &self.state {
            if *old != window {
                self.intent = None; // 창이 바뀌면 깊이 기억 초기화(02 §1.3)
            }
        }
        let level = first_level(&chain, self.intent.as_ref());
        self.state = State::Hover { chain, level, window, fresh: false };
    }

    fn confirm(&mut self, rect: PhysRect) -> Effect {
        self.state = State::Confirmed { rect };
        Effect::Redraw
    }

    pub fn handle(&mut self, input: Input, ctx: &mut dyn Context) -> Effect {
        match input {
            Input::Move(p) => self.on_move(p, ctx),
            Input::Wheel(dy) => self.on_wheel(dy, ctx),
            Input::Down(p, Button::Left) => {
                self.cursor = p;
                self.pressed = Some(p);
                Effect::None
            }
            Input::Up(p, Button::Left) => {
                let was = self.pressed.take();
                if let State::Drag { anchor, snap, edges, .. } = &self.state {
                    let end = if self.magnet { snap_point(p, edges, ctx.scale_at(p), &mut snap.clone()) } else { p };
                    let r = rect_between(*anchor, end);
                    return self.confirm(r);
                }
                match (was, self.rect()) {
                    // 클릭 = 지금 강조를 확정(S08: 손잡이·방향키)
                    (Some(_), Some(r)) if matches!(self.state, State::Hover { .. }) => self.confirm(r),
                    _ => Effect::None,
                }
            }
            Input::Down(_, Button::Right) => match self.state {
                State::Hover { .. } => Effect::Cancel,
                _ => {
                    // 범위 다시 정하기 → 한 번 더 = 취소
                    self.pressed = None;
                    self.rehover(ctx);
                    Effect::Redraw
                }
            },
            Input::Up(_, Button::Right) => Effect::None,
            Input::DoubleClick(p) => {
                self.cursor = p;
                self.rect().map_or(Effect::None, Effect::Copy)
            }
            Input::Key(k) => self.on_key(k, ctx),
        }
    }

    fn on_move(&mut self, p: PhysPoint, ctx: &mut dyn Context) -> Effect {
        let moved = p != self.cursor;
        self.cursor = p;
        self.magnet = !ctx.alt_down();
        if let Some(down) = self.pressed {
            let far = (p.x - down.x).abs() > DRAG_SLOP || (p.y - down.y).abs() > DRAG_SLOP;
            if far && !matches!(self.state, State::Drag { .. }) {
                let mut edges = ctx.edges(down);
                edges.extend([ctx.monitor_at(down).x, ctx.monitor_at(down).right()], [ctx.monitor_at(down).y, ctx.monitor_at(down).bottom()]);
                let mut snap = SnapState::default();
                let anchor = if self.magnet { snap_point(down, &edges, ctx.scale_at(down), &mut snap) } else { down };
                self.state = State::Drag { anchor, cur: p, snap: SnapState::default(), edges };
            }
        }
        match &mut self.state {
            State::Drag { cur, snap, edges, .. } => {
                *cur = if self.magnet { snap_point(p, edges, ctx.scale_at(p), snap) } else {
                    *snap = SnapState::default();
                    p
                };
                Effect::Redraw
            }
            State::Hover { fresh: true, .. } => {
                if (p.x - self.entry.x).abs() > FRESH_SLOP || (p.y - self.entry.y).abs() > FRESH_SLOP {
                    self.rehover(ctx);
                    Effect::Redraw
                } else {
                    Effect::None
                }
            }
            State::Hover { .. } if moved => {
                let before = self.rect();
                self.rehover(ctx);
                if self.rect() != before {
                    Effect::Redraw
                } else {
                    Effect::None
                }
            }
            _ => Effect::None,
        }
    }

    fn on_wheel(&mut self, dy: i32, ctx: &mut dyn Context) -> Effect {
        if let State::Hover { fresh: true, chain, .. } = &self.state {
            // 진입 직후 휠: 인식 사슬을 만들고 모니터 자리에서 시작
            let mon = chain.first().map(|n| n.rect);
            self.rehover(ctx);
            if let State::Hover { chain, level, .. } = &mut self.state {
                if let Some(i) = chain.iter().position(|n| Some(n.rect) == mon) {
                    *level = i;
                }
            }
        }
        let State::Hover { chain, level, .. } = &mut self.state else { return Effect::None };
        let next = (*level as i32 - dy).clamp(0, chain.len() as i32 - 1) as usize;
        if next == *level {
            return Effect::None;
        }
        *level = next;
        self.intent = LevelIntent::of(chain, next);
        Effect::Redraw
    }

    fn on_key(&mut self, k: Key, ctx: &mut dyn Context) -> Effect {
        let code = k.code.as_str();
        match (code, k.key.as_str()) {
            ("Escape", _) | (_, "Escape") => return Effect::Cancel,
            ("Enter" | "NumpadEnter", _) => return self.rect().map_or(Effect::None, Effect::Copy),
            ("KeyC", _) if k.ctrl => return self.rect().map_or(Effect::None, Effect::Copy),
            ("KeyS", _) if k.ctrl => return self.rect().map_or(Effect::None, |rect| Effect::Save { rect, quick: k.shift }),
            ("KeyA", _) if k.ctrl => {
                let mon = ctx.monitor_at(self.cursor);
                let target = if self.rect() == Some(mon) { ctx.virtual_bounds() } else { mon };
                return self.confirm(target);
            }
            ("Tab", _) => {
                self.elements = !self.elements;
                self.intent = None;
                self.rehover(ctx);
                return Effect::Redraw;
            }
            ("Backquote", _) => {
                self.cursor_included = !self.cursor_included;
                return Effect::Redraw;
            }
            (_, "?") | ("Slash", _) if k.shift || k.key == "?" => {
                self.help = !self.help;
                return Effect::Redraw;
            }
            ("KeyR", _) if !k.ctrl => {
                return match ctx.history().first() {
                    Some(r) => {
                        self.history_at = Some(0);
                        self.confirm(*r)
                    }
                    None => Effect::Notice("지난 범위가 없어요"),
                };
            }
            ("Comma" | "Period", _) => {
                let list = ctx.history();
                if list.is_empty() {
                    return Effect::Notice("지난 범위가 없어요");
                }
                let i = match (self.history_at, code) {
                    (None, _) => 0,
                    (Some(i), "Comma") => (i + 1).min(list.len() - 1),
                    (Some(i), _) => i.saturating_sub(1),
                };
                self.history_at = Some(i);
                return self.confirm(list[i]);
            }
            _ => {}
        }
        // 방향키: 범위가 정해진 뒤(강조 중이면 그 범위를 확정하고) 1 px
        let dir = match code {
            "ArrowLeft" => Some((-1, 0)),
            "ArrowRight" => Some((1, 0)),
            "ArrowUp" => Some((0, -1)),
            "ArrowDown" => Some((0, 1)),
            _ => None,
        };
        if let Some((dx, dy)) = dir {
            let Some(r) = self.rect() else { return Effect::None };
            let v = ctx.virtual_bounds();
            let next = if k.ctrl {
                // 그쪽 변을 늘림
                let (mut l, mut t, mut rr, mut b) = (r.x, r.y, r.right(), r.bottom());
                match (dx, dy) {
                    (-1, _) => l -= 1,
                    (1, _) => rr += 1,
                    (_, -1) => t -= 1,
                    _ => b += 1,
                }
                PhysRect::from_edges(l.max(v.x), t.max(v.y), rr.min(v.right()), b.min(v.bottom()))
            } else if k.shift {
                // 그쪽 변을 줄임(최소 1×1)
                let (mut l, mut t, mut rr, mut b) = (r.x, r.y, r.right(), r.bottom());
                match (dx, dy) {
                    (-1, _) => rr = (rr - 1).max(l + 1),
                    (1, _) => l = (l + 1).min(rr - 1),
                    (_, -1) => b = (b - 1).max(t + 1),
                    _ => t = (t + 1).min(b - 1),
                }
                PhysRect::from_edges(l, t, rr, b)
            } else {
                let x = (r.x + dx).clamp(v.x, v.right() - r.w);
                let y = (r.y + dy).clamp(v.y, v.bottom() - r.h);
                PhysRect::new(x, y, r.w, r.h)
            };
            return self.confirm(next);
        }
        // WASD: 범위 정하기 전 커서 1 px
        if matches!(self.state, State::Hover { .. }) && !k.ctrl && !k.alt {
            let d = match code {
                "KeyW" => Some((0, -1)),
                "KeyA" => Some((-1, 0)),
                "KeyS" => Some((0, 1)),
                "KeyD" => Some((1, 0)),
                _ => None,
            };
            if let Some((dx, dy)) = d {
                let p = PhysPoint { x: self.cursor.x + dx, y: self.cursor.y + dy };
                self.on_move(p, ctx);
                return Effect::SetCursor(p);
            }
        }
        Effect::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recog::Control;

    struct Ctx {
        hist: Vec<PhysRect>,
        alt: bool,
    }

    const MON: PhysRect = PhysRect::new(0, 0, 1920, 1080);
    const WIN: PhysRect = PhysRect::new(100, 100, 1200, 800);

    fn node(rect: PhysRect, kind: NodeKind) -> Node {
        Node { rect, kind, score: 0.8, sources: 1, hidden_container: false }
    }

    impl Context for Ctx {
        fn chain_at(&mut self, p: PhysPoint, elements: bool) -> (Vec<Node>, Option<u64>) {
            let mut c = Vec::new();
            if elements && WIN.contains(p) {
                c.push(node(PhysRect::new(300, p.y / 40 * 40, 300, 40), NodeKind::Element(Control::DataItem)));
                c.push(node(PhysRect::new(300, 200, 600, 600), NodeKind::Element(Control::Table)));
            }
            if WIN.contains(p) {
                c.push(node(WIN, NodeKind::Window));
            }
            c.push(node(MON, NodeKind::Monitor));
            (c, WIN.contains(p).then_some(7))
        }
        fn edges(&mut self, _around: PhysPoint) -> Edges {
            Edges::from_rects(&[WIN])
        }
        fn monitor_at(&self, _p: PhysPoint) -> PhysRect {
            MON
        }
        fn virtual_bounds(&self) -> PhysRect {
            PhysRect::new(0, 0, 3840, 1080)
        }
        fn scale_at(&self, _p: PhysPoint) -> f64 {
            1.0
        }
        fn history(&self) -> Vec<PhysRect> {
            self.hist.clone()
        }
        fn alt_down(&self) -> bool {
            self.alt
        }
    }

    fn ctx() -> Ctx {
        Ctx { hist: vec![PhysRect::new(10, 10, 50, 50), PhysRect::new(20, 20, 60, 60)], alt: false }
    }

    fn key(code: &str) -> Input {
        Input::Key(Key { key: code.into(), code: code.into(), ..Default::default() })
    }

    #[test]
    fn 진입은_모니터_전체이고_움직이면_인식이_이어받는다() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        assert_eq!(s.rect(), Some(MON));
        assert_eq!(s.handle(Input::Move(PhysPoint { x: 401, y: 331 }), &mut c), Effect::None, "3 px 안은 그대로");
        assert_eq!(s.rect(), Some(MON));
        assert_eq!(s.handle(Input::Move(PhysPoint { x: 410, y: 330 }), &mut c), Effect::Redraw);
        assert_eq!(s.rect(), Some(PhysRect::new(300, 320, 300, 40)), "행");
        let v = s.view();
        assert_eq!(v.path, vec!["창", "표", "행"]);
        assert_eq!(v.level, 2);
    }

    #[test]
    fn 휠은_부모_자식이고_깊이를_기억한다() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        s.handle(Input::Move(PhysPoint { x: 410, y: 330 }), &mut c);
        s.handle(Input::Wheel(-1), &mut c);
        assert_eq!(s.rect(), Some(PhysRect::new(300, 200, 600, 600)), "표");
        s.handle(Input::Move(PhysPoint { x: 420, y: 500 }), &mut c);
        assert_eq!(s.rect(), Some(PhysRect::new(300, 200, 600, 600)), "옮겨도 표");
        s.handle(Input::Wheel(1), &mut c);
        assert_eq!(s.rect().unwrap().h, 40);
    }

    #[test]
    fn 진입_직후_휠은_모니터에서_시작() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        s.handle(Input::Wheel(1), &mut c);
        assert_eq!(s.rect(), Some(WIN), "모니터 → 한 단계 작게 = 창");
    }

    #[test]
    fn tab은_창만_보기() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        s.handle(Input::Move(PhysPoint { x: 410, y: 330 }), &mut c);
        s.handle(key("Tab"), &mut c);
        assert_eq!(s.rect(), Some(WIN));
    }

    #[test]
    fn 클릭은_확정_방향키로_다듬고_enter는_복사() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        s.handle(Input::Move(PhysPoint { x: 410, y: 330 }), &mut c);
        s.handle(Input::Down(PhysPoint { x: 410, y: 330 }, Button::Left), &mut c);
        s.handle(Input::Up(PhysPoint { x: 410, y: 330 }, Button::Left), &mut c);
        assert_eq!(s.view().phase, Phase::Confirmed);
        s.handle(key("ArrowRight"), &mut c);
        assert_eq!(s.rect(), Some(PhysRect::new(301, 320, 300, 40)));
        s.handle(Input::Key(Key { code: "ArrowDown".into(), key: "ArrowDown".into(), ctrl: true, ..Default::default() }), &mut c);
        assert_eq!(s.rect().unwrap().h, 41);
        s.handle(Input::Key(Key { code: "ArrowLeft".into(), key: "ArrowLeft".into(), shift: true, ..Default::default() }), &mut c);
        assert_eq!(s.rect().unwrap().w, 299);
        assert_eq!(s.handle(key("Enter"), &mut c), Effect::Copy(PhysRect::new(301, 320, 299, 41)));
    }

    #[test]
    fn 드래그는_자석에_붙고_alt면_끈다() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 50, y: 50 }, &c);
        s.handle(Input::Down(PhysPoint { x: 104, y: 97 }, Button::Left), &mut c);
        s.handle(Input::Move(PhysPoint { x: 600, y: 500 }), &mut c);
        assert_eq!(s.view().phase, Phase::Drag);
        assert_eq!(s.rect().unwrap().x, 100, "시작점이 창 왼쪽 변에 붙음");
        s.handle(Input::Move(PhysPoint { x: 1296, y: 500 }), &mut c);
        assert_eq!(s.rect().unwrap().right(), 1300, "끝점이 창 오른쪽 변에 붙음");
        assert_eq!(s.view().guides.len(), 1);
        c.alt = true;
        s.handle(Input::Move(PhysPoint { x: 1296, y: 500 }), &mut c);
        assert_eq!(s.rect().unwrap().right(), 1296);
        assert!(!s.view().magnet);
        let e = s.handle(Input::Up(PhysPoint { x: 1296, y: 500 }, Button::Left), &mut c);
        assert_eq!(e, Effect::Redraw);
        assert_eq!(s.view().phase, Phase::Confirmed);
    }

    #[test]
    fn 우클릭은_다시_정하기_한_번_더면_취소() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        s.handle(key("KeyR"), &mut c);
        assert_eq!(s.rect(), Some(PhysRect::new(10, 10, 50, 50)), "R = 직전 범위");
        assert_eq!(s.handle(Input::Down(PhysPoint { x: 400, y: 330 }, Button::Right), &mut c), Effect::Redraw);
        assert_eq!(s.view().phase, Phase::Hover);
        assert_eq!(s.handle(Input::Down(PhysPoint { x: 400, y: 330 }, Button::Right), &mut c), Effect::Cancel);
    }

    #[test]
    fn 지난_범위와_전체_화면과_보조_키() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        s.handle(key("Comma"), &mut c);
        s.handle(key("Comma"), &mut c);
        assert_eq!(s.rect().unwrap().x, 20);
        s.handle(key("Period"), &mut c);
        assert_eq!(s.rect().unwrap().x, 10);
        let ctrl_a = Input::Key(Key { code: "KeyA".into(), key: "a".into(), ctrl: true, ..Default::default() });
        s.handle(ctrl_a.clone(), &mut c);
        assert_eq!(s.rect(), Some(MON));
        s.handle(ctrl_a, &mut c);
        assert_eq!(s.rect(), Some(PhysRect::new(0, 0, 3840, 1080)), "한 번 더 = 전체");
        s.handle(key("Backquote"), &mut c);
        assert!(s.view().cursor_included);
        assert_eq!(s.handle(Input::Key(Key { code: "KeyS".into(), key: "s".into(), ctrl: true, shift: true, ..Default::default() }), &mut c), Effect::Save { rect: PhysRect::new(0, 0, 3840, 1080), quick: true });
        assert_eq!(s.handle(key("Escape"), &mut c), Effect::Cancel);
    }

    #[test]
    fn wasd는_커서를_1px() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        // 한글 입력 중이어도 code로
        let e = s.handle(Input::Key(Key { code: "KeyD".into(), key: "ㅇ".into(), ..Default::default() }), &mut c);
        assert_eq!(e, Effect::SetCursor(PhysPoint { x: 401, y: 330 }));
    }
}
