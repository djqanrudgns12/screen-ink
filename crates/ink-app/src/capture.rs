//! F1 고르기·결과(단계 3): 오버레이 입력 → 고르기 상태 기계 → 강조(네이티브)·보기(UI) → 복사·저장·기록.
//!
//! 근거는 정지 순간 스냅숏(W) + 자식 창(H, 창마다 한 번) + UIA 색인(A, 일꾼 스레드 — 기다리지 않음) +
//! 이미지(V, 배경 전처리)에서 모아 `recog::build_chain`으로 합친다. 커서가 움직일 때 다른 앱에 묻지 않는다.

use crate::ports::{ElementSource, Frame, MonitorProbe, Ports, VisionSource, WindowCatalog};
use crate::service::InkService;
use crate::state::{CaptureView, IslandButton, IslandEvent, IslandKind};
use ink_domain::geometry::{PhysPoint, PhysRect};
use ink_domain::history::{History, HistoryEntry};
use ink_domain::image::{self, FrameRef};
use ink_domain::magnet::Edges;
use ink_domain::mode::Mode;
use ink_domain::naming;
use ink_domain::recog::{build_chain, Evidence, Node};
use ink_domain::modekeys::CaptureKeys;
use ink_domain::selection::{Button, Command, Context, Effect, Input, Key, Selector};
use ink_domain::settings::ExcludedApp;
use ink_domain::snapshot::{RawWindow, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;

/// 느린 앱 문턱(R-CAP-15): 하위 트리 색인이 이 시간을 2번 넘으면 그 앱은 창 단위로만.
/// 색인은 일꾼 스레드라 화면은 기다리지 않는다 — 문턱은 "강조가 늦게 바뀌어 튀어 보이는" 앱을 거른다.
/// 학교 PC 릴리스 실측(2026-11-26): Chrome·Claude 80~210 ms → 2.4배 여유.
const SLOW_INDEX_MS: f64 = 500.0;

/// 오버레이가 넘기는 입력(packages/ink-client CaptureInput).
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CaptureInput {
    Move { x: i32, y: i32 },
    Wheel { dy: i32 },
    Down { x: i32, y: i32, #[serde(default)] button: u8 },
    Up { x: i32, y: i32, #[serde(default)] button: u8 },
    Dblclick { x: i32, y: i32 },
    Key { key: String, code: String, #[serde(default)] ctrl: bool, #[serde(default)] shift: bool, #[serde(default)] alt: bool, #[serde(default)] meta: bool },
    /// 선택 툴바 단추(R-CAP-27): "copy" | "save" | "quickSave" | "cancel"
    Command { name: String },
}

impl From<CaptureInput> for Input {
    fn from(i: CaptureInput) -> Input {
        let btn = |b: u8| if b == 2 { Button::Right } else { Button::Left };
        match i {
            CaptureInput::Move { x, y } => Input::Move(PhysPoint { x, y }),
            CaptureInput::Wheel { dy } => Input::Wheel(dy.signum()),
            CaptureInput::Down { x, y, button } => Input::Down(PhysPoint { x, y }, btn(button)),
            CaptureInput::Up { x, y, button } => Input::Up(PhysPoint { x, y }, btn(button)),
            CaptureInput::Dblclick { x, y } => Input::DoubleClick(PhysPoint { x, y }),
            CaptureInput::Key { key, code, ctrl, shift, alt, meta } => Input::Key(Key { key, code, ctrl, shift, alt, meta }),
            CaptureInput::Command { name } => match name.as_str() {
                "copy" => Input::Command(Command::Copy),
                "pin" => Input::Command(Command::Pin),
                "save" => Input::Command(Command::Save),
                "quickSave" => Input::Command(Command::QuickSave),
                // 모르는 단추는 취소로 바꾸지 않는다 — 아무 일도 없게(빈 키)
                "cancel" => Input::Command(Command::Cancel),
                _ => Input::Key(Key::default()),
            },
        }
    }
}

/// 마지막 결과(섬의 [저장]·[HEX 복사]가 모드 밖에서 쓴다).
#[derive(Clone)]
pub(crate) struct LastCapture {
    pub w: u32,
    pub h: u32,
    pub bgra: Arc<Vec<u8>>,
    pub app: Option<String>,
    pub hex: Option<String>,
}

/// 결과 섬 [판서로]가 정지 프레임을 쥐고 있는 시간(B §2.3·§5 — M3 대기 메모리). 지나면 프레임만 놓고,
/// 그 뒤 [판서로]는 지금 화면을 새로 찍어 같은 배율 공식으로 연다.
pub const CAPTURE_FRAME_TTL: std::time::Duration = std::time::Duration::from_secs(30);

/// 결과 섬 [판서로](3e-handoff): 방금 고른 범위와 그 모니터 전체 정지 프레임(Arc — 복사 없음, 4K ≈ 33 MB 한 장).
/// 다음 모드 진입·잠금·화면 구성 변경이면 통째로 버리고, 30초가 지나면 프레임만 놓는다.
#[derive(Clone)]
pub(crate) struct CaptureHandoff {
    /// 고른 범위(가상 화면 물리 px)
    pub rect: PhysRect,
    /// 범위 가운데가 속한 모니터
    pub monitor_id: u32,
    pub layout_fingerprint: String,
    pub frame: Option<Frame>,
    pub at: Instant,
}

#[derive(Default)]
pub struct CaptureState {
    pub(crate) selector: Option<Selector>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) children: HashMap<u64, Vec<PhysRect>>,
    pub(crate) ranges: History,
    pub(crate) last: Option<LastCapture>,
    /// 결과 섬 [판서로]가 쓸 범위·정지 프레임(3e-handoff)
    pub(crate) handoff: Option<CaptureHandoff>,
    /// 오늘 저장한 순번({n})
    pub(crate) save_seq: u32,
    pub(crate) save_day: (u16, u8, u8),
    /// 실행 파일 → 색인이 느렸던 횟수
    pub(crate) slow: HashMap<String, u32>,
    /// 진입 순간의 커서 모양
    pub(crate) cursor: Option<image::CursorSprite>,
    /// 마지막으로 그린 강조(바뀔 때만 진단 로그)
    pub(crate) drawn: Option<PhysRect>,
    /// 개발용: 확정할 때마다 인식 시험 사례를 쓸 폴더(`SCREEN_INK_CASE_DIR`)
    pub(crate) case_dir: Option<String>,
    /// 사례용 거르기 전 창 목록(우리 창 뺌) — 사례를 쓸 때만 채운다
    pub(crate) raw: Vec<RawWindow>,
    /// 진입 계측(08 §4.7): 첫 근거 도착·커서 이동 질의 시간 — 모드가 끝날 때 한 줄
    pub(crate) probe: EntryProbe,
    /// 캡처 핀(R-CAP-29): id → 그림(복사·저장용). 앱을 끄면 사라진다.
    pub(crate) pins: std::collections::BTreeMap<u64, LastCapture>,
    pub(crate) pin_seq: u64,
    /// 캡처 주석(4.9): 오버레이가 알린 주석 개수·결과 부탁·지금 합칠 층
    pub(crate) annot: AnnotState,
}

/// 한 번의 고르기 동안 잰 것(08 §4.7 "첫 강조 ≤ 진입 + 30 ms", "커서 이동 질의 ≤ 5 ms").
#[derive(Default)]
pub(crate) struct EntryProbe {
    /// 단축키를 받은 순간
    pub(crate) pressed: Option<Instant>,
    /// 진입 순간 커서 아래 창
    pub(crate) window: Option<u64>,
    /// 커서 자리 V 전처리가 처음 준비된 시각: (배경에서 끝난 때, 메인 스레드가 받은 때) 단축키부터 ms
    pub(crate) v_ms: Option<(f64, f64)>,
    /// 커서 아래 창 UIA 색인: (배경에서 끝난 때, 메인 스레드가 받은 때, 색인에 걸린 시간) ms
    pub(crate) a_ms: Option<(f64, f64, f64)>,
    /// 커서 이동 한 번의 질의(고르기 상태 기계 + 사슬 만들기) ms
    pub(crate) moves: Vec<f64>,
}

const MAX_PROBE_MOVES: usize = 20_000;

fn ms_since(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

impl EntryProbe {
    /// `selection probe` 한 줄(종류·시간만, 창 제목 없음).
    pub(crate) fn line(&self) -> Option<String> {
        self.pressed?;
        let moves = ink_domain::timing::stats(&self.moves)
            .map_or("moves 0".to_string(), |s| format!("moves {} p50 {:.2} p95 {:.2} max {:.2} ms", s.n, s.p50, s.p95, s.max));
        let v = self.v_ms.map_or("-".to_string(), |(bg, main)| format!("{bg:.1} (main {main:.1})"));
        let a = self.a_ms.map_or("-".to_string(), |(bg, main, took)| format!("{bg:.1} (main {main:.1}, index {took:.0})"));
        Some(format!("selection probe: V first {v} ms, A first {a} ms, {moves}"))
    }
}

impl CaptureState {
    pub fn with_ranges(ranges: History) -> Self {
        CaptureState { ranges, ..Default::default() }
    }
}

/// 고르기 상태 기계에 주는 바깥 사정.
struct Ctx<'a> {
    snap: &'a Snapshot,
    windows: &'a dyn WindowCatalog,
    elements: &'a dyn ElementSource,
    vision: &'a dyn VisionSource,
    monitors: &'a dyn MonitorProbe,
    children: &'a mut HashMap<u64, Vec<PhysRect>>,
    history: Vec<PhysRect>,
}

impl Context for Ctx<'_> {
    fn chain_at(&mut self, p: PhysPoint, elements_on: bool) -> (Vec<Node>, Option<u64>) {
        let w = self.snap.window_at(p, true);
        let monitor = self.snap.layout.nearest(p).map(|m| m.bounds).unwrap_or_default();
        let virt = self.snap.layout.virtual_bounds();
        let (els, kids) = match w {
            Some(w) if elements_on && !w.window_only => {
                self.elements.request(w.handle, &w.exe);
                let windows = self.windows;
                let kids = self.children.entry(w.handle).or_insert_with(|| windows.children(w.handle)).clone();
                (self.elements.elements_at(w.handle, p), kids)
            }
            _ => (Vec::new(), Vec::new()),
        };
        let vis = if elements_on { self.vision.query(p) } else { Vec::new() };
        let vision = self.vision;
        let v_ready = vision.support(monitor).is_some();
        let sup = |r: PhysRect| vision.support(r).unwrap_or(0.5);
        let snap = |r: PhysRect| vision.snap(r);
        // 그 창보다 앞에 있는 창들(가림 인지 R2-CAP-1)
        let occ = w.map(|w| ink_domain::recog::occluders_of(&self.snap.windows, w.handle)).unwrap_or_default();
        let ev = Evidence {
            point: p,
            window: w.map(|w| w.rect),
            children: &kids,
            elements: &els,
            vision: &vis,
            monitor,
            virtual_bounds: virt,
            occluders: &occ,
            support: if v_ready { Some(&sup) } else { None },
            snap: if v_ready { Some(&snap) } else { None },
        };
        (build_chain(&ev), w.map(|w| w.handle))
    }

    fn edges(&mut self, around: PhysPoint) -> Edges {
        let mut rects: Vec<PhysRect> = self.snap.windows.iter().map(|w| w.rect).collect();
        rects.extend(self.snap.layout.monitors.iter().flat_map(|m| [m.bounds, m.work]));
        if let Some(w) = self.snap.window_at(around, true) {
            rects.extend(self.elements.rects(w.handle));
            if let Some(kids) = self.children.get(&w.handle) {
                rects.extend(kids.iter().copied());
            }
        }
        let mut e = Edges::from_rects(rects.iter());
        let (xs, ys) = self.vision.edges_near(around);
        e.extend(xs, ys);
        e
    }

    fn monitor_at(&self, p: PhysPoint) -> PhysRect {
        self.snap.layout.nearest(p).map(|m| m.bounds).unwrap_or_default()
    }

    fn virtual_bounds(&self) -> PhysRect {
        self.snap.layout.virtual_bounds()
    }

    fn scale_at(&self, p: PhysPoint) -> f64 {
        self.snap.layout.nearest(p).map(|m| m.scale).unwrap_or(1.0)
    }

    fn history(&self) -> Vec<PhysRect> {
        self.history.clone()
    }

    fn alt_down(&self) -> bool {
        self.monitors.alt_down()
    }
}

/// 서비스 필드에서 Ctx를 만든다(필드별로 빌려 상태 기계와 함께 쓸 수 있게).
macro_rules! ctx {
    ($s:expr, $snap:expr) => {{
        let Ports { windows, elements, vision, monitors, .. } = &$s.ports;
        Ctx {
            snap: $snap,
            windows: windows.as_ref(),
            elements: elements.as_ref(),
            vision: vision.as_ref(),
            monitors: monitors.as_ref(),
            children: &mut $s.cap.children,
            history: $s.cap.ranges.for_layout(&$snap.layout.fingerprint()),
        }
    }};
}

impl InkService {
    /// 근거 준비 시작(배경): 커서 아래 창 UIA 색인 부탁 + V 전처리. 정지 화면을 띄우기 **전에** 부른다.
    pub(crate) fn warm_evidence(&mut self, snap: &Snapshot, frames: &[Frame], cursor: PhysPoint) {
        if let Some(w) = snap.window_at(cursor, true) {
            if !w.window_only {
                self.ports.elements.request(w.handle, &w.exe);
            }
        }
        self.ports.vision.prepare(frames, &snap.layout, cursor);
    }

    /// 진입 직후: 고르기 상태 기계를 만들고 첫 보기를 내보낸다(근거는 `warm_evidence`가 이미 준비 중).
    pub(crate) fn start_selection(&mut self, frames: Vec<Frame>, cursor: PhysPoint) {
        let Some(snap) = self.snapshot.take() else { return };
        self.cap.frames = frames;
        // 화면 안 키는 지금 설정값(R-KEY-6) — 설정을 바꾸면 다음 진입부터
        let keys = CaptureKeys::from_lookup(|id| self.settings.shortcut(id).to_string());
        let evidence = self.settings.bool("capture.evidence");
        let sel = {
            let ctx = ctx!(self, &snap);
            Selector::new(cursor, &ctx).with_keys(keys).with_evidence(evidence)
        };
        self.cap.selector = Some(sel);
        self.snapshot = Some(snap);
        self.emit_capture();
    }

    /// 모드 끝: 색인·전처리·이미지를 버린다.
    pub(crate) fn end_selection(&mut self) {
        if let Some(line) = self.cap.probe.line() {
            log::info!("{line}");
        }
        self.cap.probe = EntryProbe::default();
        self.cap.selector = None;
        self.cap.frames.clear();
        self.cap.children.clear();
        self.cap.cursor = None;
        self.cap.drawn = None;
        self.cap.raw.clear();
        self.ports.elements.clear();
        self.ports.vision.clear();
    }

    fn session(&self) -> Option<u64> {
        match self.mode {
            Mode::Capture { session, .. } => Some(session),
            _ => None,
        }
    }

    fn emit_capture(&self) {
        let (Some(session), Some(sel)) = (self.session(), self.cap.selector.as_ref()) else { return };
        self.ports.notifier.capture(&CaptureView { session, selection: sel.view() });
    }

    /// 오버레이 입력 한 건.
    pub fn capture_input(&mut self, input: CaptureInput) {
        // F2는 창 하나만 고른다(요소 사슬 없음, 02 §2) — 단계 6 창 캡처
        if self.window_picking() {
            return self.window_input(input);
        }
        let (Some(_), Some(mut sel), Some(snap)) = (self.session(), self.cap.selector.take(), self.snapshot.take()) else { return };
        // F3: 확정 직전 고르기 방식(창 단계였는지)으로 녹화 대상을 정한다
        let before = (sel.mode_name(), sel.rect());
        let is_move = matches!(input, CaptureInput::Move { .. });
        let t = Instant::now();
        let effect = {
            let mut ctx = ctx!(self, &snap);
            sel.handle(input.into(), &mut ctx)
        };
        if is_move && self.cap.probe.moves.len() < MAX_PROBE_MOVES {
            self.cap.probe.moves.push(ms_since(t));
        }
        self.cap.selector = Some(sel);
        self.snapshot = Some(snap);
        if self.record_picking() && self.record_effect(&effect, before) {
            return;
        }
        self.apply(effect);
    }

    /// UIA 색인이 끝났다(어댑터 일꾼 → 메인 스레드). 느린 앱은 창 단위로 강등(R-CAP-15).
    pub fn on_elements_ready(&mut self, window: u64, exe: &str, took_ms: f64) {
        self.on_elements_ready_at(window, exe, took_ms, Instant::now());
    }

    /// `at` = 일꾼 스레드에서 색인이 끝난 순간(메인 스레드로 넘어오는 지연과 나눠 잼).
    pub fn on_elements_ready_at(&mut self, window: u64, exe: &str, took_ms: f64, at: Instant) {
        if let (Some(p), None) = (self.cap.probe.pressed, self.cap.probe.a_ms) {
            if self.cap.probe.window == Some(window) {
                let bg = at.saturating_duration_since(p).as_secs_f64() * 1000.0;
                self.cap.probe.a_ms = Some((bg, ms_since(p), took_ms));
            }
        }
        if took_ms > SLOW_INDEX_MS && !exe.is_empty() {
            let n = self.cap.slow.entry(exe.to_string()).or_default();
            *n += 1;
            if *n == 2 {
                self.demote_slow_app(exe);
            }
        }
        let (Some(_), Some(mut sel), Some(snap)) = (self.session(), self.cap.selector.take(), self.snapshot.take()) else { return };
        let related = snap.windows.iter().any(|w| w.handle == window);
        let effect = if related {
            let mut ctx = ctx!(self, &snap);
            sel.refresh(&mut ctx)
        } else {
            Effect::None
        };
        self.cap.selector = Some(sel);
        self.snapshot = Some(snap);
        self.apply(effect);
    }

    /// V 전처리가 끝났다 — 강조 사슬을 다시 만든다.
    pub fn on_vision_ready(&mut self) {
        self.on_vision_ready_at(Instant::now());
    }

    /// `at` = 배경 스레드에서 전처리가 끝난 순간.
    pub fn on_vision_ready_at(&mut self, at: Instant) {
        if let (Some(p), None, Some(snap)) = (self.cap.probe.pressed, self.cap.probe.v_ms, self.snapshot.as_ref()) {
            // 커서 자리가 준비됐을 때만(다른 모니터 전처리가 먼저 끝난 것은 세지 않음)
            if self.ports.vision.support(PhysRect::new(snap.cursor.x, snap.cursor.y, 1, 1)).is_some() {
                let bg = at.saturating_duration_since(p).as_secs_f64() * 1000.0;
                self.cap.probe.v_ms = Some((bg, ms_since(p)));
            }
        }
        let (Some(_), Some(mut sel), Some(snap)) = (self.session(), self.cap.selector.take(), self.snapshot.take()) else { return };
        let effect = {
            let mut ctx = ctx!(self, &snap);
            sel.refresh(&mut ctx)
        };
        self.cap.selector = Some(sel);
        self.snapshot = Some(snap);
        self.apply(effect);
    }

    fn demote_slow_app(&mut self, exe: &str) {
        let mut list: Vec<ExcludedApp> = self.settings.get("capture.excluded").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        if list.iter().any(|e| e.exe.eq_ignore_ascii_case(exe)) {
            return;
        }
        list.push(ExcludedApp { exe: exe.to_string(), mode: "windowOnly".into(), auto: true });
        if self.update_setting("capture.excluded", &serde_json::to_value(list).unwrap_or_default()).ok {
            self.island(IslandEvent::new(IslandKind::Info, format!("‘{exe}’는 창 단위로만 찾을게요")).undo_button());
        }
    }

    fn apply(&mut self, effect: Effect) {
        match effect {
            Effect::None => {}
            Effect::Redraw => self.redraw(),
            Effect::SetCursor(p) => {
                self.ports.monitors.set_cursor(p);
                self.redraw();
            }
            Effect::Notice(text) => self.island(IslandEvent::new(IslandKind::Info, text)),
            Effect::Cancel => self.end_capture(),
            // 주석이 없으면 commit = 지금 경로 그대로(finish·pin_rect), 있으면 오버레이에 주석 층을 부탁
            Effect::Copy(rect) => self.commit(CommitAction::Copy, rect),
            Effect::Save { rect, quick } => self.commit(if quick { CommitAction::QuickSave } else { CommitAction::Save }, rect),
            Effect::Pin(rect) => self.commit(CommitAction::Pin, rect),
        }
    }

    fn redraw(&mut self) {
        let (Some(session), Some(sel)) = (self.session(), self.cap.selector.as_ref()) else { return };
        let rect = sel.rect();
        if rect != self.cap.drawn && log::log_enabled!(log::Level::Debug) {
            // 진단: 종류 이름만(창 제목·파일 이름 없음, 05 §9)
            let v = sel.view();
            log::debug!("highlight {:?} level {} path {}", rect, v.level, v.path.join(" > "));
        }
        self.cap.drawn = rect;
        self.ports.overlay.highlight(session, rect);
        self.emit_capture();
    }

    /// 개발용 사례 폴더(사람 정답 도구). `None`이면 끈다.
    pub fn set_case_dir(&mut self, dir: Option<String>) {
        self.cap.case_dir = dir.filter(|d| !d.trim().is_empty());
    }

    pub(crate) fn case_dir_on(&self) -> bool {
        self.cap.case_dir.is_some()
    }

    /// 확정한 범위를 정답으로 진입 순간을 사례로 쓴다(08 §5.2). 점 = 강조를 고른 커서 자리(드래그면 가운데).
    fn write_case(&mut self, truth: PhysRect) {
        let (Some(dir), Some(snap)) = (self.cap.case_dir.clone(), self.snapshot.as_ref()) else { return };
        let t = self.ports.clock.local();
        let id = format!("case-{:04}{:02}{:02}-{:02}{:02}{:02}-{}", t.year, t.month, t.day, t.hour, t.minute, t.second, snap.id);
        let at = self.cap.selector.as_ref().and_then(|s| s.hover_point()).unwrap_or(PhysPoint { x: truth.x + truth.w / 2, y: truth.y + truth.h / 2 });
        let label = self.cap.selector.as_ref().map(|s| s.view().path.last().cloned().unwrap_or_default()).unwrap_or_default();
        let mut elements = std::collections::BTreeMap::new();
        for w in &self.cap.raw {
            let els = self.ports.elements.elements(w.handle);
            if !els.is_empty() {
                elements.insert(w.handle, els);
            }
        }
        let case = crate::case::Case {
            schema: crate::case::CASE_SCHEMA,
            id: id.clone(),
            note: String::new(),
            monitors: snap
                .layout
                .monitors
                .iter()
                .filter(|m| self.cap.frames.iter().any(|f| f.monitor_id == m.id))
                .map(|m| crate::case::CaseMonitor { monitor: m.clone(), frame: crate::case::Case::frame_name(&id, m.id) })
                .collect(),
            cursor: snap.cursor,
            windows: self.cap.raw.clone(),
            elements,
            children: self.cap.children.iter().map(|(k, v)| (*k, v.clone())).collect(),
            points: vec![crate::case::CasePoint { at, truth, label }],
        };
        match self.ports.files.write_case(&dir, &case, &self.cap.frames) {
            Ok(path) => log::info!("case written {path}"),
            Err(e) => log::warn!("case not written: {e}"),
        }
    }

    /// 범위 확정 → 자르기 → (복사 | 저장) → 기록 → 모드 끝 → 결과 섬.
    pub(crate) fn finish(&mut self, rect: PhysRect, save: bool, _quick: bool) {
        if self.cap.case_dir.is_some() {
            self.write_case(rect);
        }
        let Some((w, h, px, app)) = self.render_rect(rect) else { return };
        self.remember_range(rect);
        let hex = image::uniform_color(w, h, &px).map(|c| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]));

        let afterimage = self.session().zip(self.snapshot.as_ref().map(|s| s.layout.clone()));
        let (tw, th, tpx) = image::thumbnail(w, h, &px, 160, 96);
        let thumb = self.ports.overlay.thumbnail(tw, th, tpx);
        self.cap.last = Some(LastCapture { w, h, bgra: Arc::new(px), app, hex: hex.clone() });
        // [판서로]용 범위·그 모니터 정지 프레임 — 모드가 끝나면 프레임 목록을 비우므로 그 전에 쥔다
        let handoff = self.handoff_of(rect);
        // 화면을 먼저 돌려준다(클립보드·파일은 그 뒤)
        self.end_capture();
        if let Some((session, layout)) = afterimage {
            self.ports.overlay.flash_afterimage(session, rect, &layout);
        }
        let to_draw = handoff.is_some();
        self.cap.handoff = handoff;
        let draw_key = self.settings.shortcut("draw").to_string();
        let with_to_draw = |ev: IslandEvent| if to_draw && ev.kind == IslandKind::Result { ev.secondary("판서로", "capture.toDraw", Some(&draw_key)) } else { ev };

        let result = self.settings.str("capture.result").to_string();
        let do_copy = !save && result != "save";
        let do_save = save || result == "save" || result == "both";
        let size = format!("{w}×{h}");
        let mut copied = false;
        if do_copy {
            let Some(last) = self.cap.last.clone() else { return };
            match self.ports.clipboard.set_image(last.w, last.h, &last.bgra) {
                Ok(()) => copied = true,
                Err(e) => {
                    log::warn!("clipboard: {e}");
                    self.island(IslandEvent::new(IslandKind::Problem, "클립보드를 쓸 수 없어요"));
                    return;
                }
            }
        }
        if do_save {
            if let Some(mut ev) = self.save_last_inner() {
                if copied {
                    ev.text = "복사하고 저장했어요".into();
                }
                ev.thumb_url = thumb;
                self.island(with_to_draw(ev));
            }
            return;
        }
        let save_key = self.key_text("c-save");
        let mut ev = IslandEvent::new(IslandKind::Result, "복사했어요").variant("result").button("저장", "capture.saveLast", save_key.as_deref());
        ev.value = Some(size);
        ev.detail = Some(format!("한글·PPT에 {}", self.paste_text()));
        ev.thumb_url = thumb;
        if let Some(h) = hex {
            ev.chip = Some(IslandButton { label: format!("{h} 복사"), action: "capture.copyHex".into() });
        }
        self.island(with_to_draw(ev));
    }

    /// [판서로]에 넘길 것: 범위 가운데가 속한 모니터(없으면 가까운 모니터)와 그 모니터 정지 프레임(Arc 복제 — 픽셀 복사 없음).
    fn handoff_of(&self, rect: PhysRect) -> Option<CaptureHandoff> {
        let snap = self.snapshot.as_ref()?;
        let center = PhysPoint { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 };
        let mon = snap.layout.monitor_at(center).or_else(|| snap.layout.nearest(center))?;
        let frame = self.cap.frames.iter().find(|f| f.monitor_id == mon.id).cloned();
        Some(CaptureHandoff { rect, monitor_id: mon.id, layout_fingerprint: snap.layout.fingerprint(), frame, at: Instant::now() })
    }

    /// [판서로] 프레임을 30초 넘게 쥐고 있으면 놓는다(어댑터가 결과 섬을 띄운 뒤 그만큼 지나 부른다). 놓았으면 true.
    /// 범위는 남겨 둔다 — 그 뒤 [판서로]는 지금 화면을 새로 찍는다.
    pub fn expire_capture_frame(&mut self) -> bool {
        self.expire_capture_frame_at(Instant::now())
    }

    /// `now` = 지금 시각(시험은 30초 뒤를 준다).
    pub fn expire_capture_frame_at(&mut self, now: Instant) -> bool {
        match self.cap.handoff.as_mut() {
            Some(h) if h.frame.is_some() && now.saturating_duration_since(h.at) >= CAPTURE_FRAME_TTL => {
                h.frame = None;
                true
            }
            _ => false,
        }
    }

    /// [판서로]가 정지 프레임을 쥐고 있는지(시험·진단).
    pub fn capture_frame_held(&self) -> bool {
        self.cap.handoff.as_ref().is_some_and(|h| h.frame.is_some())
    }

    /// 확정한 범위의 결과 그림(정지 이미지 자르기 + 커서 합성)과 그 자리 앱 이름(파일 이름 {app}).
    fn render_rect(&self, rect: PhysRect) -> Option<(u32, u32, Vec<u8>, Option<String>)> {
        let snap = self.snapshot.as_ref()?;
        let refs: Vec<FrameRef> = self
            .cap
            .frames
            .iter()
            .filter_map(|f| snap.layout.monitors.iter().find(|m| m.id == f.monitor_id).map(|m| FrameRef { bounds: m.bounds, w: f.width, h: f.height, bgra: &f.bgra }))
            .collect();
        let (w, h, mut px) = image::crop(&refs, rect);
        // 커서 포함 = 설정("커서도 찍기") XOR 이번에 ` 키
        let toggled = self.cap.selector.as_ref().is_some_and(|s| s.cursor_included());
        if self.settings.bool("capture.cursor") != toggled {
            if let Some(c) = self.cap.cursor.as_ref() {
                image::composite_cursor(&mut px, w, h, (rect.x, rect.y), c);
            }
        }
        // 캡처 주석(4.9): 모자이크(원본 화소) → 주석 층 알파 합성. 결과·핀·저장 모두 이 그림을 쓴다
        if let Some(layer) = self.cap.annot.layer.as_ref() {
            layer.apply(&mut px, w, h);
        }
        let center = PhysPoint { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 };
        let app = snap.window_at(center, true).map(|w| w.exe.trim_end_matches(".exe").to_string()).filter(|s| !s.is_empty());
        Some((w, h, px, app))
    }

    /// 범위 기록(사각형만 — R·쉼표·마침표로 다시 고르기).
    pub(crate) fn remember_range(&mut self, rect: PhysRect) {
        let Some(snap) = self.snapshot.as_ref() else { return };
        let mode_name = self.cap.selector.as_ref().map(|s| s.mode_name()).unwrap_or("drag");
        let layout = snap.layout.fingerprint();
        let max = self.settings.get("capture.historyMax").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        self.cap.ranges.push(HistoryEntry { rect, layout, mode: mode_name.into(), at: self.ports.clock.unix() }, max);
        if !self.readonly {
            if let Ok(v) = serde_json::to_value(&self.cap.ranges) {
                let _ = self.ports.store.save_section("history", &v);
            }
        }
    }

    /// 툴바 [고정](R-CAP-29): 고른 범위를 원래 자리에 떠 있는 그림 창으로. 모드는 끝난다.
    fn pin_rect(&mut self, rect: PhysRect) {
        let Some((w, h, px, app)) = self.render_rect(rect) else { return };
        self.remember_range(rect);
        self.end_capture();
        self.cap.pin_seq += 1;
        let id = self.cap.pin_seq;
        let bgra = Arc::new(px);
        match self.ports.pins.open(id, rect, w, h, bgra.clone()) {
            Ok(()) => {
                self.cap.pins.insert(id, LastCapture { w, h, bgra, app, hex: None });
                self.emit_state();
            }
            Err(e) => {
                log::warn!("pin open failed: {e}");
                self.island(IslandEvent::new(IslandKind::Problem, "화면에 고정하지 못했어요"));
            }
        }
    }

    /// 핀 닫기(더블클릭·Esc·메뉴).
    pub fn pin_close(&mut self, id: u64) {
        if self.cap.pins.remove(&id).is_some() {
            self.ports.pins.close(id);
            self.emit_state();
        }
    }

    /// 핀 모두 닫기(메뉴·트레이). 닫은 개수.
    pub fn pin_close_all(&mut self) -> usize {
        let ids: Vec<u64> = self.cap.pins.keys().copied().collect();
        for id in &ids {
            self.ports.pins.close(*id);
        }
        self.cap.pins.clear();
        if !ids.is_empty() {
            self.emit_state();
        }
        ids.len()
    }

    /// 핀 메뉴 [복사].
    pub fn pin_copy(&mut self, id: u64) {
        let Some(p) = self.cap.pins.get(&id).cloned() else { return };
        match self.ports.clipboard.set_image(p.w, p.h, &p.bgra) {
            Ok(()) => {
                let mut ev = IslandEvent::new(IslandKind::Result, "복사했어요").variant("result");
                ev.value = Some(format!("{}×{}", p.w, p.h));
                self.island(ev);
            }
            Err(e) => {
                log::warn!("clipboard: {e}");
                self.island(IslandEvent::new(IslandKind::Problem, "클립보드를 쓸 수 없어요"));
            }
        }
    }

    /// 핀 메뉴 [저장].
    pub fn pin_save(&mut self, id: u64) {
        let Some(p) = self.cap.pins.get(&id).cloned() else { return };
        let ev = self.save_image(&p);
        self.island(ev);
    }

    pub fn pin_count(&self) -> usize {
        self.cap.pins.len()
    }

    /// 섬 [저장] — 마지막 결과를 저장 폴더에.
    pub fn save_last(&mut self) {
        if let Some(ev) = self.save_last_inner() {
            self.island(ev);
        } else if self.cap.last.is_none() {
            self.island(IslandEvent::new(IslandKind::Info, "저장할 캡처가 없어요"));
        }
    }

    pub(crate) fn save_last_inner(&mut self) -> Option<IslandEvent> {
        let last = self.cap.last.clone()?;
        Some(self.save_image(&last))
    }

    /// 그림 하나를 저장 폴더에(이름 규칙·오늘 순번) → 결과 섬.
    pub(crate) fn save_image(&mut self, last: &LastCapture) -> IslandEvent {
        let mut t = self.ports.clock.local();
        let day = (t.year, t.month, t.day);
        if self.cap.save_day != day {
            self.cap.save_day = day;
            self.cap.save_seq = 0;
        }
        self.cap.save_seq += 1;
        t.n = self.cap.save_seq;
        t.app = last.app.clone();
        let template = self.settings.str("output.template").to_string();
        let stem = naming::render(&template, &t).unwrap_or_else(|_| "Shookshok".into());
        let folder = self.settings.str("output.folder").to_string();
        match self.ports.files.save_png(&folder, &stem, last.w, last.h, &last.bgra) {
            Ok(f) => {
                let mut ev = IslandEvent::new(IslandKind::Result, if f.fell_back { "사진 폴더에 저장했어요" } else { "저장했어요" })
                    .variant("result")
                    .button("폴더 열기", "folder.open", None);
                ev.detail = Some(f.name);
                ev
            }
            Err(e) => {
                log::warn!("save failed: {e}");
                IslandEvent::new(IslandKind::Problem, "저장하지 못했어요")
            }
        }
    }

    /// 섬 칩 [#RRGGBB 복사].
    pub fn copy_last_hex(&mut self) {
        let Some(hex) = self.cap.last.as_ref().and_then(|l| l.hex.clone()) else { return };
        match self.ports.clipboard.set_text(&hex) {
            Ok(()) => self.island(IslandEvent::new(IslandKind::Result, format!("{hex} 복사했어요")).variant("result")),
            Err(_) => self.island(IslandEvent::new(IslandKind::Problem, "클립보드를 쓸 수 없어요")),
        }
    }

    /// 범위 기록 모두 지우기(트레이·설정 [기록 지우기]).
    pub fn clear_ranges(&mut self) -> usize {
        let n = self.cap.ranges.entries.len();
        self.cap.ranges.entries.clear();
        if let Ok(v) = serde_json::to_value(&self.cap.ranges) {
            let _ = self.ports.store.save_section("history", &v);
        }
        n
    }
}

