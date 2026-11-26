//! screen-ink 유스케이스 층(03 §3). 운영체제·Tauri는 포트(trait)로만 만난다.
//! UI의 `InkClient` 명령 하나 = `InkService` 메서드 하나.

pub mod capture;
pub mod case;
pub mod draw;
pub mod ports;
pub mod service;
pub mod state;
pub mod vision;

pub use ports::{
    Clipboard, Clock, ElementSource, FileSink, Frame, HotkeyError, HotkeyRegistry, Loaded, MonitorProbe, NoPins, NoVision, Notifier, OverlayHost, PinHost, Ports, SavedFile, ScreenSource, NO_SCREEN_PERMISSION,
    SettingsStore, StoreError, StoreLocation, SystemEvent, SystemProbe, VisionSource, WindowCatalog,
};
pub use capture::CaptureInput;
pub use draw::{DrawExportMeta, DrawExported, DrawFinish, DrawIslandPatch, DrawRestoreResult, ExportAction, KeptInk, KeptPayload, PenSample};
pub use ink_domain::mode::DrawKind;
pub use vision::VisionEngine;
pub use service::{HotkeyOutcome, ImportPreview, InkService, PAUSE_SLOT};
pub use state::{
    CaptureView, ConflictSummary, DrawSession, DrawViewTransform, InkState, IslandButton, IslandEvent, IslandHint, IslandKind, KeptCompat, OverlayMonitor, OverlaySession,
    PreviewResult, RowStatus, SetShortcutResult, ShortcutRow, UpdateResult,
};
