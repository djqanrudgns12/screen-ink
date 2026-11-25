//! 근거 융합 → 사슬(08 §4.4~4.5, 02 §1.1·1.3). 순수 함수 — 근거는 어댑터·V 엔진이 모아서 넘긴다.
//!
//! 근거: W 창(스냅숏) · H 자식 창 · A UIA 요소 · V 이미지 상자. 어느 하나도 혼자 믿지 않는다:
//! A·H 사각형은 V 곧은 경계로 붙이고(경계 맞춤), 경계 지지율이 낮은 그릇(Pane·Group·Custom)은 첫 강조에서 뺀다.

use crate::geometry::{PhysPoint, PhysRect};
use serde::{Deserialize, Serialize};

/// UIA 컨트롤 종류(필요한 것만). 이름·글자는 담지 않는다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Control {
    Button,
    Edit,
    Text,
    Image,
    List,
    ListItem,
    Table,
    DataItem,
    Header,
    TabItem,
    Tab,
    Menu,
    MenuItem,
    ToolBar,
    Document,
    Hyperlink,
    CheckBox,
    ComboBox,
    Tree,
    TreeItem,
    Pane,
    Group,
    Custom,
    /// 제목 표시줄(D6) — 이 위에서는 첫 강조가 창
    TitleBar,
    Other,
}

impl Control {
    /// UIA ControlTypeId(50000~) → 종류.
    pub fn from_uia(id: i32) -> Control {
        match id {
            50000 => Control::Button,
            50002 => Control::CheckBox,
            50003 => Control::ComboBox,
            50004 => Control::Edit,
            50005 => Control::Hyperlink,
            50006 => Control::Image,
            50007 => Control::ListItem,
            50008 => Control::List,
            50009 => Control::Menu,
            50011 => Control::MenuItem,
            50018 => Control::Tab,
            50019 => Control::TabItem,
            50020 => Control::Text,
            50021 => Control::ToolBar,
            50023 => Control::Tree,
            50024 => Control::TreeItem,
            50025 => Control::Custom,
            50026 => Control::Group,
            50028 | 50036 => Control::Table, // DataGrid · Table
            50029 => Control::DataItem,
            50030 => Control::Document,
            50033 => Control::Pane,
            50034 | 50035 => Control::Header,
            50037 => Control::TitleBar,
            _ => Control::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Control::Button => "단추",
            Control::Edit => "입력 칸",
            Control::Text => "글자",
            Control::Image => "그림",
            Control::List => "목록",
            Control::ListItem => "항목",
            Control::Table => "표",
            Control::DataItem => "행",
            Control::Header => "머리",
            Control::TabItem => "탭",
            Control::Tab => "탭 묶음",
            Control::Menu => "메뉴",
            Control::MenuItem => "메뉴 항목",
            Control::ToolBar => "도구 모음",
            Control::Document => "문서",
            Control::Hyperlink => "링크",
            Control::CheckBox => "확인란",
            Control::ComboBox => "고르기 칸",
            Control::Tree => "트리",
            Control::TreeItem => "트리 항목",
            Control::Pane => "영역",
            Control::Group => "묶음",
            Control::TitleBar => "제목 표시줄",
            Control::Custom | Control::Other => "요소",
        }
    }

    /// 의미 점수(08 §4.4-7): 사람이 고르고 싶은 조각일수록 높다.
    fn meaning(self) -> f32 {
        match self {
            Control::Button | Control::ListItem | Control::DataItem | Control::TabItem | Control::MenuItem | Control::Hyperlink | Control::Image | Control::Edit => 1.0,
            Control::Table | Control::Document | Control::List | Control::Menu | Control::Tree | Control::TreeItem | Control::CheckBox | Control::ComboBox => 0.85,
            Control::Text | Control::Header | Control::ToolBar | Control::Tab => 0.6,
            Control::Pane | Control::Group | Control::Custom | Control::TitleBar | Control::Other => 0.2,
        }
    }

    fn is_container(self) -> bool {
        matches!(self, Control::Pane | Control::Group | Control::Custom | Control::Other)
    }
}

/// V 상자 종류(ink-vision이 채움).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VisionKind {
    Box,
    Flat,
}

