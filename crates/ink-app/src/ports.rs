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
    /// 지금 활성(전경) 창 핸들 — `Shift+F2`·F2 두 번(R-WIN-2).
    fn foreground_window(&self) -> Option<u64> {
        None
    }
    /// 맥 권한 `(화면 기록, 손쉬운 사용)`(M10, 묻지 않고 확인만). 권한 개념이 없는 플랫폼은 None.
    fn permissions(&self) -> Option<(bool, bool)> {
        None
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
    /// F2 고르기 중 커서 아래 창이 정해졌다 — 창 캡처 준비(item·pool·session)를 미리 해 둔다. `None` = 준비한 것 버리기.
    fn prepare_window(&mut self, _window: Option<u64>) {}
    /// F2 창 단독 캡처(가려진 부분 포함). 첫 프레임 시간 제한·보호 화면 판정(`windowshot::after_primary`)은 어댑터가 한다.
    /// 보호 화면이면 `WINDOW_PROTECTED`로 시작하는 오류. 그 밖의 실패·검정은 서비스가 정지 스냅숏 자르기로 대체한다.
    fn capture_window(&mut self, _window: u64, _bounds: ink_domain::geometry::PhysRect) -> Result<WindowShot, String> {
        Err("window capture not supported".into())
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
    /// 판서·칠판(A §2.5): `session.monitor`의 오버레이 창 하나를 보이고 키를 받게 한다. 판서는 확대·이동과 같은
    /// 변환으로 배경을 움직여야 해서 네이티브 정지 창을 쓰지 않고, 웹뷰가 돌려준 주소의 이미지를 직접 그린다.
    /// `frame`이 없으면(칠판) 주소도 없음. 끝은 `hide(session)`. 못 하는 곳은 실패로 알린다(조용히 빈 모드가 되지 않게).
    fn show_draw(&mut self, _session: &crate::state::DrawSession, _frame: Option<Frame>) -> Result<Option<String>, String> {
        Err("draw overlay not supported".into())
    }
    /// 확정 직후 고른 자리에 주황 잔상 0.4초(R-CAP-26). 클릭 통과·비포커스로 띄우고 400 ms 뒤 숨긴다.
    /// 기본 구현은 no-op(가짜·미지원 어댑터 호환).
    fn flash_afterimage(&mut self, _session: u64, _rect: ink_domain::geometry::PhysRect, _layout: &MonitorLayout) {}
    /// 캡처 주석(4.9): 주석이 있는 범위의 결과를 내기 전에 오버레이에 주석 층을 부탁한다
    /// (`screen-ink://capture-commit`). 오버레이는 `InkService::capture_commit_annotated`로 답한다.
    /// 부탁을 보낼 수 없으면(기본 구현) `false` → 서비스는 주석 없이 지금 경로로 끝낸다.
    fn request_commit(&mut self, _request: &crate::capture::CommitRequest) -> bool {
        false
    }
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
    /// 판서 세션(`screen-ink://draw`) — 상태(`mode: draw`)보다 먼저 보내 오버레이가 세션을 쥔 채 화면을 바꾼다.
    fn draw(&self, _session: &crate::state::DrawSession) {}
    /// 섬 단추 → 판서 엔진(`screen-ink://draw-command`): `acceptSuggestion` · `undo`. 섬 창과 오버레이 창이 달라 서비스가 이어 준다.
    fn draw_command(&self, _session: u64, _command: &str) {}
    /// 실시간 확대 상태(`screen-ink://zoom`) — 확대 HUD(후광·위치 지도)가 그린다. 확대가 끝나면 `None`.
    fn zoom(&self, _state: Option<&crate::state::LiveZoomState>) {}
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

/// `Magnifier::start` 오류가 이 말로 시작하면 이 운영체제·빌드에는 실시간 확대가 없다는 뜻(맥 1차·가짜 기본).
pub const ZOOM_UNSUPPORTED: &str = "unsupported:live-zoom";

/// 실시간 확대(03 §3 `Magnifier`, R-ZOOM-1·4): 모니터를 덮는 돋보기 창을 띄우고 원본 사각형·배율만 바꾼다.
/// 판단(따라가기·단계·끝내기)은 서비스가 한다. 자기 창(섬·HUD·팝오버)을 돋보기 원본에서 빼는 일은 어댑터가 안다.
pub trait Magnifier: Send {
    /// `monitor`(가상 화면 물리 px) 전체를 덮어 `source`를 `scale`배로 비춘다. 첫 그림이 나올 때까지 기다린다.
    fn start(&mut self, monitor: ink_domain::geometry::PhysRect, source: ink_domain::geometry::PhysRect, scale: f64) -> Result<(), String>;
    /// 틱마다 원본·배율(같은 값이어도 부른다 — 어댑터의 멈춤 감시 신호). 돋보기가 스스로 멈췄으면 `Err`.
    fn update(&mut self, source: ink_domain::geometry::PhysRect, scale: f64) -> Result<(), String>;
    fn stop(&mut self);
    /// `stop` 뒤 곧바로 정지 캡처를 할 때(5.3 판서 이어받기·확대 중 캡처 키) 숨긴 돋보기가 화면에서 실제로
    /// 사라질 때까지(화면 갱신 한 번) 기다린다 — 확대 화면이 캡처에 찍히지 않게.
    /// 기본 = 기다리지 않음(가짜·미지원, 또는 `stop`이 이미 기다리는 어댑터 — Windows `WinMagnifier`는 숨긴 뒤 `DwmFlush`).
    fn wait_hidden(&mut self) {}
}

/// 실시간 확대가 없는 곳(맥 1차·시험 기본값). 서비스는 "아직 쓸 수 없어요" 섬을 띄운다.
pub struct NoMagnifier;

impl Magnifier for NoMagnifier {
    fn start(&mut self, _monitor: ink_domain::geometry::PhysRect, _source: ink_domain::geometry::PhysRect, _scale: f64) -> Result<(), String> {
        Err(ZOOM_UNSUPPORTED.into())
    }

    fn update(&mut self, _source: ink_domain::geometry::PhysRect, _scale: f64) -> Result<(), String> {
        Err(ZOOM_UNSUPPORTED.into())
    }

    fn stop(&mut self) {}
}

/// 패키지 모드(MSIX) 런타임 사실 읽기(D3 · 11-distribution §5).
/// 어댑터는 OS 사실만 돌려주고, 정책 판정은 `ink_domain::package`가 한다.
pub trait PackageRuntime: Send + Sync {
    fn channel(&self) -> ink_domain::package::InstallChannel {
        ink_domain::package::InstallChannel::Unpackaged
    }

    /// 패키지 모드일 때만 `Some(PackageFullName)`.
    fn package_full_name(&self) -> Option<String> {
        None
    }

    /// `.appinstaller`로 설치된 직접 배포판일 때만 `Some(uri)`.
    fn app_installer_uri(&self) -> Option<String> {
        None
    }

    /// 발표·전체 화면 앱이 앞에 있는지(`SHQueryUserNotificationState`).
    fn fullscreen_app_active(&self) -> bool {
        false
    }

    /// 업데이트 적용 직후 자동 재시작 등록(`RegisterApplicationRestart`).
    fn register_restart(&self, _args: &str) -> Result<(), String> {
        Ok(())
    }
}

/// 비패키지(NSIS·포터블)·맥·시험용 기본 패키지 런타임.
pub struct UnpackagedRuntime;

impl PackageRuntime for UnpackagedRuntime {}

/// 로그인 자동 실행 제어 포트(D3: 비패키지 Run 키 / MSIX StartupTask 분기).
pub trait AutostartControl: Send {
    fn backend(&self) -> ink_domain::package::AutostartBackend {
        ink_domain::package::AutostartBackend::RegistryRun
    }

    fn is_enabled(&self) -> Result<bool, String> {
        Ok(false)
    }

    fn set_enabled(&mut self, enabled: bool) -> Result<ink_domain::package::AutostartNotice, String> {
        Ok(ink_domain::package::AutostartNotice {
            effective_enabled: enabled,
            blocked_by_user: false,
            blocked_by_policy: false,
            guidance_ko: None,
        })
    }
}

/// 개발 빌드·미지원 환경·시험용 기본 자동 실행 제어.
pub struct NoopAutostart;

impl AutostartControl for NoopAutostart {}

/// 사전 다운로드까지 끝난 업데이트 정보(D10 · H-distribution §2.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadyUpdate {
    pub version: String,
    /// `"store"` | `"direct"`
    pub channel: &'static str,
}

/// 운영체제 API 기반 업데이터 포트(D3·D10: 앱 프로세스 네트워크 0 원칙 유지).
pub trait Updater: Send {
    /// OS API로 업데이트 확인 + 백그라운드 사전 다운로드 완료 여부 확인.
    /// 이미 준비 완료된 새 버전이 있으면 `Some(ReadyUpdate)`. 실패 시 `Err`(무시, 앱 동작 영향 0).
    fn check_and_stage(&mut self) -> Result<Option<ReadyUpdate>, String> {
        Ok(None)
    }

    /// 클릭 한 번 설치 + `RegisterApplicationRestart` 자동 재시작.
    fn install_and_restart(&mut self) -> Result<(), String> {
        Err("updater not available".into())
    }
}

/// 1차 비패키지 빌드(NSIS·포터블)·맥·시험용 기본 업데이터.
pub struct NoopUpdater;

impl Updater for NoopUpdater {}
// ───────────── F2 창 캡처 · F3 녹화(단계 6) ─────────────

/// `ScreenSource::capture_window` 오류가 이 말로 시작하면 보호 화면(표시 선호도·DRM) — "이 창은 가져올 수 없어요".
pub const WINDOW_PROTECTED: &str = "window:protected";

/// 창 한 장(BGRA, 위→아래, 물리 px).
#[derive(Clone, Debug)]
pub struct WindowShot {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    /// 디버그 로그용 방식 이름("wgc" …) — 창 제목은 담지 않는다
    pub method: &'static str,
    /// Win11 둥근 모서리 반지름(물리 px, 0 = 각짐) — 서비스가 알파로 깎는다
    pub corner_radius: u32,
}

/// `Recorder::start` 오류가 이 말로 시작하면 이 플랫폼에는 아직 녹화가 없다.
pub const REC_UNSUPPORTED: &str = "recording not supported";
/// 녹화하던 창이 닫혀 스스로 멈춤(`Recorder::take_failure`).
pub const REC_WINDOW_CLOSED: &str = "rec:window-closed";
/// 인코더를 만들지 못해 녹화가 시작되지 못함(`Recorder::take_failure`) — 저장할 것이 없다.
pub const REC_ENCODER_FAILED: &str = "rec:encoder";

#[derive(Clone, Debug)]
pub struct RecStartOpts {
    pub target: ink_domain::recording::RecTarget,
    /// `%TEMP%\shookshok\rec-<id>.mp4` — 마무리가 끝나야 저장 폴더로 옮긴다
    pub temp_path: std::path::PathBuf,
    /// 30 | 60
    pub fps: u32,
    /// 목표 비트레이트(bps, `recording::bitrate`)
    pub bitrate: u32,
    /// `None` = 마이크 끔, `Some("")` = 기본 장치
    pub mic_device: Option<String>,
    pub capture_cursor: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecStats {
    pub elapsed_ms: u64,
    pub bytes: u64,
    pub frames: u64,
    /// 영상 크기(짝수). 시작 전이면 0
    pub width: u32,
    pub height: u32,
}

/// 화면 녹화(03 §3 `Recorder`, 05 §8). 파일 옮기기·남은 녹화 찾기도 여기 — 임시 폴더는 어댑터가 안다.
pub trait Recorder: Send {
    /// F3 고르기에 들어갈 때 — 시간이 드는 준비(MF 시작·장치)를 녹화 시작 전에 끝낸다.
    fn prepare(&mut self) {}
    fn start(&mut self, opts: RecStartOpts) -> Result<(), String>;
    fn stats(&self) -> RecStats;
    /// 정상 마무리(인코더 비우기 + moov). 시작 1초 안이어도 최소 1프레임을 넣어 손상 0(R-REC-1).
    /// 실패하면 임시 파일은 남겨 둔다(다음 시작 때 복구 제안).
    fn stop(&mut self) -> Result<RecStats, String>;
    /// 버리기 — 인코더를 멈추고 임시 파일을 지운다.
    fn abort(&mut self);
    /// 녹화가 스스로 멈춘 이유(창 닫힘 `REC_WINDOW_CLOSED`·장치 오류). 한 번만 돌려준다.
    fn take_failure(&mut self) -> Option<String> {
        None
    }
    /// 마이크를 함께 녹음할 수 있는지(R-REC-3). 아니면 서비스가 마이크를 끄고 한 번 알린다.
    fn mic_supported(&self) -> bool {
        false
    }
    /// 마이크 장치 목록 `[(id, 이름)]`(설정 창).
    fn audio_inputs(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    /// 임시 녹화 폴더(`%TEMP%\shookshok`). 없으면 녹화를 시작하지 않는다.
    fn temp_dir(&self) -> Option<std::path::PathBuf> {
        None
    }
    /// 끝나지 않은 녹화(임시 폴더의 `rec-*.mp4`, 지금 녹화 중인 것 빼고).
    fn leftovers(&self) -> Vec<ink_domain::recording::Leftover> {
        Vec::new()
    }
    /// 저장 폴더(빈 값 = 기본 폴더)의 남은 바이트.
    fn free_bytes(&self, _folder: &str) -> Option<u64> {
        None
    }
    /// 임시 파일을 저장 폴더로(이름 규칙 `stem` + `.mp4`, 같은 이름이면 ` (2)`, 다른 드라이브면 복사 → 확인 → 지움).
    fn keep_file(&mut self, _temp: &std::path::Path, _folder: &str, _stem: &str) -> Result<SavedFile, String> {
        Err(REC_UNSUPPORTED.into())
    }
    /// 남은 녹화 지우기(복구 섬 [지우기]).
    fn delete_file(&mut self, _temp: &std::path::Path) -> Result<(), String> {
        Err(REC_UNSUPPORTED.into())
    }
}

/// 녹화가 없는 곳(맥 1차·시험 기본).
pub struct NoRecorder;

impl Recorder for NoRecorder {
    fn start(&mut self, _opts: RecStartOpts) -> Result<(), String> {
        Err(REC_UNSUPPORTED.into())
    }
    fn stats(&self) -> RecStats {
        RecStats::default()
    }
    fn stop(&mut self) -> Result<RecStats, String> {
        Err(REC_UNSUPPORTED.into())
    }
    fn abort(&mut self) {}
}
