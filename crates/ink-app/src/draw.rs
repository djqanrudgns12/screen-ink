//! 판서·칠판(Alt+1 · Alt+3 · Alt+Shift+1, A §2.5·§3.7): 진입·보관·복원·내보내기·섬·계측.
//!
//! 경계(A §3.7): 포인터·획 보정·렌더·실행 취소는 모두 웹뷰(`inkEngine.js` + `DrawCanvas`)가 맡는다.
//! Rust는 ① 진입 순간 커서 모니터 한 장 캡처·그 모니터 오버레이 띄우기 ② `keptInk` 메모리 보관·복원 판정
//! ③ 합성 결과의 클립보드 복사·파일 저장 ④ 판서 섬·전역 단축키·시스템 사건(잠금·모니터 변경) 종료만 한다.
//!
//! 판서 문서는 웹뷰에만 있다. 그래서 Rust가 먼저 끝낸 경우(같은 키·다른 모드 키·잠금·화면 구성 변경)에는
//! 끝난 세션을 하나 기억해 두고, 웹뷰가 뒤이어 보내는 `drawFinish`(늦은 끝) 한 번을 받아 보관한다.

use crate::capture::{LastCapture, CAPTURE_FRAME_TTL};
use crate::ports::{Frame, SystemEvent, NO_SCREEN_PERMISSION};
use crate::service::{HotkeyOutcome, InkService};
use crate::state::{DrawSession, DrawViewTransform, IslandEvent, IslandKind, KeptCompat, OverlayMonitor};
use ink_domain::geometry::MonitorLayout;
use ink_domain::i18n::{tr, trf, Lang};
use ink_domain::image;
use ink_domain::mode::{self, DrawKind, Mode, ModeCommand};
use ink_domain::settings::Settings;
use ink_domain::shortcuts::{spoken_for, Platform};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

/// 내보내기(`draw_export`) 원본 바이트 IPC의 머리글 이름(소문자). 본문 = 헤더 없는 RGBA(또는 BGRA) 바이트.
pub const EXPORT_ACTION: &str = "x-ink-action";
pub const EXPORT_W: &str = "x-ink-w";
pub const EXPORT_H: &str = "x-ink-h";
/// 선택 — 있으면 지금 세션과 같을 때만 받는다
pub const EXPORT_SESSION: &str = "x-ink-session";
/// 선택 — `rgba`(기본, 캔버스 getImageData) | `bgra`
pub const EXPORT_FORMAT: &str = "x-ink-format";
/// 한 변 상한(16K) — 잘못된 머리글로 큰 버퍼를 잡지 않게
const EXPORT_MAX_SIDE: u32 = 16_384;
/// 섬 안내 한 줄 길이 상한(글자)
const NOTICE_MAX_CHARS: usize = 60;
/// 결과 섬이 떠 있다고 보는 시간 — UI 결과 섬 1.2초(동작 줄이기면 2초, `islandQueue`) 중 긴 쪽.
/// 이 안의 판서 키는 [판서로]와 같다. 지나면 보통 판서(커서 모니터 1×).
pub const RESULT_ISLAND_TO_DRAW: std::time::Duration = std::time::Duration::from_millis(2000);
/// 섬 단추가 판서 엔진에 보낼 수 있는 명령(`run` 동작 `draw.<명령>`)
pub const DRAW_COMMANDS: [&str; 2] = ["acceptSuggestion", "undo"];

const INK_COLORS: [&str; 8] = ["R", "Y", "G", "O", "B", "L", "W", "K"];
const TOOLS: [&str; 6] = ["pen", "marker", "text", "select", "eraser", "pan"];
const BACKGROUNDS: [&str; 3] = ["frame", "white", "black"];

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

// ───────────────────────── UI와 주고받는 모양 ─────────────────────────

/// 보관 판서(05 §7) — 메모리에 한 장, 디스크에 쓰지 않는다. 모니터·지문·크기·배율은 세션에서 Rust가 채운다.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeptInk {
    pub monitor_id: u32,
    pub layout_fingerprint: String,
    pub phys_w: u32,
    pub phys_h: u32,
    pub scale: f64,
    /// 칠판(Alt+3)에서 보관함 — 직전 판서 열기(Alt+Shift+1)는 칠판으로 다시 연다
    #[serde(default)]
    pub board: bool,
    /// InkDocument JSON v1(A §2.2). Rust는 항목 수만 센다.
    pub document: Value,
}

/// `drawFinish` 내용(A §2.5).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawFinish {
    pub session: u64,
    /// 보관할지(설정 `draw.keep`이 꺼져 있으면 무시)
    #[serde(default)]
    pub keep: bool,
    #[serde(default)]
    pub kept: Option<KeptPayload>,
}

/// UI가 넘기는 보관 내용 — `{document}`만 쓴다(KeptInk 모양 전체를 보내도 나머지는 세션 값으로 바뀐다).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct KeptPayload {
    #[serde(default)]
    pub document: Value,
}

/// `drawRestore` 결과. `kept`는 Same·Rescaled일 때만.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawRestoreResult {
    pub status: KeptCompat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kept: Option<KeptInk>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportAction {
    Copy,
    Save,
}

/// 내보내기 머리글을 읽은 것.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawExportMeta {
    pub action: ExportAction,
    pub w: u32,
    pub h: u32,
    pub session: Option<u64>,
    /// 본문이 이미 BGRA(기본은 RGBA → Rust가 바꾼다)
    pub bgra: bool,
}

impl DrawExportMeta {
    /// 머리글 하나 읽기(`get(이름)` — 대소문자는 어댑터가 맞춘다) → 메타. 바이트 수는 [`Self::byte_len`]과 견준다.
    pub fn parse(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let action = match get(EXPORT_ACTION).as_deref().map(str::trim) {
            Some("copy") => ExportAction::Copy,
            Some("save") => ExportAction::Save,
            other => return Err(format!("unknown export action: {}", other.unwrap_or("-"))),
        };
        let side = |name: &str| -> Result<u32, String> {
            let v: u32 = get(name).and_then(|s| s.trim().parse().ok()).ok_or_else(|| format!("missing {name}"))?;
            if v == 0 || v > EXPORT_MAX_SIDE {
                return Err(format!("bad {name}: {v}"));
            }
            Ok(v)
        };
        let (w, h) = (side(EXPORT_W)?, side(EXPORT_H)?);
        let session = match get(EXPORT_SESSION) {
            Some(s) => Some(s.trim().parse::<u64>().map_err(|_| format!("bad {EXPORT_SESSION}"))?),
            None => None,
        };
        let bgra = match get(EXPORT_FORMAT).as_deref().map(str::trim) {
            None | Some("rgba") => false,
            Some("bgra") => true,
            Some(other) => return Err(format!("unknown pixel format: {other}")),
        };
        Ok(DrawExportMeta { action, w, h, session, bgra })
    }

    pub fn byte_len(&self) -> usize {
        self.w as usize * self.h as usize * 4
    }
}

/// `drawExport` 결과(`{ok, name?}`). `error`는 진단용(화면 문구는 섬이 보인다).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawExported {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl DrawExported {
    fn fail(e: &str) -> Self {
        DrawExported { ok: false, name: None, error: Some(e.into()) }
    }
}