// ─────────────── 캡처 주석 결과(4.9, R-CAP-28 · B §2.1 "합성 결과 전달 계약") ───────────────
//
// 주석이 0개 = 지금 경로 그대로(`finish`·`pin_rect`). 1개 이상이면:
//   ① 오버레이가 개수를 알린다(`capture_annotations`, 0 ↔ n이 바뀔 때)
//   ② 고르기 상태 기계가 복사·저장·핀을 내면(키·단추·더블클릭 — 판정은 늘 Rust, R-KEY-6)
//      서비스가 오버레이에 주석 층을 부탁한다(`OverlayHost::request_commit` → `screen-ink://capture-commit`)
//   ③ 오버레이가 투명 주석 층 RGBA + 모자이크 사각형을 원본 바이트로 보낸다(`capture_commit_annotated`)
//   ④ `render_rect`가 자른 원본에 모자이크(원본 화소) → 주석 층 알파 합성 → 복사·저장·핀(R-CAP-29)

/// 주석 결과로 낼 동작.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommitAction {
    Copy,
    Save,
    QuickSave,
    Pin,
}

/// 서비스 → 오버레이: "이 범위(물리 px)의 주석 층을 보내 줘". `seq`가 같은 답만 받는다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitRequest {
    pub session: u64,
    pub seq: u64,
    pub action: CommitAction,
    pub rect: PhysRect,
}

