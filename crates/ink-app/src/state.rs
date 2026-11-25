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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IslandKind {
    Result,
    Problem,
    Info,
    Mode,
    Off,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IslandButton {
    pub label: String,
    /// `InkClient.run`/`undo`로 되돌아올 동작 이름
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IslandEvent {
    pub kind: IslandKind,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<IslandButton>,
}

impl IslandEvent {
    pub fn new(kind: IslandKind, text: impl Into<String>) -> Self {
        Self { kind, text: text.into(), detail: None, button: None }
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
