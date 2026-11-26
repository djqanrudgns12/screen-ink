//! 바로 캡처 키 4종(단계 6.2, R-CAP-13·R-KEY-5, 시안 E §2.2):
//! `last`(Shift+F1) 직전 범위 · `full`(Ctrl+F1) 커서 모니터 · `active`(Shift+F2) 활성 창 · `delay`(Alt+F1) 지연 캡처.
//! 고르기 화면 없이 지금 화면을 찍어 F1 확정과 같은 결과(복사·저장·기록·결과 섬)로 끝낸다.

use crate::ports::NO_SCREEN_PERMISSION;
use crate::service::InkService;
use crate::state::{IslandEvent, IslandHint, IslandKind};
use ink_domain::geometry::{MonitorLayout, PhysRect};
use ink_domain::settings::ExcludedApp;
use ink_domain::shortcuts::to_accelerator;
use ink_domain::snapshot::{filter_windows, CaptureKind, Snapshot};

/// 지연 캡처: 서비스 시계로 끝나는 때와 섬에 보인 남은 초.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delay {
    pub due_ms: u64,
    pub shown: u32,
}

impl InkService {
    pub(crate) fn quick_hotkey(&mut self, slot: &str) {
        match slot {
            "last" => self.capture_last_range(),
            "full" => self.capture_cursor_monitor(),
            "active" => {
                self.end_capture();
                self.capture_active_window();
            }
            "delay" => self.toggle_delay(),
            _ => {}
        }
    }

    /// 보안 데스크톱이면 조용히(1회 안내) 막는다(R-SYS-3).
    pub(crate) fn secure_blocked(&mut self) -> bool {
        if !self.ports.probe.secure_desktop() {
            return false;
        }
        if !self.secure_notified {
            self.secure_notified = true;
            self.island(IslandEvent::new(IslandKind::Off, "보안 화면에서는 쓸 수 없어요").variant("security"));
        }
        true
    }

    /// `Shift+F1`: 지금 모니터 구성에서 쓴 마지막 범위.
    pub fn capture_last_range(&mut self) {
        if self.secure_blocked() {
            return;
        }
        self.end_capture();
        let layout = self.ports.monitors.layout();
        let Some(rect) = self.cap.ranges.for_layout(&layout.fingerprint()).first().copied() else {
            self.island(IslandEvent::new(IslandKind::Info, "지난 범위가 없어요"));
            return;
        };
        self.capture_rect_now(rect, layout);
    }

    /// `Ctrl+F1`: 커서가 있는 모니터 전체(작업 표시줄 포함).
    pub fn capture_cursor_monitor(&mut self) {
        if self.secure_blocked() {
            return;
        }
        self.end_capture();
        let layout = self.ports.monitors.layout();
        let cursor = self.ports.monitors.cursor();
        let Some(rect) = layout.monitor_at(cursor).or_else(|| layout.nearest(cursor)).map(|m| m.bounds) else {
            self.island(IslandEvent::new(IslandKind::Problem, "화면을 가져오지 못했어요"));
            return;
        };
        self.capture_rect_now(rect, layout);
    }

    /// 지금 화면을 찍어 F1 확정과 같은 길(`finish`)로 — 잠깐 스냅숏을 쥐었다가 버린다.
    fn capture_rect_now(&mut self, rect: PhysRect, layout: MonitorLayout) {
        let frames = match self.ports.screen.capture(&layout) {
            Ok(f) if !f.is_empty() => f,
            Err(e) if e.starts_with(NO_SCREEN_PERMISSION) => {
                log::warn!("capture failed: {e}");
                self.island(IslandEvent::new(IslandKind::Problem, "화면 기록 권한이 필요해요").detail("시스템 설정 › 개인정보 보호 및 보안 › 화면 기록에서 켜 주세요"));
                return;
            }
            other => {
                log::warn!("capture failed: {}", other.err().as_deref().unwrap_or("no frames"));
                self.island(IslandEvent::new(IslandKind::Problem, "화면을 가져오지 못했어요"));
                return;
            }
        };
        self.cap.cursor = self.ports.screen.cursor_sprite();
        let raw = self.ports.windows.windows();
        let excluded: Vec<ExcludedApp> = self.settings.get("capture.excluded").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        let windows = filter_windows(&raw, &layout, std::process::id(), &excluded);
        let cursor = self.ports.monitors.cursor();
        self.session_seq += 1;
        self.snapshot = Some(Snapshot { id: self.session_seq, kind: CaptureKind::Region, layout, windows, cursor });
        self.cap.frames = frames;
        self.finish(rect, false, false);
        // 고르기 모드가 아니라 `end_capture`가 비우지 않는다
        self.snapshot = None;
        self.cap.frames.clear();
        self.cap.cursor = None;
    }