/// 오버레이 → 서비스 머리 JSON. `w`·`h` = 주석 층 크기(= 범위 물리 크기), 층이 없으면 0.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitMeta {
    pub session: u64,
    pub seq: u64,
    pub action: CommitAction,
    #[serde(default)]
    pub w: u32,
    #[serde(default)]
    pub h: u32,
    /// 결과 그림 기준 물리 px(왼쪽 위 = 범위 왼쪽 위)
    #[serde(default)]
    pub mosaics: Vec<image::MosaicRect>,
}

/// 원본 바이트 IPC 본문을 푼 것.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotatedCommit {
    pub meta: CommitMeta,
    /// 곧은 알파 RGBA(`getImageData` 그대로) w×h×4, 또는 빈 것(모자이크만)
    pub overlay: Vec<u8>,
}

/// 머리 JSON 한도(모자이크 4096개도 넉넉) · 모자이크 개수 한도.
pub const MAX_COMMIT_META: usize = 1 << 20;
pub const MAX_COMMIT_MOSAICS: usize = 4096;

impl AnnotatedCommit {
    /// 본문 = `[머리 길이 u32 LE][머리 JSON(UTF-8)][주석 층 RGBA]` (packages/ink-client `encodeAnnotatedCommit`과 1:1).
    pub fn decode(body: &[u8]) -> Result<Self, String> {
        let len: [u8; 4] = body.get(..4).and_then(|b| b.try_into().ok()).ok_or("annotation body too short")?;
        let n = u32::from_le_bytes(len) as usize;
        if n > MAX_COMMIT_META {
            return Err("annotation meta too large".into());
        }
        let head = body.get(4..4 + n).ok_or("annotation meta cut short")?;
        let meta: CommitMeta = serde_json::from_slice(head).map_err(|e| format!("annotation meta: {e}"))?;
        Ok(Self { meta, overlay: body[4 + n..].to_vec() })
    }
}

