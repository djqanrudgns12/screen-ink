//! 가짜 어댑터 — 상태를 `Arc<Mutex<_>>`로 바깥에 내놓아 시험이 "이 PC 장면"을 꾸미고 결과를 들여다본다.

use ink_app::{
    DrawSession, Frame, HotkeyError, HotkeyRegistry, InkState, IslandEvent, Loaded, MonitorProbe, Notifier, OverlayHost, OverlaySession, Ports, ScreenSource, SettingsStore,
    StoreError, StoreLocation, SystemProbe, WindowCatalog,
};
use ink_domain::geometry::{Monitor, MonitorLayout, PhysPoint, PhysRect};
use ink_domain::shortcuts::LiveBinding;
use ink_domain::snapshot::RawWindow;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

pub mod replay;
pub mod truth;

#[derive(Debug, Default)]
pub struct HotkeyBook {
    /// 가속기 → 칸
    pub registered: BTreeMap<String, String>,
    /// 다른 앱이 이미 잡은 가속기(등록하면 Taken)
    pub taken: BTreeSet<String>,
}

#[derive(Clone, Default)]
pub struct FakeHotkeys(pub Arc<Mutex<HotkeyBook>>);

impl HotkeyRegistry for FakeHotkeys {
    fn register(&mut self, slot: &str, accelerator: &str) -> Result<(), HotkeyError> {
        let mut b = self.0.lock().unwrap();
        if b.taken.contains(accelerator) {
            return Err(HotkeyError::Taken);
        }
        if b.registered.contains_key(accelerator) {
            return Err(HotkeyError::Other(format!("already registered: {accelerator}")));
        }
        b.registered.insert(accelerator.to_string(), slot.to_string());
        Ok(())
    }

    fn unregister(&mut self, accelerator: &str) {
        self.0.lock().unwrap().registered.remove(accelerator);
    }
}

#[derive(Debug, Default)]
pub struct ProbeScene {
    pub running: Vec<String>,
    pub live: Option<Vec<LiveBinding>>,
    pub secure_desktop: bool,
    /// 지금 활성 창(Shift+F2·F2 두 번)
    pub foreground: Option<u64>,
    /// 맥 권한 `(화면 기록, 손쉬운 사용)` — None = 권한 개념 없음(Windows)
    pub permissions: Option<(bool, bool)>,
}

#[derive(Clone, Default)]
pub struct FakeProbe(pub Arc<Mutex<ProbeScene>>);

impl SystemProbe for FakeProbe {
    fn running_apps(&self) -> Vec<String> {
        self.0.lock().unwrap().running.clone()
    }

    fn live_bindings(&self) -> Option<Vec<LiveBinding>> {
        self.0.lock().unwrap().live.clone()
    }

    fn secure_desktop(&self) -> bool {
        self.0.lock().unwrap().secure_desktop
    }

    fn foreground_window(&self) -> Option<u64> {
        self.0.lock().unwrap().foreground
    }

    fn permissions(&self) -> Option<(bool, bool)> {
        self.0.lock().unwrap().permissions
    }
}

/// 화면 장면: 모니터 배치·커서·창 목록·캡처 실패 여부.
#[derive(Debug)]
pub struct Screen {
    pub layout: MonitorLayout,
    pub cursor: PhysPoint,
    pub windows: Vec<RawWindow>,
    pub capture_fails: bool,
    /// `capture_fails`일 때 돌려줄 오류(없으면 "capture failed")
    pub fail_reason: Option<String>,
    pub captures: usize,
    /// 진짜 이미지(재생 시험). 없으면 검은 작은 이미지.
    pub frames: Option<Vec<Frame>>,
    /// 창 → H 자식 창
    pub children: HashMap<u64, Vec<PhysRect>>,
    /// 창 → WGC 결과(없으면 "지원 안 함" — 서비스가 스냅숏 자르기로 대체)
    pub window_shots: HashMap<u64, Result<ink_app::WindowShot, String>>,
    /// `prepare_window` 기록(F2 고르기 중 미리 준비)
    pub prepared_windows: Vec<Option<u64>>,
    /// `capture_window`를 부른 창
    pub window_captures: Vec<u64>,
}