/// `drawIsland` 조각 — 보낸 것만 바꾼다(나머지는 지난 값).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DrawIslandPatch {
    /// 있으면 지금 세션과 같을 때만
    pub session: Option<u64>,
    /// R·Y·G·O·B·L·W·K
    pub color: Option<String>,
    /// pen · marker · text · select · eraser · pan
    pub tool: Option<String>,
    /// 화면 기준 굵기 1~30
    pub width_px: Option<f64>,
    /// 확대 배율 1~8
    pub zoom: Option<f64>,
    pub page_index: Option<u32>,
    pub page_count: Option<u32>,
    /// frame · white · black — 판서 중 W·K로 바탕을 바꿨을 때
    pub background: Option<String>,
    /// 4초 무동작 물러남 — 섬 창에 그대로 넘긴다
    pub idle_dim: Option<bool>,
    /// 커서가 섬 가까이 — 섬이 비켜 감
    pub avoid: Option<bool>,
    /// 보정·안내 한 줄("수평으로 맞췄어요" 등) — 모드 섬 위에 잠깐
    pub notice: Option<String>,
    /// 안내에 붙일 단추: `undo` = [되돌리기](모두 지우기 뒤)
    pub notice_action: Option<String>,
    /// 0.5초 멈춤 도형 제안("원으로 바꿀까요? · Enter") — 조각마다 보낸다(없으면 걷힘). 섬 [바꾸기] = 엔진 acceptSuggestion
    pub suggestion: Option<String>,
}

/// `drawPenMetrics` — 소프트웨어 펜 지연(M5) 요약.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
pub struct PenSample {
    pub n: u32,
    pub p50: f64,
    pub p95: f64,
    pub max: f64,
}

impl PenSample {
    /// 계측 한 줄(05 §9). 값이 이상하면 `None`.
    pub fn log_line(&self) -> Option<String> {
        let ok = |v: f64| v.is_finite() && (0.0..60_000.0).contains(&v);
        if self.n == 0 || !ok(self.p50) || !ok(self.p95) || !ok(self.max) {
            return None;
        }
        Some(format!("pen latency n={} p50={:.1} p95={:.1} max={:.1} ms", self.n, self.p50, self.p95, self.max))
    }
}

// ───────────────────────── 순수 판정 ─────────────────────────

/// 보관 판서를 이 모니터에 불러올 수 있는지(02 §3.6).
/// 같은 모니터 id·같은 물리 크기·같은 배율 = Same(다른 모니터 구성은 상관없음 — 문서는 그 모니터 왼쪽 위 기준).
/// 같은 id인데 배율·해상도가 바뀌었어도 가로세로 비율이 같으면 한 비율로 옮길 수 있어 Rescaled. 그 밖은 DifferentMonitor.
pub fn kept_compat(kept: Option<&KeptInk>, monitor: &OverlayMonitor) -> KeptCompat {
    let Some(k) = kept else { return KeptCompat::None };
    if k.monitor_id != monitor.id || k.phys_w == 0 || k.phys_h == 0 || monitor.bounds.w <= 0 || monitor.bounds.h <= 0 {
        return KeptCompat::DifferentMonitor;
    }
    let (w, h) = (monitor.bounds.w as u32, monitor.bounds.h as u32);
    if k.phys_w == w && k.phys_h == h && (k.scale - monitor.scale).abs() < 0.005 {
        return KeptCompat::Same;
    }
    let kept_aspect = k.phys_w as f64 / k.phys_h as f64;
    let aspect = w as f64 / h as f64;
    if (kept_aspect - aspect).abs() / aspect < 0.01 {
        KeptCompat::Rescaled
    } else {
        KeptCompat::DifferentMonitor
    }
}

/// InkDocument의 항목 수(쪽마다 `items`). 쪽이 없으면 맨 위 `items`(05 §6 모양).
pub fn ink_items(doc: &Value) -> usize {
    let items = |v: &Value| v.get("items").and_then(Value::as_array).map_or(0, Vec::len);
    match doc.get("pages").and_then(Value::as_array) {
        Some(pages) => pages.iter().map(items).sum(),
        None => items(doc),
    }
}

/// RGBA(캔버스 `getImageData`) → BGRA(클립보드·PNG 포트) 제자리 바꾸기.
pub fn rgba_to_bgra(px: &mut [u8]) {
    for p in px.chunks_exact_mut(4) {
        p.swap(0, 2);
    }
}

fn tool_label(tool: &str, lang: Lang) -> String {
    let key = match tool {
        "marker" => "island.draw.tool.marker",
        "text" => "island.draw.tool.text",
        "select" => "island.draw.tool.select",
        "eraser" => "island.draw.tool.eraser",
        "pan" => "island.draw.tool.pan",
        _ => "island.draw.tool.pen",
    };
    tr(lang, key)
}

/// 확대 배율 글자: 1.25·1.75는 두 자리, 나머지는 한 자리(`×2.0`, 실시간 확대 섬과 같은 모양).
fn zoom_label(z: f64) -> String {
    let r = (z * 100.0).round() / 100.0;
    if ((r * 10.0).round() - r * 10.0).abs() < 1e-9 {
        format!("×{r:.1}")
    } else {
        format!("×{r:.2}")
    }
}

/// `mode enter draw [N ms] (capture a, show b, ui paint c)` — 진입 계측(M4) 한 줄. 칠판은 `mode enter board`.
pub fn entry_line(board: bool, total_ms: f64, capture_ms: f64, show_ms: f64, paint_ms: Option<f64>) -> String {
    entry_line_from(DrawSource::Key, 1.0, board, total_ms, capture_ms, show_ms, paint_ms)
}

/// 판서 진입이 어디서 왔나 — 계측 줄 `from …`(05 §9 숫자만).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawSource {
    /// 판서 키·트레이(커서 모니터를 새로 찍는다) — 줄 모양은 예전 그대로
    Key,
    /// 실시간 확대 중 Alt+1. 숨김 시간에는 새로 찍을 때만 화면 갱신 대기가 포함된다.
    Zoom { hide_ms: f64, reused: bool },
    /// 결과 섬 [판서로](3e-handoff) — `reused` = 방금 고른 범위의 정지 프레임을 다시 씀(새로 찍지 않음)
    Capture { reused: bool },
    /// 트레이에서 고른 캡처. 현재 화면을 새로 찍지 않는다.
    Recent { id: u64 },
}

/// 진입 계측 한 줄(출처 포함): `mode enter draw [61 ms] (from zoom, scale 2.5, hide 18, capture 35, show 3, ui paint 5)`,
/// `mode enter draw [12 ms] (from capture, scale 4.22, frame reused, capture 0, show 3, ui paint 6)`. 키 = [`entry_line`]과 같음.
pub fn entry_line_from(source: DrawSource, scale: f64, board: bool, total_ms: f64, capture_ms: f64, show_ms: f64, paint_ms: Option<f64>) -> String {
    let what = if board { "board" } else { "draw" };
    let paint = paint_ms.filter(|v| v.is_finite() && *v >= 0.0).map_or(String::new(), |p| format!(", ui paint {p:.0}"));
    let s = (scale * 100.0).round() / 100.0;
    let from = match source {
        DrawSource::Key => String::new(),
        DrawSource::Recent { .. } => format!("from recent, scale {s}, "),
        DrawSource::Zoom { hide_ms, reused: true } => format!("from zoom, scale {s}, frame reused, hide {hide_ms:.0}, "),
        DrawSource::Zoom { hide_ms, reused: false } => format!("from zoom, scale {s}, hide {hide_ms:.0}, "),
        DrawSource::Capture { reused: true } => format!("from capture, scale {s}, frame reused, "),
        DrawSource::Capture { reused: false } => format!("from capture, scale {s}, "),
    };
    format!("mode enter {what} [{total_ms:.0} ms] ({from}capture {capture_ms:.0}, show {show_ms:.0}{paint})")
}

