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

    /// macOS 접근성 역할(`AXRole`, 탭 단추는 `AXSubrole` `AXTabButton`을 넘긴다) → 종류(M9, I-mac §2.1).
    pub fn from_ax(role: &str) -> Control {
        match role {
            "AXButton" | "AXPopUpButton" | "AXMenuButton" | "AXDisclosureTriangle" | "AXIncrementor" | "AXColorWell" => Control::Button,
            "AXTextField" | "AXTextArea" | "AXSearchField" | "AXSecureTextField" => Control::Edit,
            "AXStaticText" | "AXHeading" => Control::Text,
            "AXImage" => Control::Image,
            "AXList" | "AXCollection" => Control::List,
            "AXTable" | "AXGrid" => Control::Table,
            "AXOutline" | "AXBrowser" => Control::Tree,
            "AXRow" => Control::DataItem,
            "AXCell" => Control::ListItem,
            "AXTabGroup" => Control::Tab,
            "AXTabButton" => Control::TabItem,
            "AXRadioButton" => Control::Button,
            "AXMenu" | "AXMenuBar" => Control::Menu,
            "AXMenuItem" | "AXMenuBarItem" => Control::MenuItem,
            "AXToolbar" => Control::ToolBar,
            "AXWebArea" | "AXDocument" => Control::Document,
            "AXLink" => Control::Hyperlink,
            "AXCheckBox" | "AXSwitch" => Control::CheckBox,
            "AXComboBox" => Control::ComboBox,
            "AXGroup" | "AXScrollArea" | "AXRadioGroup" | "AXSegmentedControl" => Control::Group,
            "AXSplitGroup" | "AXSheet" | "AXLayoutArea" | "AXDrawer" => Control::Pane,
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
    /// true = MSAA 대체 경로(M 근거), false = UIA(A 근거). 옛 사례 파일에는 없다 → 기본 false, false면 쓰지 않음(사례 스키마 1 그대로)
    #[serde(default, rename = "fromMsaa", skip_serializing_if = "is_false")]
    pub from_msaa: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
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
    /// 그 창보다 앞에 있어 겹치는 창들(R2-CAP-1 가림 인지) — 요소·자식 창 사각형에서 이 부분을 뺀다. 없으면 빈 칸.
    pub occluders: &'a [PhysRect],
    /// 사각형의 경계 지지율(V). V가 없으면 None.
    pub support: Option<&'a dyn Fn(PhysRect) -> f32>,
    /// V 곧은 경계로 붙이기(±3 px). V가 없으면 None.
    pub snap: Option<&'a dyn Fn(PhysRect) -> PhysRect>,
}

const MIN_SIDE: i32 = 4;
const MIN_AREA: i64 = 16;
/// 사다리·후보 겹침 합침 IoU 문턱(R2-CAP-4).
pub const MERGE_IOU: f64 = 0.90;
/// 앞 창 경계 ±2 px 제외 허용 오차(R2-CAP-2).
pub const OCCLUDER_EDGE_TOL: i32 = 2;
const HIDDEN_SUPPORT: f32 = 0.15;
/// 가린 뒤 보이는 넓이가 원래의 이 비율보다 작으면 후보에서 뺀다(R2-CAP-1).
pub const MIN_VISIBLE: f64 = 0.30;
/// 가림 계산에 쓰는 앞 창 수 상한(갈래가 4^n으로 늘지 않게 — 커서를 담는 쪽만 따라가 실제로는 몇 갈래뿐).
const MAX_OCCLUDERS: usize = 8;

/// 융합 추가 옵션(R2-CAP-3): 이미지 요소의 여백·그림자를 걷어내어 실제 그림 가장자리를 다시 잰다.
#[derive(Default)]
pub struct RecogOptions<'a> {
    pub refine_image: Option<&'a dyn Fn(PhysRect) -> PhysRect>,
}

/// R2-CAP-6 계측 준비용 요약 숫자(창 제목·파일 이름 없음).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecogMetrics {
    pub sources: u8,
    pub occluded_ratio: f32,
    pub picked_level: usize,
    pub chain_len: usize,
}

#[inline]
fn h_edge_touches_occluders(y: i32, x0: i32, x1: i32, occluders: &[PhysRect]) -> bool {
    occluders.iter().any(|o| {
        ((y - o.y).abs() <= OCCLUDER_EDGE_TOL || (y - o.bottom()).abs() <= OCCLUDER_EDGE_TOL)
            && x1 > o.x
            && x0 < o.right()
    })
}

#[inline]
fn v_edge_touches_occluders(x: i32, y0: i32, y1: i32, occluders: &[PhysRect]) -> bool {
    occluders.iter().any(|o| {
        ((x - o.x).abs() <= OCCLUDER_EDGE_TOL || (x - o.right()).abs() <= OCCLUDER_EDGE_TOL)
            && y1 > o.y
            && y0 < o.bottom()
    })
}

/// V 후보 사각형이 앞 창과 겹치거나 네 변 중 하나가 앞 창 경계 ±2 px에 붙는지 판정한다(R2-CAP-2).
pub fn touches_occluder_edge(r: PhysRect, occluders: &[PhysRect]) -> bool {
    occluders.iter().any(|o| r.intersect(o).is_some_and(|i| i.area() > 0))
        || h_edge_touches_occluders(r.y, r.x, r.right(), occluders)
        || h_edge_touches_occluders(r.bottom(), r.x, r.right(), occluders)
        || v_edge_touches_occluders(r.x, r.y, r.bottom(), occluders)
        || v_edge_touches_occluders(r.right(), r.y, r.bottom(), occluders)
}

