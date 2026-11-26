//! UI로 나가는 모양 — `packages/ink-client/src/index.js`의 JSDoc 타입과 1:1(camelCase).

use crate::ports::StoreLocation;
use ink_domain::shortcuts::Suggestion;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RowStatus {
    Ok,
    Warn,
    Conflict,
    Off,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutRow {
    pub id: String,
    pub scope: String,
    pub group: String,
    pub label: String,
    pub combo: String,
    pub status: RowStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub later: bool,
    pub windows_only: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictSummary {
    pub count: usize,
    /// 겹친 원인이 모두 한 앱이면 그 이름(S02 맨 위 알림 · R-KEY-5 "끄는 법" 먼저)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    pub combos: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub active: bool,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InkState {
    /// "windows" | "macos"
    pub platform: &'static str,
    pub theme: String,
    pub hotkeys_enabled: bool,
    pub pinned_count: u32,
    /// 캡처 핀 개수(R-CAP-29) — 트레이 "핀 n개 · 모두 닫기"
    pub pins: u32,
    pub recording: Recording,
    pub settings: BTreeMap<String, Value>,
    pub shortcuts: Vec<ShortcutRow>,
    pub conflicts: ConflictSummary,
    pub version: String,
    /// 더 새 스키마·쓸 수 없는 폴더 — 바꿔도 저장하지 않음
    pub readonly: bool,
    pub location: StoreLocation,
    /// 시작할 때 한 번 알려 줄 문구(손상 복구·읽을 수 없던 값)
    pub notices: Vec<String>,
    /// 지금 모드(`{mode:'idle'}` · `{mode:'capture', session, kind, startedMs}`)
    pub mode: ink_domain::mode::Mode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IslandKind {
    Result,
    Problem,
    Info,
    Mode,
    Off,
    /// 모드가 끝남 — 모드 섬을 거둔다
    End,
}

/// 오버레이 한 세션(UI로 나감). 정지 이미지 주소는 어댑터가 채운다.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySession {
    pub session: u64,
    pub kind: ink_domain::snapshot::CaptureKind,
    pub monitors: Vec<OverlayMonitor>,
    /// 단축키 순간 커서(물리 px, 가상 화면)
    pub cursor: ink_domain::geometry::PhysPoint,
    /// 정지 화면(50% 어둡게 + 강조만 밝게)을 네이티브 창이 이미 그리고 있음 — UI는 이미지·어둡게를 깔지 않고
    /// 테두리·꺾쇠·확대경·크기만 그린다(이미지는 확대경용으로 늦게 읽음).
    pub native_frame: bool,
    /// 첫 강조(가상 화면 물리 px) — 단계 2는 커서 아래 창, 단계 3에서 요소까지
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<ink_domain::geometry::PhysRect>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayMonitor {
    pub id: u32,
    pub bounds: ink_domain::geometry::PhysRect,
    pub work: ink_domain::geometry::PhysRect,
    pub scale: f64,
    pub primary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IslandButton {
    pub label: String,
    /// `InkClient.run`/`undo`로 되돌아올 동작 이름
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IslandHint {
    pub key: String,
    pub label: String,
}

/// 고르기 상태(UI로) — `SelectionView` + 세션.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureView {
    pub session: u64,
    #[serde(flatten)]
    pub selection: ink_domain::selection::SelectionView,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IslandEvent {
    pub kind: IslandKind,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<IslandButton>,
    /// 섬 모양(region·window·record·security …) — 없으면 kind로 정함
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// 모드 섬의 모드 이름(사용 횟수 세기 — 키 안내 둘째 줄은 처음 3번만)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// 고정폭 값(크기 `718×40` 등)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// 둘째 줄 키 안내
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<IslandHint>,
    /// 결과 섬 썸네일 주소
    #[serde(rename = "thumbUrl", skip_serializing_if = "Option::is_none")]
    pub thumb_url: Option<String>,
    /// 단추 옆 키캡(예: "Ctrl+S")
    #[serde(rename = "buttonKey", skip_serializing_if = "Option::is_none")]
    pub button_key: Option<String>,
    /// 추천 칩 하나(02 §1.6 — 지금 할 수 있을 때만)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chip: Option<IslandButton>,
    /// 판서 섬(`variant: "draw"`)의 지금 색 점(R·Y·G·O·B·L·W·K)
    #[serde(rename = "penColor", skip_serializing_if = "Option::is_none")]
    pub pen_color: Option<String>,
    /// 판서 섬의 굵기(`4px`)
    #[serde(rename = "penWidth", skip_serializing_if = "Option::is_none")]
    pub pen_width: Option<String>,
    /// 주 단추 옆 보조 단추(결과 섬 [판서로] → `capture.toDraw`, B §2.3) — 누르면 `InkClient.run(action)`
    #[serde(rename = "secondaryButton", skip_serializing_if = "Option::is_none")]
    pub secondary_button: Option<IslandButton>,
    /// 보조 단추 옆 키캡(예: "Alt+1") — 단추가 넘치면 UI가 생략(사용자 결정 #4)
    #[serde(rename = "secondaryButtonKey", skip_serializing_if = "Option::is_none")]
    pub secondary_button_key: Option<String>,
}

impl IslandEvent {
    pub fn new(kind: IslandKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            detail: None,
            button: None,
            variant: None,
            mode: None,
            value: None,
            hints: Vec::new(),
            thumb_url: None,
            button_key: None,
            chip: None,
            pen_color: None,
            pen_width: None,
            secondary_button: None,
            secondary_button_key: None,
        }
    }

    pub fn button(mut self, label: &str, action: &str, key: Option<&str>) -> Self {
        self.button = Some(IslandButton { label: label.into(), action: action.into() });
        self.button_key = key.map(String::from);
        self
    }

    /// 주 단추 옆 보조 단추(결과 섬 [판서로]).
    pub fn secondary(mut self, label: &str, action: &str, key: Option<&str>) -> Self {
        self.secondary_button = Some(IslandButton { label: label.into(), action: action.into() });
        self.secondary_button_key = key.filter(|k| !k.is_empty()).map(String::from);
        self
    }

    pub fn variant(mut self, v: &str) -> Self {
        self.variant = Some(v.into());
        self
    }

    pub fn hints(mut self, list: &[(&str, &str)]) -> Self {
        self.hints = list.iter().map(|(k, l)| IslandHint { key: (*k).into(), label: (*l).into() }).collect();
        self
    }

    pub fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = Some(d.into());
        self
    }

    pub fn undo_button(mut self) -> Self {
        self.button = Some(IslandButton { label: "되돌리기".into(), action: "undo".into() });
        self
    }
}

// ───────────── 판서·칠판(A §2.5) ─────────────

/// 확대·이동(`view = {s, tx, ty}`, A §2.1). `s` 1~8, `tx`·`ty` = 화면 CSS px에서 문서 원점 자리(≤ 0).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawViewTransform {
    pub s: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Default for DrawViewTransform {
    fn default() -> Self {
        DrawViewTransform { s: 1.0, tx: 0.0, ty: 0.0 }
    }
}

/// 보관 판서를 지금 모니터에 불러올 수 있는지(02 §3.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeptCompat {
    /// 보관한 판서가 없음
    None,
    /// 같은 모니터·같은 크기·같은 배율 — 그대로
    Same,
    /// 같은 모니터인데 배율(또는 같은 비율의 해상도)만 바뀜 — 문서 크기 비율로 옮김
    Rescaled,
    /// 다른 모니터 — 불러올 수 없음
    DifferentMonitor,
}

/// 판서 한 번(UI로 `screen-ink://draw`). 오버레이 창 `ink-overlay-<monitor.id>` 하나만 쓴다.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawSession {
    pub session: u64,
    /// false = Alt+1 판서(정지 화면 위), true = Alt+3 칠판(빈 바탕, `monitor.frameUrl` 없음)
    pub board: bool,
    /// 대상 모니터(bounds·scale·frameUrl). 판서는 웹뷰가 frameUrl을 직접 그린다(네이티브 정지 창 없음)
    pub monitor: OverlayMonitor,
    /// 진입 순간 모니터 구성 지문(`MonitorLayout::fingerprint`)
    pub layout_fingerprint: String,
    /// 시작 확대·이동 — 기본 {s:1,tx:0,ty:0}(실시간 확대·[판서로]에서 넘어오면 그 값)
    pub initial_view: DrawViewTransform,
    /// Alt+Shift+1(직전 판서 바로 열기) — 열자마자 `drawRestore`. 판서 중 다시 누르면 같은 세션이 true로 한 번 더 온다
    pub restore_on_open: bool,
    /// 보관한 판서가 있음(섬 "Ctrl+Shift+R 이전 판서")
    pub has_kept: bool,
    pub kept_compatible: KeptCompat,
    /// 단축키 순간 커서(물리 px, 가상 화면) — 첫 이동 전 펜 고리 자리
    pub cursor: ink_domain::geometry::PhysPoint,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetShortcutResult {
    pub ok: bool,
    pub status: RowStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub suggestions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    pub status: RowStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub state: InkState,
}

/// 실시간 확대 상태(`screen-ink://zoom`, R-ZOOM-6 · 04 S13) — 확대 HUD(커서 주황 후광 + 위치 지도)가 그린다.
/// 확대가 끝나면 이벤트 값은 `null`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveZoomState {
    pub session: u64,
    pub monitor_id: u32,
    /// "모니터 1"(배치 순서) — 창 제목·장치 이름은 쓰지 않는다
    pub monitor_label: String,
    pub scale: f64,
    /// 섬·배지 글자(`×2.0`)
    pub scale_label: String,
    /// 위치 지도용 0..1 사각형(원본 / 모니터)
    pub viewport: ink_domain::zoom::NormRect,
    /// 확대 화면 위 커서(그 모니터 HUD 창 안 CSS px) — 44 px 주황 후광 자리. 커서가 확대 화면 밖이면 없음
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_css: Option<ink_domain::geometry::LogicalPoint>,
    /// 설정 `zoom.minimap`
    pub show_minimap: bool,
}