/// 판서 섬 한 줄의 지금 값(색·도구·굵기·배율·쪽·바탕·도형 제안).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Look {
    color: String,
    tool: String,
    width: f64,
    zoom: f64,
    page: u32,
    pages: u32,
    background: String,
    suggestion: Option<String>,
    idle_dim: Option<bool>,
    avoid: Option<bool>,
}

impl Default for Look {
    fn default() -> Self {
        Look { color: "R".into(), tool: "pen".into(), width: 4.0, zoom: 1.0, page: 0, pages: 1, background: "frame".into(), suggestion: None, idle_dim: None, avoid: None }
    }
}

impl Look {
    fn from_settings(s: &Settings, board: bool) -> Self {
        let mut look = Look::default();
        let color = s.str("draw.color");
        if INK_COLORS.contains(&color) {
            look.color = color.into();
        }
        if let Some(w) = s.get("draw.width").and_then(Value::as_f64) {
            look.width = w.clamp(1.0, 30.0);
        }
        if board {
            look.background = "white".into();
        }
        look
    }

    fn merge(&mut self, p: &DrawIslandPatch) {
        if let Some(c) = p.color.as_deref().filter(|c| INK_COLORS.contains(c)) {
            self.color = c.into();
        }
        if let Some(t) = p.tool.as_deref().filter(|t| TOOLS.contains(t)) {
            self.tool = t.into();
        }
        if let Some(w) = p.width_px.filter(|w| w.is_finite()) {
            self.width = w.clamp(1.0, 30.0);
        }
        if let Some(z) = p.zoom.filter(|z| z.is_finite()) {
            self.zoom = z.clamp(1.0, 8.0);
        }
        if let Some(n) = p.page_count.filter(|n| *n >= 1) {
            self.pages = n;
        }
        if let Some(i) = p.page_index {
            self.page = i;
        }
        self.page = self.page.min(self.pages - 1);
        if let Some(b) = p.background.as_deref().filter(|b| BACKGROUNDS.contains(b)) {
            self.background = b.into();
        }
        if p.idle_dim.is_some() {
            self.idle_dim = p.idle_dim;
        }
        if p.avoid.is_some() {
            self.avoid = p.avoid;
        }
        self.suggestion = p.suggestion.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.chars().take(NOTICE_MAX_CHARS).collect());
    }
}

/// 판서 모드 섬(04 S12·S14): 판서 = `[●] 그리기 · 펜 4px ×1.75`, 칠판 = `칠판 · 흰색 1 / 1`.
pub(crate) fn draw_island_event(board: bool, look: &Look, kept_hint: Option<&str>, lang: Lang) -> IslandEvent {
    let on_board = board || look.background != "frame";
    let mut ev = if on_board {
        let bg = tr(lang, if look.background == "black" { "island.draw.board.black" } else { "island.draw.board.white" });
        let h_bg = tr(lang, "island.draw.hint.bg");
        let h_keep = tr(lang, "island.draw.hint.keep");
        let h_keys = tr(lang, "island.hint.shortcuts");
        let mut ev = IslandEvent::new(IslandKind::Mode, trf(lang, "island.draw.boardTitle", &[("bg", &bg)]))
            .variant("board")
            .hints(&[("W·K", &h_bg), ("Esc", &h_keep), ("?", &h_keys)]);
        ev.value = Some(format!("{} / {}", look.page + 1, look.pages));
        // 칠판에서도 지금 펜 색 점(검은 칠판의 K는 화면이 흰색으로 바꿔 그림)
        ev.pen_color = Some(look.color.clone());
        ev
    } else {
        let tool = tool_label(&look.tool, lang);
        let h_keep = tr(lang, "island.draw.hint.keep");
        let h_keys = tr(lang, "island.hint.shortcuts");
        let mut ev = IslandEvent::new(IslandKind::Mode, trf(lang, "island.draw.title", &[("tool", &tool)]))
            .variant("draw")
            .hints(&[("Esc", &h_keep), ("?", &h_keys)]);
        ev.pen_color = Some(look.color.clone());
        ev.pen_width = Some(format!("{}px", look.width.round() as i64));
        if look.zoom > 1.0 + 1e-9 {
            ev.value = Some(zoom_label(look.zoom));
        }
        ev
    };
    ev.mode = Some(if board { "board" } else { "draw" }.into());
    ev.idle_dim = look.idle_dim;
    ev.avoid = look.avoid;
    if let Some(h) = kept_hint {
        ev.detail = Some(h.into());
    }
    // 멈춤 도형 제안(제안형): 그동안 섬 글이 제안이 되고 [바꾸기] = 엔진 acceptSuggestion(Enter와 같음)
    if let Some(s) = &look.suggestion {
        ev.text = s.clone();
        ev.detail = None;
        ev = ev.button(&tr(lang, "island.draw.acceptSuggestion"), "draw.acceptSuggestion", None);
    }
    ev
}

// ───────────────────────── 서비스 상태 ─────────────────────────

/// Rust가 먼저 끝낸 세션 — 늦은 `drawFinish` 한 번을 받는다.
#[derive(Clone, Debug)]
pub(crate) struct Ended {
    session: DrawSession,
    /// 보관했어요 섬을 띄우지 않음(다른 모드로 넘어감·잠금·화면 변경)
    quiet: bool,
}

/// 진입 계측(M4): 단축키 순간과 단계별 시간 — UI가 그렸다고 알리면 한 줄.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DrawEntry {
    pressed: Instant,
    capture_ms: f64,
    show_ms: f64,
    source: DrawSource,
    scale: f64,
}

/// 판서를 어떻게 여나 — 기본(`Default`) = 판서 키: 커서 모니터를 새로 찍고 1×.
/// 5.3(확대 → 판서)은 그 모니터·확대 변환, 3e-handoff([판서로])는 그 모니터·고른 범위 맞춤 변환·이미 찍은 프레임.
#[derive(Clone, Debug)]
pub(crate) struct DrawStart {
    /// 대상 모니터(없으면 커서 모니터). 배치에서 사라졌으면 커서 모니터 + 1×(변환·프레임은 버림)
    pub monitor_id: Option<u32>,
    /// 다시 쓸 정지 이미지(그 모니터 것일 때만 쓴다) — 있으면 화면을 다시 찍지 않는다
    pub frame: Option<Frame>,
    pub view: DrawViewTransform,
    pub source: DrawSource,
}

impl Default for DrawStart {
    fn default() -> Self {
        DrawStart { monitor_id: None, frame: None, view: DrawViewTransform::default(), source: DrawSource::Key }
    }
}

#[derive(Default)]
pub struct DrawState {
    /// 지금 판서(모드가 Draw일 때만)
    pub(crate) session: Option<DrawSession>,
    pub(crate) kept: Option<KeptInk>,
    pub(crate) ended: Option<Ended>,
    pub(crate) entry: Option<DrawEntry>,
    pub(crate) look: Look,
    /// 이번 판서에서 이미 불러옴 — 섬의 "이전 판서" 힌트를 거둔다
    pub(crate) restored: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EndWhy {
    /// 같은 판서 키 — 사용자가 끝냄(보관 섬을 보인다)
    Hotkey,
    /// 다른 모드 키 — 곧 다른 모드 섬이 뜬다
    Switch,
    /// 잠금·절전
    System,
    /// 모니터 연결·해제·배율 변경
    Display,
}

impl InkService {
    // ───────────── 읽기 ─────────────