    /// `Alt+F1`: 3·5·10초(`capture.delay`) 뒤 F1. 카운트다운 중 같은 키 = 취소.
    pub fn toggle_delay(&mut self) {
        if self.rec.delay.take().is_some() {
            self.release_delay_esc();
            self.island(IslandEvent::new(IslandKind::End, ""));
            self.island(IslandEvent::new(IslandKind::Info, "지연 캡처를 취소했어요"));
            return;
        }
        if self.secure_blocked() {
            return;
        }
        self.end_capture();
        let secs = self.settings.get("capture.delay").and_then(|v| v.as_u64()).unwrap_or(3).clamp(1, 60) as u32;
        self.rec.delay = Some(Delay { due_ms: self.clock_ms() + secs as u64 * 1000, shown: secs });
        self.hold_delay_esc();
        self.island(self.delay_island(secs));
    }

    /// 사용자 결정: 카운트다운 동안만 Esc를 전역으로 잡는다(실시간 확대 `z-exit`와 같은 방식). 칸은 `delay` —
    /// Esc와 Alt+F1이 같은 칸으로 들어와 둘 다 취소가 된다. 다른 앱이 Esc를 쥐고 있으면 Alt+F1로만 취소.
    fn hold_delay_esc(&mut self) {
        let combo = self.settings.shortcut("c-cancel").to_string();
        let combo = if combo.is_empty() { "Esc".to_string() } else { combo };
        let Some(accel) = to_accelerator(&combo, self.platform) else { return };
        if self.registered.values().any(|a| *a == accel) {
            return;
        }
        match self.ports.hotkeys.register("delay", &accel) {
            Ok(()) => self.rec.delay_esc = Some(accel),
            Err(e) => log::info!("delay cancel key not registered: {e}"),
        }
    }

    fn release_delay_esc(&mut self) {
        if let Some(accel) = self.rec.delay_esc.take() {
            self.ports.hotkeys.unregister(&accel);
        }
    }

    pub fn delay_pending(&self) -> Option<Delay> {
        self.rec.delay
    }

    fn delay_island(&self, left: u32) -> IslandEvent {
        let mut ev = IslandEvent::new(IslandKind::Mode, "초 뒤 범위 캡처").variant("delay");
        ev.countdown = Some(left);
        ev.mode = Some("delay".into());
        let key = if self.rec.delay_esc.is_some() { self.settings.shortcut("c-cancel") } else { self.settings.shortcut("delay") };
        if !key.is_empty() {
            ev.hints = vec![IslandHint { key: key.to_string(), label: "취소".into() }];
        }
        ev
    }

    /// 카운트다운 — 초가 바뀔 때만 섬을 다시, 0이 되면 섬을 거두고 F1(섬은 캡처 제외 창이라 결과에 안 찍힘).
    pub(crate) fn tick_delay(&mut self, now: u64) {
        let Some(d) = self.rec.delay else { return };
        if now >= d.due_ms {
            self.rec.delay = None;
            self.release_delay_esc();
            self.island(IslandEvent::new(IslandKind::End, ""));
            self.start_capture(CaptureKind::Region);
            return;
        }
        let left = (d.due_ms - now).div_ceil(1000) as u32;
        if left != d.shown {
            self.rec.delay = Some(Delay { shown: left, ..d });
            self.island(self.delay_island(left));
        }
    }
}
