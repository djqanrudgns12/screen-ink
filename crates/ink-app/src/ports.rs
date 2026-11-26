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

/// `ScreenSource::capture` 오류가 이 말로 시작하면 운영체제 권한이 없다는 뜻(맥 화면 기록, M10).
/// 서비스는 "화면을 가져오지 못했어요" 대신 권한 안내를 띄운다.
pub const NO_SCREEN_PERMISSION: &str = "permission:screen-recording";

/// 정지 캡처(03 §3 `ScreenSource`).
pub trait ScreenSource: Send {
    fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String>;
    /// 같은 순간의 커서 모양(` 키·"커서도 찍기"로 결과에 합성).
    fn cursor_sprite(&mut self) -> Option<ink_domain::image::CursorSprite> {
        None
    }
}

/// Z 순서 창 목록(03 §3 `WindowCatalog`). 창 제목은 읽지 않는다. 캡처와 나란히 찍도록 `Sync`.
pub trait WindowCatalog: Send + Sync {
    fn windows(&self) -> Vec<RawWindow>;
    /// H 근거: 그 창의 보이는 자식 창 사각형(깊이 4, 물리 px). 다른 앱에 메시지를 보내지 않는 질의만.
    fn children(&self, _window: u64) -> Vec<ink_domain::geometry::PhysRect> {
        Vec::new()
    }
}

/// 모니터 배치·배율·커서(03 §3 `MonitorProbe`).
pub trait MonitorProbe: Send {
    fn layout(&self) -> MonitorLayout;
    fn cursor(&self) -> PhysPoint;
    /// WASD 1 px 이동(S07).
    fn set_cursor(&self, _p: PhysPoint) {}
    /// Alt를 누르고 있는지(드래그 자석 끔).
    fn alt_down(&self) -> bool {
        false
    }
}

/// A 근거(UIA): 창 하나의 하위 트리를 일꾼 스레드에서 색인해 두고, 커서 질의는 메모리에서만(08 §4.1-3, D17·D18).
pub trait ElementSource: Send {
    /// 색인을 부탁(이미 있거나 진행 중이면 무시). 끝나면 어댑터가 서비스의 `on_elements_ready`를 부른다.
    fn request(&self, window: u64, exe: &str);
    /// 색인에서 그 점을 담는 요소(없으면 빈 목록 — 아직 색인 중이어도 기다리지 않는다).
    fn elements_at(&self, window: u64, p: PhysPoint) -> Vec<ink_domain::recog::ElementCand>;
    /// 그 창 색인의 모든 사각형(드래그 자석 후보).
    fn rects(&self, window: u64) -> Vec<ink_domain::geometry::PhysRect> {
        self.elements(window).into_iter().map(|e| e.rect).collect()
    }
    /// 그 창 색인 전부(시험 사례 뜨기). 색인 전이면 빈 목록.
    fn elements(&self, _window: u64) -> Vec<ink_domain::recog::ElementCand> {
        Vec::new()
    }
    /// 모드가 끝나면 색인을 버린다(창이 바뀌었을 수 있음).
    fn clear(&self);
}

/// V 근거(정지 이미지에서 사각형 찾기). 좌표는 가상 화면 물리 px.
pub trait VisionSource: Send {
    /// 모니터 이미지 전처리를 시작(배경). 커서 모니터부터.
    fn prepare(&mut self, frames: &[Frame], layout: &MonitorLayout, cursor: PhysPoint);
    fn query(&self, _p: PhysPoint) -> Vec<ink_domain::recog::VisionCand> {
        Vec::new()
    }
    /// 경계 지지율(전처리 전이면 None).
    fn support(&self, _r: ink_domain::geometry::PhysRect) -> Option<f32> {
        None
    }
    /// ±3 px 경계 맞춤.
    fn snap(&self, r: ink_domain::geometry::PhysRect) -> ink_domain::geometry::PhysRect {
        r
    }
    /// 드래그 자석용 근처 곧은 경계(세로 x들, 가로 y들).
    fn edges_near(&self, _p: PhysPoint) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }
    fn clear(&mut self);
}