    pub fn draw_session(&self) -> Option<&DrawSession> {
        self.draw.session.as_ref()
    }

    pub fn kept_ink(&self) -> Option<&KeptInk> {
        self.draw.kept.as_ref()
    }

    fn active_draw(&self, session: u64) -> Option<&DrawSession> {
        match self.mode {
            Mode::Draw { session: s, .. } if s == session => self.draw.session.as_ref().filter(|d| d.session == session),
            _ => None,
        }
    }

    /// 섬에 쓸 키 글자: Windows = 표준 표기(`Alt+Shift+1`), 맥 = 기호. 비운 칸이면 None.
    pub(crate) fn key_text(&self, slot: &str) -> Option<String> {
        let combo = self.settings.shortcut(slot);
        if combo.is_empty() {
            return None;
        }
        Some(match self.platform {
            Platform::Win => combo.to_string(),
            Platform::Mac => spoken_for(combo, Platform::Mac, self.lang()),
        })
    }

    // ───────────── 단축키 · 트레이 ─────────────

    /// 판서 칸이거나 판서 중이면 처리해서 결과를, 아니면 `None`(캡처 흐름으로).
    /// 판서 중 캡처 키 = 판서를 끝내고(보관은 늦은 끝으로) `None` — 캡처 흐름이 쉬는 상태에서 시작한다.
    pub(crate) fn on_draw_hotkey(&mut self, slot: &str, pressed: Instant) -> Option<HotkeyOutcome> {
        let drawing = matches!(self.mode, Mode::Draw { .. });
        // 결과 섬이 떠 있는 동안 판서 키 = 결과 섬 [판서로](키캡 `Alt 1`과 맞춤, 메인 결정) — 고른 범위가 꽉 차게
        if slot == "draw" && matches!(self.mode, Mode::Idle | Mode::Recording { .. }) && self.result_island_up(pressed) {
            self.capture_to_draw_at(pressed);
            return Some(HotkeyOutcome::Draw);
        }
        // 판서 중 실시간 확대 키 = 판서를 끝내고(보관은 늦은 끝으로) 확대로 — 확대 판정은 쉬는 상태에서 다시
        if drawing && slot == ink_domain::zoom::ZOOM_SLOT {
            self.end_draw(EndWhy::Switch);
            return Some(self.zoom_hotkey(slot, pressed).unwrap_or(HotkeyOutcome::Ignored));
        }
        let now = self.epoch.elapsed().as_millis() as u64;
        match mode::on_hotkey(&self.mode, slot, now) {
            ModeCommand::StartDraw(kind) => {
                self.begin_draw(kind, pressed);
                Some(HotkeyOutcome::Draw)
            }
            ModeCommand::SwitchDraw(kind) => {
                self.end_for_switch();
                self.begin_draw(kind, pressed);
                Some(HotkeyOutcome::Draw)
            }
            ModeCommand::DrawRestore => {
                self.restore_in_place();
                Some(HotkeyOutcome::Draw)
            }
            ModeCommand::End if drawing => {
                self.end_draw(EndWhy::Hotkey);
                Some(HotkeyOutcome::Draw)
            }
            ModeCommand::Switch(_) if drawing => {
                self.end_draw(EndWhy::Switch);
                None
            }
            _ => None,
        }
    }

    /// 트레이·명령에서 시작(단축키가 꺼져 있어도). 지금 모드는 끝내고 새로 — 판서 중 "직전 판서 열기"는 그 자리 복원,
    /// 실시간 확대 중 판서는 Alt+1과 같이 그 순간 확대 화면 그대로 이어받는다(5.3).
    pub fn start_draw(&mut self, kind: DrawKind) {
        if kind == DrawKind::Restore && matches!(self.mode, Mode::Draw { board: false, .. }) {
            self.restore_in_place();
            return;
        }
        if kind == DrawKind::Draw && matches!(self.mode, Mode::LiveZoom { .. }) {
            self.zoom_to_draw(Instant::now());
            return;
        }
        self.end_for_switch();
        self.begin_draw(kind, Instant::now());
    }

    /// 방금 확정한 캡처의 결과 섬([판서로] 단추)이 아직 떠 있을 때인지 — 섬은 화면이 걷어서 Rust는 시간으로 가늠한다.
    fn result_island_up(&self, pressed: Instant) -> bool {
        self.cap.handoff.as_ref().is_some_and(|h| pressed.saturating_duration_since(h.at) < RESULT_ISLAND_TO_DRAW)
    }

    /// 판서 중이면 다른 모드로 넘어가려고 끝낸다(보관은 웹뷰의 늦은 `drawFinish`로). 판서가 아니면 아무것도 안 함.
    pub(crate) fn end_draw_for_switch(&mut self) {
        if matches!(self.mode, Mode::Draw { .. }) {
            self.end_draw(EndWhy::Switch);
        }
    }

    /// 다른 모드로 넘어가기 전 지금 모드 끝. 확대는 돋보기가 화면에서 사라질 때까지 기다린다(곧 찍을 수 있다).
    fn end_for_switch(&mut self) {
        match self.mode {
            Mode::Capture { .. } => self.end_capture(),
            Mode::Draw { .. } => self.end_draw(EndWhy::Switch),
            Mode::LiveZoom { .. } => {
                self.end_zoom_settled();
            }
            Mode::Idle | Mode::Recording { .. } => {}
        }
    }

    /// 결과 섬 [판서로](3e-handoff, `capture.toDraw`): 방금 고른 범위가 꽉 차게 확대한 채 그 모니터 판서로(≤ 150 ms).
    /// 확정 순간 쥐어 둔 그 모니터 정지 프레임을 다시 쓴다(화면을 다시 찍지 않음 — 섬이 떠 있던 장면 그대로).
    /// 프레임이 30초를 넘었거나 모니터 구성이 바뀌었으면 지금 화면을 새로 찍어 같은 배율 공식으로 연다.
    pub fn capture_to_draw(&mut self) {
        self.capture_to_draw_at(Instant::now());
    }

    /// `pressed` = 단추를 받은 순간(진입 계측의 시작점이자 30초 판정의 지금 — 시험은 나중 시각을 준다).
    pub fn capture_to_draw_at(&mut self, pressed: Instant) {
        let Some(h) = self.cap.handoff.take() else {
            self.island(IslandEvent::new(IslandKind::Info, self.t("island.draw.noCapture")));
            return;
        };
        let layout = self.ports.monitors.layout();
        let same_layout = layout.fingerprint() == h.layout_fingerprint;
        let young = pressed.saturating_duration_since(h.at) < CAPTURE_FRAME_TTL;
        let fresh = h.frame.filter(|_| same_layout && young);
        let center = ink_domain::geometry::PhysPoint { x: h.rect.x + h.rect.w / 2, y: h.rect.y + h.rect.h / 2 };
        // 같은 구성이면 그 모니터, 바뀌었으면 지금 구성에서 범위 가운데의 모니터(없으면 커서 모니터)
        let mon = if same_layout { layout.monitors.iter().find(|m| m.id == h.monitor_id) } else { layout.monitor_at(center) }.cloned();
        let Some(mon) = mon else {
            self.end_for_switch();
            self.begin_draw(DrawKind::Draw, pressed);
            return;
        };
        let (s, tx, ty) = ink_domain::zoom::fit_rect_view(mon.bounds, mon.scale, h.rect);
        let reused = fresh.is_some();
        self.end_for_switch();
        self.begin_draw_with(
            DrawKind::Draw,
            pressed,
            DrawStart { monitor_id: Some(mon.id), frame: fresh, view: DrawViewTransform { s, tx, ty }, source: DrawSource::Capture { reused } },
        );
    }