impl Default for Screen {
    fn default() -> Self {
        Screen {
            layout: MonitorLayout::new(vec![
                Monitor { id: 0, bounds: PhysRect::new(0, 0, 1920, 1080), work: PhysRect::new(0, 0, 1920, 1040), scale: 1.0, primary: true },
                Monitor { id: 1, bounds: PhysRect::new(1920, 0, 2880, 1800), work: PhysRect::new(1920, 0, 2880, 1728), scale: 1.5, primary: false },
            ]),
            cursor: PhysPoint { x: 400, y: 300 },
            windows: vec![RawWindow {
                handle: 1,
                pid: 7,
                exe: "chrome.exe".into(),
                class: "Chrome_WidgetWin_1".into(),
                rect: PhysRect::new(100, 100, 1200, 800),
                visible: true,
                ..Default::default()
            }],
            capture_fails: false,
            fail_reason: None,
            captures: 0,
            frames: None,
            children: HashMap::new(),
            window_shots: HashMap::new(),
            prepared_windows: Vec::new(),
            window_captures: Vec::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct FakeScreen(pub Arc<Mutex<Screen>>);

impl ScreenSource for FakeScreen {
    fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String> {
        let mut s = self.0.lock().unwrap();
        if s.capture_fails {
            return Err(s.fail_reason.clone().unwrap_or_else(|| "capture failed".into()));
        }
        s.captures += 1;
        if let Some(f) = &s.frames {
            return Ok(f.clone());
        }
        Ok(layout.monitors.iter().map(|m| Frame { monitor_id: m.id, width: m.bounds.w as u32, height: m.bounds.h as u32, bgra: Arc::new(vec![0; 64]) }).collect())
    }

    fn prepare_window(&mut self, window: Option<u64>) {
        self.0.lock().unwrap().prepared_windows.push(window);
    }

    fn capture_window(&mut self, window: u64, _bounds: PhysRect) -> Result<ink_app::WindowShot, String> {
        let mut s = self.0.lock().unwrap();
        s.window_captures.push(window);
        s.window_shots.get(&window).cloned().unwrap_or_else(|| Err("window capture not supported".into()))
    }
}

impl WindowCatalog for FakeScreen {
    fn windows(&self) -> Vec<RawWindow> {
        self.0.lock().unwrap().windows.clone()
    }

    fn children(&self, window: u64) -> Vec<PhysRect> {
        self.0.lock().unwrap().children.get(&window).cloned().unwrap_or_default()
    }
}

impl MonitorProbe for FakeScreen {
    fn layout(&self) -> MonitorLayout {
        self.0.lock().unwrap().layout.clone()
    }

    fn cursor(&self) -> PhysPoint {
        self.0.lock().unwrap().cursor
    }
}

#[derive(Debug, Default)]
pub struct OverlayLog {
    pub prepared: usize,
    /// 지금 보이는 세션과 쥔 이미지 수
    pub showing: Option<(u64, usize)>,
    pub shown: Vec<OverlaySession>,
    pub fail_show: bool,
    /// 네이티브 정지 화면 창이 있는 척(진입 시간 끝점 시험)
    pub native: bool,
    pub highlights: Vec<Option<PhysRect>>,
    /// 판서로 띄운 세션과 받은 배경 이미지 크기(칠판은 None)
    pub draws: Vec<(DrawSession, Option<(u32, u32)>)>,
    /// 캡처 주석(4.9): 오버레이에 부탁한 주석 층(받은 순서)
    pub commit_requests: Vec<ink_app::capture::CommitRequest>,
    /// 주석 층을 부탁할 길이 없는 척(request_commit = false)
    pub no_commit: bool,
}

#[derive(Clone, Default)]
pub struct FakeOverlay(pub Arc<Mutex<OverlayLog>>);

impl OverlayHost for FakeOverlay {
    fn prepare(&mut self, _layout: &MonitorLayout) {
        self.0.lock().unwrap().prepared += 1;
    }

    fn show(&mut self, session: &OverlaySession, frames: Vec<Frame>) -> Result<Option<std::time::Instant>, String> {
        let mut o = self.0.lock().unwrap();
        if o.fail_show {
            return Err("webview gone".into());
        }
        o.showing = Some((session.session, frames.len()));
        o.shown.push(session.clone());
        Ok(o.native.then(std::time::Instant::now))
    }

    fn highlight(&mut self, _session: u64, rect: Option<PhysRect>) {
        self.0.lock().unwrap().highlights.push(rect);
    }

    fn hide(&mut self, session: u64) {
        let mut o = self.0.lock().unwrap();
        if o.showing.map(|(s, _)| s) == Some(session) {
            o.showing = None;
        }
    }

    fn request_commit(&mut self, request: &ink_app::capture::CommitRequest) -> bool {
        let mut o = self.0.lock().unwrap();
        if o.no_commit {
            return false;
        }
        o.commit_requests.push(*request);
        true
    }

    fn show_draw(&mut self, session: &DrawSession, frame: Option<Frame>) -> Result<Option<String>, String> {
        let mut o = self.0.lock().unwrap();
        if o.fail_show {
            return Err("webview gone".into());
        }
        o.showing = Some((session.session, usize::from(frame.is_some())));
        o.draws.push((session.clone(), frame.as_ref().map(|f| (f.width, f.height))));
        Ok(frame.map(|f| format!("fake://frame/{}/{}", session.session, f.monitor_id)))
    }
}

#[derive(Debug, Default)]
pub struct Disk {
    pub file: Option<Value>,
    pub fail_writes: bool,
    pub read_only: bool,
    pub writes: usize,
}

#[derive(Clone, Default)]
pub struct MemoryStore(pub Arc<Mutex<Disk>>);

impl SettingsStore for MemoryStore {
    fn load(&mut self) -> Loaded {
        Loaded { file: self.0.lock().unwrap().file.clone(), notice: None }
    }

    fn save_section(&mut self, name: &str, data: &Value) -> Result<(), StoreError> {
        let mut d = self.0.lock().unwrap();
        if d.fail_writes || d.read_only {
            return Err(StoreError::Io("disk full".into()));
        }
        let mut file = d.file.clone().unwrap_or_else(|| serde_json::json!({ "schemaVersion": 3, "sections": {} }));
        let rev = file["sections"][name]["revision"].as_u64().unwrap_or(0) + 1;
        file["sections"][name] = serde_json::json!({ "revision": rev, "data": data });
        d.file = Some(file);
        d.writes += 1;
        Ok(())
    }

    fn location(&self) -> StoreLocation {
        StoreLocation { path: "memory://settings.json".into(), portable: false, read_only: self.0.lock().unwrap().read_only }
    }
}

#[derive(Debug, Default)]
pub struct Inbox {
    pub islands: Vec<IslandEvent>,
    pub states: usize,
    /// `screen-ink://draw`로 보낸 판서 세션
    pub draws: Vec<DrawSession>,
    /// 섬 단추 → 판서 엔진 명령(세션, 명령)
    pub draw_commands: Vec<(u64, String)>,
    /// 실시간 확대 HUD 상태(`None` = 끝)
    pub zooms: Vec<Option<ink_app::LiveZoomState>>,
}

#[derive(Clone, Default)]
pub struct RecordingNotifier(pub Arc<Mutex<Inbox>>);

impl Notifier for RecordingNotifier {
    fn state(&self, _state: &InkState) {
        self.0.lock().unwrap().states += 1;
    }

    fn island(&self, event: &IslandEvent) {
        self.0.lock().unwrap().islands.push(event.clone());
    }

    fn draw(&self, session: &DrawSession) {
        self.0.lock().unwrap().draws.push(session.clone());
    }

    fn draw_command(&self, session: u64, command: &str) {
        self.0.lock().unwrap().draw_commands.push((session, command.to_string()));
    }

    fn zoom(&self, state: Option<&ink_app::LiveZoomState>) {
        self.0.lock().unwrap().zooms.push(state.cloned());
    }
}

/// UIA 색인 흉내: 창 → 요소들. 부탁받은 창을 기록한다.
#[derive(Debug, Default)]
pub struct ElementScene {
    pub by_window: HashMap<u64, Vec<ink_domain::recog::ElementCand>>,
    pub requested: Vec<u64>,
    pub cleared: usize,
}

#[derive(Clone, Default)]
pub struct FakeElements(pub Arc<Mutex<ElementScene>>);

impl ink_app::ElementSource for FakeElements {
    fn request(&self, window: u64, _exe: &str) {
        let mut s = self.0.lock().unwrap();
        if !s.requested.contains(&window) {
            s.requested.push(window);
        }
    }

    fn elements_at(&self, window: u64, p: PhysPoint) -> Vec<ink_domain::recog::ElementCand> {
        self.0.lock().unwrap().by_window.get(&window).map(|v| v.iter().filter(|e| e.rect.contains(p)).copied().collect()).unwrap_or_default()
    }

    fn elements(&self, window: u64) -> Vec<ink_domain::recog::ElementCand> {
        self.0.lock().unwrap().by_window.get(&window).cloned().unwrap_or_default()
    }

    fn clear(&self) {
        self.0.lock().unwrap().cleared += 1;
    }
}

#[derive(Debug, Default)]
pub struct Desk {
    pub images: Vec<(u32, u32)>,
    pub texts: Vec<String>,
    pub saved: Vec<(String, String, u32, u32)>,
    pub clipboard_locked: bool,
    /// 클립보드에 넣은 그림 화소(BGRA) — `images`와 같은 순서
    pub image_bgra: Vec<Vec<u8>>,
    /// 저장한 그림 화소(BGRA) — `saved`와 같은 순서
    pub saved_bgra: Vec<Vec<u8>>,
}

/// 클립보드·저장 폴더 흉내.
#[derive(Clone, Default)]
pub struct FakeDesk(pub Arc<Mutex<Desk>>);

impl ink_app::Clipboard for FakeDesk {
    fn set_image(&mut self, w: u32, h: u32, bgra: &[u8]) -> Result<(), String> {
        let mut d = self.0.lock().unwrap();
        if d.clipboard_locked {
            return Err("locked".into());
        }
        d.images.push((w, h));
        d.image_bgra.push(bgra.to_vec());
        Ok(())
    }

    fn set_text(&mut self, text: &str) -> Result<(), String> {
        self.0.lock().unwrap().texts.push(text.into());
        Ok(())
    }
}

impl ink_app::FileSink for FakeDesk {
    fn save_png(&mut self, folder: &str, stem: &str, w: u32, h: u32, bgra: &[u8]) -> Result<ink_app::SavedFile, String> {
        let mut d = self.0.lock().unwrap();
        d.saved.push((folder.into(), stem.into(), w, h));
        d.saved_bgra.push(bgra.to_vec());
        drop(d);
        Ok(ink_app::SavedFile { name: format!("{stem}.png"), path: format!("{folder}\\{stem}.png"), fell_back: folder.is_empty() })
    }

    fn write_case(&mut self, dir: &str, case: &ink_app::case::Case, frames: &[Frame]) -> Result<String, String> {
        replay::save(std::path::Path::new(dir), case, frames)?;
        Ok(format!("{dir}/{}.json", case.id))
    }
}

/// 핀 창 흉내: 연 핀(id → 자리·크기)과 닫은 id를 기록한다.
#[derive(Debug, Default)]
pub struct PinBoard {
    pub open: BTreeMap<u64, (PhysRect, u32, u32)>,
    pub closed: Vec<u64>,
    pub fail_open: bool,
    /// 핀 그림 화소(BGRA, 열린 핀)
    pub bgra: BTreeMap<u64, Arc<Vec<u8>>>,
}

#[derive(Clone, Default)]
pub struct FakePins(pub Arc<Mutex<PinBoard>>);

impl ink_app::PinHost for FakePins {
    fn open(&mut self, id: u64, rect: PhysRect, w: u32, h: u32, bgra: Arc<Vec<u8>>) -> Result<(), String> {
        let mut b = self.0.lock().unwrap();
        if b.fail_open {
            return Err("no window".into());
        }
        b.open.insert(id, (rect, w, h));
        b.bgra.insert(id, bgra);
        Ok(())
    }

    fn close(&mut self, id: u64) {
        let mut b = self.0.lock().unwrap();
        b.open.remove(&id);
        b.bgra.remove(&id);
        b.closed.push(id);
    }
}

/// 고정 시계(2026-03-04 09:05:07).
#[derive(Clone, Default)]
pub struct FixedClock;

impl ink_app::Clock for FixedClock {
    fn local(&self) -> ink_domain::naming::NameContext {
        ink_domain::naming::NameContext { year: 2026, month: 3, day: 4, hour: 9, minute: 5, second: 7, app: None, n: 0 }
    }

    fn unix(&self) -> u64 {
        1_772_582_707
    }
}

/// 여러 가짜에 걸친 호출 순서(5.3: 돋보기 숨김 → 화면 갱신 기다림 → 캡처 → 판서 창).
/// `FakeWorld::ports()`·`magnifier_port()`가 화면·돋보기·오버레이를 감싸 이름을 남긴다:
/// `magnifier.start` · `magnifier.stop` · `magnifier.wait_hidden` · `screen.capture` · `overlay.show` · `overlay.show_draw` · `overlay.hide`.
#[derive(Clone, Default)]
pub struct Timeline(pub Arc<Mutex<Vec<&'static str>>>);

impl Timeline {
    pub fn push(&self, what: &'static str) {
        self.0.lock().unwrap().push(what);
    }

    pub fn events(&self) -> Vec<&'static str> {
        self.0.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

struct TracedScreen(FakeScreen, Timeline);

impl ScreenSource for TracedScreen {
    fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String> {
        self.1.push("screen.capture");
        self.0.capture(layout)
    }

    fn cursor_sprite(&mut self) -> Option<ink_domain::image::CursorSprite> {
        self.0.cursor_sprite()
    }

    // 단계 6 창 캡처도 그대로 넘긴다(감싸개가 기본 구현 "지원 안 함"으로 가리지 않게)
    fn prepare_window(&mut self, window: Option<u64>) {
        self.0.prepare_window(window);
    }

    fn capture_window(&mut self, window: u64, bounds: PhysRect) -> Result<ink_app::WindowShot, String> {
        self.1.push("screen.captureWindow");
        self.0.capture_window(window, bounds)
    }
}

struct TracedOverlay(FakeOverlay, Timeline);

impl OverlayHost for TracedOverlay {
    fn prepare(&mut self, layout: &MonitorLayout) {
        self.0.prepare(layout);
    }

    fn show(&mut self, session: &OverlaySession, frames: Vec<Frame>) -> Result<Option<std::time::Instant>, String> {
        self.1.push("overlay.show");
        self.0.show(session, frames)
    }

    fn highlight(&mut self, session: u64, rect: Option<PhysRect>) {
        self.0.highlight(session, rect);
    }

    fn hide(&mut self, session: u64) {
        self.1.push("overlay.hide");
        self.0.hide(session);
    }

    fn thumbnail(&mut self, w: u32, h: u32, bgra: Vec<u8>) -> Option<String> {
        self.0.thumbnail(w, h, bgra)
    }

    fn show_draw(&mut self, session: &DrawSession, frame: Option<Frame>) -> Result<Option<String>, String> {
        self.1.push("overlay.show_draw");
        self.0.show_draw(session, frame)
    }

    fn flash_afterimage(&mut self, session: u64, rect: PhysRect, layout: &MonitorLayout) {
        self.0.flash_afterimage(session, rect, layout);
    }

    fn request_commit(&mut self, request: &ink_app::capture::CommitRequest) -> bool {
        self.1.push("overlay.request_commit");
        self.0.request_commit(request)
    }
}

struct TracedMagnifier(FakeMagnifier, Timeline);

impl ink_app::Magnifier for TracedMagnifier {
    fn start(&mut self, monitor: PhysRect, source: PhysRect, scale: f64) -> Result<(), String> {
        self.1.push("magnifier.start");
        self.0.start(monitor, source, scale)
    }

    fn update(&mut self, source: PhysRect, scale: f64) -> Result<(), String> {
        self.0.update(source, scale)
    }

    fn stop(&mut self) {
        self.1.push("magnifier.stop");
        self.0.stop();
    }

    fn wait_hidden(&mut self) {
        self.1.push("magnifier.wait_hidden");
        self.0.wait_hidden();
    }
}

/// 한 장면에 필요한 가짜 넷을 함께 만들고, 손잡이를 돌려준다.
#[derive(Clone, Default)]
pub struct FakeWorld {
    pub hotkeys: FakeHotkeys,
    pub probe: FakeProbe,
    pub store: MemoryStore,
    pub notifier: RecordingNotifier,
    pub screen: FakeScreen,
    pub overlay: FakeOverlay,
    pub elements: FakeElements,
    pub desk: FakeDesk,
    pub pins: FakePins,
    /// 실시간 확대 돋보기(`InkService::set_magnifier(w.magnifier_port())`로 꽂는다)
    pub magnifier: FakeMagnifier,
    /// 진짜 V 엔진(그 자리에서 전처리)을 쓸지
    pub vision: bool,
    /// 화면·돋보기·오버레이 호출 순서
    pub timeline: Timeline,
    /// 녹화기(`install_recorder`로 서비스에 끼운다)
    pub recorder: FakeRecorder,
}

impl FakeWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ports(&self) -> Ports {
        Ports {
            hotkeys: Box::new(self.hotkeys.clone()),
            probe: Box::new(self.probe.clone()),
            store: Box::new(self.store.clone()),
            notifier: Box::new(self.notifier.clone()),
            screen: Box::new(TracedScreen(self.screen.clone(), self.timeline.clone())),
            windows: Box::new(self.screen.clone()),
            monitors: Box::new(self.screen.clone()),
            overlay: Box::new(TracedOverlay(self.overlay.clone(), self.timeline.clone())),
            elements: Box::new(self.elements.clone()),
            vision: if self.vision { Box::new(ink_app::VisionEngine::blocking()) } else { Box::new(ink_app::NoVision) },
            clipboard: Box::new(self.desk.clone()),
            files: Box::new(self.desk.clone()),
            clock: Box::new(FixedClock),
            pins: Box::new(self.pins.clone()),
        }
    }

    pub fn running(&self, exe: &[&str]) -> &Self {
        self.probe.0.lock().unwrap().running = exe.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn taken(&self, accels: &[&str]) -> &Self {
        self.hotkeys.0.lock().unwrap().taken = accels.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn is_registered(&self, accel: &str) -> bool {
        self.hotkeys.0.lock().unwrap().registered.contains_key(accel)
    }

    pub fn registered_count(&self) -> usize {
        self.hotkeys.0.lock().unwrap().registered.len()
    }

    pub fn last_island(&self) -> Option<IslandEvent> {
        self.notifier.0.lock().unwrap().islands.last().cloned()
    }

    pub fn saved(&self) -> Option<Value> {
        self.store.0.lock().unwrap().file.clone()
    }

    /// 가짜 녹화기를 서비스에 끼운다(끼우는 순간 남은 녹화를 찾는다).
    pub fn install_recorder(&self, svc: &mut ink_app::InkService) {
        svc.set_recorder(Box::new(self.recorder.clone()));
    }

    pub fn islands(&self) -> Vec<IslandEvent> {
        self.notifier.0.lock().unwrap().islands.clone()
    }
}

// ───────────── 녹화(단계 6) ─────────────

/// 녹화기 흉내: 파일은 메모리 표(경로 → 바이트)에만 쓴다.
#[derive(Debug, Default)]
pub struct RecBook {
    pub prepared: usize,
    pub started: Vec<ink_app::RecStartOpts>,
    pub active: bool,
    pub stopped: usize,
    pub aborted: usize,
    /// 다음 `start`가 이 오류로 실패
    pub fail_start: Option<String>,
    /// 다음 `stop`이 이 오류로 실패(임시 파일은 남김)
    pub fail_stop: Option<String>,
    /// 다음 상태 틱에 "스스로 멈춤"으로 돌려줄 이유
    pub failure: Option<String>,
    pub mic: bool,
    pub free: Option<u64>,
    /// 녹화가 쓴 바이트(멈출 때 통계로)
    pub bytes: u64,
    pub elapsed_ms: u64,
    /// 임시 폴더의 파일(경로 → 바이트)
    pub temp_files: BTreeMap<String, u64>,
    /// 저장 폴더로 옮긴 것: (임시 경로, 폴더, 이름)
    pub kept: Vec<(String, String, String)>,
    pub deleted: Vec<String>,
    pub leftovers: Vec<ink_domain::recording::Leftover>,
}

#[derive(Clone, Default)]
pub struct FakeRecorder(pub Arc<Mutex<RecBook>>);

pub const FAKE_TEMP_DIR: &str = "T:/tmp/shookshok";

impl ink_app::Recorder for FakeRecorder {
    fn prepare(&mut self) {
        self.0.lock().unwrap().prepared += 1;
    }

    fn start(&mut self, opts: ink_app::RecStartOpts) -> Result<(), String> {
        let mut b = self.0.lock().unwrap();
        if let Some(e) = b.fail_start.take() {
            return Err(e);
        }
        b.temp_files.insert(opts.temp_path.to_string_lossy().into_owned(), 0);
        b.started.push(opts);
        b.active = true;
        Ok(())
    }

    fn stats(&self) -> ink_app::RecStats {
        let b = self.0.lock().unwrap();
        let (width, height) = b.started.last().map(|o| ink_domain::recording::even_size(o.target.rect().w as u32, o.target.rect().h as u32)).unwrap_or((0, 0));
        ink_app::RecStats { elapsed_ms: b.elapsed_ms, bytes: b.bytes, frames: b.elapsed_ms * 30 / 1000, width, height }
    }

    fn stop(&mut self) -> Result<ink_app::RecStats, String> {
        let stats = self.stats();
        let mut b = self.0.lock().unwrap();
        b.active = false;
        b.stopped += 1;
        if let Some(e) = b.fail_stop.take() {
            return Err(e);
        }
        if let Some(o) = b.started.last() {
            let path = o.temp_path.to_string_lossy().into_owned();
            let bytes = b.bytes;
            b.temp_files.insert(path, bytes);
        }
        Ok(stats)
    }

    fn abort(&mut self) {
        let mut b = self.0.lock().unwrap();
        b.active = false;
        b.aborted += 1;
        if let Some(o) = b.started.last() {
            let path = o.temp_path.to_string_lossy().into_owned();
            b.temp_files.remove(&path);
        }
    }

    fn take_failure(&mut self) -> Option<String> {
        self.0.lock().unwrap().failure.take()
    }

    fn mic_supported(&self) -> bool {
        self.0.lock().unwrap().mic
    }

    fn temp_dir(&self) -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from(FAKE_TEMP_DIR))
    }

    fn leftovers(&self) -> Vec<ink_domain::recording::Leftover> {
        self.0.lock().unwrap().leftovers.clone()
    }

    fn free_bytes(&self, _folder: &str) -> Option<u64> {
        self.0.lock().unwrap().free
    }

    fn keep_file(&mut self, temp: &std::path::Path, folder: &str, stem: &str) -> Result<ink_app::SavedFile, String> {
        let mut b = self.0.lock().unwrap();
        let t = temp.to_string_lossy().into_owned();
        if b.temp_files.remove(&t).is_none() && !b.leftovers.iter().any(|l| l.path == t) {
            return Err("no such temp file".into());
        }
        b.kept.push((t, folder.into(), stem.into()));
        Ok(ink_app::SavedFile { name: format!("{stem}.mp4"), path: format!("{folder}/{stem}.mp4"), fell_back: folder.is_empty() })
    }

    fn delete_file(&mut self, temp: &std::path::Path) -> Result<(), String> {
        let mut b = self.0.lock().unwrap();
        let t = temp.to_string_lossy().into_owned();
        b.temp_files.remove(&t);
        b.deleted.push(t);
        Ok(())
    }
}

/// 실시간 확대 돋보기 흉내: 띄운 모니터·원본·배율과 갱신 기록. 실패도 꾸밀 수 있다.
#[derive(Debug, Default)]
pub struct MagnifierLog {
    /// 지금 띄운 돋보기(모니터, 원본, 배율)
    pub running: Option<(PhysRect, PhysRect, f64)>,
    pub starts: usize,
    pub stops: usize,
    /// 원본·배율 갱신(틱·휠)
    pub updates: Vec<(PhysRect, f64)>,
    /// `Some(오류)`면 start가 실패
    pub fail_start: Option<String>,
    /// 다음 update부터 실패(돋보기가 스스로 멈춘 흉내)
    pub fail_update: bool,
    /// 숨겼지만 화면 합성에는 아직 남아 있음(`stop` 뒤 `wait_hidden` 전) — 이때 찍으면 확대 화면이 찍힌다
    pub on_screen: bool,
    pub waits: usize,
}

#[derive(Clone, Default)]
pub struct FakeMagnifier(pub Arc<Mutex<MagnifierLog>>);

impl ink_app::Magnifier for FakeMagnifier {
    fn start(&mut self, monitor: PhysRect, source: PhysRect, scale: f64) -> Result<(), String> {
        let mut m = self.0.lock().unwrap();
        if let Some(e) = m.fail_start.clone() {
            return Err(e);
        }
        if m.running.is_some() {
            return Err("already running".into());
        }
        m.starts += 1;
        m.running = Some((monitor, source, scale));
        m.on_screen = true;
        Ok(())
    }

    fn update(&mut self, source: PhysRect, scale: f64) -> Result<(), String> {
        let mut m = self.0.lock().unwrap();
        if m.fail_update || m.running.is_none() {
            m.running = None;
            return Err("magnifier stopped".into());
        }
        m.updates.push((source, scale));
        if let Some(r) = m.running.as_mut() {
            r.1 = source;
            r.2 = scale;
        }
        Ok(())
    }

    fn stop(&mut self) {
        let mut m = self.0.lock().unwrap();
        if m.running.take().is_some() {
            m.stops += 1;
        }
    }

    fn wait_hidden(&mut self) {
        let mut m = self.0.lock().unwrap();
        m.waits += 1;
        if m.running.is_none() {
            m.on_screen = false;
        }
    }
}

impl FakeWorld {
    /// `InkService::set_magnifier`에 넘길 돋보기(같은 기록·같은 호출 순서를 본다).
    pub fn magnifier_port(&self) -> Box<dyn ink_app::Magnifier> {
        Box::new(TracedMagnifier(self.magnifier.clone(), self.timeline.clone()))
    }

    pub fn last_zoom(&self) -> Option<Option<ink_app::LiveZoomState>> {
        self.notifier.0.lock().unwrap().zooms.last().cloned()
    }
}
