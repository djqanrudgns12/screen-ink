//! screen-ink 도메인 — 운영체제·Tauri를 모르는 순수 규칙만 둔다(03 §2).
//! 모든 판정은 여기서 하고, 어댑터는 사실(실행 중인 앱, 등록 결과)만 넘긴다.

pub mod bmp;
pub mod geometry;
pub mod history;
pub mod image;
pub mod magnet;
pub mod modekeys;
pub mod mode;
pub mod naming;
pub mod package;
pub mod pin;
pub mod recog;
pub mod recording;
pub mod selection;
pub mod settings;
pub mod shortcuts;
pub mod snapshot;
pub mod text;
pub mod timing;
pub mod zoom;
pub mod windowshot;

pub use shortcuts::{Level, Platform};