    /// 트레이에서 고른 원본으로 판서를 연다. 찍은 뒤 바뀐 화면이나 다른 판서의 주석은 섞지 않는다.
    pub fn reopen_capture(&mut self, id: u64) {
        if self.cap.recent.get(id, self.clock_ms()).is_none() {
            return;
        }
        self.end_for_switch();
        self.begin_draw_with(DrawKind::Draw, Instant::now(), DrawStart { source: DrawSource::Recent { id }, ..Default::default() });
    }

    // ───────────── 진입 ─────────────

    fn begin_draw(&mut self, kind: DrawKind, pressed: Instant) {
        self.begin_draw_with(kind, pressed, DrawStart::default());
    }

    /// 모니터 한 장 캡처(칠판은 건너뜀, 넘겨받은 프레임이 있으면 그대로) → 그 모니터 오버레이 창 → 세션 알림 → 모드 → 섬.
    pub(crate) fn begin_draw_with(&mut self, kind: DrawKind, pressed: Instant, start: DrawStart) {
        if self.ports.probe.secure_desktop() {
            if !self.secure_notified {
                self.secure_notified = true;
                self.island(IslandEvent::new(IslandKind::Off, self.t("island.security")).variant("security"));
            }
            return;
        }
        // 새 모드에 들어가면 결과 섬 [판서로]·되살리기는 더 누를 수 없다 — 쥐어 둔 프레임을 놓는다
        self.cap.handoff = None;
        self.clear_discarded_capture();
        self.prepare_overlays();
        let layout = self.prepared.clone().unwrap_or_default();
        let cursor = self.ports.monitors.cursor();
        let DrawStart { monitor_id, mut frame, mut view, mut source } = start;
        let wanted = monitor_id.and_then(|id| layout.monitors.iter().find(|m| m.id == id)).cloned();
        if let (Some(id), None) = (monitor_id, wanted.as_ref()) {
            log::warn!("draw: monitor {id} is gone — cursor monitor at 1x");
            frame = None;
            view = DrawViewTransform::default();
        }
        let Some(mon) = wanted.or_else(|| layout.nearest(cursor).cloned()) else {
            log::warn!("draw: no monitor in layout");
            self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.screenError")));
            return;
        };
        if !self.settings.bool("draw.keep") {
            self.draw.kept = None;
        }
        // 직전 판서 열기: 칠판에서 보관했으면 칠판으로 연다
        let board = match kind {
            DrawKind::Board => true,
            DrawKind::Draw => false,
            DrawKind::Restore => self.draw.kept.as_ref().is_some_and(|k| k.board),
        };

        // 넘겨받은 모니터와 물리 해상도가 지금도 같아야 원본을 쓸 수 있다.
        let reuse = frame.take().filter(|f| f.monitor_id == mon.id && f.width == mon.bounds.w as u32 && f.height == mon.bounds.h as u32);
        if let DrawSource::Zoom { hide_ms, reused } = &mut source {
            *reused = reuse.is_some();
            if !*reused {
                let t = Instant::now();
                self.zoom.magnifier.wait_hidden();
                *hide_ms += ms(t.elapsed());
            }
        }
        let t = Instant::now();
        let frame = if board {
            None
        } else if let DrawSource::Recent { id } = source {
            let Some(entry) = self.cap.recent.get(id, self.clock_ms()) else { return };
            let (frame, rect) = entry.draw_frame(mon.id, mon.bounds);
            let (s, tx, ty) = ink_domain::zoom::fit_rect_view(mon.bounds, mon.scale, rect);
            view = DrawViewTransform { s, tx, ty };
            Some(frame)
        } else if let Some(f) = reuse {
            Some(f)
        } else {
            match self.ports.screen.capture(&MonitorLayout::new(vec![mon.clone()])) {
                Ok(frames) => match frames.into_iter().find(|f| f.monitor_id == mon.id) {
                    Some(f) => Some(f),
                    None => {
                        log::warn!("draw capture: no frame for the cursor monitor");
                        self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.screenError")));
                        return;
                    }
                },
                Err(e) if e.starts_with(NO_SCREEN_PERMISSION) => {
                    log::warn!("draw capture failed: {e}");
                    self.island(self.no_screen_permission_island());
                    return;
                }
                Err(e) => {
                    log::warn!("draw capture failed: {e}");
                    self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.screenError")));
                    return;
                }
            }
        };
        let capture_ms = ms(t.elapsed());

        self.session_seq += 1;
        let session = self.session_seq;
        let monitor = OverlayMonitor { id: mon.id, bounds: mon.bounds, work: mon.work, scale: mon.scale, primary: mon.primary, frame_url: None };
        let kept_compatible = kept_compat(self.draw.kept.as_ref(), &monitor);
        // 칠판은 늘 1×(이어받기는 정지 화면 판서만)
        let initial_view = if board { DrawViewTransform::default() } else { view };
        let mut ds = DrawSession {
            session,
            board,
            monitor,
            layout_fingerprint: layout.fingerprint(),
            initial_view,
            restore_on_open: kind == DrawKind::Restore,
            has_kept: self.draw.kept.is_some(),
            kept_compatible,
            cursor,
        };

        let t = Instant::now();
        match self.ports.overlay.show_draw(&ds, frame) {
            Ok(url) => ds.monitor.frame_url = url,
            Err(e) => {
                log::warn!("draw overlay show failed: {e}");
                self.ports.overlay.hide(session);
                self.island(IslandEvent::new(IslandKind::Problem, self.t("island.draw.openError")));
                return;
            }
        }
        let show_ms = ms(t.elapsed());

        self.draw.entry = Some(DrawEntry { pressed, capture_ms, show_ms, source, scale: initial_view.s });
        self.draw.restored = false;
        self.draw.look = Look::from_settings(&self.settings, board);
        // 섬의 배율도 처음부터 이어받은 값(UI는 첫 모양을 다시 보내지 않는다)
        self.draw.look.zoom = initial_view.s.clamp(ink_domain::zoom::DRAW_MIN_ZOOM, ink_domain::zoom::DRAW_MAX_ZOOM);
        self.mode = Mode::Draw { session, board, monitor_id: ds.monitor.id, started_ms: self.epoch.elapsed().as_millis() as u64 };
        // 세션 먼저, 상태 나중 — 오버레이가 세션을 쥔 채 판서 화면으로 바뀐다
        self.ports.notifier.draw(&ds);
        self.draw.session = Some(ds);
        self.emit_state();
        self.emit_draw_island();
    }

    /// UI가 배경을 그렸다 → 진입 계측(M4) 한 줄. 진입 시간(ms)을 돌려준다.
    pub fn draw_ready(&mut self, session: u64, paint_ms: Option<f64>) -> Option<f64> {
        self.active_draw(session)?;
        let (total, line) = self.entry_measure(paint_ms)?;
        self.draw.entry = None;
        log::info!("{line}");
        Some(total)
    }

    /// 지금 판서가 아직 그려지기 전이면 진입 계측 줄(`draw_ready`가 남기는 것과 같은 모양, 출처 포함).
    pub fn draw_entry_line(&self, paint_ms: Option<f64>) -> Option<String> {
        self.entry_measure(paint_ms).map(|(_, line)| line)
    }

    fn entry_measure(&self, paint_ms: Option<f64>) -> Option<(f64, String)> {
        let board = self.draw.session.as_ref()?.board;
        let e = self.draw.entry.as_ref()?;
        let total = ms(e.pressed.elapsed());
        Some((total, entry_line_from(e.source, e.scale, board, total, e.capture_ms, e.show_ms, paint_ms)))
    }