/// 사슬 마디 종류 — 깊이 기억(02 §1.3)의 "어떤 종류의 조각인지".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "camelCase")]
pub enum NodeKind {
    Element(Control),
    Child,
    Vision(VisionKind),
    Window,
    Monitor,
    Virtual,
}

impl NodeKind {
    pub fn label(self) -> &'static str {
        match self {
            NodeKind::Element(c) => c.label(),
            NodeKind::Child => "칸",
            NodeKind::Vision(VisionKind::Box) => "상자",
            NodeKind::Vision(VisionKind::Flat) => "영역",
            NodeKind::Window => "창",
            NodeKind::Monitor => "화면",
            NodeKind::Virtual => "전체 화면",
        }
    }
}

/// 근거 비트(W·H·A·M·V).
pub mod src {
    pub const W: u8 = 1;
    pub const H: u8 = 2;
    pub const A: u8 = 4;
    pub const M: u8 = 8;
    pub const V: u8 = 16;
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub rect: PhysRect,
    pub kind: NodeKind,
    pub score: f32,
    /// 근거 비트 합
    pub sources: u8,
    /// 보이지 않는 그릇 — 휠로는 가지만 첫 강조에서는 뺀다
    pub hidden_container: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElementCand {
    pub rect: PhysRect,
    pub control: Control,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisionCand {
    pub rect: PhysRect,
    pub kind: VisionKind,
    pub support: f32,
}

/// 융합에 넘기는 근거 묶음(모두 가상 화면 물리 px).
pub struct Evidence<'a> {
    pub point: PhysPoint,
    pub window: Option<PhysRect>,
    pub children: &'a [PhysRect],
    pub elements: &'a [ElementCand],
    pub vision: &'a [VisionCand],
    pub monitor: PhysRect,
    pub virtual_bounds: PhysRect,
    /// 사각형의 경계 지지율(V). V가 없으면 None.
    pub support: Option<&'a dyn Fn(PhysRect) -> f32>,
    /// V 곧은 경계로 붙이기(±3 px). V가 없으면 None.
    pub snap: Option<&'a dyn Fn(PhysRect) -> PhysRect>,
}

const MIN_SIDE: i32 = 4;
const MIN_AREA: i64 = 16;
const MERGE_IOU: f64 = 0.92;
const HIDDEN_SUPPORT: f32 = 0.15;
pub const FIRST_SCORE: f32 = 0.5;

pub fn iou(a: &PhysRect, b: &PhysRect) -> f64 {
    let inter = a.intersect(b).map_or(0, |r| r.area()) as f64;
    let uni = (a.area() + b.area()) as f64 - inter;
    if uni <= 0.0 {
        0.0
    } else {
        inter / uni
    }
}

fn near_same(a: &PhysRect, b: &PhysRect) -> bool {
    (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1 && (a.right() - b.right()).abs() <= 1 && (a.bottom() - b.bottom()).abs() <= 1
}

/// 크기 적합(08 §4.4-7): 24×16 이상이고 창의 60% 이하면 1.
fn size_fit(r: &PhysRect, window: Option<PhysRect>) -> f32 {
    if r.w < 24 || r.h < 16 {
        return 0.3;
    }
    match window {
        Some(w) if w.area() > 0 && r.area() as f64 > w.area() as f64 * 0.6 => 0.5,
        _ => 1.0,
    }
}

fn score(n: &Node, support: f32, window: Option<PhysRect>) -> f32 {
    let meaning = match n.kind {
        NodeKind::Element(c) => c.meaning(),
        NodeKind::Child => 0.7,
        NodeKind::Vision(VisionKind::Box) => 0.75,
        NodeKind::Vision(VisionKind::Flat) => 0.5,
        NodeKind::Window | NodeKind::Monitor | NodeKind::Virtual => 0.4,
    };
    let count = n.sources.count_ones().min(3) as f32 / 3.0;
    0.35 * support + 0.25 * meaning + 0.20 * count + 0.20 * size_fit(&n.rect, window)
}

/// 커서 점에서 사슬(작은 것 → 큰 것). 마지막은 언제나 창(있으면) → 모니터 → 가상 화면.
pub fn build_chain(ev: &Evidence) -> Vec<Node> {
    let p = ev.point;
    let window = ev.window;
    let clip_to = window.unwrap_or(ev.monitor);
    let support = |r: PhysRect| ev.support.map(|f| f(r));
    let snap = |r: PhysRect| ev.snap.map_or(r, |f| f(r));

    let mut raw: Vec<Node> = Vec::new();
    let mut push = |rect: PhysRect, kind: NodeKind, sources: u8| {
        // 검증(08 §4.4-2): 창 밖으로 2 px 넘게 나가면 자르고, 모니터로도 자르고, 커서를 안 담거나 너무 작으면 버린다
        let Some(r) = rect.intersect(&PhysRect::new(clip_to.x - 2, clip_to.y - 2, clip_to.w + 4, clip_to.h + 4)).and_then(|r| r.intersect(&clip_to)).and_then(|r| r.intersect(&ev.monitor)) else { return };
        if !r.contains(p) || r.w < MIN_SIDE || r.h < MIN_SIDE || r.area() < MIN_AREA {
            return;
        }
        raw.push(Node { rect: r, kind, score: 0.0, sources, hidden_container: false });
    };
    for e in ev.elements {
        push(snap(e.rect), NodeKind::Element(e.control), src::A);
    }
    for c in ev.children {
        push(snap(*c), NodeKind::Child, src::H);
    }
    for v in ev.vision {
        push(v.rect, NodeKind::Vision(v.kind), src::V);
    }

    // 중복 합치기(IoU ≥ 0.92): 뜻이 있는 종류(요소 > 자식 창 > V)를 남기고 근거를 합친다
    let rank = |k: NodeKind| match k {
        NodeKind::Element(c) if !c.is_container() => 3,
        NodeKind::Element(_) => 2,
        NodeKind::Child => 2,
        NodeKind::Vision(_) => 1,
        _ => 0,
    };
    let meaning = |k: NodeKind| match k {
        NodeKind::Element(c) => c.meaning(),
        _ => 0.0,
    };
    let mut merged: Vec<Node> = Vec::new();
    for n in raw {
        if let Some(m) = merged.iter_mut().find(|m| iou(&m.rect, &n.rect) >= MERGE_IOU) {
            m.sources |= n.sources;
            let better = rank(n.kind) > rank(m.kind) || (rank(n.kind) == rank(m.kind) && meaning(n.kind) > meaning(m.kind));
            if better {
                m.kind = n.kind;
                m.rect = n.rect;
            }
        } else {
            merged.push(n);
        }
    }

    // 창 크기와 같은 조각은 창으로 합친다
    let mut tail: Vec<Node> = Vec::new();
    if let Some(w) = window {
        let extra = merged.iter().filter(|n| iou(&n.rect, &w) >= MERGE_IOU).fold(0u8, |a, n| a | n.sources);
        merged.retain(|n| iou(&n.rect, &w) < MERGE_IOU);
        tail.push(Node { rect: w, kind: NodeKind::Window, score: 0.0, sources: src::W | extra, hidden_container: false });
    }
    tail.push(Node { rect: ev.monitor, kind: NodeKind::Monitor, score: 0.0, sources: 0, hidden_container: false });
    tail.push(Node { rect: ev.virtual_bounds, kind: NodeKind::Virtual, score: 0.0, sources: 0, hidden_container: false });

    // 점수·보이지 않는 그릇
    for n in merged.iter_mut() {
        let s = support(n.rect);
        if let (NodeKind::Element(c), Some(s)) = (n.kind, s) {
            n.hidden_container = c.is_container() && s < HIDDEN_SUPPORT;
        }
        n.score = score(n, s.unwrap_or(0.5), window);
    }

    // 사슬: 넓이 순, 엄격히 커지게 — 앞 단계와 ±1 px 같은 것은 빼서 휠 한 칸마다 강조가 반드시 바뀐다(02 §1.1)
    merged.sort_by_key(|n| n.rect.area());
    let mut chain: Vec<Node> = Vec::new();
    for n in merged.into_iter().chain(tail) {
        if let Some(last) = chain.last_mut() {
            if near_same(&last.rect, &n.rect) {
                // 조각과 창·모니터가 같으면 창·모니터 이름을 살리고, 창·모니터끼리 같으면 앞(좁은 뜻)을 살린다
                let sources = last.sources | n.sources;
                if rank(last.kind) > 0 && rank(n.kind) == 0 {
                    *last = Node { sources, ..n };
                } else {
                    last.sources = sources;
                }
                continue;
            }
            if n.rect.area() < last.rect.area() {
                continue;
            }
        }
        chain.push(n);
    }
    chain
}

/// 깊이 기억(02 §1.3): 휠로 고른 단계를 숫자가 아니라 "종류 + 창에서 떨어진 거리"로.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LevelIntent {
    pub kind: NodeKind,
    /// 창 마디에서 아래로 몇 단계
    pub depth_from_window: usize,
}

pub fn window_index(chain: &[Node]) -> Option<usize> {
    chain.iter().position(|n| n.kind == NodeKind::Window)
}

impl LevelIntent {
    pub fn of(chain: &[Node], level: usize) -> Option<LevelIntent> {
        let n = chain.get(level)?;
        let wi = window_index(chain).unwrap_or(chain.len().saturating_sub(3));
        Some(LevelIntent { kind: n.kind, depth_from_window: wi.saturating_sub(level) })
    }
}

/// 첫 강조 단계(08 §4.5): 깊이 기억 → 점수 ≥ 0.5 중 가장 작은 것(보이지 않는 그릇 제외) → 창 → 모니터.
pub fn first_level(chain: &[Node], intent: Option<&LevelIntent>) -> usize {
    if let Some(it) = intent {
        if let Some(i) = chain.iter().position(|n| n.kind == it.kind) {
            return i;
        }
        if let Some(wi) = window_index(chain) {
            if it.depth_from_window <= wi {
                return wi - it.depth_from_window;
            }
        }
    }
    // 제목 표시줄 규칙(D6): 커서가 제목 표시줄 안이면 제목 글자가 아니라 창(제목 글자는 휠 아래로)
    if chain.iter().any(|n| n.kind == NodeKind::Element(Control::TitleBar)) {
        if let Some(wi) = window_index(chain) {
            return wi;
        }
    }
    if let Some(i) = chain.iter().position(|n| !n.hidden_container && n.score >= FIRST_SCORE && !matches!(n.kind, NodeKind::Window | NodeKind::Monitor | NodeKind::Virtual)) {
        return i;
    }
    window_index(chain).or_else(|| chain.iter().position(|n| n.kind == NodeKind::Monitor)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MON: PhysRect = PhysRect::new(0, 0, 1920, 1080);
    const WIN: PhysRect = PhysRect::new(100, 100, 1200, 800);

    fn ev<'a>(p: PhysPoint, elements: &'a [ElementCand], vision: &'a [VisionCand]) -> Evidence<'a> {
        Evidence { point: p, window: Some(WIN), children: &[], elements, vision, monitor: MON, virtual_bounds: MON, support: None, snap: None }
    }

    #[test]
    fn 표_사슬은_작은_것부터_창_화면까지() {
        let p = PhysPoint { x: 400, y: 330 };
        let els = [
            ElementCand { rect: PhysRect::new(300, 300, 300, 40), control: Control::DataItem },
            ElementCand { rect: PhysRect::new(300, 260, 600, 400), control: Control::Table },
            ElementCand { rect: PhysRect::new(100, 100, 1200, 800), control: Control::Pane }, // 창과 같은 그릇 → 창으로 합침
        ];
        let c = build_chain(&ev(p, &els, &[]));
        let kinds: Vec<&str> = c.iter().map(|n| n.kind.label()).collect();
        assert_eq!(kinds, vec!["행", "표", "창", "화면"], "가상 화면이 모니터와 같으면 합친다");
        assert_eq!(first_level(&c, None), 0, "행이 첫 강조");
        assert!(c[2].sources & src::A != 0, "창에 합친 근거를 남긴다");
    }

    #[test]
    fn 같은_사각형_단계는_빼고_커서_밖_작은_것은_버린다() {
        let p = PhysPoint { x: 400, y: 330 };
        let els = [
            ElementCand { rect: PhysRect::new(300, 300, 300, 40), control: Control::Text },
            ElementCand { rect: PhysRect::new(300, 300, 301, 41), control: Control::Button }, // ±1 px 같음 → 합침, 단추 이름
            ElementCand { rect: PhysRect::new(700, 700, 50, 50), control: Control::Button },  // 커서 밖
            ElementCand { rect: PhysRect::new(399, 329, 3, 3), control: Control::Image },     // 너무 작음
        ];
        let c = build_chain(&ev(p, &els, &[]));
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].kind, NodeKind::Element(Control::Button));
    }