/// 지금 결과에 합칠 주석(서비스가 `render_rect` 동안만 쥔다).
#[derive(Clone, Debug, Default)]
pub(crate) struct AnnotLayer {
    mosaics: Vec<image::MosaicRect>,
    overlay: Vec<u8>,
}

impl AnnotLayer {
    fn apply(&self, px: &mut [u8], w: u32, h: u32) {
        image::apply_mosaic(px, w, h, &self.mosaics);
        if !self.overlay.is_empty() {
            image::composite_rgba_over_bgra(px, &self.overlay, w, h);
        }
    }
}

/// 세션 하나의 주석 사정. 다른 세션 값은 쓰지 않는다(모드가 끝나도 지울 필요 없음).
#[derive(Debug, Default)]
pub(crate) struct AnnotState {
    session: u64,
    /// 모니터(오버레이 창) → 그 창 주석 개수
    items: BTreeMap<u32, u32>,
    seq: u64,
    pending: Option<CommitRequest>,
    layer: Option<AnnotLayer>,
}

impl InkService {
    /// 오버레이가 주석 개수를 알린다(0 ↔ n이 바뀔 때). 지금 세션이 아니면 무시.
    /// 부탁해 둔 결과가 있는데 주석이 모두 사라졌으면(창이 주석을 버림) 기다리지 않고 지금 경로로 끝낸다.
    pub fn capture_annotations(&mut self, session: u64, monitor: u32, items: u32) {
        if self.session() != Some(session) {
            return;
        }
        let a = &mut self.cap.annot;
        if a.session != session {
            *a = AnnotState { session, ..Default::default() };
        }
        if items == 0 {
            a.items.remove(&monitor);
        } else {
            a.items.insert(monitor, items);
        }
        if !self.annotated() {
            if let Some(req) = self.cap.annot.pending.take() {
                self.commit_plain(req.action, req.rect);
            }
        }
    }

