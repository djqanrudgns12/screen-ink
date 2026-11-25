//! 가짜 어댑터 — 상태를 `Arc<Mutex<_>>`로 바깥에 내놓아 시험이 "이 PC 장면"을 꾸미고 결과를 들여다본다.

use ink_app::{HotkeyError, HotkeyRegistry, InkState, IslandEvent, Loaded, Notifier, Ports, SettingsStore, StoreError, StoreLocation, SystemProbe};
use ink_domain::shortcuts::LiveBinding;
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