/// 비어 있는 V — 엔진이 없거나 끈 경우.
pub struct NoVision;

impl VisionSource for NoVision {
    fn prepare(&mut self, _frames: &[Frame], _layout: &MonitorLayout, _cursor: PhysPoint) {}
    fn clear(&mut self) {}
}

/// 캡처 핀(R-CAP-29): 고른 범위를 화면 위에 떠 있는 그림 창으로. 창·확대·끌기·투명도는 어댑터,
/// 목록과 그림(복사·저장용)은 서비스가 쥔다. 그림은 Arc라 복사 없이 함께 쓴다.
pub trait PinHost: Send {
    /// `rect`(가상 화면 물리 px, 원래 자리·같은 크기)에 그림 창을 띄운다.
    fn open(&mut self, id: u64, rect: ink_domain::geometry::PhysRect, w: u32, h: u32, bgra: std::sync::Arc<Vec<u8>>) -> Result<(), String>;
    fn close(&mut self, id: u64);
}

/// 핀 창을 못 만드는 곳(시험·미지원).
pub struct NoPins;

impl PinHost for NoPins {
    fn open(&mut self, _id: u64, _rect: ink_domain::geometry::PhysRect, _w: u32, _h: u32, _bgra: std::sync::Arc<Vec<u8>>) -> Result<(), String> {
        Err("pins not supported".into())
    }
    fn close(&mut self, _id: u64) {}
}

/// 클립보드(03 §3): 이미지는 CF_DIB + PNG, 글은 유니코드. 잠겨 있으면 어댑터가 3번 다시 해 본다(50 ms).
pub trait Clipboard: Send {
    fn set_image(&mut self, w: u32, h: u32, bgra: &[u8]) -> Result<(), String>;
    fn set_text(&mut self, text: &str) -> Result<(), String>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedFile {
    /// 화면에 보일 파일 이름(경로 없이)
    pub name: String,
    pub path: String,
    /// 설정 폴더가 없어 사진 폴더로 바꿔 저장함(R-OUT-2)
    pub fell_back: bool,
}

/// 저장 폴더(03 §3 `FileSink`): 폴더가 없으면 사진 폴더로, 같은 이름이면 ` (2)`, PNG(알파 없음).
pub trait FileSink: Send {
    fn save_png(&mut self, folder: &str, stem: &str, w: u32, h: u32, bgra: &[u8]) -> Result<SavedFile, String>;
    /// 인식 시험 사례 쓰기(개발용, `case::Case`): `dir/<id>.json` + 모니터별 PNG(`case.monitors[].frame`). 쓴 JSON 경로.
    fn write_case(&mut self, _dir: &str, _case: &crate::case::Case, _frames: &[Frame]) -> Result<String, String> {
        Err("unsupported".into())
    }
}

/// 시계: 이름 규칙용 지역 시각과 기록용 유닉스 초.
pub trait Clock: Send {
    fn local(&self) -> ink_domain::naming::NameContext;
    fn unix(&self) -> u64;
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
    /// 결과 섬 썸네일을 쥐고 주소를 돌려준다(작은 BGRA).
    fn thumbnail(&mut self, _w: u32, _h: u32, _bgra: Vec<u8>) -> Option<String> {
        None
    }
    /// 확정 직후 고른 자리에 주황 잔상 0.4초(R-CAP-26). 클릭 통과·비포커스로 띄우고 400 ms 뒤 숨긴다.
    /// 기본 구현은 no-op(가짜·미지원 어댑터 호환).
    fn flash_afterimage(&mut self, _session: u64, _rect: ink_domain::geometry::PhysRect, _layout: &MonitorLayout) {}
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
    /// 고르기 상태(`screen-ink://capture`) — 모든 오버레이·섬이 같은 것을 그린다.
    fn capture(&self, _view: &crate::state::CaptureView) {}
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
    pub elements: Box<dyn ElementSource>,
    pub vision: Box<dyn VisionSource>,
    pub clipboard: Box<dyn Clipboard>,
    pub files: Box<dyn FileSink>,
    pub pins: Box<dyn PinHost>,
    pub clock: Box<dyn Clock>,
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
