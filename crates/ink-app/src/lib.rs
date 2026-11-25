//! screen-ink 유스케이스 층(03 §3). 운영체제·Tauri는 포트(trait)로만 만난다.
//! UI의 `InkClient` 명령 하나 = `InkService` 메서드 하나.

pub mod ports;
pub mod service;
pub mod state;

pub use ports::{
    Frame, HotkeyError, HotkeyRegistry, Loaded, MonitorProbe, Notifier, OverlayHost, Ports, ScreenSource, SettingsStore, StoreError, StoreLocation, SystemEvent, SystemProbe,
    WindowCatalog,
};
pub use service::{HotkeyOutcome, ImportPreview, InkService, PAUSE_SLOT};
pub use state::{
    ConflictSummary, InkState, IslandButton, IslandEvent, IslandHint, IslandKind, OverlayMonitor, OverlaySession, PreviewResult, RowStatus, SetShortcutResult, ShortcutRow,
    UpdateResult,
};
