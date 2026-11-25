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
use ink_domain::selection::{Button, Context, Effect, Input, Key, Selector};
use ink_domain::settings::ExcludedApp;
use ink_domain::snapshot::Snapshot;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// 느린 앱 문턱(R-CAP-15): 색인이 이 시간을 2번 넘으면 그 앱은 창 단위로만.
const SLOW_INDEX_MS: f64 = 150.0;

/// 오버레이가 넘기는 입력(packages/ink-client CaptureInput).
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CaptureInput {
    Move { x: i32, y: i32 },
    Wheel { dy: i32 },
    Down { x: i32, y: i32, #[serde(default)] button: u8 },
    Up { x: i32, y: i32, #[serde(default)] button: u8 },
    Dblclick { x: i32, y: i32 },
    Key { key: String, code: String, #[serde(default)] ctrl: bool, #[serde(default)] shift: bool, #[serde(default)] alt: bool },
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
            CaptureInput::Key { key, code, ctrl, shift, alt } => Input::Key(Key { key, code, ctrl, shift, alt }),
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
    /// 진입 직후: 상태 기계 + UIA 미리 깨우기 + V 전처리(배경). 첫 보기를 내보낸다.
    pub(crate) fn start_selection(&mut self, frames: Vec<Frame>, cursor: PhysPoint) {
        let Some(snap) = self.snapshot.take() else { return };
        if let Some(w) = snap.window_at(cursor, true) {
            if !w.window_only {
                self.ports.elements.request(w.handle, &w.exe);
            }
        }
        self.ports.vision.prepare(&frames, &snap.layout, cursor);
        self.cap.frames = frames;
        let sel = {
            let ctx = ctx!(self, &snap);
            Selector::new(cursor, &ctx)
        };
        self.cap.selector = Some(sel);
        self.snapshot = Some(snap);
        self.emit_capture();
    }

    /// 모드 끝: 색인·전처리·이미지를 버린다.
    pub(crate) fn end_selection(&mut self) {
        self.cap.selector = None;
        self.cap.frames.clear();
        self.cap.children.clear();
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
        let effect = {
            let mut ctx = ctx!(self, &snap);
            sel.handle(input.into(), &mut ctx)
        };
        self.cap.selector = Some(sel);
        self.snapshot = Some(snap);
        self.apply(effect);
    }

    /// UIA 색인이 끝났다(어댑터 일꾼 → 메인 스레드). 느린 앱은 창 단위로 강등(R-CAP-15).
    pub fn on_elements_ready(&mut self, window: u64, exe: &str, took_ms: f64) {
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
        self.ports.overlay.highlight(session, rect);
        self.emit_capture();
    }

    /// 범위 확정 → 자르기 → (복사 | 저장) → 기록 → 모드 끝 → 결과 섬.
    fn finish(&mut self, rect: PhysRect, save: bool, _quick: bool) {
        let Some(snap) = self.snapshot.as_ref() else { return };
        let mode_name = self.cap.selector.as_ref().map(|s| s.mode_name()).unwrap_or("drag");
        let refs: Vec<FrameRef> = self
            .cap
            .frames
            .iter()
            .filter_map(|f| snap.layout.monitors.iter().find(|m| m.id == f.monitor_id).map(|m| FrameRef { bounds: m.bounds, w: f.width, h: f.height, bgra: &f.bgra }))
            .collect();
        let (w, h, px) = image::crop(&refs, rect);
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