    #[test]
    fn v_상자는_접근성이_없을_때_주_근거() {
        let p = PhysPoint { x: 500, y: 500 };
        let vis = [
            VisionCand { rect: PhysRect::new(450, 480, 200, 40), kind: VisionKind::Box, support: 0.9 },
            VisionCand { rect: PhysRect::new(400, 400, 600, 300), kind: VisionKind::Box, support: 0.8 },
        ];
        let sup = |r: PhysRect| if r.w == 200 { 0.9 } else { 0.8 };
        let mut e = ev(p, &[], &vis);
        e.support = Some(&sup);
        let c = build_chain(&e);
        assert_eq!(c.iter().map(|n| n.kind.label()).collect::<Vec<_>>(), vec!["상자", "상자", "창", "화면"]);
        assert_eq!(first_level(&c, None), 0);
        assert!(c[0].score >= FIRST_SCORE);
    }

    #[test]
    fn 경계가_안_보이는_큰_그릇은_첫_강조에서_뺀다() {
        let p = PhysPoint { x: 500, y: 500 };
        let els = [ElementCand { rect: PhysRect::new(200, 200, 800, 600), control: Control::Pane }];
        let sup = |_r: PhysRect| 0.05;
        let mut e = ev(p, &els, &[]);
        e.support = Some(&sup);
        let c = build_chain(&e);
        assert!(c[0].hidden_container);
        assert_eq!(c[first_level(&c, None)].kind, NodeKind::Window, "그릇 대신 창");
    }

