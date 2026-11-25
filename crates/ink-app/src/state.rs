//! UI로 나가는 모양 — `packages/ink-client/src/index.js`의 JSDoc 타입과 1:1(camelCase).

use crate::ports::StoreLocation;
use ink_domain::shortcuts::Suggestion;
use serde::Serialize;
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
}

impl IslandEvent {
    pub fn new(kind: IslandKind, text: impl Into<String>) -> Self {
        Self { kind, text: text.into(), detail: None, button: None, variant: None, mode: None, value: None, hints: Vec::new() }
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
