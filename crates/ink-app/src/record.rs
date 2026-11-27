//! F3 화면 녹화(단계 6.3·6.5, 05 §8 `RecordingSession`, 02 §5).
//!
//! F3 → F1과 같은 범위 고르기(모니터 클릭·창 클릭 = 창 추적·드래그 = 범위) → 확정 즉시 녹화. 녹화는 모드와 겹칠 수 있다:
//! 녹화 중에도 F1·판서·확대에 들어가고, 그 모드가 끝나면 `Mode::Recording`으로 돌아온다(`rest_mode`).
//! 잠금·절전·녹화하던 모니터가 사라지면 자동 중지·저장, 마무리하지 못한 임시 파일은 다음 시작 때 복구를 제안한다.

use crate::ports::{MicAccess, NoRecorder, RecStartOpts, Recorder, SystemEvent, REC_ENCODER_FAILED, REC_UNSUPPORTED, REC_WINDOW_CLOSED};
use crate::service::{HotkeyOutcome, InkService};
use crate::state::{IslandButton, IslandEvent, IslandKind, Recording};
use ink_domain::geometry::{MonitorLayout, PhysPoint, PhysRect};
use ink_domain::mode::Mode;
use ink_domain::naming;
use ink_domain::recording::{self, Leftover, RecPhase, RecTarget};
use ink_domain::selection::{Effect, Phase};
use ink_domain::shortcuts::Platform;
use ink_domain::snapshot::CaptureKind;
use std::path::PathBuf;

/// 녹화 섬 경과를 다시 그리는 간격.
pub const STATUS_EVERY_MS: u64 = 1000;
/// 첫 녹화 안내("판서는 영상에 함께 · 이 알약은 안 찍혀요")를 보이는 시간(04 S11).
const FIRST_HINT_MS: u64 = 2000;
/// 공간 부족·마이크 안내를 보이는 동안 녹화 섬으로 덮지 않는다.
const NOTICE_MS: u64 = 3000;

/// 자동 중지 이유.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StopReason {
    Lock,
    Suspend,
    DisplayChanged,
    WindowClosed,
    Stalled,
}

impl StopReason {
    fn key(self) -> &'static str {
        match self {
            Self::Lock => "island.record.reason.lock",
            Self::Suspend => "island.record.reason.suspend",
            Self::DisplayChanged => "island.record.reason.display",
            Self::WindowClosed => "island.record.reason.windowClosed",
            Self::Stalled => "island.record.reason.stalled",
        }
    }
}

/// 녹화·바로 캡처·F2 고르기의 서비스 쪽 상태(단계 6). 녹화기는 어댑터가 `set_recorder`로 끼운다.
pub struct RecordState {
    pub(crate) recorder: Box<dyn Recorder>,
    pub(crate) phase: RecPhase,
    pub(crate) target: Option<RecTarget>,
    pub(crate) temp: Option<PathBuf>,
    /// 서비스 시계(ms)로 녹화를 시작한 때
    pub(crate) started_ms: u64,
    pub(crate) last_status_ms: u64,
    /// 이때까지는 상태 틱이 녹화 섬을 다시 띄우지 않는다(안내가 보이는 중)
    pub(crate) quiet_until: u64,
    pub(crate) size: (u32, u32),
    pub(crate) fps: u32,
    pub(crate) mic_on: bool,
    /// 창 추적 녹화 중 마지막으로 알린 꺾쇠 범위(`None` = 아직 안 알림, `Some(None)` = 숨김)
    pub(crate) tracked_rect: Option<Option<PhysRect>>,
    /// 모니터·범위 녹화의 모니터 경계 — 배치가 바뀌어 이 경계가 사라지면 자동 중지
    pub(crate) monitor_bounds: Option<PhysRect>,
    /// 파일 이름 {app}(창 추적일 때만)
    pub(crate) app: Option<String>,
    pub(crate) leftovers: Vec<Leftover>,
    pub(crate) leftovers_pending: bool,
    pub(crate) mic_notice_done: bool,
    /// 지연 캡처 카운트다운(R-CAP-13 `Alt+F1`)
    pub(crate) delay: Option<crate::quick::Delay>,
    /// 카운트다운 동안만 잡은 Esc(가속기) — 끝나면 놓는다
    pub(crate) delay_esc: Option<String>,
    /// F2 고르기 중 커서 아래 창(준비해 둔 창 캡처)
    pub(crate) hovered: Option<u64>,
}