#[inline]
fn rect_overlaps_occluders(r: PhysRect, occluders: &[PhysRect]) -> bool {
    !r.is_empty() && occluders.iter().any(|o| r.intersect(o).is_some_and(|i| i.area() > 0))
}

/// 앞 창에 잘린 변은 원래 잘린 좌표를 유지하고, 안 잘린 변도 앞 창 경계 ±2 px로 끌리거나
/// 원래보다 앞 창과 더 겹치게 되면 그 변만 되돌린다(R2-CAP-2).
fn snap_safe(orig: PhysRect, snapped: PhysRect, clipped: (bool, bool, bool, bool), occluders: &[PhysRect]) -> PhysRect {
    if occluders.is_empty() {
        return snapped;
    }
    let pick = |clipped_side: bool, o: i32, s: i32, blocked: bool| -> i32 {
        if clipped_side || (s != o && blocked) { o } else { s }
    };
    let (cl, ct, cr, cb) = clipped;
    let t_blocked = h_edge_touches_occluders(snapped.y, orig.x, orig.right(), occluders)
        || (snapped.y < orig.y
            && rect_overlaps_occluders(PhysRect::from_edges(orig.x, snapped.y, orig.right(), orig.y), occluders));
    let b_blocked = h_edge_touches_occluders(snapped.bottom(), orig.x, orig.right(), occluders)
        || (snapped.bottom() > orig.bottom()
            && rect_overlaps_occluders(PhysRect::from_edges(orig.x, orig.bottom(), orig.right(), snapped.bottom()), occluders));
    let mut t = pick(ct, orig.y, snapped.y, t_blocked);
    let mut b = pick(cb, orig.bottom(), snapped.bottom(), b_blocked);
    if b <= t {
        t = orig.y;
        b = orig.bottom();
    }
    let l_blocked = v_edge_touches_occluders(snapped.x, t, b, occluders)
        || (snapped.x < orig.x
            && rect_overlaps_occluders(PhysRect::from_edges(snapped.x, t, orig.x, b), occluders));
    let r_blocked = v_edge_touches_occluders(snapped.right(), t, b, occluders)
        || (snapped.right() > orig.right()
            && rect_overlaps_occluders(PhysRect::from_edges(orig.right(), t, snapped.right(), b), occluders));
    let mut l = pick(cl, orig.x, snapped.x, l_blocked);
    let mut r = pick(cr, orig.right(), snapped.right(), r_blocked);
    if r <= l {
        l = orig.x;
        r = orig.right();
    }
    let out = PhysRect::from_edges(l, t, r, b);
    if out.is_empty() { orig } else { out }
}

/// 사슬·근거에서 계측용 숫자(근거 비트·가림 비율·고른 단계)를 요약한다(R2-CAP-6).
pub fn metrics_of(ev: &Evidence, chain: &[Node], picked_level: usize) -> RecogMetrics {
    let sources = chain.get(picked_level).map_or(0, |n| n.sources);
    let occluded_ratio = match ev.window {
        Some(w) if w.area() > 0 && !ev.occluders.is_empty() => {
            let occ_area: i64 = ev
                .occluders
                .iter()
                .filter_map(|o| w.intersect(o).map(|i| i.area()))
                .sum();
            (occ_area as f64 / w.area() as f64).clamp(0.0, 1.0) as f32
        }
        _ => 0.0,
    };
    RecogMetrics {
        sources,
        occluded_ratio,
        picked_level,
        chain_len: chain.len(),
    }
}

/// 그 창보다 앞(z가 작음)에 있고 겹치는 창 사각형들 — 근거의 `occluders`. 오버레이로 의심되는 창은 뺀다.
pub fn occluders_of(windows: &[crate::snapshot::WindowInfo], target: u64) -> Vec<PhysRect> {
    let Some(t) = windows.iter().find(|w| w.handle == target) else { return Vec::new() };
    windows
        .iter()
        .filter(|w| w.handle != target && w.z < t.z && !w.suspect_overlay && w.rect.intersect(&t.rect).is_some_and(|r| r.area() > 0))
        .map(|w| w.rect)
        .take(MAX_OCCLUDERS)
        .collect()
}

