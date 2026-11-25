//! screen-ink 유스케이스 층(03 §3). 운영체제·Tauri는 포트(trait)로만 만난다.
//! UI의 `InkClient` 명령 하나 = `InkService` 메서드 하나.

pub mod capture;
pub mod ports;
pub mod service;
pub mod state;

pub use ports::{
    Clipboard, Clock, ElementSource, FileSink, Frame, HotkeyError, HotkeyRegistry, Loaded, MonitorProbe, NoVision, Notifier, OverlayHost, Ports, SavedFile, ScreenSource,
    SettingsStore, StoreError, StoreLocation, SystemEvent, SystemProbe, VisionSource, WindowCatalog,
};
pub use capture::CaptureInput;
pub use service::{HotkeyOutcome, ImportPreview, InkService, PAUSE_SLOT};
pub use state::{
    CaptureView, ConflictSummary, InkState, IslandButton, IslandEvent, IslandHint, IslandKind, OverlayMonitor, OverlaySession, PreviewResult, RowStatus, SetShortcutResult,
    ShortcutRow, UpdateResult,
};