    // ───────────── 끝 · 보관 ─────────────

    /// 오버레이를 숨기고 쉬는 상태로. 끝난 세션을 돌려준다.
    fn close_draw(&mut self) -> Option<DrawSession> {
        let Mode::Draw { session, .. } = self.mode else { return None };
        self.ports.overlay.hide(session);
        // 녹화 중이면 녹화 상태로 돌아간다(단계 6)
        self.mode = self.rest_mode();
        self.draw.entry = None;
        let ds = self.draw.session.take();
        self.island(IslandEvent::new(IslandKind::End, ""));
        self.emit_state();
        ds
    }

    /// Rust가 먼저 끝냄(같은 키·다른 모드·잠금·화면 변경). 문서는 웹뷰에 있어 늦은 `drawFinish`를 기다린다.
    fn end_draw(&mut self, why: EndWhy) {
        let Some(ds) = self.close_draw() else { return };
        self.draw.ended = Some(Ended { session: ds, quiet: why != EndWhy::Hotkey });
        if why == EndWhy::Display {
            self.island(IslandEvent::new(IslandKind::Info, self.t("island.draw.displayChanged")));
        }
    }

    /// 판서 끝(Esc·우클릭 1×) 또는 Rust가 먼저 끝낸 세션의 늦은 끝. `keep && 항목 ≥ 1 && draw.keep`이면 보관.
    pub fn draw_finish(&mut self, f: DrawFinish) {
        if let Some(ds) = self.active_draw(f.session).cloned() {
            let kept = self.keep_ink(&ds, f.keep, f.kept);
            self.close_draw();
            if kept {
                self.island(self.kept_island());
            }
            return;
        }
        // 기다리던 세션일 때만 꺼낸다 — 모르는 세션의 끝이 기다리던 것을 지우지 않게
        if self.draw.ended.as_ref().is_some_and(|e| e.session.session == f.session) {
            if let Some(e) = self.draw.ended.take() {
                if self.keep_ink(&e.session, f.keep, f.kept) && !e.quiet {
                    self.island(self.kept_island());
                }
            }
        }
        // 그 밖(이미 받은 끝·모르는 세션)은 무시
    }

    fn keep_ink(&mut self, ds: &DrawSession, keep: bool, kept: Option<KeptPayload>) -> bool {
        if !self.settings.bool("draw.keep") {
            // 설정을 끄면 보관도 지운다(05 §7 D)
            self.draw.kept = None;
            return false;
        }
        let Some(document) = kept.map(|k| k.document) else { return false };
        if !keep || ink_items(&document) == 0 {
            return false;
        }
        self.draw.kept = Some(KeptInk {
            monitor_id: ds.monitor.id,
            layout_fingerprint: ds.layout_fingerprint.clone(),
            phys_w: ds.monitor.bounds.w.max(0) as u32,
            phys_h: ds.monitor.bounds.h.max(0) as u32,
            scale: ds.monitor.scale,
            board: ds.board,
            document,
        });
        true
    }

    fn kept_island(&self) -> IslandEvent {
        let mut ev = IslandEvent::new(IslandKind::Result, self.t("island.draw.kept"));
        if let Some(k) = self.key_text("restore") {
            ev = ev.detail(self.tf("island.draw.keptDetail", &[("key", &k)]));
        }
        ev
    }

    /// 전체 초기화(05 §7 D).
    pub(crate) fn clear_kept_ink(&mut self) {
        self.draw.kept = None;
    }

    /// 잠금·절전·화면 구성 변경이면 판서를 안전하게 끝낸다(보관은 늦은 끝으로, 02 §7·05 §11-7).
    pub(crate) fn draw_on_system_event(&mut self, ev: SystemEvent) {
        if !matches!(self.mode, Mode::Draw { .. }) {
            return;
        }
        match ev {
            SystemEvent::Lock | SystemEvent::Suspend => self.end_draw(EndWhy::System),
            SystemEvent::DisplayChange => self.end_draw(EndWhy::Display),
            SystemEvent::Unlock | SystemEvent::Resume | SystemEvent::ThemeChange => {}
        }
    }

    // ───────────── 복원 ─────────────

    /// 판서 중 직전 판서 키 — 같은 세션을 `restoreOnOpen: true`로 다시 알린다(UI가 `drawRestore`를 부른다).
    fn restore_in_place(&mut self) {
        let Some(ds) = self.draw.session.as_mut() else { return };
        ds.restore_on_open = true;
        let ds = ds.clone();
        self.ports.notifier.draw(&ds);
    }

    /// Ctrl+Shift+R · Alt+Shift+1: 모니터·배율 규칙(02 §3.6)으로 판정하고 섬으로 알린다.
    pub fn draw_restore(&mut self, session: u64) -> DrawRestoreResult {
        let Some(monitor) = self.active_draw(session).map(|d| d.monitor.clone()) else {
            return DrawRestoreResult { status: KeptCompat::None, kept: None };
        };
        if !self.settings.bool("draw.keep") {
            self.draw.kept = None;
        }
        let status = kept_compat(self.draw.kept.as_ref(), &monitor);
        let kept = if matches!(status, KeptCompat::Same | KeptCompat::Rescaled) { self.draw.kept.clone() } else { None };
        if let Some(ds) = self.draw.session.as_mut() {
            ds.restore_on_open = false;
        }
        if kept.is_some() {
            self.draw.restored = true;
            // 섬의 "이전 판서" 안내를 거둔다
            self.emit_draw_island();
        }
        // 불러옴·배율 맞춤·다른 모니터 문구는 엔진(restoreKept)이 drawIsland 안내로 낸다 — 겹치지 않게 Rust는 "없음"만
        if status == KeptCompat::None {
            self.island(IslandEvent::new(IslandKind::Info, self.t("island.draw.noKept")));
        }
        DrawRestoreResult { status, kept }
    }

    // ───────────── 내보내기 ─────────────

    /// 합성된 원본 바이트(배경 + 판서, base64 아님) → 클립보드 또는 저장 폴더(캡처 저장과 같은 이름 규칙).
    /// 확대 중이어도 늘 **모니터 전체**(물리 해상도 그대로)를 받는다 — 크기가 대상 모니터와 다르면 거절(사용자 결정 2026-11-26).
    pub fn draw_export(&mut self, meta: DrawExportMeta, mut px: Vec<u8>) -> DrawExported {
        let Mode::Draw { session, .. } = self.mode else { return DrawExported::fail("not drawing") };
        if meta.session.is_some_and(|s| s != session) {
            return DrawExported::fail("stale session");
        }
        let Some(bounds) = self.draw.session.as_ref().map(|d| d.monitor.bounds) else { return DrawExported::fail("not drawing") };
        if i64::from(meta.w) != i64::from(bounds.w) || i64::from(meta.h) != i64::from(bounds.h) {
            log::warn!("draw export rejected: {}x{} is not the monitor size {}x{}", meta.w, meta.h, bounds.w, bounds.h);
            return DrawExported::fail("not monitor size");
        }
        if px.len() != meta.byte_len() {
            log::warn!("draw export size mismatch: {} bytes for {}x{}", px.len(), meta.w, meta.h);
            return DrawExported::fail("size mismatch");
        }
        if !meta.bgra {
            rgba_to_bgra(&mut px);
        }
        let (w, h) = (meta.w, meta.h);
        let (tw, th, tpx) = image::thumbnail(w, h, &px, 160, 96);
        let thumb = self.ports.overlay.thumbnail(tw, th, tpx);
        match meta.action {
            ExportAction::Copy => match self.ports.clipboard.set_image(w, h, &px) {
                Ok(()) => {
                    let mut ev = IslandEvent::new(IslandKind::Result, self.t("island.capture.copied")).variant("result").detail(self.t("island.capture.pasteHint"));
                    ev.value = Some(format!("{w}×{h}"));
                    ev.thumb_url = thumb;
                    self.island(ev);
                    DrawExported { ok: true, name: None, error: None }
                }
                Err(e) => {
                    log::warn!("clipboard: {e}");
                    self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.copyError")));
                    DrawExported::fail("clipboard")
                }
            },
            ExportAction::Save => {
                let mut ev = self.save_image(&LastCapture { w, h, bgra: Arc::new(px), app: None, hex: None });
                let ok = ev.kind == IslandKind::Result;
                let name = if ok { ev.detail.clone() } else { None };
                if ok {
                    ev.thumb_url = thumb;
                }
                self.island(ev);
                DrawExported { ok, name, error: (!ok).then(|| "save failed".to_string()) }
            }
        }
    }

