//! screen-ink 유스케이스 층(03 §3). 운영체제·Tauri는 포트(trait)로만 만난다.
//! UI의 `InkClient` 명령 하나 = `InkService` 메서드 하나.

pub mod capture;
pub mod case;
pub mod draw;
pub mod live_zoom;
pub mod package;
pub mod ports;
mod quick;
pub mod record;
pub mod service;
pub mod state;
pub mod vision;
mod window_capture;

pub use ports::{
    AutostartControl, Clipboard, Clock, ElementSource, FileSink, Frame, HotkeyError, HotkeyRegistry, Loaded, MonitorProbe, NoPins, NoVision, NoopAutostart, NoopUpdater, Notifier,
    OverlayHost, PackageRuntime, PinHost, Ports, ReadyUpdate, SavedFile, ScreenSource, SettingsStore, StoreError, StoreLocation, SystemEvent, SystemProbe, UnpackagedRuntime, Updater,
    VisionSource, WindowCatalog, NO_SCREEN_PERMISSION,
};
pub use ports::{NoRecorder, RecStartOpts, RecStats, Recorder, WindowShot, REC_ENCODER_FAILED, REC_UNSUPPORTED, REC_WINDOW_CLOSED, WINDOW_PROTECTED};
pub use quick::Delay;
pub use capture::CaptureInput;
pub use draw::{DrawExportMeta, DrawExported, DrawFinish, DrawIslandPatch, DrawRestoreResult, ExportAction, KeptInk, KeptPayload, PenSample};
pub use ink_domain::mode::DrawKind;
pub use vision::VisionEngine;
pub use service::{HotkeyOutcome, ImportPreview, InkService, ACTION_PERM_AX, ACTION_PERM_SCREEN, PAUSE_SLOT};
pub use state::{
    CaptureView, ConflictSummary, DrawSession, DrawViewTransform, InkState, IslandButton, IslandEvent, IslandHint, IslandKind, KeptCompat, OverlayMonitor, OverlaySession,
    PreviewResult, RowStatus, SetShortcutResult, ShortcutRow, UpdateResult,
};
pub use live_zoom::{ZoomHandoff, ZoomInput};
pub use ports::{Magnifier, NoMagnifier, ZOOM_UNSUPPORTED};
pub use state::LiveZoomState;