impl Default for RecordState {
    fn default() -> Self {
        RecordState {
            recorder: Box::new(NoRecorder),
            phase: RecPhase::Idle,
            target: None,
            temp: None,
            started_ms: 0,
            last_status_ms: 0,
            quiet_until: 0,
            size: (0, 0),
            fps: 30,
            mic_on: false,
            tracked_rect: None,
            monitor_bounds: None,
            app: None,
            leftovers: Vec::new(),
            leftovers_pending: false,
            mic_notice_done: false,
            delay: None,
            delay_esc: None,
            hovered: None,
        }
    }
}

impl InkService {
    /// 녹화 어댑터를 끼운다(Windows = MF + WGC). 끼우는 순간 끝나지 않은 녹화를 찾아 둔다 — 첫 틱에 섬으로 알린다.
    pub fn set_recorder(&mut self, recorder: Box<dyn Recorder>) {
        self.rec.recorder = recorder;
        self.rec.leftovers = self.rec.recorder.leftovers();
        self.rec.leftovers_pending = !self.rec.leftovers.is_empty();
        if self.rec.leftovers_pending {
            log::info!("{} unfinished recording(s) found", self.rec.leftovers.len());
        }
    }

    /// 서비스 단조 시계(ms) — 틱·카운트다운 시험이 같은 시계로 시간을 앞당긴다.
    pub fn clock_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    pub fn recording_active(&self) -> bool {
        matches!(self.rec.phase, RecPhase::Starting | RecPhase::Recording)
    }

    /// 끝나지 않은 녹화 목록(시험·설정 화면).
    pub fn leftover_recordings(&self) -> &[Leftover] {
        &self.rec.leftovers
    }

    /// 마이크 장치 목록(설정 창, R-REC-3).
    pub fn audio_inputs(&self) -> Vec<(String, String)> {
        self.rec.recorder.audio_inputs()
    }

    /// 모드가 끝났을 때 돌아갈 곳 — 녹화 중이면 녹화.
    pub(crate) fn rest_mode(&self) -> Mode {
        if self.recording_active() {
            Mode::Recording { started_ms: self.rec.started_ms }
        } else {
            Mode::Idle
        }
    }

    /// 단계 6 키: 녹화 중 F3 = 중지, 바로 캡처 4종. 처리했으면 `Some`.
    pub(crate) fn stage6_hotkey(&mut self, slot: &str) -> Option<HotkeyOutcome> {
        match slot {
            "record" if self.recording_active() => {
                self.stop_recording();
                Some(HotkeyOutcome::Captured)
            }
            "last" | "full" | "active" | "delay" => {
                self.quick_hotkey(slot);
                Some(HotkeyOutcome::Captured)
            }
            _ => None,
        }
    }

    /// 캡처 모드에 막 들어왔다(`begin_capture` 끝).
    pub(crate) fn after_entry(&mut self, kind: CaptureKind, cursor: PhysPoint) {
        match kind {
            CaptureKind::Window => {
                self.rec.hovered = None;
                self.hover_window(cursor, true);
            }
            CaptureKind::Record => self.rec.recorder.prepare(),
            CaptureKind::Region => {}
        }
    }

    /// 캡처 모드가 끝났다(`end_capture`) — 준비해 둔 창 캡처를 버린다.
    pub(crate) fn after_capture_end(&mut self) {
        if self.rec.hovered.take().is_some() {
            self.ports.screen.prepare_window(None);
        }
    }