    #[test]
    fn 경계_맞춤은_a_사각형을_v_경계로_붙인다() {
        let p = PhysPoint { x: 400, y: 330 };
        let els = [ElementCand { rect: PhysRect::new(301, 299, 298, 42), control: Control::Button }];
        let snap = |r: PhysRect| if (r.x - 300).abs() <= 3 { PhysRect::new(300, 300, 300, 40) } else { r };
        let mut e = ev(p, &els, &[]);
        e.snap = Some(&snap);
        assert_eq!(build_chain(&e)[0].rect, PhysRect::new(300, 300, 300, 40));
    }

    #[test]
    fn 깊이_기억은_같은_종류를_다시_고른다() {
        let row = |y| [
            ElementCand { rect: PhysRect::new(300, y, 300, 40), control: Control::DataItem },
            ElementCand { rect: PhysRect::new(300, 260, 600, 400), control: Control::Table },
        ];
        let a = build_chain(&ev(PhysPoint { x: 400, y: 330 }, &row(300), &[]));
        // 휠로 표(1)를 골랐다
        let it = LevelIntent::of(&a, 1).unwrap();
        let b = build_chain(&ev(PhysPoint { x: 400, y: 390 }, &row(380), &[]));
        assert_eq!(b[first_level(&b, Some(&it))].kind, NodeKind::Element(Control::Table));
        // 행을 골랐으면 옮겨도 행
        let it = LevelIntent::of(&a, 0).unwrap();
        assert_eq!(b[first_level(&b, Some(&it))].rect.y, 380);
    }

    #[test]
    fn 제목_표시줄_위에서는_창이_첫_강조() {
        let p = PhysPoint { x: 400, y: 115 };
        let els = [
            ElementCand { rect: PhysRect::new(380, 105, 120, 20), control: Control::Text },
            ElementCand { rect: PhysRect::new(100, 100, 1200, 32), control: Control::TitleBar },
        ];
        let c = build_chain(&ev(p, &els, &[]));
        assert_eq!(c[first_level(&c, None)].kind, NodeKind::Window);
        assert_eq!(c[0].kind, NodeKind::Element(Control::Text), "제목 글자는 휠 아래로 남는다");
    }

    #[test]
    fn uia_종류_번호() {
        assert_eq!(Control::from_uia(50000), Control::Button);
        assert_eq!(Control::from_uia(50029), Control::DataItem);
        assert_eq!(Control::from_uia(99999), Control::Other);
    }
}