    // ───────────── 섬 · 계측 ─────────────

    fn kept_hint(&self) -> Option<String> {
        let ds = self.draw.session.as_ref()?;
        if self.draw.restored || ds.restore_on_open || !matches!(ds.kept_compatible, KeptCompat::Same | KeptCompat::Rescaled) {
            return None;
        }
        let key = self.key_text("d-restore").or_else(|| self.key_text("restore"))?;
        Some(self.tf("island.draw.restoreHint", &[("key", &key)]))
    }

    pub(crate) fn emit_draw_island(&self) {
        let Some(ds) = self.draw.session.as_ref() else { return };
        let hint = self.kept_hint();
        self.island(draw_island_event(ds.board, &self.draw.look, hint.as_deref(), self.lang()));
    }

    /// 판서 섬 한 줄 갱신(색·도구·굵기·배율·쪽) + 안내 한 줄.
    pub fn draw_island(&mut self, patch: DrawIslandPatch) {
        let Mode::Draw { session, .. } = self.mode else { return };
        if patch.session.is_some_and(|s| s != session) {
            return;
        }
        self.draw.look.merge(&patch);
        self.emit_draw_island();
        if let Some(n) = patch.notice.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
            let mut ev = IslandEvent::new(IslandKind::Info, n.chars().take(NOTICE_MAX_CHARS).collect::<String>());
            if patch.notice_action.as_deref() == Some("undo") {
                ev = ev.button(&self.t("island.action.undo"), "draw.undo", None);
            }
            self.island(ev);
        }
    }

    /// 섬 단추(`draw.acceptSuggestion` · `draw.undo`) → 지금 판서 오버레이의 엔진으로. 판서 중이 아니면 무시.
    pub fn draw_command(&mut self, command: &str) -> bool {
        let Mode::Draw { session, .. } = self.mode else { return false };
        if !DRAW_COMMANDS.contains(&command) {
            return false;
        }
        self.ports.notifier.draw_command(session, command);
        true
    }

