//! 가짜 어댑터 — 상태를 `Arc<Mutex<_>>`로 바깥에 내놓아 시험이 "이 PC 장면"을 꾸미고 결과를 들여다본다.

use ink_app::{
    Frame, HotkeyError, HotkeyRegistry, InkState, IslandEvent, Loaded, MonitorProbe, Notifier, OverlayHost, OverlaySession, Ports, ScreenSource, SettingsStore, StoreError,
    StoreLocation, SystemProbe, WindowCatalog,
};
use ink_domain::geometry::{Monitor, MonitorLayout, PhysPoint, PhysRect};
use ink_domain::shortcuts::LiveBinding;
use ink_domain::snapshot::RawWindow;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

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
}

/// 화면 장면: 모니터 배치·커서·창 목록·캡처 실패 여부.
#[derive(Debug)]
pub struct Screen {
    pub layout: MonitorLayout,
    pub cursor: PhysPoint,
    pub windows: Vec<RawWindow>,
    pub capture_fails: bool,
    pub captures: usize,
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
            captures: 0,
        }
    }
}

#[derive(Clone, Default)]
pub struct FakeScreen(pub Arc<Mutex<Screen>>);

impl ScreenSource for FakeScreen {
    fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String> {
        let mut s = self.0.lock().unwrap();
        if s.capture_fails {
            return Err("capture failed".into());
        }
        s.captures += 1;
        Ok(layout.monitors.iter().map(|m| Frame { monitor_id: m.id, width: m.bounds.w as u32, height: m.bounds.h as u32, bgra: Arc::new(vec![0; 64]) }).collect())
    }
}

impl WindowCatalog for FakeScreen {
    fn windows(&self) -> Vec<RawWindow> {
        self.0.lock().unwrap().windows.clone()
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
            screen: Box::new(self.screen.clone()),
            windows: Box::new(self.screen.clone()),
            monitors: Box::new(self.screen.clone()),
            overlay: Box::new(self.overlay.clone()),
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
}
