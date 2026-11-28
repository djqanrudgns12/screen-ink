//! F1 고르기 상태 기계(04 S07~S09, 02 §1.3~1.5) — 순수. 사슬·경계·기록은 `Context`가 준다.
//!
//! - 진입 = 커서 모니터 전체(작업 표시줄 포함). 커서를 3 px 넘게 움직이면 인식 사슬이 이어받는다.
//! - 휠 ↑ 부모 / ↓ 자식 · Tab 창 ↔ 요소 · 클릭 = 확정(S08) · 드래그 = 직접(자석) · 더블클릭·Enter·Ctrl+C = 복사
//! - 확정 뒤 방향키 1 px 이동 · Ctrl+방향 늘리기 · Shift+방향 줄이기 · Ctrl+S 저장
//! - 우클릭 = 범위 다시 정하기 → 한 번 더 = 취소 · Esc = 취소 · Ctrl+A 모니터 → 한 번 더 전체 · R 직전 · , . 지난 범위

use crate::geometry::{PhysPoint, PhysRect};
use crate::magnet::{guides, rect_between, snap_point, Edges, SnapState};
use crate::modekeys::{matches, CaptureKeys};
use crate::recog::{first_level, src, window_index, LevelIntent, Node, NodeKind};
use serde::{Deserialize, Serialize};

/// 진입 뒤 이만큼 움직여야 인식이 시작된다(손떨림·키 누를 때의 미세 이동).
const FRESH_SLOP: i32 = 3;
/// 누른 채 이만큼 움직이면 드래그.
const DRAG_SLOP: i32 = 4;
/// 확정한 범위의 변·귀를 잡는 거리(논리 px). 화면 쪽 커서 모양(`selectionGrip.js`)도 같은 값을 쓴다.
pub const GRIP_LOGICAL: f64 = 6.0;

/// 확정한 범위를 손볼 때 잡은 곳(2026-11-28 사용자 요청: 안을 끌면 옮기기, 변·귀를 끌면 그쪽만 늘리기·줄이기).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grip {
    /// 범위 안 = 통째로 옮기기
    Move,
    /// 잡은 변(귀 = 두 변)만 커서를 따라간다
    Edges { left: bool, top: bool, right: bool, bottom: bool },
}

/// 커서 점이 확정한 범위의 어디를 잡는지. 변·귀는 `tol` px 안(안팎 모두), 그 밖의 범위 안은 옮기기, 범위 밖은 `None`.
/// 아주 작은 범위에서 양쪽 변이 모두 가까우면 더 가까운 쪽을 잡는다.
pub fn grip_at(rect: PhysRect, p: PhysPoint, tol: i32) -> Option<Grip> {
    let tol = tol.max(1);
    let inside_x = p.x >= rect.x - tol && p.x <= rect.right() + tol;
    let inside_y = p.y >= rect.y - tol && p.y <= rect.bottom() + tol;
    if !inside_x || !inside_y {
        return None;
    }
    let pick = |a: i32, b: i32| -> (bool, bool) {
        let (na, nb) = (a.abs() <= tol, b.abs() <= tol);
        match (na, nb) {
            (true, true) => (a.abs() <= b.abs(), a.abs() > b.abs()),
            other => other,
        }
    };
    let (left, right) = pick(p.x - rect.x, p.x - rect.right());
    let (top, bottom) = pick(p.y - rect.y, p.y - rect.bottom());
    if left || right || top || bottom {
        return Some(Grip::Edges { left, top, right, bottom });
    }
    rect.contains(p).then_some(Grip::Move)
}