/// 가림 인지(R2-CAP-1): `rect`에서 앞 창들에 가린 곳을 뺀 "보이는 부분" 가운데 **커서를 담는 가장 큰 사각형**.
/// 가림이 없으면 그대로, 보이는 넓이가 원래의 [`MIN_VISIBLE`]보다 작으면 None(후보에서 뺀다).
/// 가린 창 하나를 빼면 남는 최대 사각형은 위·아래·왼쪽·오른쪽 띠 넷이다 — 커서를 담는 띠만 따라가며 다음 창을 뺀다.
pub fn visible_part(rect: PhysRect, occluders: &[PhysRect], p: PhysPoint) -> Option<PhysRect> {
    fn go(r: PhysRect, occ: &[PhysRect], p: PhysPoint) -> Option<PhysRect> {
        let Some((o, rest)) = occ.split_first() else { return Some(r) };
        let Some(i) = r.intersect(o).filter(|i| i.area() > 0) else { return go(r, rest, p) };
        if i.contains(p) {
            return None;
        }
        let strips = [
            PhysRect::new(r.x, r.y, r.w, i.y - r.y),                     // 위
            PhysRect::new(r.x, i.bottom(), r.w, r.bottom() - i.bottom()), // 아래
            PhysRect::new(r.x, r.y, i.x - r.x, r.h),                     // 왼쪽
            PhysRect::new(i.right(), r.y, r.right() - i.right(), r.h),   // 오른쪽
        ];
        strips
            .into_iter()
            .filter(|s| s.w > 0 && s.h > 0 && s.contains(p))
            .filter_map(|s| go(s, rest, p))
            .max_by_key(|s| s.area())
    }
    if occluders.is_empty() {
        return Some(rect);
    }
    let v = go(rect, occluders, p)?;
    (v.area() as f64 >= rect.area() as f64 * MIN_VISIBLE).then_some(v)
}
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
    build_chain_with(ev, &RecogOptions::default())
}