    /// 지금 세션에 주석이 하나라도 있는지.
    pub fn capture_annotated(&self) -> bool {
        self.annotated()
    }

    fn annotated(&self) -> bool {
        self.session().is_some_and(|s| self.cap.annot.session == s && self.cap.annot.items.values().any(|&n| n > 0))
    }

    /// 고르기 상태 기계가 낸 결과. 주석이 없으면 지금 경로, 있으면 오버레이에 주석 층을 부탁한다.
    fn commit(&mut self, action: CommitAction, rect: PhysRect) {
        let Some(session) = self.session() else { return };
        if !self.annotated() {
            self.commit_plain(action, rect);
            return;
        }
        self.cap.annot.seq += 1;
        let req = CommitRequest { session, seq: self.cap.annot.seq, action, rect };
        if self.ports.overlay.request_commit(&req) {
            self.cap.annot.pending = Some(req);
        } else {
            // 부탁할 길이 없음(미지원 어댑터) — 주석 없이라도 결과는 낸다
            self.cap.annot.pending = None;
            log::warn!("annotation request not delivered; result without annotations");
            self.commit_plain(action, rect);
        }
    }

    fn commit_plain(&mut self, action: CommitAction, rect: PhysRect) {
        match action {
            CommitAction::Copy => self.finish(rect, false, false),
            CommitAction::Save => self.finish(rect, true, false),
            CommitAction::QuickSave => self.finish(rect, true, true),
            CommitAction::Pin => self.pin_rect(rect),
        }
    }