/// 잡은 곳을 `from` → `to`로 끈 결과(가상 화면 안, 최소 1×1). 변을 반대편 너머로 끌면 뒤집힌다.
pub fn adjust_rect(base: PhysRect, grip: Grip, from: PhysPoint, to: PhysPoint, bounds: PhysRect) -> PhysRect {
    match grip {
        Grip::Move => {
            let x = (base.x + to.x - from.x).clamp(bounds.x, (bounds.right() - base.w).max(bounds.x));
            let y = (base.y + to.y - from.y).clamp(bounds.y, (bounds.bottom() - base.h).max(bounds.y));
            PhysRect::new(x, y, base.w, base.h)
        }
        Grip::Edges { left, top, right, bottom } => {
            let cx = to.x.clamp(bounds.x, bounds.right());
            let cy = to.y.clamp(bounds.y, bounds.bottom());
            let (l, r) = (if left { cx } else { base.x }, if right { cx } else { base.right() });
            let (t, b) = (if top { cy } else { base.y }, if bottom { cy } else { base.bottom() });
            let (l, r) = (l.min(r), l.max(r).max(l.min(r) + 1));
            let (t, b) = (t.min(b), t.max(b).max(t.min(b) + 1));
            PhysRect::from_edges(l, t, r, b)
        }
    }
}

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
    /// 맥 Cmd · Windows 키
    #[serde(default)]
    pub meta: bool,
}

/// 선택 툴바 단추(R-CAP-27) — 키와 같은 결과를 낸다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Copy,
    Save,
    QuickSave,
    Cancel,
    /// 캡처 핀(R-CAP-29)
    Pin,
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
    /// 선택 툴바 단추
    Command(Command),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// 바뀐 것 없음
    None,
    /// 범위·표시가 바뀜
    Redraw,
    Copy(PhysRect),
    Save { rect: PhysRect, quick: bool },
    /// 그 범위를 화면에 고정(캡처 핀)
    Pin(PhysRect),
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

/// 숨은 "인식 근거 보기"(08 §4.5): 사슬 마디 하나.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceNode {
    pub rect: PhysRect,
    /// 종류 이름(창·표·상자…)
    pub label: String,
    pub score: f32,
    /// 근거 글자 "W·A·V"(W 창 · H 자식 창 · A 접근성 · M 앱 가족 · V 이미지)
    pub sources: String,
    /// 지금 강조
    pub current: bool,
    /// 보이지 않는 그릇(첫 강조에서 뺌)
    pub hidden: bool,
}

/// 근거 비트 → "W·A·V".
pub fn source_letters(bits: u8) -> String {
    [(src::W, "W"), (src::H, "H"), (src::A, "A"), (src::M, "M"), (src::V, "V")].iter().filter(|(b, _)| bits & b != 0).map(|(_, l)| *l).collect::<Vec<_>>().join("·")
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
    /// 숨은 설정 `capture.evidence`가 켜졌을 때만(작은 것 → 큰 것)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceNode>,
}