/// 추가 옵션(`refine_image` 등)과 함께 사슬을 만든다(R2-CAP-2·3·4).
pub fn build_chain_with(ev: &Evidence, opts: &RecogOptions) -> Vec<Node> {
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
    // 요소·자식 창: 앞 창 가림 제거(R2-CAP-1) → Image면 창·모니터로 자른 뒤 평탄도 재측정(R2-CAP-3) → 앞 창 경계 보호 경계 맞춤(R2-CAP-2)
    for e in ev.elements {
        if let Some(vis) = visible_part(e.rect, ev.occluders, p) {
            let occ_l = vis.x != e.rect.x;
            let occ_t = vis.y != e.rect.y;
            let occ_r = vis.right() != e.rect.right();
            let occ_b = vis.bottom() != e.rect.bottom();
            let base = if e.control == Control::Image {
                if let Some(refine) = opts.refine_image {
                    if let Some(bounded) = vis.intersect(&clip_to).and_then(|r| r.intersect(&ev.monitor)) {
                        // 앞 창에 잘린 변은 ±2 px 경계·그림자 띠를 피해 재측정을 요청한 뒤, 잘린 변은 원래 경계로 복원한다
                        let inset_l = if occ_l { OCCLUDER_EDGE_TOL } else { 0 };
                        let inset_t = if occ_t { OCCLUDER_EDGE_TOL } else { 0 };
                        let inset_r = if occ_r { OCCLUDER_EDGE_TOL } else { 0 };
                        let inset_b = if occ_b { OCCLUDER_EDGE_TOL } else { 0 };
                        let probe_cand = PhysRect::from_edges(
                            bounded.x + inset_l,
                            bounded.y + inset_t,
                            bounded.right() - inset_r,
                            bounded.bottom() - inset_b,
                        );
                        let probe = if probe_cand.w >= 16 && probe_cand.h >= 16 && probe_cand.contains(p) {
                            probe_cand
                        } else {
                            bounded
                        };
                        let raw_cand = refine(probe);
                        let cand = if let Some(c) = probe.intersect(&raw_cand) {
                            // 잘린 변이 그대로 닿아 있거나 다른 변이 안 변한 채 잘린 변만 줄었다면(가린 단색 오탐) 원래 경계 유지
                            let top = if occ_t && (c.y <= probe.y || (c.x == probe.x && c.right() == probe.right())) {
                                bounded.y
                            } else {
                                c.y
                            };
                            let bot = if occ_b && (c.bottom() >= probe.bottom() || (c.x == probe.x && c.right() == probe.right())) {
                                bounded.bottom()
                            } else {
                                c.bottom()
                            };
                            let left = if occ_l && (c.x <= probe.x || (c.y == probe.y && c.bottom() == probe.bottom())) {
                                bounded.x
                            } else {
                                c.x
                            };
                            let right = if occ_r && (c.right() >= probe.right() || (c.y == probe.y && c.bottom() == probe.bottom())) {
                                bounded.right()
                            } else {
                                c.right()
                            };
                            PhysRect::from_edges(left, top, right, bot)
                        } else {
                            bounded
                        };
                        bounded
                            .intersect(&cand)
                            .filter(|r| r.contains(p) && r.w >= MIN_SIDE && r.h >= MIN_SIDE && r.area() >= MIN_AREA)
                            .unwrap_or(bounded)
                    } else {
                        vis
                    }
                } else {
                    vis
                }
            } else {
                vis
            };
            let clipped = (
                occ_l && base.x == vis.x,
                occ_t && base.y == vis.y,
                occ_r && base.right() == vis.right(),
                occ_b && base.bottom() == vis.bottom(),
            );
            // 항목은 줄 사이에 선이 없어도 접근성 경계 자체가 선택 범위다.
            // 메뉴 바깥 프레임이나 인접 행의 테두리에 붙이지 않는다.
            let snapped = if matches!(e.control, Control::MenuItem | Control::ListItem | Control::DataItem) {
                base
            } else {
                snap_safe(base, snap(base), clipped, ev.occluders)
            };
            push(snapped, NodeKind::Element(e.control), if e.from_msaa { src::M } else { src::A });
        }
    }
    for c in ev.children {
        if let Some(vis) = visible_part(*c, ev.occluders, p) {
            let clipped = (
                vis.x != c.x,
                vis.y != c.y,
                vis.right() != c.right(),
                vis.bottom() != c.bottom(),
            );
            let snapped = snap_safe(vis, snap(vis), clipped, ev.occluders);
            push(snapped, NodeKind::Child, src::H);
        }
    }
    // V 후보: 앞 창 경계 ±2 px에 붙었거나 앞 창과 겹치는 후보는 버린다(R2-CAP-2)
    for v in ev.vision {
        if !ev.occluders.is_empty() && touches_occluder_edge(v.rect, ev.occluders) {
            continue;
        }
        push(v.rect, NodeKind::Vision(v.kind), src::V);
    }

    // 중복 합치기(IoU ≥ 0.90): 뜻이 있는 종류(요소 > 자식 창 > V)를 남기고 근거를 합친다(R2-CAP-4)
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
    // 면적 오름차순으로 합쳐 테두리 속면(Flat)·안쪽 래퍼·바깥 상자가 입력 순서와 무관하게 추이적으로 합쳐지게 한다(R2-CAP-4)
    raw.sort_by_key(|n| n.rect.area());
    let mut merged: Vec<Node> = Vec::new();
    for n in raw {
        if let Some(m) = merged.iter_mut().find(|m| iou(&m.rect, &n.rect) >= MERGE_IOU) {
            m.sources |= n.sources;
            let better = rank(n.kind) > rank(m.kind)
                || (rank(n.kind) == rank(m.kind) && meaning(n.kind) > meaning(m.kind))
                || (rank(n.kind) == rank(m.kind)
                    && (meaning(n.kind) - meaning(m.kind)).abs() <= f32::EPSILON
                    && n.rect.area() > m.rect.area());
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
        // V가 직접 확인한 색 경계 지지율을 재사용한다. 창에 잘린 후보는 다시 잰다.
        let s = match n.kind {
            NodeKind::Vision(_) => ev.vision.iter().find(|v| v.rect == n.rect)
                .map(|v| v.support).or_else(|| support(n.rect)),
            _ => support(n.rect),
        };
        if let (NodeKind::Element(c), Some(s)) = (n.kind, s) {
            n.hidden_container = c.is_container() && s < HIDDEN_SUPPORT;
        }
        n.score = score(n, s.unwrap_or(0.5), window);
    }

    // 사슬: 넓이 순, 엄격히 커지게 — IoU ≥ 0.90 또는 ±1 px 같은 것은 합쳐서 휠·Tab마다 강조가 반드시 바뀐다(R2-CAP-4)
    merged.sort_by_key(|n| n.rect.area());
    let mut chain: Vec<Node> = Vec::new();
    for n in merged.into_iter().chain(tail) {
        if let Some(last) = chain.last_mut() {
            let same_tail = rank(last.kind) == 0 && rank(n.kind) == 0;
            let dup = if same_tail {
                near_same(&last.rect, &n.rect)
            } else {
                near_same(&last.rect, &n.rect) || iou(&last.rect, &n.rect) >= MERGE_IOU
            };
            if dup {
                // 조각과 창·모니터가 같으면 창·모니터 이름을 살리고, 조각끼리 같으면 더 뜻있고 점수 높은 쪽을 살린다
                let sources = last.sources | n.sources;
                if rank(last.kind) > 0 && rank(n.kind) == 0 {
                    *last = Node { sources, ..n };
                } else if rank(last.kind) > 0 && rank(n.kind) > 0 {
                    let better = rank(n.kind) > rank(last.kind)
                        || (rank(n.kind) == rank(last.kind) && meaning(n.kind) > meaning(last.kind))
                        || (rank(n.kind) == rank(last.kind)
                            && (meaning(n.kind) - meaning(last.kind)).abs() <= f32::EPSILON
                            && n.score > last.score + 0.001);
                    if better {
                        *last = Node { sources, ..n };
                    } else {
                        last.sources = sources;
                    }
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

/// 첫 강조 단계(08 §4.5). 깊이 기억, 제목 표시줄, 구조로 확인한 조작 요소, 점수 문턱을 차례로 적용한다.
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
    let window_level = window_index(chain);
    let window = window_level.map(|i| chain[i].rect);
    if let Some(wi) = window_level {
        // 내용 안쪽에 남은 오래된 제목 표시줄 좌표가 화면 전체 선택을 가로채지 않게 한다.
        if chain[..wi].iter().any(|n| {
            n.kind == NodeKind::Element(Control::TitleBar) && n.rect.y - chain[wi].rect.y <= n.rect.h
        }) {
            return wi;
        }
    }

    // 자식 창 또는 접근성 목록 항목을 기준으로 고른다. 문서 전체를 감싸는
    // 큰 편집 창에서는 그 안의 영상 후보를 계속 쓸 수 있게 한다.
    let preferred = chain.iter().position(|n| {
        let native = n.sources & src::H != 0 && size_fit(&n.rect, window) == 1.0;
        match n.kind {
            NodeKind::Element(Control::ListItem | Control::DataItem | Control::MenuItem) => true,
            NodeKind::Child | NodeKind::Element(Control::Other) => native,
            NodeKind::Element(c) => native && !n.hidden_container
                && !matches!(c, Control::Pane | Control::Custom | Control::Document | Control::TitleBar),
            _ => false,
        }
    });
    if let Some(i) = preferred {
        let parent = chain[i];
        let row = matches!(parent.kind, NodeKind::Element(Control::ListItem | Control::DataItem | Control::MenuItem));
        // 행 안의 단순 글자·편집 레이블은 휠로 남기고 따로 확인된 조작 요소는 먼저 고른다.
        let inner = chain[..i].iter().position(|n| {
            matches!(n.kind, NodeKind::Element(c) if c.meaning() >= 0.85)
                && n.score >= FIRST_SCORE
                && n.rect.intersect(&parent.rect) == Some(n.rect)
                && (!row || n.sources & (src::V | src::H) != 0)
        });
        return inner.unwrap_or(i);
    }
    if let Some(i) = chain.iter().position(|n| !n.hidden_container && n.score >= FIRST_SCORE && !matches!(n.kind, NodeKind::Window | NodeKind::Monitor | NodeKind::Virtual)) {
        return i;
    }
    window_level.or_else(|| chain.iter().position(|n| n.kind == NodeKind::Monitor)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msaa_요소는_근거_m으로_uia와_합치면_둘_다() {
        let p = PhysPoint { x: 400, y: 330 };
        let m = ElementCand { rect: PhysRect::new(300, 300, 300, 40), control: Control::Button, from_msaa: true };
        let c = build_chain(&ev(p, &[m], &[]));
        assert_eq!(c[0].kind, NodeKind::Element(Control::Button));
        assert!(c[0].sources & src::M != 0 && c[0].sources & src::A == 0, "MSAA만 = M: {:b}", c[0].sources);
        // 같은 자리 UIA 요소와 합치면 근거 A·M 둘 다
        let a = ElementCand { from_msaa: false, ..m };
        let c = build_chain(&ev(p, &[m, a], &[]));
        assert_eq!(c[0].sources & (src::A | src::M), src::A | src::M);
    }

    #[test]
    fn 옛_사례의_요소는_msaa_표시가_없어도_읽히고_uia면_쓰지_않는다() {
        let old: ElementCand = serde_json::from_str(r#"{"rect":{"x":1,"y":2,"w":30,"h":40},"control":"button"}"#).unwrap();
        assert!(!old.from_msaa);
        assert!(!serde_json::to_string(&old).unwrap().contains("fromMsaa"), "UIA 요소는 사례 파일 모양 그대로");
        let m = ElementCand { from_msaa: true, ..old };
        let back: ElementCand = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert!(back.from_msaa);
    }

    const MON: PhysRect = PhysRect::new(0, 0, 1920, 1080);
    const WIN: PhysRect = PhysRect::new(100, 100, 1200, 800);

    fn ev<'a>(p: PhysPoint, elements: &'a [ElementCand], vision: &'a [VisionCand]) -> Evidence<'a> {
        Evidence { point: p, window: Some(WIN), children: &[], elements, vision, monitor: MON, virtual_bounds: MON, occluders: &[], support: None, snap: None }
    }

    #[test]
    fn 표_사슬은_작은_것부터_창_화면까지() {
        let p = PhysPoint { x: 400, y: 330 };
        let els = [
            ElementCand { rect: PhysRect::new(300, 300, 300, 40), control: Control::DataItem, from_msaa: false },
            ElementCand { rect: PhysRect::new(300, 260, 600, 400), control: Control::Table, from_msaa: false },
            ElementCand { rect: PhysRect::new(100, 100, 1200, 800), control: Control::Pane, from_msaa: false }, // 창과 같은 그릇 → 창으로 합침
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
            ElementCand { rect: PhysRect::new(300, 300, 300, 40), control: Control::Text, from_msaa: false },
            ElementCand { rect: PhysRect::new(300, 300, 301, 41), control: Control::Button, from_msaa: false }, // ±1 px 같음 → 합침, 단추 이름
            ElementCand { rect: PhysRect::new(700, 700, 50, 50), control: Control::Button, from_msaa: false },  // 커서 밖
            ElementCand { rect: PhysRect::new(399, 329, 3, 3), control: Control::Image, from_msaa: false },     // 너무 작음
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
        let els = [ElementCand { rect: PhysRect::new(200, 200, 800, 600), control: Control::Pane, from_msaa: false }];
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
        let els = [ElementCand { rect: PhysRect::new(301, 299, 298, 42), control: Control::Button, from_msaa: false }];
        let snap = |r: PhysRect| if (r.x - 300).abs() <= 3 { PhysRect::new(300, 300, 300, 40) } else { r };
        let mut e = ev(p, &els, &[]);
        e.snap = Some(&snap);
        assert_eq!(build_chain(&e)[0].rect, PhysRect::new(300, 300, 300, 40));
    }

    #[test]
    fn 깊이_기억은_같은_종류를_다시_고른다() {
        let row = |y| [
            ElementCand { rect: PhysRect::new(300, y, 300, 40), control: Control::DataItem, from_msaa: false },
            ElementCand { rect: PhysRect::new(300, 260, 600, 400), control: Control::Table, from_msaa: false },
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
            ElementCand { rect: PhysRect::new(380, 105, 120, 20), control: Control::Text, from_msaa: false },
            ElementCand { rect: PhysRect::new(100, 100, 1200, 32), control: Control::TitleBar, from_msaa: false },
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

    #[test]
    fn ax_역할문자열_종류_매핑() {
        for (role, want) in [
            ("AXButton", Control::Button),
            ("AXPopUpButton", Control::Button),
            ("AXTextField", Control::Edit),
            ("AXSearchField", Control::Edit),
            ("AXStaticText", Control::Text),
            ("AXImage", Control::Image),
            ("AXList", Control::List),
            ("AXTable", Control::Table),
            ("AXOutline", Control::Tree),
            ("AXRow", Control::DataItem),
            ("AXCell", Control::ListItem),
            ("AXTabGroup", Control::Tab),
            ("AXTabButton", Control::TabItem),
            ("AXMenuItem", Control::MenuItem),
            ("AXToolbar", Control::ToolBar),
            ("AXWebArea", Control::Document),
            ("AXLink", Control::Hyperlink),
            ("AXCheckBox", Control::CheckBox),
            ("AXComboBox", Control::ComboBox),
            ("AXGroup", Control::Group),
            ("AXSplitGroup", Control::Pane),
            ("AXSlider", Control::Other),
            ("", Control::Other),
        ] {
            assert_eq!(Control::from_ax(role), want, "{role}");
        }
    }

    #[test]
    fn 위가_가린_사진은_보이는_부분만_후보() {
        let photo = PhysRect::new(100, 100, 400, 300);
        // 위 60 px를 다른 창이 가림
        let occ = [PhysRect::new(0, 0, 800, 160)];
        assert_eq!(visible_part(photo, &occ, PhysPoint { x: 300, y: 300 }), Some(PhysRect::new(100, 160, 400, 240)));
        // 가림이 없으면 그대로, 겹치지 않는 창은 무시
        assert_eq!(visible_part(photo, &[], PhysPoint { x: 300, y: 300 }), Some(photo));
        assert_eq!(visible_part(photo, &[PhysRect::new(600, 0, 100, 100)], PhysPoint { x: 300, y: 300 }), Some(photo));
    }

    #[test]
    fn 모서리를_가리면_커서를_담는_가장_큰_띠() {
        let photo = PhysRect::new(100, 100, 400, 300);
        // 왼쪽 위 모서리 가림 → 아래 띠(400×200)와 오른쪽 띠(250×300) 중 커서를 담는 큰 것
        let occ = [PhysRect::new(0, 0, 250, 200)];
        assert_eq!(visible_part(photo, &occ, PhysPoint { x: 300, y: 300 }), Some(PhysRect::new(100, 200, 400, 200)));
        // 커서가 오른쪽 위면 오른쪽 띠만 커서를 담는다
        assert_eq!(visible_part(photo, &occ, PhysPoint { x: 400, y: 150 }), Some(PhysRect::new(250, 100, 250, 300)));
        // 두 창이 위·왼쪽을 가림
        let occ2 = [PhysRect::new(0, 0, 800, 150), PhysRect::new(0, 0, 180, 900)];
        assert_eq!(visible_part(photo, &occ2, PhysPoint { x: 300, y: 300 }), Some(PhysRect::new(180, 150, 320, 250)));
    }

    #[test]
    fn 거의_다_가리면_후보에서_뺀다() {
        let photo = PhysRect::new(100, 100, 400, 300);
        // 아래 40 px만 보임 → 13% < 30%
        let occ = [PhysRect::new(0, 0, 800, 360)];
        assert_eq!(visible_part(photo, &occ, PhysPoint { x: 300, y: 380 }), None);
    }

    #[test]
    fn 사슬에서_가린_요소가_가린_창_위로_넘어가지_않는다() {
        let p = PhysPoint { x: 300, y: 300 };
        let img = [ElementCand { rect: PhysRect::new(100, 100, 400, 300), control: Control::Image, from_msaa: false }];
        let occ = [PhysRect::new(0, 0, 1920, 160)];
        let ev = Evidence { point: p, window: Some(WIN), children: &[], elements: &img, vision: &[], monitor: MON, virtual_bounds: MON, occluders: &occ, support: None, snap: None };
        let chain = build_chain(&ev);
        let first = chain.iter().find(|n| matches!(n.kind, NodeKind::Element(Control::Image))).expect("사진 마디");
        assert_eq!(first.rect, PhysRect::new(100, 160, 400, 240));
    }

    #[test]
    fn 앞_창만_가림으로_본다() {
        use crate::snapshot::WindowInfo;
        let w = |handle: u64, z: u32, rect: PhysRect, overlay: bool| WindowInfo { handle, pid: 1, exe: String::new(), class: String::new(), rect, z, topmost: false, window_only: false, suspect_overlay: overlay };
        let wins = [
            w(1, 0, PhysRect::new(0, 0, 1920, 1080), true),   // 오버레이 의심 — 뺀다
            w(2, 1, PhysRect::new(0, 0, 800, 160), false),    // 앞, 겹침
            w(3, 2, PhysRect::new(1500, 900, 100, 100), false), // 앞, 안 겹침
            w(4, 3, PhysRect::new(50, 50, 900, 700), false),  // 대상
            w(5, 4, PhysRect::new(0, 0, 1920, 1080), false),  // 뒤
        ];
        assert_eq!(occluders_of(&wins, 4), vec![PhysRect::new(0, 0, 800, 160)]);
        assert!(occluders_of(&wins, 99).is_empty());
    }

    #[test]
    fn 가린_창_모서리에_붙은_v_후보는_버리고_snap은_끌리지_않는다() {
        let p = PhysPoint { x: 300, y: 300 };
        let photo = PhysRect::new(100, 100, 400, 300);
        let occ = [PhysRect::new(0, 0, 1920, 145)]; // 위 15% (45 px) 가림
        let img = [ElementCand { rect: photo, control: Control::Image, from_msaa: false }];
        // 앞 창 아래 모서리(y=145) ±2 px에 붙은 V 후보 + 그 안에 있는 정상 카드
        let vis = [
            VisionCand { rect: PhysRect::new(100, 146, 400, 254), kind: VisionKind::Box, support: 0.9 },
            VisionCand { rect: PhysRect::new(180, 180, 240, 180), kind: VisionKind::Box, support: 0.85 },
        ];
        // snap이 잘린 윗변(145)을 그림자(147)로 끌고 가려 해도 snap_safe가 막는다
        let snap = |r: PhysRect| PhysRect::new(r.x, r.y + 2, r.w, r.h - 2);
        let ev = Evidence {
            point: p,
            window: Some(WIN),
            children: &[],
            elements: &img,
            vision: &vis,
            monitor: MON,
            virtual_bounds: MON,
            occluders: &occ,
            support: None,
            snap: Some(&snap),
        };
        let chain = build_chain(&ev);
        assert!(!chain.iter().any(|n| matches!(n.kind, NodeKind::Vision(_)) && n.rect.y <= 147), "앞 창 ±2px V 후보는 제외: {chain:?}");
        let img_node = chain.iter().find(|n| matches!(n.kind, NodeKind::Element(Control::Image))).unwrap();
        assert_eq!(img_node.rect, PhysRect::new(100, 145, 400, 255), "잘린 윗변은 앞 창 그림자에 끌리지 않는다");

        // 가려지지 않았지만 앞 창 아래 2 px에 붙어 놓인 요소도 snap이 y=145로 붙이지 않는다
        let near = [ElementCand { rect: PhysRect::new(100, 147, 400, 250), control: Control::Image, from_msaa: false }];
        let snap_to_occ = |r: PhysRect| PhysRect::new(r.x, 145, r.w, r.h + 2);
        let ev2 = Evidence { elements: &near, vision: &[], snap: Some(&snap_to_occ), ..ev };
        let c2 = build_chain(&ev2);
        let img2 = c2.iter().find(|n| matches!(n.kind, NodeKind::Element(Control::Image))).unwrap();
        assert_eq!(img2.rect.y, 147);
    }

    #[test]
    fn 이미지_요소는_평탄도_재측정으로_여백을_걷는다() {
        let p = PhysPoint { x: 300, y: 250 };
        let elem = PhysRect::new(100, 100, 400, 300);
        let inner = PhysRect::new(130, 125, 340, 250);
        let els = [ElementCand { rect: elem, control: Control::Image, from_msaa: false }];
        let ev = ev(p, &els, &[]);
        let opts = RecogOptions { refine_image: Some(&|_| inner) };
        let chain = build_chain_with(&ev, &opts);
        assert_eq!(chain[0].rect, inner);
    }

    #[test]
    fn 사다리_겹침_정리는_iou_090_이상_후보를_하나로_합친다() {
        let p = PhysPoint { x: 400, y: 210 };
        // ListItem(800×120) vs Group(790×110) → IoU = 0.9052 >= 0.90
        let item = PhysRect::new(110, 160, 800, 120);
        let wrapper = PhysRect::new(115, 165, 790, 110);
        let els = [
            ElementCand { rect: wrapper, control: Control::Group, from_msaa: false },
            ElementCand { rect: item, control: Control::ListItem, from_msaa: false },
        ];
        let chain = build_chain(&ev(p, &els, &[]));
        let non_tail: Vec<&Node> = chain
            .iter()
            .filter(|n| !matches!(n.kind, NodeKind::Window | NodeKind::Monitor | NodeKind::Virtual))
            .collect();
        assert_eq!(non_tail.len(), 1, "IoU ≥ 0.90 겹침은 하나로 합친다: {non_tail:?}");
        assert_eq!(non_tail[0].kind, NodeKind::Element(Control::ListItem));
        assert_eq!(non_tail[0].rect, item);
    }

    #[test]
    fn 계측_요약은_근거와_가림_비율과_단계를_담는다() {
        let p = PhysPoint { x: 300, y: 300 };
        let img = [ElementCand { rect: PhysRect::new(100, 100, 400, 300), control: Control::Image, from_msaa: false }];
        let occ = [PhysRect::new(100, 100, 600, 160)];
        let ev = Evidence { point: p, window: Some(WIN), children: &[], elements: &img, vision: &[], monitor: MON, virtual_bounds: MON, occluders: &occ, support: None, snap: None };
        let chain = build_chain(&ev);
        let picked = first_level(&chain, None);
        let m = metrics_of(&ev, &chain, picked);
        assert_eq!(m.sources, src::A);
        assert!(m.occluded_ratio > 0.05 && m.occluded_ratio < 0.15);
        assert_eq!(m.picked_level, picked);
        assert_eq!(m.chain_len, chain.len());
    }

    #[test]
    fn snap_safe는_앞_창_안쪽_3px로_끌려_겹치는_변만_되돌린다() {
        let p = PhysPoint { x: 300, y: 300 };
        let occ = [PhysRect::new(0, 0, 1920, 160)];
        let els = [ElementCand { rect: PhysRect::new(160, 160, 320, 220), control: Control::Image, from_msaa: false }];
        let snap = |r: PhysRect| PhysRect::from_edges(r.x, r.y - 3, r.right() + 1, r.bottom());
        let ev = Evidence {
            point: p,
            window: Some(WIN),
            children: &[],
            elements: &els,
            vision: &[],
            monitor: MON,
            virtual_bounds: MON,
            occluders: &occ,
            support: None,
            snap: Some(&snap),
        };
        let chain = build_chain(&ev);
        let img = chain.iter().find(|n| matches!(n.kind, NodeKind::Element(Control::Image))).unwrap();
        assert_eq!(img.rect, PhysRect::from_edges(160, 160, 481, 380));
    }

    #[test]
    fn 이미지_여백_재측정_변은_앞_창_가림이_있어도_스냅을_막지_않는다() {
        let p = PhysPoint { x: 300, y: 250 };
        let elem = PhysRect::new(100, 100, 400, 300);
        let els = [ElementCand { rect: elem, control: Control::Image, from_msaa: false }];
        let occ = [PhysRect::new(900, 100, 300, 300)];
        let inner = PhysRect::new(120, 120, 360, 260);
        let snapped_inner = PhysRect::new(121, 121, 358, 258);
        let snap = |r: PhysRect| if r == inner { snapped_inner } else { r };
        let ev = Evidence {
            point: p,
            window: Some(WIN),
            children: &[],
            elements: &els,
            vision: &[],
            monitor: MON,
            virtual_bounds: MON,
            occluders: &occ,
            support: None,
            snap: Some(&snap),
        };
        let opts = RecogOptions { refine_image: Some(&|_| inner) };
        let chain = build_chain_with(&ev, &opts);
        let img = chain.iter().find(|n| matches!(n.kind, NodeKind::Element(Control::Image))).unwrap();
        assert_eq!(img.rect, snapped_inner);
    }

    #[test]
    fn refine_image는_창과_모니터로_자른_영역만_받는다() {
        use std::cell::Cell;
        let mon = PhysRect::new(0, 0, 900, 700);
        let win = PhysRect::new(100, 100, 900, 700);
        let elem = PhysRect::new(50, 50, 1000, 750);
        let els = [ElementCand { rect: elem, control: Control::Image, from_msaa: false }];
        let p = PhysPoint { x: 300, y: 250 };
        let received = Cell::new(None);
        let refine = |r: PhysRect| {
            received.set(Some(r));
            PhysRect::new(120, 120, 700, 500)
        };
        let ev = Evidence {
            point: p,
            window: Some(win),
            children: &[],
            elements: &els,
            vision: &[],
            monitor: mon,
            virtual_bounds: mon,
            occluders: &[],
            support: None,
            snap: None,
        };
        let opts = RecogOptions { refine_image: Some(&refine) };
        let chain = build_chain_with(&ev, &opts);
        assert_eq!(received.get(), Some(PhysRect::new(100, 100, 800, 600)));
        let img = chain.iter().find(|n| matches!(n.kind, NodeKind::Element(Control::Image))).unwrap();
        assert_eq!(img.rect, PhysRect::new(120, 120, 700, 500));
    }

    #[test]
    fn 앞_창에_가린_이미지_재측정은_잘린_변을_2px_피해_묻고_원래_가림_경계를_복원한다() {
        use std::cell::Cell;
        let elem = PhysRect::new(175, 160, 540, 319);
        let occ = [PhysRect::new(0, 0, 1200, 236)];
        let els = [ElementCand { rect: elem, control: Control::Image, from_msaa: false }];
        let p = PhysPoint { x: 445, y: 337 };
        let received = Cell::new(None);
        // (1) 잘린 윗변이 2 px 안쪽(238)으로 전달되고 좌·우·하 여백을 걷었을 때 윗변은 236으로 복원된다
        let refine = |r: PhysRect| {
            received.set(Some(r));
            PhysRect::from_edges(216, r.y, 675, 439)
        };
        let ev1 = Evidence {
            point: p,
            window: Some(WIN),
            children: &[],
            elements: &els,
            vision: &[],
            monitor: MON,
            virtual_bounds: MON,
            occluders: &occ,
            support: None,
            snap: None,
        };
        let opts1 = RecogOptions { refine_image: Some(&refine) };
        let c1 = build_chain_with(&ev1, &opts1);
        assert_eq!(received.get(), Some(PhysRect::new(175, 238, 540, 241)));
        let img1 = c1.iter().find(|n| matches!(n.kind, NodeKind::Element(Control::Image))).unwrap();
        assert_eq!(img1.rect, PhysRect::new(216, 236, 459, 203));

        // (2) 좌·우·하가 안 변한 채 잘린 윗변만 단독 축소(가린 하늘 오탐)되면 윗변을 원래 경계(236)로 복원한다
        let refine_sky_false_pos = |r: PhysRect| PhysRect::from_edges(r.x, r.y + 40, r.right(), r.bottom());
        let opts2 = RecogOptions { refine_image: Some(&refine_sky_false_pos) };
        let c2 = build_chain_with(&ev1, &opts2);
        let img2 = c2.iter().find(|n| matches!(n.kind, NodeKind::Element(Control::Image))).unwrap();
        assert_eq!(img2.rect, PhysRect::new(175, 236, 540, 243));
    }
}
