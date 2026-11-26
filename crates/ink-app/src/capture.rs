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
use serde::Deserialize;
use std::collections::HashMap;
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

#[derive(Default)]
pub struct CaptureState {
    pub(crate) selector: Option<Selector>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) children: HashMap<u64, Vec<PhysRect>>,
    pub(crate) ranges: History,
    pub(crate) last: Option<LastCapture>,
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
        let ev = Evidence {
            point: p,
            window: w.map(|w| w.rect),
            children: &kids,
            elements: &els,
            vision: &vis,
            monitor,
            virtual_bounds: virt,
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
            Mode::Idle => None,
        }
    }

    fn emit_capture(&self) {
        let (Some(session), Some(sel)) = (self.session(), self.cap.selector.as_ref()) else { return };
        self.ports.notifier.capture(&CaptureView { session, selection: sel.view() });
    }

    /// 오버레이 입력 한 건.
    pub fn capture_input(&mut self, input: CaptureInput) {
        let (Some(_), Some(mut sel), Some(snap)) = (self.session(), self.cap.selector.take(), self.snapshot.take()) else { return };
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
            Effect::Copy(rect) => self.finish(rect, false, false),
            Effect::Save { rect, quick } => self.finish(rect, true, quick),
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
    fn finish(&mut self, rect: PhysRect, save: bool, _quick: bool) {
        if self.cap.case_dir.is_some() {
            self.write_case(rect);
        }
        let Some(snap) = self.snapshot.as_ref() else { return };
        let mode_name = self.cap.selector.as_ref().map(|s| s.mode_name()).unwrap_or("drag");
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
        let center = PhysPoint { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 };
        let app = snap.window_at(center, true).map(|w| w.exe.trim_end_matches(".exe").to_string()).filter(|s| !s.is_empty());
        let hex = image::uniform_color(w, h, &px).map(|c| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]));
        let layout = snap.layout.fingerprint();

        // 기록(사각형만)
        let max = self.settings.get("capture.historyMax").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        self.cap.ranges.push(HistoryEntry { rect, layout, mode: mode_name.into(), at: self.ports.clock.unix() }, max);
        if !self.readonly {
            if let Ok(v) = serde_json::to_value(&self.cap.ranges) {
                let _ = self.ports.store.save_section("history", &v);
            }
        }

        let (tw, th, tpx) = image::thumbnail(w, h, &px, 160, 96);
        let thumb = self.ports.overlay.thumbnail(tw, th, tpx);
        self.cap.last = Some(LastCapture { w, h, bgra: Arc::new(px), app, hex: hex.clone() });
        // 화면을 먼저 돌려준다(클립보드·파일은 그 뒤)
        self.end_capture();

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
                self.island(ev);
            }
            return;
        }
        let mut ev = IslandEvent::new(IslandKind::Result, "복사했어요").variant("result").button("저장", "capture.saveLast", Some("Ctrl+S"));
        ev.value = Some(size);
        ev.detail = Some("한글·PPT에 Ctrl+V".into());
        ev.thumb_url = thumb;
        if let Some(h) = hex {
            ev.chip = Some(IslandButton { label: format!("{h} 복사"), action: "capture.copyHex".into() });
        }
        self.island(ev);
    }

    /// 섬 [저장] — 마지막 결과를 저장 폴더에.
    pub fn save_last(&mut self) {
        if let Some(ev) = self.save_last_inner() {
            self.island(ev);
        } else if self.cap.last.is_none() {
            self.island(IslandEvent::new(IslandKind::Info, "저장할 캡처가 없어요"));
        }
    }

    fn save_last_inner(&mut self) -> Option<IslandEvent> {
        let last = self.cap.last.clone()?;
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
        let stem = naming::render(&template, &t).unwrap_or_else(|_| "screen-ink".into());
        let folder = self.settings.str("output.folder").to_string();
        match self.ports.files.save_png(&folder, &stem, last.w, last.h, &last.bgra) {
            Ok(f) => {
                let mut ev = IslandEvent::new(IslandKind::Result, if f.fell_back { "사진 폴더에 저장했어요" } else { "저장했어요" })
                    .variant("result")
                    .button("폴더 열기", "folder.open", None);
                ev.detail = Some(f.name);
                Some(ev)
            }
            Err(e) => {
                log::warn!("save failed: {e}");
                Some(IslandEvent::new(IslandKind::Problem, "저장하지 못했어요"))
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
}