#[derive(Clone, Debug)]
enum State {
    Hover { chain: Vec<Node>, level: usize, window: Option<u64>, fresh: bool },
    Drag { anchor: PhysPoint, cur: PhysPoint, snap: SnapState, edges: Edges },
    Confirmed { rect: PhysRect, picked_level: Option<usize> },
    /// 확정한 범위를 옮기거나 변·귀를 끄는 중
    Adjust { grip: Grip, base: PhysRect, from: PhysPoint, cur: PhysRect, snap: SnapState, edges: Edges },
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
    /// 화면 안 키(R-KEY-6) — 설정된 `c-*` 값
    keys: CaptureKeys,
    /// 숨은 "인식 근거 보기"
    show_evidence: bool,
    /// 확정한 범위 위에서 눌렀을 때 잡은 곳과 그때 범위(움직이면 손보기로)
    grip_down: Option<(Grip, PhysRect)>,
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
            keys: CaptureKeys::default(),
            show_evidence: false,
            grip_down: None,
        }
    }

    /// 숨은 설정 `capture.evidence` — 보기에 사슬 마디마다 근거·점수를 담는다.
    pub fn with_evidence(mut self, on: bool) -> Selector {
        self.show_evidence = on;
        self
    }

    /// 설정된 화면 안 키로 판정한다(R-KEY-6).
    pub fn with_keys(mut self, keys: CaptureKeys) -> Selector {
        self.keys = keys;
        self
    }

    /// 확정한 범위로 바로 시작(캡처 주석 되살리기 R2-UNDO-1).
    pub fn with_confirmed(mut self, rect: PhysRect) -> Selector {
        self.state = State::Confirmed { rect, picked_level: None };
        self
    }

    /// 마지막 고르기 입력의 물리 좌표.
    pub fn cursor(&self) -> PhysPoint {
        self.cursor
    }

    /// 지금 범위.
    pub fn rect(&self) -> Option<PhysRect> {
        match &self.state {
            State::Hover { chain, level, .. } => chain.get(*level).map(|n| n.rect),
            State::Drag { anchor, cur, .. } => Some(rect_between(*anchor, *cur)),
            State::Confirmed { rect, .. } => Some(*rect),
            State::Adjust { cur, .. } => Some(*cur),
        }
    }

    /// 인식 사슬에서 실제 고른 단계(작은 것부터 0). 직접 정한 범위에는 없다.
    pub fn picked_level(&self) -> Option<usize> {
        match self.state {
            State::Hover { level, fresh: false, .. } => Some(level),
            State::Confirmed { picked_level, .. } => picked_level,
            _ => None,
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
            State::Drag { .. } | State::Confirmed { .. } | State::Adjust { .. } => "drag",
        }
    }

    /// 강조를 커서로 고르고 있으면 그 자리(드래그·확정 뒤면 None).
    pub fn hover_point(&self) -> Option<PhysPoint> {
        matches!(self.state, State::Hover { .. }).then_some(self.cursor)
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
            // 손보는 동안은 드래그처럼(툴바 숨김·크기 배지·자석 안내선)
            State::Adjust { cur, snap, .. } => {
                let g = guides(cur, snap).into_iter().map(|(x1, y1, x2, y2)| Guide { x1, y1, x2, y2 }).collect();
                (Phase::Drag, Vec::new(), -1, g)
            }
        };
        let evidence = match (&self.state, self.show_evidence) {
            (State::Hover { chain, level, .. }, true) => chain
                .iter()
                .enumerate()
                .filter(|(_, n)| n.kind != NodeKind::Virtual)
                .map(|(i, n)| EvidenceNode { rect: n.rect, label: n.kind.label().to_string(), score: n.score, sources: source_letters(n.sources), current: i == *level, hidden: n.hidden_container })
                .collect(),
            _ => Vec::new(),
        };
        SelectionView { phase, rect: self.rect(), path, level, guides: guides_v, magnet: self.magnet, cursor_included: self.cursor_included, help: self.help, evidence }
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

    fn confirm(&mut self, rect: PhysRect, picked_level: Option<usize>) -> Effect {
        self.state = State::Confirmed { rect, picked_level };
        Effect::Redraw
    }

    pub fn handle(&mut self, input: Input, ctx: &mut dyn Context) -> Effect {
        match input {
            Input::Move(p) => self.on_move(p, ctx),
            Input::Wheel(dy) => self.on_wheel(dy, ctx),
            Input::Down(p, Button::Left) => {
                self.cursor = p;
                self.pressed = Some(p);
                self.grip_down = match self.state {
                    State::Confirmed { rect, .. } => grip_at(rect, p, (GRIP_LOGICAL * ctx.scale_at(p)).round() as i32).map(|g| (g, rect)),
                    _ => None,
                };
                Effect::None
            }
            Input::Up(p, Button::Left) => {
                let was = self.pressed.take();
                self.grip_down = None;
                if let State::Adjust { cur, .. } = &self.state {
                    let r = *cur;
                    return self.confirm(r, None);
                }
                if let State::Drag { anchor, snap, edges, .. } = &self.state {
                    let end = if self.magnet { snap_point(p, edges, ctx.scale_at(p), &mut snap.clone()) } else { p };
                    let r = rect_between(*anchor, end);
                    return self.confirm(r, None);
                }
                match (was, self.rect()) {
                    // 클릭 = 지금 강조를 확정(S08: 손잡이·방향키)
                    (Some(_), Some(r)) if matches!(self.state, State::Hover { .. }) => self.confirm(r, self.picked_level()),
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
            Input::Command(c) => match c {
                Command::Cancel => Effect::Cancel,
                Command::Copy => self.rect().map_or(Effect::None, Effect::Copy),
                Command::Save => self.rect().map_or(Effect::None, |rect| Effect::Save { rect, quick: false }),
                Command::QuickSave => self.rect().map_or(Effect::None, |rect| Effect::Save { rect, quick: true }),
                Command::Pin => self.rect().map_or(Effect::None, Effect::Pin),
            },
        }
    }

    fn on_move(&mut self, p: PhysPoint, ctx: &mut dyn Context) -> Effect {
        let moved = p != self.cursor;
        self.cursor = p;
        self.magnet = !ctx.alt_down();
        if let Some(down) = self.pressed {
            let far = (p.x - down.x).abs() > DRAG_SLOP || (p.y - down.y).abs() > DRAG_SLOP;
            if far && matches!(self.state, State::Confirmed { .. }) {
                if let Some((grip, base)) = self.grip_down.take() {
                    let mut edges = ctx.edges(p);
                    let mon = ctx.monitor_at(p);
                    edges.extend([mon.x, mon.right()], [mon.y, mon.bottom()]);
                    self.state = State::Adjust { grip, base, from: down, cur: base, snap: SnapState::default(), edges };
                }
            }
            if far && !matches!(self.state, State::Drag { .. } | State::Adjust { .. }) {
                let mut edges = ctx.edges(down);
                edges.extend([ctx.monitor_at(down).x, ctx.monitor_at(down).right()], [ctx.monitor_at(down).y, ctx.monitor_at(down).bottom()]);
                let mut snap = SnapState::default();
                let anchor = if self.magnet { snap_point(down, &edges, ctx.scale_at(down), &mut snap) } else { down };
                self.state = State::Drag { anchor, cur: p, snap: SnapState::default(), edges };
            }
        }
        match &mut self.state {
            State::Adjust { grip, base, from, cur, snap, edges } => {
                // 옮기기는 자석 없이, 변·귀는 드래그처럼 가까운 경계에 붙는다(Alt = 끔)
                let to = if self.magnet && *grip != Grip::Move { snap_point(p, edges, ctx.scale_at(p), snap) } else {
                    *snap = SnapState::default();
                    p
                };
                let next = adjust_rect(*base, *grip, *from, to, ctx.virtual_bounds());
                if next == *cur {
                    return Effect::None;
                }
                *cur = next;
                Effect::Redraw
            }
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
        let on = |combo: &str| matches(combo, &k);
        // Esc는 설정과 상관없이 늘 취소(빠져나갈 길)
        if on(&self.keys.cancel) || code == "Escape" || k.key == "Escape" {
            return Effect::Cancel;
        }
        // Ctrl+C·Cmd+C는 늘 복사(다른 앱과 같은 습관)
        let plain_c = code == "KeyC" && (k.ctrl || k.meta) && !k.shift && !k.alt;
        if on(&self.keys.copy) || plain_c {
            return self.rect().map_or(Effect::None, Effect::Copy);
        }
        if on(&self.keys.quick) {
            return self.rect().map_or(Effect::None, |rect| Effect::Save { rect, quick: true });
        }
        if on(&self.keys.save) {
            return self.rect().map_or(Effect::None, |rect| Effect::Save { rect, quick: false });
        }
        if on(&self.keys.full) {
            let mon = ctx.monitor_at(self.cursor);
            let target = if self.rect() == Some(mon) { ctx.virtual_bounds() } else { mon };
            return self.confirm(target, None);
        }
        if on(&self.keys.detect) {
            self.elements = !self.elements;
            self.intent = None;
            self.rehover(ctx);
            return Effect::Redraw;
        }
        if on(&self.keys.cursor) {
            self.cursor_included = !self.cursor_included;
            return Effect::Redraw;
        }
        if k.key == "?" || (code == "Slash" && k.shift) {
            self.help = !self.help;
            return Effect::Redraw;
        }
        if on(&self.keys.last) {
            return match ctx.history().first() {
                Some(r) => {
                    self.history_at = Some(0);
                    self.confirm(*r, None)
                }
                None => Effect::Notice("island.capture.noRecent"),
            };
        }
        let (older, newer) = (on(&self.keys.prev), on(&self.keys.next));
        if older || newer {
            let list = ctx.history();
            if list.is_empty() {
                return Effect::Notice("island.capture.noRecent");
            }
            let i = match self.history_at {
                None => 0,
                Some(i) if older => (i + 1).min(list.len() - 1),
                Some(i) => i.saturating_sub(1),
            };
            self.history_at = Some(i);
            return self.confirm(list[i], None);
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
            return self.confirm(next, None);
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
    fn 직접_정하거나_되살린_범위에는_인식_단계가_없다() {
        let mut c = ctx();
        let p = PhysPoint { x: 410, y: 330 };
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        s.handle(Input::Move(p), &mut c);
        assert_eq!(s.picked_level(), Some(0));
        s.handle(key("KeyR"), &mut c);
        assert_eq!(s.rect(), c.hist.first().copied());
        assert_eq!(s.picked_level(), None);

        s.handle(Input::Down(p, Button::Right), &mut c);
        s.handle(Input::Down(p, Button::Left), &mut c);
        let end = PhysPoint { x: 700, y: 500 };
        s.handle(Input::Move(end), &mut c);
        s.handle(Input::Up(end, Button::Left), &mut c);
        assert_eq!(s.picked_level(), None);
        s.handle(Input::Down(p, Button::Right), &mut c);
        assert!(s.picked_level().is_some());
        assert_eq!(s.with_confirmed(WIN).picked_level(), None);
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

    #[test]
    fn 화면_안_키는_설정값을_따른다() {
        // R-KEY-6: 복사를 Space로 바꾸면 Space = 복사, Enter = 아무것도 안 함. Esc는 늘 취소.
        let mut c = ctx();
        let keys = CaptureKeys { copy: "Space".into(), cancel: "Q".into(), save: "Cmd+S".into(), ..CaptureKeys::default() };
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c).with_keys(keys);
        s.handle(Input::Move(PhysPoint { x: 420, y: 330 }), &mut c);
        let r = s.rect().unwrap();
        assert_eq!(s.handle(key("Enter"), &mut c), Effect::None, "Enter는 이제 복사가 아님");
        assert_eq!(s.handle(key("Space"), &mut c), Effect::Copy(r));
        let ctrl_s = Input::Key(Key { code: "KeyS".into(), key: "s".into(), ctrl: true, ..Default::default() });
        assert_ne!(s.handle(ctrl_s, &mut c), Effect::Save { rect: r, quick: false }, "맥 값 Cmd+S로 바뀌었으니 Ctrl+S는 저장 아님");
        let cmd_s = Input::Key(Key { code: "KeyS".into(), key: "s".into(), meta: true, ..Default::default() });
        assert_eq!(s.handle(cmd_s, &mut c), Effect::Save { rect: r, quick: false });
        let cmd_c = Input::Key(Key { code: "KeyC".into(), key: "c".into(), meta: true, ..Default::default() });
        assert_eq!(s.handle(cmd_c, &mut c), Effect::Copy(r), "Cmd+C·Ctrl+C는 늘 복사");
        assert_eq!(s.handle(key("KeyQ"), &mut c), Effect::Cancel);
        assert_eq!(s.handle(key("Escape"), &mut c), Effect::Cancel, "Esc는 늘 빠져나감");
    }

    #[test]
    fn 툴바_단추는_키와_같은_결과() {
        let mut c = ctx();
        let mut s = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        s.handle(Input::Move(PhysPoint { x: 420, y: 330 }), &mut c);
        s.handle(Input::Down(PhysPoint { x: 420, y: 330 }, Button::Left), &mut c);
        s.handle(Input::Up(PhysPoint { x: 420, y: 330 }, Button::Left), &mut c);
        assert_eq!(s.view().phase, Phase::Confirmed);
        let r = s.rect().unwrap();
        assert_eq!(s.handle(Input::Command(Command::Copy), &mut c), Effect::Copy(r));
        assert_eq!(s.handle(Input::Command(Command::Save), &mut c), Effect::Save { rect: r, quick: false });
        assert_eq!(s.handle(Input::Command(Command::QuickSave), &mut c), Effect::Save { rect: r, quick: true });
        assert_eq!(s.handle(Input::Command(Command::Pin), &mut c), Effect::Pin(r));
        assert_eq!(s.handle(Input::Command(Command::Cancel), &mut c), Effect::Cancel);
    }

    #[test]
    fn 근거_보기는_켰을_때만_마디마다() {
        let mut c = ctx();
        let mut off = Selector::new(PhysPoint { x: 400, y: 330 }, &c);
        off.handle(Input::Move(PhysPoint { x: 420, y: 330 }), &mut c);
        assert!(off.view().evidence.is_empty(), "기본은 꺼짐");
        let mut on = Selector::new(PhysPoint { x: 400, y: 330 }, &c).with_evidence(true);
        on.handle(Input::Move(PhysPoint { x: 420, y: 330 }), &mut c);
        let v = on.view();
        assert!(!v.evidence.is_empty());
        assert_eq!(v.evidence.iter().filter(|e| e.current).count(), 1, "지금 강조는 하나");
        assert_eq!(v.evidence.iter().find(|e| e.current).map(|e| e.rect), v.rect);
        assert_eq!(source_letters(src::W | src::A | src::V), "W·A·V");
        assert_eq!(source_letters(0), "");
    }

    #[test]
    fn 확정_범위로_바로_시작할_수_있다() {
        let c = ctx();
        let rect = PhysRect::new(120, 140, 320, 180);
        let s = Selector::new(PhysPoint { x: 200, y: 200 }, &c).with_confirmed(rect);
        let v = s.view();
        assert_eq!(v.phase, Phase::Confirmed);
        assert_eq!(s.rect(), Some(rect));
        assert_eq!(v.rect, Some(rect));
    }

    fn pt(x: i32, y: i32) -> PhysPoint {
        PhysPoint { x, y }
    }

    /// 확정한 범위 R = (400,300)~(800,600). 자석 후보(WIN 변)에서 멀리 둔다.
    const R: PhysRect = PhysRect::new(400, 300, 400, 300);

    fn confirmed(c: &mut Ctx) -> Selector {
        Selector::new(pt(10, 10), c).with_confirmed(R)
    }

    fn drag(s: &mut Selector, c: &mut Ctx, from: PhysPoint, to: PhysPoint) -> Effect {
        s.handle(Input::Down(from, Button::Left), c);
        s.handle(Input::Move(PhysPoint { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 }), c);
        s.handle(Input::Move(to), c);
        s.handle(Input::Up(to, Button::Left), c)
    }

    #[test]
    fn 잡는_자리는_귀_변_안_밖() {
        let e = |left, top, right, bottom| Some(Grip::Edges { left, top, right, bottom });
        assert_eq!(grip_at(R, pt(400, 300), 6), e(true, true, false, false), "왼쪽 위 귀");
        assert_eq!(grip_at(R, pt(805, 596), 6), e(false, false, true, true), "오른쪽 아래 귀(바깥 5 px)");
        assert_eq!(grip_at(R, pt(600, 302), 6), e(false, true, false, false), "위 변");
        assert_eq!(grip_at(R, pt(395, 450), 6), e(true, false, false, false), "왼쪽 변(바깥)");
        assert_eq!(grip_at(R, pt(600, 450), 6), Some(Grip::Move), "안 = 옮기기");
        assert_eq!(grip_at(R, pt(300, 450), 6), None, "밖");
        // 아주 좁은 범위: 양쪽 변이 다 가까우면 더 가까운 쪽
        let thin = PhysRect::new(100, 100, 4, 50);
        assert_eq!(grip_at(thin, pt(104, 120), 6), e(false, false, true, false));
    }

    #[test]
    fn 끌기_결과는_화면_안이고_변을_넘기면_뒤집힌다() {
        let b = PhysRect::new(0, 0, 3840, 1080);
        assert_eq!(adjust_rect(R, Grip::Move, pt(500, 400), pt(560, 380), b), PhysRect::new(460, 280, 400, 300));
        assert_eq!(adjust_rect(R, Grip::Move, pt(500, 400), pt(-900, 400), b).x, 0, "왼쪽 끝에서 멈춤");
        let right = Grip::Edges { left: false, top: false, right: true, bottom: false };
        assert_eq!(adjust_rect(R, right, pt(800, 450), pt(900, 450), b), PhysRect::new(400, 300, 500, 300));
        assert_eq!(adjust_rect(R, right, pt(800, 450), pt(300, 450), b), PhysRect::new(300, 300, 100, 300), "왼쪽 변 너머 = 뒤집힘");
        assert_eq!(adjust_rect(R, right, pt(800, 450), pt(400, 450), b).w, 1, "최소 1 px");
    }

    #[test]
    fn 확정한_범위_안을_끌면_옮기고_크기는_그대로() {
        let mut c = ctx();
        let mut s = confirmed(&mut c);
        assert_eq!(drag(&mut s, &mut c, pt(600, 450), pt(650, 420)), Effect::Redraw);
        assert_eq!(s.rect(), Some(PhysRect::new(450, 270, 400, 300)));
        assert_eq!(s.view().phase, Phase::Confirmed, "놓으면 다시 확정");
    }

    #[test]
    fn 변과_귀를_끌면_그쪽만_바뀐다() {
        let mut c = ctx();
        let mut s = confirmed(&mut c);
        drag(&mut s, &mut c, pt(799, 450), pt(950, 470));
        assert_eq!(s.rect(), Some(PhysRect::new(400, 300, 550, 300)), "오른쪽 변만(세로 움직임은 무시)");
        drag(&mut s, &mut c, pt(400, 300), pt(380, 250));
        assert_eq!(s.rect(), Some(PhysRect::new(380, 250, 570, 350)), "왼쪽 위 귀 = 두 변");
    }

    #[test]
    fn 손보는_동안은_드래그_단계이고_밖을_끌면_새로_그린다() {
        let mut c = ctx();
        let mut s = confirmed(&mut c);
        s.handle(Input::Down(pt(600, 450), Button::Left), &mut c);
        s.handle(Input::Move(pt(640, 450)), &mut c);
        assert_eq!(s.view().phase, Phase::Drag, "툴바를 숨기고 크기 배지");
        s.handle(Input::Up(pt(640, 450), Button::Left), &mut c);
        // 범위 밖에서 끌면 예전처럼 새 범위
        drag(&mut s, &mut c, pt(1500, 900), pt(1700, 1000));
        assert_eq!(s.rect(), Some(PhysRect::new(1500, 900, 200, 100)));
    }

    #[test]
    fn 손볼_때_변은_자석에_붙고_alt면_끈다() {
        let mut c = ctx();
        let mut s = Selector::new(pt(10, 10), &c).with_confirmed(PhysRect::new(400, 300, 400, 300));
        // 오른쪽 변을 WIN 오른쪽(1300) 근처 1296까지 → 1300에 붙음
        drag(&mut s, &mut c, pt(800, 450), pt(1296, 450));
        assert_eq!(s.rect().unwrap().right(), 1300);
        c.alt = true;
        let mut s = Selector::new(pt(10, 10), &c).with_confirmed(PhysRect::new(400, 300, 400, 300));
        drag(&mut s, &mut c, pt(800, 450), pt(1296, 450));
        assert_eq!(s.rect().unwrap().right(), 1296);
    }
}
