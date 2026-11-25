//! 포트 — 어댑터(Windows·맥·가짜·Tauri)가 채운다. 판정은 하지 않고 사실만 돌려준다.

use crate::state::{InkState, IslandEvent};
use ink_domain::shortcuts::LiveBinding;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum HotkeyError {
    /// 운영체제가 거절 — 다른 앱이 먼저 등록함
    #[error("다른 앱이 먼저 쓰고 있어요")]
    Taken,
    #[error("이 키는 등록할 수 없어요")]
    Unsupported,
    #[error("{0}")]
    Other(String),
}

/// 전역 단축키(03 §3 `HotkeyRegistry`). 눌림은 어댑터가 `InkService::on_hotkey(slot)`로 넘긴다.
pub trait HotkeyRegistry: Send {
    /// `accelerator` = `shortcuts::to_accelerator` 결과. `slot`은 눌렸을 때 돌려줄 칸 ID.
    fn register(&mut self, slot: &str, accelerator: &str) -> Result<(), HotkeyError>;
    fn unregister(&mut self, accelerator: &str);
}

/// 이 PC 사정(03 §3 `ProcessProbe` + PowerToys 설정 읽기). 창 제목은 보지 않는다.
pub trait SystemProbe: Send {
    /// 실행 중인 프로세스의 실행 파일 이름(소문자).
    fn running_apps(&self) -> Vec<String>;
    /// 다른 앱 설정 파일에서 읽은, 지금 잡혀 있는 전역 키(저수준 훅이라 등록 시험으로 안 보이는 것).
    /// 그 앱이 실행 중이 아니면 `Some(빈 목록)`, 읽을 수 없으면 `None`.
    fn live_bindings(&self) -> Option<Vec<LiveBinding>>;
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StoreError {
    #[error("설정을 저장하지 못했어요")]
    Io(String),
}

#[derive(Clone, Debug, Default)]
pub struct Loaded {
    /// 파일 전체(`{schemaVersion, sections}`). 첫 실행이면 `None`.
    pub file: Option<Value>,
    /// 손상 복구 등 한 번 알려 줄 문구
    pub notice: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreLocation {
    pub path: String,
    pub portable: bool,
    /// 폴더에 쓸 수 없음(CD·잠긴 USB) — 메모리로만 실행
    pub read_only: bool,
}

/// 설정 파일(05 공통 규칙: 원자적 쓰기·백업 교대·손상 복구).
pub trait SettingsStore: Send {
    fn load(&mut self) -> Loaded;
    fn save_section(&mut self, name: &str, data: &Value) -> Result<(), StoreError>;
    fn location(&self) -> StoreLocation;
}

/// UI로 알리기(Tauri 이벤트 `screen-ink://state`·`screen-ink://island`).
pub trait Notifier: Send + Sync {
    fn state(&self, state: &InkState);
    fn island(&self, event: &IslandEvent);
}

pub struct Ports {
    pub hotkeys: Box<dyn HotkeyRegistry>,
    pub probe: Box<dyn SystemProbe>,
    pub store: Box<dyn SettingsStore>,
    pub notifier: Box<dyn Notifier>,
}