    /// 오버레이가 보낸 주석 층으로 부탁해 둔 결과를 낸다. 세션·순번이 다르면(늦은 답·두 번 답) 무시.
    /// 층 크기가 범위와 다르면 주석 없이 낸다(잘못 겹치지 않게).
    pub fn capture_commit_annotated(&mut self, commit: AnnotatedCommit) {
        let Some(session) = self.session() else { return };
        let AnnotatedCommit { meta, overlay } = commit;
        let Some(req) = self.cap.annot.pending.filter(|r| r.session == session && meta.session == session && r.seq == meta.seq) else {
            log::debug!("annotation answer ignored (session {} seq {})", meta.session, meta.seq);
            return;
        };
        self.cap.annot.pending = None;
        let (w, h) = (req.rect.w.max(0) as u32, req.rect.h.max(0) as u32);
        let sized = overlay.is_empty() || (meta.w == w && meta.h == h && overlay.len() as u64 == u64::from(w) * u64::from(h) * 4);
        if !sized {
            log::warn!("annotation layer {}x{} ({} bytes) does not match range {w}x{h}", meta.w, meta.h, overlay.len());
            self.commit_plain(req.action, req.rect);
            return;
        }
        let mut mosaics = meta.mosaics;
        mosaics.truncate(MAX_COMMIT_MOSAICS);
        self.cap.annot.layer = Some(AnnotLayer { mosaics, overlay });
        self.commit_plain(req.action, req.rect);
        self.cap.annot.layer = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 진입_계측은_진입했을_때만_한_줄() {
        assert_eq!(EntryProbe::default().line(), None, "진입 안 했으면 없음");
        let p = EntryProbe { pressed: Some(Instant::now()), window: Some(1), v_ms: Some((12.34, 40.0)), a_ms: Some((95.0, 120.0, 80.4)), moves: vec![0.5, 1.0, 2.0] };
        let line = p.line().unwrap();
        assert!(line.contains("V first 12.3 (main 40.0) ms"), "{line}");
        assert!(line.contains("A first 95.0 (main 120.0, index 80) ms"), "{line}");
        assert!(line.contains("moves 3 p50 1.00 p95 2.00 max 2.00 ms"), "{line}");
        let none = EntryProbe { pressed: Some(Instant::now()), ..Default::default() }.line().unwrap();
        assert!(none.contains("V first - ms, A first - ms, moves 0"), "{none}");
    }

    #[test]
    fn 확정시_잔상_호출되고_취소시_호출안됨() {
        use crate::ports::*;
        use crate::state::{InkState, OverlaySession};
        use ink_domain::geometry::{Monitor, MonitorLayout};
        use ink_domain::naming::NameContext;
        use ink_domain::shortcuts::Platform;
        use ink_domain::snapshot::CaptureKind;
        use std::sync::Mutex;

        #[derive(Clone, Default)]
        struct Log(Arc<Mutex<Vec<String>>>);
        impl OverlayHost for Log {
            fn prepare(&mut self, _layout: &MonitorLayout) {}
            fn show(&mut self, _session: &OverlaySession, _frames: Vec<Frame>) -> Result<Option<Instant>, String> {
                Ok(None)
            }
            fn highlight(&mut self, _session: u64, _rect: Option<PhysRect>) {}
            fn hide(&mut self, session: u64) {
                self.0.lock().unwrap().push(format!("hide:{session}"));
            }
            fn flash_afterimage(&mut self, session: u64, rect: PhysRect, layout: &MonitorLayout) {
                self.0.lock().unwrap().push(format!("flash:{session}:{}x{}:{}", rect.w, rect.h, layout.monitors.len()));
            }
        }

        struct Stub;
        impl HotkeyRegistry for Stub {
            fn register(&mut self, _s: &str, _a: &str) -> Result<(), HotkeyError> { Ok(()) }
            fn unregister(&mut self, _a: &str) {}
        }
        impl SystemProbe for Stub {
            fn running_apps(&self) -> Vec<String> { vec![] }
            fn live_bindings(&self) -> Option<Vec<ink_domain::shortcuts::LiveBinding>> { Some(vec![]) }
        }
        impl SettingsStore for Stub {
            fn load(&mut self) -> Loaded { Loaded::default() }
            fn save_section(&mut self, _n: &str, _d: &serde_json::Value) -> Result<(), StoreError> { Ok(()) }
            fn location(&self) -> StoreLocation { StoreLocation::default() }
        }
        impl Notifier for Stub {
            fn state(&self, _s: &InkState) {}
            fn island(&self, _e: &IslandEvent) {}
        }
        impl ScreenSource for Stub {
            fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String> {
                Ok(layout.monitors.iter().map(|m| Frame { monitor_id: m.id, width: m.bounds.w as u32, height: m.bounds.h as u32, bgra: Arc::new(vec![0; (m.bounds.w * m.bounds.h * 4) as usize]) }).collect())
            }
        }
        impl WindowCatalog for Stub {
            fn windows(&self) -> Vec<RawWindow> { vec![] }
        }
        impl MonitorProbe for Stub {
            fn layout(&self) -> MonitorLayout {
                MonitorLayout::new(vec![Monitor { id: 0, bounds: PhysRect::new(0, 0, 64, 48), work: PhysRect::new(0, 0, 64, 48), scale: 1.0, primary: true }])
            }
            fn cursor(&self) -> PhysPoint { PhysPoint { x: 10, y: 10 } }
        }
        impl ElementSource for Stub {
            fn request(&self, _w: u64, _e: &str) {}
            fn elements_at(&self, _w: u64, _p: PhysPoint) -> Vec<ink_domain::recog::ElementCand> { vec![] }
            fn clear(&self) {}
        }
        impl Clipboard for Stub {
            fn set_image(&mut self, _w: u32, _h: u32, _bgra: &[u8]) -> Result<(), String> { Ok(()) }
            fn set_text(&mut self, _t: &str) -> Result<(), String> { Ok(()) }
        }
        impl FileSink for Stub {
            fn save_png(&mut self, _f: &str, stem: &str, _w: u32, _h: u32, _bgra: &[u8]) -> Result<SavedFile, String> {
                Ok(SavedFile { name: format!("{stem}.png"), path: format!("{stem}.png"), fell_back: false })
            }
        }
        impl Clock for Stub {
            fn local(&self) -> NameContext { NameContext { year: 2026, month: 11, day: 26, hour: 10, minute: 0, second: 0, app: None, n: 0 } }
            fn unix(&self) -> u64 { 0 }
        }

        let log = Log::default();
        let ports = Ports {
            hotkeys: Box::new(Stub),
            probe: Box::new(Stub),
            store: Box::new(Stub),
            notifier: Box::new(Stub),
            screen: Box::new(Stub),
            windows: Box::new(Stub),
            monitors: Box::new(Stub),
            overlay: Box::new(log.clone()),
            elements: Box::new(Stub),
            vision: Box::new(NoVision),
            clipboard: Box::new(Stub),
            files: Box::new(Stub),
            pins: Box::new(NoPins),
            clock: Box::new(Stub),
        };
        let mut svc = InkService::new(ports, Platform::Win, "test");
        svc.start_capture(CaptureKind::Region);
        svc.capture_input(CaptureInput::Command { name: "copy".into() });
        assert_eq!(log.0.lock().unwrap().as_slice(), ["hide:1", "flash:1:64x48:1"]);

        log.0.lock().unwrap().clear();
        svc.start_capture(CaptureKind::Region);
        svc.capture_input(CaptureInput::Command { name: "cancel".into() });
        assert_eq!(log.0.lock().unwrap().as_slice(), ["hide:2"]);
    }

    /// `[머리 길이 u32 LE][머리 JSON][RGBA]` 본문 만들기(ink-client encodeAnnotatedCommit과 같은 모양).
    fn body(meta: &str, rgba: &[u8]) -> Vec<u8> {
        let mut b = (meta.len() as u32).to_le_bytes().to_vec();
        b.extend_from_slice(meta.as_bytes());
        b.extend_from_slice(rgba);
        b
    }

    #[test]
    fn 주석_본문은_머리_길이_json_rgba로_푼다() {
        let c = AnnotatedCommit::decode(&body(
            r#"{"session":3,"seq":2,"action":"quickSave","w":1,"h":1,"mosaics":[{"x":0,"y":0,"w":4,"h":4,"block":2}]}"#,
            &[255, 0, 0, 255],
        ))
        .unwrap();
        assert_eq!(c.meta.session, 3);
        assert_eq!(c.meta.seq, 2);
        assert_eq!(c.meta.action, CommitAction::QuickSave);
        assert_eq!(c.meta.mosaics, vec![image::MosaicRect { x: 0, y: 0, w: 4, h: 4, block: 2 }]);
        assert_eq!(c.overlay, vec![255, 0, 0, 255]);
        // 층 없이 모자이크만·w/h 생략
        let only = AnnotatedCommit::decode(&body(r#"{"session":1,"seq":1,"action":"pin"}"#, &[])).unwrap();
        assert!(only.overlay.is_empty() && only.meta.mosaics.is_empty() && only.meta.w == 0);
        // 망가진 본문
        assert!(AnnotatedCommit::decode(&[1, 0]).is_err(), "길이 4바이트도 없음");
        assert!(AnnotatedCommit::decode(&body("{", &[])).is_err(), "JSON 아님");
        let mut cut = body(r#"{"session":1,"seq":1,"action":"copy"}"#, &[]);
        cut.truncate(10);
        assert!(AnnotatedCommit::decode(&cut).is_err(), "머리가 잘림");
        assert!(AnnotatedCommit::decode(&body(r#"{"session":1,"seq":1,"action":"print"}"#, &[])).is_err(), "모르는 동작");
        let huge = ((MAX_COMMIT_META + 1) as u32).to_le_bytes();
        assert!(AnnotatedCommit::decode(&huge).is_err(), "머리 한도");
    }
}