    pub(crate) fn record_picking(&self) -> bool {
        matches!(self.mode, Mode::Capture { kind: CaptureKind::Record, .. })
    }

    /// F3 고르기에서 범위가 정해지는 결과(클릭 확정·Enter·툴바)면 바로 녹화를 시작한다. 시작했으면 true.
    /// `before` = 입력 직전 (고르기 방식, 범위) — 창 단계에서 고른 그대로면 창 추적.
    pub(crate) fn record_effect(&mut self, effect: &Effect, before: (&'static str, Option<PhysRect>)) -> bool {
        let rect = match effect {
            Effect::Copy(r) | Effect::Pin(r) => *r,
            Effect::Save { rect, .. } => *rect,
            Effect::Redraw => match self.cap.selector.as_ref() {
                Some(s) if s.view().phase == Phase::Confirmed => match s.rect() {
                    Some(r) => r,
                    None => return false,
                },
                _ => return false,
            },
            _ => return false,
        };
        let hover = if before.1 == Some(rect) { before.0 } else { "drag" };
        self.begin_recording(rect, hover);
        true
    }

    fn begin_recording(&mut self, rect: PhysRect, hover: &str) {
        let Some(snap) = self.snapshot.as_ref() else { return };
        let center = PhysPoint { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 };
        let window = snap.window_at(center, true).map(|w| (w.handle, w.rect, w.exe.clone()));
        let layout = snap.layout.clone();
        let target = recording::target_for(hover, rect, window.as_ref().map(|(h, r, _)| (*h, *r)), &layout);
        let app = match (&target, &window) {
            (Some(RecTarget::Window { .. }), Some((_, _, exe))) => Some(exe.trim_end_matches(".exe").to_string()).filter(|s| !s.is_empty()),
            _ => None,
        };
        // 고르기 화면(정지 화면·오버레이)은 캡처 제외가 아니다 → 먼저 숨긴다(첫 프레임은 어댑터가 잠깐 기다려 받는다)
        self.end_capture();
        match target {
            Some(t) => {
                self.rec.app = app;
                self.start_recording(t, &layout);
            }
            None => self.island(IslandEvent::new(IslandKind::Problem, self.t("island.record.noTarget"))),
        }
    }

    /// 대상이 정해진 녹화 시작(05 §8 C): 공간 확인 → 임시 파일 → 인코더 → 트레이 빨간 점·녹화 섬.
    pub fn start_recording(&mut self, target: RecTarget, layout: &MonitorLayout) {
        if self.rec.phase != RecPhase::Idle {
            return;
        }
        self.clear_discarded_capture();
        let fps = self.settings.get("record.fps").and_then(|v| v.as_u64()).unwrap_or(30) as u32;
        let r = target.rect();
        let (w, h) = recording::even_size(r.w.max(0) as u32, r.h.max(0) as u32);
        let folder = self.settings.str("output.folder").to_string();
        let warn = recording::space_warning(self.rec.recorder.free_bytes(&folder), w, h, fps);
        let Some(dir) = self.rec.recorder.temp_dir() else {
            self.island(IslandEvent::new(IslandKind::Problem, self.t("island.record.unsupported")));
            return;
        };
        let id = self.ports.clock.unix() * 1000 + self.clock_ms() % 1000;
        let temp = dir.join(recording::temp_name(id));
        let mic_wanted = self.settings.bool("record.mic");
        let mic_supported = self.rec.recorder.mic_supported();
        let mic_access = if mic_wanted && mic_supported {
            let access = self.rec.recorder.mic_access();
            if access == MicAccess::NotDetermined {
                self.rec.recorder.request_mic_access();
            }
            access
        } else {
            MicAccess::Granted
        };
        let mic_on = mic_wanted && mic_supported && mic_access == MicAccess::Granted;
        let opts = RecStartOpts {
            target: target.clone(),
            temp_path: temp.clone(),
            fps,
            bitrate: recording::bitrate(w, h, fps),
            mic_device: mic_on.then(|| self.settings.str("record.micDevice").to_string()),
            capture_cursor: self.settings.bool("record.cursor"),
        };
        self.rec.phase = RecPhase::Starting;
        if let Err(e) = self.rec.recorder.start(opts) {
            log::warn!("recording start failed: {e}");
            self.rec.phase = RecPhase::Idle;
            self.rec.app = None;
            let text = if e.starts_with(REC_UNSUPPORTED) { self.t("island.record.unsupported") } else { self.t("island.record.startError") };
            self.island(IslandEvent::new(IslandKind::Problem, text));
            return;
        }
        let actual_mic = if mic_on { self.rec.recorder.mic_active().unwrap_or(true) } else { false };
        #[cfg(windows)]
        let mic_access = if self.platform == Platform::Win && mic_on && !actual_mic { self.rec.recorder.mic_access() } else { mic_access };
        let now = self.clock_ms();
        self.rec.phase = RecPhase::Recording;
        self.rec.monitor_bounds = target.monitor_id().and_then(|id| layout.monitors.iter().find(|m| m.id == id)).map(|m| m.bounds);
        self.rec.target = Some(target.clone());
        self.rec.temp = Some(temp);
        self.rec.started_ms = now;
        self.rec.last_status_ms = now;
        self.rec.quiet_until = 0;
        self.rec.size = (w, h);
        self.rec.fps = fps;
        self.rec.mic_on = actual_mic;
        self.rec.tracked_rect = None;
        if self.mode == Mode::Idle {
            self.mode = Mode::Recording { started_ms: now };
        }
        // 계측 줄: 종류·크기만(창 제목·파일 이름 없음, R-LOG-1)
        log::info!("recording started: {} {w}x{h} {fps}fps", target.kind_name());
        self.emit_state();
        if let Some(update) = self.rec.recorder.take_window_rect() {
            self.on_tracked_window_rect(update);
        }

        if !self.settings.bool("record.firstHintDone") {
            let mut ev = self.record_island(0);
            ev.text = self.t("island.record.firstHint");
            self.island(ev);
            self.rec.quiet_until = now + FIRST_HINT_MS;
            if self.update_setting("record.firstHintDone", &serde_json::Value::Bool(true)).ok {
                // 설정 창 되돌리기 기록에 남기지 않는다(사용자가 바꾼 값이 아님)
                self.history.pop();
            }
        } else {
            self.island(self.record_island(0));
        }
        if mic_wanted && !mic_supported && !self.rec.mic_notice_done {
            self.rec.mic_notice_done = true;
            self.island(IslandEvent::new(IslandKind::Info, self.t("island.record.micPending")).detail(self.t("island.record.videoOnly")));
            self.rec.quiet_until = self.rec.quiet_until.max(now + NOTICE_MS);
        } else if mic_wanted && mic_supported && mic_access != MicAccess::Granted {
            let mut ev = IslandEvent::new(IslandKind::Info, self.t("island.record.micDenied")).detail(self.t("island.record.videoOnly"));
            if self.platform == Platform::Mac {
                ev = ev.button(&self.t("island.perm.openScreenBtn"), crate::ACTION_PERM_MIC, None);
            }
            #[cfg(windows)]
            if self.platform == Platform::Win {
                ev = ev.button(&self.t("island.perm.openScreenBtn"), crate::ACTION_PERM_MIC, None);
            }
            self.island(ev);
            self.rec.quiet_until = self.rec.quiet_until.max(now + NOTICE_MS);
        } else if mic_wanted && mic_supported && mic_access == MicAccess::Granted && !actual_mic {
            self.island(IslandEvent::new(IslandKind::Info, self.t("island.record.micUnavailable")).detail(self.t("island.record.videoOnly")));
            self.rec.quiet_until = self.rec.quiet_until.max(now + NOTICE_MS);
        }
        if let Some(m) = warn {
            self.island(IslandEvent::new(IslandKind::Info, self.tp("island.record.lowDisk", m)));
            self.rec.quiet_until = self.rec.quiet_until.max(now + NOTICE_MS);
        }
    }

    /// 녹화 섬(04 S11): 빨간 점 · 경과 · 크기 · fps · 마이크 꺼짐 · [■ 중지 F3].
    pub(crate) fn record_island(&self, elapsed_ms: u64) -> IslandEvent {
        let stats = self.rec.recorder.stats();
        let (w, h) = if stats.width > 0 { (stats.width, stats.height) } else { self.rec.size };
        let key = self.key_text("record");
        let mut ev = IslandEvent::new(IslandKind::Mode, self.t("island.record.recording"))
            .variant("record")
            .button(&self.t("island.record.stopBtn"), "record.stop", key.as_deref());
        ev.value = Some(format!("{} · {w}×{h} · {}fps", recording::clock_label(elapsed_ms), self.rec.fps));
        ev.mode = Some("record".into());
        ev.mic_off = !self.rec.mic_on;
        ev
    }

    /// 상태(트레이 빨간 점·녹화 섬·꺾쇠 창이 읽는다).
    pub(crate) fn recording_view(&self) -> Recording {
        let active = self.recording_active();
        let mic_supported = self.rec.recorder.mic_supported();
        if !active {
            return Recording { phase: self.rec.phase, mic_supported, ..Default::default() };
        }
        let stats = self.rec.recorder.stats();
        let (w, h) = if stats.width > 0 { (stats.width, stats.height) } else { self.rec.size };
        Recording {
            active,
            elapsed_ms: self.clock_ms().saturating_sub(self.rec.started_ms),
            phase: self.rec.phase,
            bytes: stats.bytes,
            width: w,
            height: h,
            fps: self.rec.fps,
            mic_on: self.rec.mic_on,
            mic_supported,
            rect: self.rec.target.as_ref().map(|t| t.rect()),
            target: self.rec.target.as_ref().map(|t| t.kind_name()),
        }
    }

    /// 녹화 섬 [중지]·F3·트레이.
    pub fn stop_recording(&mut self) {
        self.finish_recording(None);
    }

    /// 트레이 "녹화 시작·중지"(`record.toggle`).
    pub fn toggle_record(&mut self) {
        if self.recording_active() {
            self.stop_recording();
        } else {
            self.start_capture(CaptureKind::Record);
        }
    }

    /// 마무리 → 이름 규칙으로 저장 폴더에 옮김 → "녹화를 저장했어요 · 길이 · 크기"(05 §8 U→C).
    fn finish_recording(&mut self, why: Option<StopReason>) {
        if !self.recording_active() {
            return;
        }
        self.rec.phase = RecPhase::Finalizing;
        let result = self.rec.recorder.stop();
        let temp = self.rec.temp.take();
        self.reset_recording();
        // 까닭 글은 미리 만들어 둔다(닫힘이 self를 빌린 채로 남지 않게)
        let why_text = why.map(|r| self.t(r.key()));
        let with_why = move |tail: String| match &why_text {
            Some(w) => format!("{w} · {tail}"),
            None => tail,
        };
        let stats = match (result, temp) {
            (Ok(stats), Some(temp)) => (stats, temp),
            (r, _) => {
                if let Err(e) = r {
                    log::warn!("recording finalize failed: {e}");
                }
                self.island(IslandEvent::new(IslandKind::Problem, self.t("island.record.finalizeError")).detail(with_why(self.t("island.record.retryNextLaunch"))));
                return;
            }
        };
        let (stats, temp) = stats;
        let app = self.rec.app.take();
        let stem = self.recording_stem(app);
        let folder = self.settings.str("output.folder").to_string();
        match self.rec.recorder.keep_file(&temp, &folder, &stem) {
            Ok(f) => {
                log::info!("recording saved: {} ms, {} bytes, {} frames", stats.elapsed_ms, stats.bytes, stats.frames);
                let title = self.t(if f.fell_back { "island.record.savedPictures" } else { "island.record.saved" });
                let mut ev = IslandEvent::new(IslandKind::Result, title).variant("result").button(&self.t("island.action.openFolder"), "folder.open", None);
                ev.value = Some(format!("{} · {}", recording::clock_label(stats.elapsed_ms), recording::size_label(stats.bytes)));
                ev.detail = Some(with_why(f.name));
                self.island(ev);
            }
            Err(e) => {
                log::warn!("recording move failed: {e}");
                self.island(IslandEvent::new(IslandKind::Problem, self.t("island.record.moveError")).detail(with_why(self.t("island.record.retryNextLaunch"))));
            }
        }
    }

    /// 녹화 섬 [버리기](05 §8 D).
    pub fn discard_recording(&mut self) {
        if !self.recording_active() {
            return;
        }
        self.rec.recorder.abort();
        self.rec.temp = None;
        self.rec.app = None;
        self.reset_recording();
        self.island(IslandEvent::new(IslandKind::Info, self.t("island.record.discarded")));
    }

    fn reset_recording(&mut self) {
        self.rec.phase = RecPhase::Idle;
        self.rec.target = None;
        self.rec.tracked_rect = None;
        self.rec.monitor_bounds = None;
        if matches!(self.mode, Mode::Recording { .. }) {
            self.mode = Mode::Idle;
            self.island(IslandEvent::new(IslandKind::End, ""));
        }
        self.emit_state();
    }

    /// 창 추적 녹화 중 대상 창의 화면 자리 변화(`Some(rect)` = 표시/이동, `None` = 최소화·다른 데스크톱/Space로 숨김).
    pub fn on_tracked_window_rect(&mut self, rect: Option<PhysRect>) {
        if self.rec.phase != RecPhase::Recording || !matches!(self.rec.target, Some(RecTarget::Window { .. })) {
            return;
        }
        if self.rec.tracked_rect == Some(rect) {
            return;
        }
        self.rec.tracked_rect = Some(rect);
        self.ports.notifier.record_frame(rect);
    }

    /// 녹화 파일 이름(캡처와 같은 규칙·같은 오늘 순번).
    fn recording_stem(&mut self, app: Option<String>) -> String {
        let mut t = self.ports.clock.local();
        let day = (t.year, t.month, t.day);
        if self.cap.save_day != day {
            self.cap.save_day = day;
            self.cap.save_seq = 0;
        }
        self.cap.save_seq += 1;
        t.n = self.cap.save_seq;
        t.app = app;
        naming::render(self.settings.str("output.template"), &t).unwrap_or_else(|_| "shookshok".into())
    }

    /// 잠금·절전·녹화하던 모니터가 사라짐 → 자동 중지·저장(05 §8, 02 §7).
    pub(crate) fn record_on_system_event(&mut self, ev: SystemEvent) {
        if !self.recording_active() {
            return;
        }
        match ev {
            SystemEvent::Lock => self.finish_recording(Some(StopReason::Lock)),
            SystemEvent::Suspend => self.finish_recording(Some(StopReason::Suspend)),
            SystemEvent::DisplayChange => {
                // WM_SETTINGCHANGE도 여기로 온다 — 그 모니터가 그대로면 계속 녹화
                if let Some(b) = self.rec.monitor_bounds {
                    if !self.ports.monitors.layout().monitors.iter().any(|m| m.bounds == b) {
                        self.finish_recording(Some(StopReason::DisplayChanged));
                    }
                }
            }
            _ => {}
        }
    }

    /// 어댑터가 주기적으로(250 ms) 부른다: 지연 캡처 카운트다운 · 복구 제안 · 녹화 섬 경과 · 녹화기 스스로 멈춤.
    pub fn tick(&mut self) {
        let now = self.clock_ms();
        self.tick_at(now);
    }

    pub fn tick_at(&mut self, now: u64) {
        self.tick_topmost();
        self.expire_recent(now);
        self.expire_discarded_capture();
        self.tick_delay(now);
        if self.rec.leftovers_pending && self.rec.phase == RecPhase::Idle && self.mode == Mode::Idle {
            self.rec.leftovers_pending = false;
            self.announce_leftovers();
        }
        if self.rec.phase != RecPhase::Recording {
            return;
        }
        if let Some(f) = self.rec.recorder.take_failure() {
            log::warn!("recording stopped by itself: {f}");
            if f.starts_with(REC_ENCODER_FAILED) {
                // 인코더가 처음부터 서지 못함 — 저장할 것이 없다(임시 파일은 어댑터가 지움)
                self.rec.recorder.abort();
                self.rec.temp = None;
                self.rec.app = None;
                self.reset_recording();
                self.island(IslandEvent::new(IslandKind::Problem, self.t("island.record.startError")));
                return;
            }
            let why = if f.starts_with(REC_WINDOW_CLOSED) { StopReason::WindowClosed } else { StopReason::Stalled };
            self.finish_recording(Some(why));
            return;
        }
        if let Some(update) = self.rec.recorder.take_window_rect() {
            self.on_tracked_window_rect(update);
        }
        if now.saturating_sub(self.rec.last_status_ms) >= STATUS_EVERY_MS {
            self.rec.last_status_ms = now;
            // 다른 모드(F1·판서)가 섬을 쓰는 중이거나 안내가 보이는 중이면 덮지 않는다
            if now >= self.rec.quiet_until && matches!(self.mode, Mode::Recording { .. }) {
                self.island(self.record_island(now.saturating_sub(self.rec.started_ms)));
            }
        }
    }

    /// "저장 못 한 녹화가 있어요 [저장 시도] [지우기]"(05 §8 R).
    fn announce_leftovers(&mut self) {
        let n = self.rec.leftovers.len();
        if n == 0 {
            return;
        }
        let text = self.tp("island.record.unfinished", n as u64);
        let mut ev = IslandEvent::new(IslandKind::Info, text).button(&self.t("island.record.recoverSaveBtn"), "record.recoverSave", None);
        ev.chip = Some(IslandButton { label: self.t("island.record.recoverDeleteBtn"), action: "record.recoverDelete".into() });
        if self.rec.leftovers.iter().any(|l| !l.playable) {
            ev.detail = Some(self.t("island.record.maybeUnplayable"));
        }
        self.island(ev);
    }

    /// 복구 섬 [저장 시도](`keep` = true) · [지우기](false).
    pub fn recover_leftovers(&mut self, keep: bool) {
        let list = std::mem::take(&mut self.rec.leftovers);
        self.rec.leftovers_pending = false;
        if list.is_empty() {
            return;
        }
        let folder = self.settings.str("output.folder").to_string();
        let (mut done, mut failed) = (0usize, 0usize);
        for l in &list {
            let path = PathBuf::from(&l.path);
            let r = if keep {
                let stem = self.recording_stem(None);
                self.rec.recorder.keep_file(&path, &folder, &stem).map(|_| ())
            } else {
                self.rec.recorder.delete_file(&path)
            };
            match r {
                Ok(()) => done += 1,
                Err(e) => {
                    failed += 1;
                    log::warn!("leftover recording {}: {e}", if keep { "keep" } else { "delete" });
                }
            }
        }
        let ev = match (keep, failed) {
            (_, f) if f > 0 && done == 0 => IslandEvent::new(IslandKind::Problem, self.t(if keep { "island.record.moveError" } else { "island.record.deleteError" })),
            (true, f) => {
                let text = self.tp("island.record.recovered", done as u64);
                let mut ev = IslandEvent::new(IslandKind::Result, text).variant("result").button(&self.t("island.action.openFolder"), "folder.open", None);
                if f > 0 {
                    ev.detail = Some(self.tp("island.record.movePartialFailed", f as u64));
                }
                ev
            }
            (false, _) => IslandEvent::new(IslandKind::Info, self.t("island.record.unfinishedDeleted")),
        };
        self.island(ev);
    }
}
