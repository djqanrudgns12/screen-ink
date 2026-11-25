//! 포트 — 어댑터(Windows·맥·가짜·Tauri)가 채운다. 판정은 하지 않고 사실만 돌려준다.

use crate::state::{InkState, IslandEvent, OverlaySession};
use ink_domain::geometry::{MonitorLayout, PhysPoint};
use ink_domain::shortcuts::LiveBinding;
use ink_domain::snapshot::RawWindow;
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
    /// UAC·잠금 화면 같은 보안 데스크톱이 앞에 있음(R-SYS-3: 단축키를 조용히 무시).
    fn secure_desktop(&self) -> bool {
        false
    }
}

/// 모니터별 정지 이미지 한 장. `bgra` = 헤더 없는 BGRA(위→아래, 물리 px). 여러 곳(정지 화면 창·
/// `ink://frame/…`·자르기)이 복사 없이 함께 쓰도록 `Arc`. base64로 바꾸지 않는다(03 §4 프레임 전송).
#[derive(Clone, Debug)]
pub struct Frame {
    pub monitor_id: u32,
    pub width: u32,
    pub height: u32,
    pub bgra: std::sync::Arc<Vec<u8>>,
}

/// 정지 캡처(03 §3 `ScreenSource`).
pub trait ScreenSource: Send {
    fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String>;
}

/// Z 순서 창 목록(03 §3 `WindowCatalog`). 창 제목은 읽지 않는다. 캡처와 나란히 찍도록 `Sync`.
pub trait WindowCatalog: Send + Sync {
    fn windows(&self) -> Vec<RawWindow>;
}

/// 모니터 배치·배율·커서(03 §3 `MonitorProbe`).
pub trait MonitorProbe: Send {
    fn layout(&self) -> MonitorLayout;
    fn cursor(&self) -> PhysPoint;
}

/// 모니터별 오버레이 창(03 §3 `OverlayHost`). 미리 만들어 숨겨 두고, 보일 때마다 투명도를 다시 지정한다(R-SYS-6).
pub trait OverlayHost: Send {
    /// 배치가 바뀌었거나 깨어났을 때 창을 모니터에 다시 맞춘다.
    fn prepare(&mut self, layout: &MonitorLayout);
    /// 정지 이미지를 넘겨받아 오버레이를 보인다. 이미지는 어댑터가 쥐고 모드가 끝나면 버린다.
    /// 정지 화면이 네이티브 창에 그려졌으면 그 순간을 돌려준다(진입 시간의 끝점). 웹뷰만이면 `None`.
    fn show(&mut self, session: &OverlaySession, frames: Vec<Frame>) -> Result<Option<std::time::Instant>, String>;
    /// 강조(어둡게 한 바탕에서 밝게 뚫을 사각형, 가상 화면 물리 px)를 옮긴다.
    fn highlight(&mut self, session: u64, rect: Option<ink_domain::geometry::PhysRect>);
    fn hide(&mut self, session: u64);
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
    pub screen: Box<dyn ScreenSource>,
    pub windows: Box<dyn WindowCatalog>,
    pub monitors: Box<dyn MonitorProbe>,
    pub overlay: Box<dyn OverlayHost>,
}

/// 시스템 사건(03 §3 `SystemEvents`, 02 §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemEvent {
    Lock,
    Unlock,
    Suspend,
    Resume,
    /// 모니터 연결·해제·배치·배율 변경
    DisplayChange,
    /// 작업 표시줄 밝기 등 테마
    ThemeChange,
}