    /// 펜 지연(M5) 한 줄을 로그에. 쓴 줄을 돌려준다.
    pub fn draw_pen_metrics(&mut self, sample: PenSample) -> Option<String> {
        let line = sample.log_line()?;
        log::info!("{line}");
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ink_domain::geometry::PhysRect;
    use serde_json::json;

    fn mon(id: u32, w: i32, h: i32, scale: f64) -> OverlayMonitor {
        OverlayMonitor { id, bounds: PhysRect::new(0, 0, w, h), work: PhysRect::new(0, 0, w, h), scale, primary: id == 0, frame_url: None }
    }

    fn kept(id: u32, w: u32, h: u32, scale: f64) -> KeptInk {
        KeptInk { monitor_id: id, layout_fingerprint: String::new(), phys_w: w, phys_h: h, scale, board: false, document: json!({}) }
    }

    #[test]
    fn 보관_판서는_같은_모니터_배율_다른_모니터를_가린다() {
        let m = mon(0, 1920, 1080, 1.0);
        assert_eq!(kept_compat(None, &m), KeptCompat::None);
        assert_eq!(kept_compat(Some(&kept(0, 1920, 1080, 1.0)), &m), KeptCompat::Same);
        // 배율만 바뀜(물리 크기 같음) · 같은 비율 해상도 바뀜
        assert_eq!(kept_compat(Some(&kept(0, 1920, 1080, 1.5)), &m), KeptCompat::Rescaled);
        assert_eq!(kept_compat(Some(&kept(0, 3840, 2160, 2.0)), &m), KeptCompat::Rescaled);
        // 같은 id라도 비율이 다르면 옮길 수 없다
        assert_eq!(kept_compat(Some(&kept(0, 1280, 1024, 1.0)), &m), KeptCompat::DifferentMonitor);
        assert_eq!(kept_compat(Some(&kept(1, 1920, 1080, 1.0)), &m), KeptCompat::DifferentMonitor);
        assert_eq!(kept_compat(Some(&kept(0, 0, 0, 1.0)), &m), KeptCompat::DifferentMonitor);
    }

    #[test]
    fn 항목_수는_쪽마다_더하고_모양이_틀리면_0() {
        let doc = json!({ "pages": [ { "items": [{}, {}] }, { "items": [{}] }, {} ] });
        assert_eq!(ink_items(&doc), 3);
        assert_eq!(ink_items(&json!({ "items": [{}] })), 1);
        assert_eq!(ink_items(&json!({ "pages": [] })), 0);
        assert_eq!(ink_items(&json!(null)), 0);
        assert_eq!(ink_items(&json!({ "pages": "x" })), 0);
    }

    #[test]
    fn 내보내기_머리글을_읽고_틀리면_거절() {
        let headers = |pairs: &'static [(&'static str, &'static str)]| move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string());
        let m = DrawExportMeta::parse(headers(&[("x-ink-action", "copy"), ("x-ink-w", "1920"), ("x-ink-h", " 1080 ")])).unwrap();
        assert_eq!(m, DrawExportMeta { action: ExportAction::Copy, w: 1920, h: 1080, session: None, bgra: false });
        assert_eq!(m.byte_len(), 1920 * 1080 * 4);
        let m = DrawExportMeta::parse(headers(&[("x-ink-action", "save"), ("x-ink-w", "2"), ("x-ink-h", "3"), ("x-ink-session", "7"), ("x-ink-format", "bgra")])).unwrap();
        assert_eq!((m.action, m.session, m.bgra), (ExportAction::Save, Some(7), true));
        assert!(DrawExportMeta::parse(headers(&[("x-ink-action", "print"), ("x-ink-w", "2"), ("x-ink-h", "3")])).is_err());
        assert!(DrawExportMeta::parse(headers(&[("x-ink-action", "copy"), ("x-ink-w", "0"), ("x-ink-h", "3")])).is_err());
        assert!(DrawExportMeta::parse(headers(&[("x-ink-action", "copy"), ("x-ink-w", "99999"), ("x-ink-h", "3")])).is_err());
        assert!(DrawExportMeta::parse(headers(&[("x-ink-action", "copy"), ("x-ink-h", "3")])).is_err());
        assert!(DrawExportMeta::parse(headers(&[("x-ink-action", "copy"), ("x-ink-w", "2"), ("x-ink-h", "3"), ("x-ink-format", "argb")])).is_err());
        assert!(DrawExportMeta::parse(headers(&[("x-ink-action", "copy"), ("x-ink-w", "2"), ("x-ink-h", "3"), ("x-ink-session", "x")])).is_err());
    }

    #[test]
    fn rgba는_bgra로_빨강_파랑만_바꾼다() {
        let mut px = vec![1, 2, 3, 4, 10, 20, 30, 40];
        rgba_to_bgra(&mut px);
        assert_eq!(px, vec![3, 2, 1, 4, 30, 20, 10, 40]);
    }

    #[test]
    fn 펜_지연_한_줄과_이상한_값_거르기() {
        let s = PenSample { n: 240, p50: 8.24, p95: 14.6, max: 31.0 };
        assert_eq!(s.log_line().as_deref(), Some("pen latency n=240 p50=8.2 p95=14.6 max=31.0 ms"));
        assert_eq!(PenSample { n: 0, ..s }.log_line(), None);
        assert_eq!(PenSample { p95: f64::NAN, ..s }.log_line(), None);
        assert_eq!(PenSample { max: -1.0, ..s }.log_line(), None);
    }

    #[test]
    fn 진입_계측_한_줄() {
        assert_eq!(entry_line(false, 92.4, 41.2, 3.4, Some(12.6)), "mode enter draw [92 ms] (capture 41, show 3, ui paint 13)");
        assert_eq!(entry_line(true, 30.0, 0.0, 2.0, None), "mode enter board [30 ms] (capture 0, show 2)");
        // 출처: 확대 이어받기(5.3)·결과 섬 [판서로](3e-handoff) — 키 진입 줄은 예전 그대로
        assert_eq!(entry_line_from(DrawSource::Key, 2.0, false, 92.4, 41.2, 3.4, None), "mode enter draw [92 ms] (capture 41, show 3)");
        assert_eq!(
            entry_line_from(DrawSource::Zoom { hide_ms: 17.6, reused: false }, 2.5, false, 61.0, 35.2, 3.0, Some(5.0)),
            "mode enter draw [61 ms] (from zoom, scale 2.5, hide 18, capture 35, show 3, ui paint 5)"
        );
        assert_eq!(
            entry_line_from(DrawSource::Capture { reused: true }, 4.22, false, 12.0, 0.01, 3.0, Some(6.0)),
            "mode enter draw [12 ms] (from capture, scale 4.22, frame reused, capture 0, show 3, ui paint 6)"
        );
        assert_eq!(entry_line_from(DrawSource::Capture { reused: false }, 1.0, false, 80.0, 40.0, 3.0, None), "mode enter draw [80 ms] (from capture, scale 1, capture 40, show 3)");
    }

    #[test]
    fn 판서_섬은_색_도구_굵기_배율_칠판은_바탕과_쪽() {
        let mut look = Look::default();
        let ev = draw_island_event(false, &look, Some("Ctrl+Shift+R 방금 그리기"), Lang::Ko);
        assert_eq!((ev.kind, ev.text.as_str(), ev.variant.as_deref(), ev.mode.as_deref()), (IslandKind::Mode, "그리기 · 펜", Some("draw"), Some("draw")));
        assert_eq!((ev.pen_color.as_deref(), ev.pen_width.as_deref(), ev.value.as_deref()), (Some("R"), Some("4px"), None));
        assert_eq!(ev.detail.as_deref(), Some("Ctrl+Shift+R 방금 그리기"));

        look.merge(&DrawIslandPatch { color: Some("B".into()), tool: Some("marker".into()), width_px: Some(99.0), zoom: Some(1.75), ..Default::default() });
        let ev = draw_island_event(false, &look, None, Lang::Ko);
        assert_eq!((ev.text.as_str(), ev.pen_color.as_deref(), ev.pen_width.as_deref(), ev.value.as_deref()), ("그리기 · 형광펜", Some("B"), Some("30px"), Some("×1.75")));
        let ev_en = draw_island_event(false, &look, None, Lang::En);
        assert_eq!(ev_en.text.as_str(), "Draw · Highlighter");
        // 모르는 값은 버린다
        look.merge(&DrawIslandPatch { color: Some("pink".into()), tool: Some("laser".into()), zoom: Some(2.0), ..Default::default() });
        let ev = draw_island_event(false, &look, None, Lang::Ko);
        assert_eq!((ev.text.as_str(), ev.pen_color.as_deref(), ev.value.as_deref()), ("그리기 · 형광펜", Some("B"), Some("×2.0")));

        let mut board = Look { background: "white".into(), ..Look::default() };
        let ev = draw_island_event(true, &board, None, Lang::Ko);
        assert_eq!((ev.text.as_str(), ev.variant.as_deref(), ev.value.as_deref(), ev.mode.as_deref()), ("칠판 · 흰색", Some("board"), Some("1 / 1"), Some("board")));
        let ev_en = draw_island_event(true, &board, None, Lang::En);
        assert_eq!(ev_en.text.as_str(), "Board · White");
        board.merge(&DrawIslandPatch { background: Some("black".into()), page_index: Some(5), page_count: Some(3), ..Default::default() });
        let ev = draw_island_event(true, &board, None, Lang::Ko);
        assert_eq!((ev.text.as_str(), ev.value.as_deref()), ("칠판 · 검은색", Some("3 / 3")));
        // 판서(Alt+1) 중 W = 흰 칠판 바탕 — 섬은 칠판 모양, 모드 이름은 그대로 draw
        let w = Look { background: "white".into(), ..Look::default() };
        let ev = draw_island_event(false, &w, None, Lang::Ko);
        assert_eq!((ev.text.as_str(), ev.mode.as_deref()), ("칠판 · 흰색", Some("draw")));
    }

    #[test]
    fn 판서_세션과_보관_판서는_camel_case() {
        let ds = DrawSession {
            session: 3,
            board: false,
            monitor: mon(0, 1920, 1080, 1.0),
            layout_fingerprint: "0,0,1920,1080@100".into(),
            initial_view: DrawViewTransform::default(),
            restore_on_open: true,
            has_kept: true,
            kept_compatible: KeptCompat::DifferentMonitor,
            cursor: ink_domain::geometry::PhysPoint { x: 5, y: 6 },
        };
        let v = serde_json::to_value(&ds).unwrap();
        assert_eq!(v["layoutFingerprint"], "0,0,1920,1080@100");
        assert_eq!(v["initialView"], json!({ "s": 1.0, "tx": 0.0, "ty": 0.0 }));
        assert_eq!((v["restoreOnOpen"].clone(), v["hasKept"].clone(), v["keptCompatible"].clone()), (json!(true), json!(true), json!("differentMonitor")));
        assert_eq!(v["monitor"]["bounds"]["w"], 1920);
        let k = serde_json::to_value(kept(1, 10, 20, 1.5)).unwrap();
        assert_eq!(k, json!({ "monitorId": 1, "layoutFingerprint": "", "physW": 10, "physH": 20, "scale": 1.5, "board": false, "document": {} }));
        // UI가 KeptInk 전체를 보내도 document만 쓴다
        let f: DrawFinish = serde_json::from_value(json!({ "session": 2, "keep": true, "kept": { "monitorId": 9, "document": { "items": [1] } } })).unwrap();
        assert_eq!((f.session, f.keep, ink_items(&f.kept.unwrap().document)), (2, true, 1));
        let p: DrawIslandPatch = serde_json::from_value(json!({ "color": "G", "widthPx": 6, "pageCount": 2 })).unwrap();
        assert_eq!((p.color.as_deref(), p.width_px, p.page_count), (Some("G"), Some(6.0), Some(2)));
    }
}
