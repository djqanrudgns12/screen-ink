//! 창 제목을 보관하지 않고 우리 앱이 고정한 창만 관리한다.

use crate::ports::{BorderStyle, PinError, UnsupportedWindowPinner, WindowPinner};
use crate::{InkService, IslandEvent, IslandKind};
use ink_domain::topmost::{decide_for, display_name, CandidateWindow, TopmostDecision, UNAVAILABLE};
use std::collections::BTreeMap;

pub struct PinnedRecord {
    pub handle: u64,
    pub pid: u32,
    pub pinned_at_ms: u64,
}

pub struct TopmostState {
    port: Box<dyn WindowPinner>,
    pub(crate) pinned: BTreeMap<u64, PinnedRecord>,
    cleaned: usize,
}

impl Default for TopmostState {
    fn default() -> Self {
        Self {
            port: Box::new(UnsupportedWindowPinner),
            pinned: BTreeMap::new(),
            cleaned: 0,
        }
    }
}

impl InkService {
    pub fn set_window_pinner(&mut self, port: Box<dyn WindowPinner>) {
        self.topmost.port = port;
    }

    pub fn cleanup_on_start(&mut self) {
        self.topmost.cleaned = self.topmost.port.cleanup_orphans();
    }

    /// 섬 화면이 구독을 마친 뒤 시작 정리 결과를 한 번만 알린다.
    pub fn announce_topmost_cleanup(&mut self) {
        let cleaned = std::mem::take(&mut self.topmost.cleaned);
        if cleaned > 0 {
            self.pin_notice(self.tp("topmost.cleaned", cleaned as u64));
        }
    }

    pub(crate) fn tick_topmost(&mut self) {
        if self.topmost.pinned.is_empty() {
            return;
        }
        let gone: Vec<_> = self.topmost.pinned.keys().copied().filter(|h| !self.topmost.port.is_alive(*h)).collect();
        for hwnd in gone {
            self.on_window_destroyed(hwnd);
        }
    }

    pub fn on_window_destroyed(&mut self, hwnd: u64) {
        if !self.topmost.port.is_alive(hwnd) && self.topmost.pinned.remove(&hwnd).is_some() {
            self.topmost.port.forget_destroyed(hwnd);
            self.emit_state();
        }
    }

    fn border_style(&self) -> BorderStyle {
        BorderStyle {
            thickness_px: self.settings.get("topmost.border").and_then(|v| v.as_u64()).unwrap_or(4) as u32,
            color_rgb: 0xff9f0a,
        }
    }

    pub(crate) fn update_border_style(&mut self) {
        self.topmost.port.update_border_style(self.border_style());
    }

    fn pin_notice(&self, text: String) {
        self.island(IslandEvent::new(IslandKind::Info, text).variant("pin").duration_ms(1200));
    }

    pub fn toggle_topmost(&mut self) {
        let candidate = self.topmost.port.foreground();
        self.toggle_candidate(candidate);
    }

    pub fn foreground_for_tray(&self) -> Option<(u64, u32)> {
        self.topmost
            .port
            .foreground()
            .filter(|w| {
                !matches!(
                    w.class.as_str(),
                    "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "NotifyIconOverflowWindow" | "TopLevelWindowForOverflowXamlIsland"
                )
            })
            .map(|w| (w.handle, w.pid))
    }

    pub fn toggle_tray_target(&mut self, target: Option<(u64, u32)>) {
        let candidate = target.and_then(|(h, pid)| self.topmost.port.candidate(h).filter(|w| w.pid == pid));
        self.toggle_candidate(candidate);
    }

    fn toggle_candidate(&mut self, win: Option<CandidateWindow>) {
        let Some(win) = win else {
            self.pin_notice(self.t(UNAVAILABLE));
            return;
        };
        let h = win.handle;
        let result = match decide_for(&win, std::process::id(), self.lang()) {
            TopmostDecision::IgnoreSilent => return,
            TopmostDecision::Skip(reason) => {
                self.pin_notice(reason.into());
                return;
            }
            TopmostDecision::Pin => self.topmost.port.pin(h, self.border_style()).map(|()| {
                self.topmost.pinned.insert(
                    h,
                    PinnedRecord {
                        handle: h,
                        pid: win.pid,
                        pinned_at_ms: self.clock_ms(),
                    },
                );
                let app = display_name(&win.exe);
                let text = if app.is_empty() {
                    self.t("topmost.pinnedUnnamed")
                } else {
                    self.tf("topmost.pinned", &[("app", &app)])
                };
                let event = IslandEvent::new(IslandKind::Info, text).variant("pin").duration_ms(1200);
                let combo = self.settings.shortcut("topmost");
                self.island(if combo.is_empty() {
                    event
                } else {
                    event.hints(&[(combo, &self.t("topmost.release"))])
                });
                if self.settings.bool("topmost.sound") {
                    self.topmost.port.play_pin_sound();
                }
            }),
            TopmostDecision::Unpin => self.topmost.port.unpin(h).map(|()| {
                self.topmost.pinned.remove(&h);
                self.pin_notice(self.t("topmost.released"));
            }),
        };
        match result {
            Ok(()) => self.emit_state(),
            Err(PinError::Gone) => self.on_window_destroyed(h),
            Err(PinError::AccessDenied) => self.pin_notice(self.t("topmost.denied")),
            Err(_) => self.pin_notice(self.t(UNAVAILABLE)),
        }
    }

    pub fn unpin_all(&mut self, announce: bool) {
        self.tick_topmost();
        let before = self.topmost.pinned.len();
        self.topmost.port.unpin_all();
        self.topmost.pinned.retain(|h, _| self.topmost.port.is_alive(*h));
        let released = before - self.topmost.pinned.len();
        if announce {
            if self.topmost.pinned.is_empty() {
                self.pin_notice(self.tp("topmost.releasedAll", released as u64));
            } else {
                self.pin_notice(self.t("topmost.releaseFailed"));
            }
        }
        if released > 0 {
            self.emit_state();
        }
    }
}
