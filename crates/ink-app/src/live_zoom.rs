//! 실시간 확대(Alt+2, 단계 5): 진입 → 60 Hz 틱(배율 전환 + 커서 따라가기) → 휠 = 목표 배율 → 끝.
//! 휠은 목표 배율만 바꾸고 화면은 틱마다 그쪽으로 부드럽게 간다(커서 아래 점 고정). 1×까지 내려가도 끝내지 않는다.
//!
//! 판단은 `ink_domain::zoom`(순수), 창은 `Magnifier` 포트. 틱은 어댑터가 메인 스레드에서 부른다(`tick_zoom`).
//! 배율은 휠로만 바꾼다(어댑터 훅 — Shift+휠은 아래 앱으로). 확대 중 잠시 잡는 전역 키는 끝내기(`z-exit`, Esc) 하나뿐이고
//! 끝나면 바로 푼다(R-KEY-6). 방향키는 잡지 않는다 — 아래 앱(PPT 넘기기 등)이 받아야 한다(2026-11-26 사용자 결정).
//! 5.3 확대 중 판서는 원본 프레임과 확대 변환을 이어받는다. 원본이 없으면 돋보기를 숨기고 새로 찍는다.

use crate::draw::{DrawSource, DrawStart};
use crate::ports::{Magnifier, NoMagnifier, SystemEvent, ZOOM_UNSUPPORTED};
use crate::service::{HotkeyOutcome, InkService};
use crate::state::DrawViewTransform;
use crate::state::{IslandEvent, IslandKind, LiveZoomState};
use ink_domain::geometry::{PhysPoint, PhysRect};
use ink_domain::mode::{DrawKind, Mode};
use ink_domain::shortcuts::{spoken_for, to_accelerator};
use ink_domain::zoom::{self, FollowMode, LiveZoomView, ZoomCommand, LIVE_ZOOM_START, ZOOM_KEYS};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::Instant;

/// 틱 간격을 모를 때(첫 틱) 쓰는 값.
const FRAME_SEC: f64 = 1.0 / 60.0;

/// 확대 HUD·가짜 클라이언트가 넘기는 입력(packages/ink-client `ZoomInput`). 실제 앱의 휠은 어댑터 훅이 넘긴다.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ZoomInput {
    /// 마우스 휠 칸: 음수 위(더 확대) / 양수 아래(덜 확대). 한 칸 = ×2^(1/8), 격자 위에 선다
    Wheel { dy: i32 },
    /// 트랙패드 같은 연속 스크롤: 칸의 소수배(부호는 `Wheel`과 같음), 격자 없이 연속
    Scroll { notches: f64 },
    /// 확대 화면 안 키 칸: "z-exit"(끝). 다른 칸은 무시
    Key { slot: String },
    Exit,
}

/// 5.3 `Alt+1` 이어받기에 넘길 값: 모니터·원본 사각형·배율과 판서 초기 변환(A 시안 `DrawViewTransform`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomHandoff {
    pub monitor_id: u32,
    /// 돋보기가 비추던 원본(가상 화면 물리 px)
    pub source: PhysRect,
    pub scale: f64,
    /// 판서 `view = { s, tx, ty }`(CSS px, 문서 = 그 모니터 논리 px)
    pub s: f64,
    pub tx: f64,
    pub ty: f64,
}

pub struct ZoomState {
    pub(crate) magnifier: Box<dyn Magnifier>,
    pub(crate) view: Option<LiveZoomView>,
    /// 그 모니터 배율(CSS px 환산)
    pub(crate) monitor_scale: f64,
    pub(crate) monitor_index: usize,
    pub(crate) monitor_label: String,
    /// 마지막 틱의 커서(후광 자리)
    pub(crate) cursor: PhysPoint,
    /// 확대 중에만 잡은 키: 칸 → 가속기
    pub(crate) keys: BTreeMap<String, String>,
    pub(crate) last_tick: Option<Instant>,
    /// 마지막으로 보낸 HUD 상태(같으면 다시 보내지 않는다)
    pub(crate) last_sent: Option<LiveZoomState>,
    pub(crate) entered: Option<Instant>,
    pub(crate) ticks: u64,
}

impl Default for ZoomState {
    fn default() -> Self {
        ZoomState {
            magnifier: Box::new(NoMagnifier),
            view: None,
            monitor_scale: 1.0,
            monitor_index: 0,
            monitor_label: String::new(),
            cursor: PhysPoint::default(),
            keys: BTreeMap::new(),
            last_tick: None,
            last_sent: None,
            entered: None,
            ticks: 0,
        }
    }
}

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

impl InkService {
    /// 돋보기 어댑터를 꽂는다(기본 = `NoMagnifier`). `Ports`를 바꾸지 않으려고 따로 받는다.
    pub fn set_magnifier(&mut self, magnifier: Box<dyn Magnifier>) {
        self.end_zoom();
        self.zoom.magnifier = magnifier;
    }

    pub fn zoom_view(&self) -> Option<LiveZoomView> {
        self.zoom.view.filter(|_| self.zooming())
    }

    /// 5.3에 넘길 값(확대 중일 때만).
    pub fn zoom_handoff(&self) -> Option<ZoomHandoff> {
        let v = self.zoom_view()?;
        let (s, tx, ty) = zoom::to_draw_transform(&v, self.zoom.monitor_scale);
        Some(ZoomHandoff { monitor_id: v.monitor_id, source: v.source, scale: v.scale, s, tx, ty })
    }

    fn zooming(&self) -> bool {
        matches!(self.mode, Mode::LiveZoom { .. })
    }

    /// 트레이·명령에서 시작(단축키가 꺼져 있어도). 확대 중이면 새로 시작한다(캡처 메뉴와 같은 규칙).
    pub fn start_zoom(&mut self) {
        self.end_zoom();
        self.end_capture();
        // 판서 중이면 판서를 끝내고(보관은 늦은 끝으로) 확대로 — 판서 중 Alt+2와 같다(이어받기 일꾼이 찾은 버그)
        self.end_draw_for_switch();
        self.begin_zoom(Instant::now());
    }

    /// `on_hotkey_at`이 먼저 묻는다. `None` = 확대와 무관하거나 확대를 끝낸 뒤 원래 처리로 넘김.
    pub(crate) fn zoom_hotkey(&mut self, slot: &str, pressed: Instant) -> Option<HotkeyOutcome> {
        let cmd = zoom::on_hotkey(&self.mode, slot)?;
        // 전역 칸은 서비스가 등록한 것만(단축키를 잠시 껐으면 무시), 확대 안 키는 확대 중 잡은 것만
        let allowed = if ZOOM_KEYS.contains(&slot) { self.zoom.keys.contains_key(slot) } else { self.hotkeys_enabled && self.registered.contains_key(slot) };
        if !allowed {
            return None;
        }
        match cmd {
            ZoomCommand::Start => self.begin_zoom(pressed),
            ZoomCommand::SwitchFromCapture => {
                self.end_capture();
                self.begin_zoom(pressed);
            }
            ZoomCommand::End => self.end_zoom(),
            ZoomCommand::EndThenPass => {
                // 다음 모드가 곧 화면을 찍는다(F1·F2·F3·직전 판서 열기) — 돋보기가 사라진 뒤에 넘긴다
                self.end_zoom_settled();
                return None;
            }
            ZoomCommand::ToDraw => {
                self.zoom_to_draw(pressed);
                return Some(HotkeyOutcome::Draw);
            }
        }
        Some(HotkeyOutcome::Zoom)
    }

    /// 확대 중 판서는 그 모니터와 배율을 유지한다. 돋보기를 내리기 전에 표시 중인 원본을 확보한다.
    pub(crate) fn zoom_to_draw(&mut self, pressed: Instant) {
        let Some(h) = self.zoom_handoff() else { return };
        let frame = self.zoom.magnifier.original_frame();
        let t = Instant::now();
        self.end_zoom();
        let start = DrawStart {
            monitor_id: Some(h.monitor_id),
            frame,
            view: DrawViewTransform { s: h.s, tx: h.tx, ty: h.ty },
            source: DrawSource::Zoom { hide_ms: ms(t.elapsed()), reused: false },
        };
        self.begin_draw_with(DrawKind::Draw, pressed, start);
    }

    /// 확대를 끝내고 돋보기가 화면에서 사라질 때까지 기다린다(곧 정지 캡처를 할 때). 걸린 ms(확대 중이 아니었으면 0).
    pub(crate) fn end_zoom_settled(&mut self) -> f64 {
        if !self.zooming() {
            return 0.0;
        }
        let t = Instant::now();
        self.end_zoom();
        self.zoom.magnifier.wait_hidden();
        ms(t.elapsed())
    }

    /// 커서 모니터에 돋보기를 띄운다. 실패하면 모드는 그대로(쉬는 중).
    fn begin_zoom(&mut self, pressed: Instant) {
        if self.ports.probe.secure_desktop() {
            if !self.secure_notified {
                self.secure_notified = true;
                self.island(IslandEvent::new(IslandKind::Off, self.t("island.security")).variant("security"));
            }
            return;
        }
        let t = Instant::now();
        let layout = self.ports.monitors.layout();
        let cursor = self.ports.monitors.cursor();
        let Some(monitor) = layout.monitor_at(cursor).or_else(|| layout.nearest(cursor)).cloned() else {
            self.island(IslandEvent::new(IslandKind::Problem, self.t("island.zoom.noScreen")));
            return;
        };
        let index = layout.monitors.iter().position(|m| m.id == monitor.id).unwrap_or(0);
        let follow = FollowMode::from_setting(self.settings.str("zoom.follow"));
        let view = LiveZoomView::start(monitor.id, monitor.bounds, LIVE_ZOOM_START, cursor, follow);
        let probe_ms = ms(t.elapsed());

        let t = Instant::now();
        if let Err(e) = self.zoom.magnifier.start(monitor.bounds, view.source, view.scale) {
            log::warn!("live zoom start failed: {e}");
            let text = if e.starts_with(ZOOM_UNSUPPORTED) { self.t("island.zoom.unsupported") } else { self.t("island.zoom.startError") };
            self.island(IslandEvent::new(IslandKind::Problem, text));
            return;
        }
        let start_ms = ms(t.elapsed());

        // 확대 섬이 결과 섬을 덮는다 — [판서로]·되살리기로 쥐어 둔 프레임을 놓는다
        self.cap.handoff = None;
        self.clear_discarded_capture();
        self.session_seq += 1;
        let session = self.session_seq;
        self.mode = Mode::LiveZoom { session, monitor_id: monitor.id, started_ms: self.epoch.elapsed().as_millis() as u64 };
        self.zoom.view = Some(view);
        self.zoom.monitor_scale = monitor.scale;
        self.zoom.monitor_index = index;
        self.zoom.monitor_label = zoom::monitor_label_for(index, self.lang());
        self.zoom.cursor = cursor;
        self.zoom.last_tick = Some(Instant::now());
        self.zoom.last_sent = None;
        self.zoom.entered = Some(pressed);
        self.zoom.ticks = 0;
        self.register_zoom_keys();
        self.island(self.zoom_island(view.scale));
        self.emit_state();
        self.emit_zoom();
        // 계측 줄(05 §9): 숫자만. 진입 = 단축키 → 돋보기 첫 그림
        log::info!(
            "mode enter zoom [{} ms] (probe {}, magnifier {}, scale {}, monitors {})",
            ms(pressed.elapsed()).round() as i64,
            probe_ms.round() as i64,
            start_ms.round() as i64,
            view.scale,
            layout.monitors.len()
        );
    }

    /// 확대 끝(Alt+2·Esc·잠금·다른 모드 키). 확대 중이 아니면 아무 일도 없다.
    pub fn end_zoom(&mut self) {
        if !self.zooming() {
            return;
        }
        self.zoom.magnifier.stop();
        self.unregister_zoom_keys();
        // 녹화 중이면 녹화 상태로 돌아간다(단계 6)
        self.mode = self.rest_mode();
        if let Some(at) = self.zoom.entered.take() {
            log::info!("mode exit zoom ({} ticks, {:.1} s)", self.zoom.ticks, at.elapsed().as_secs_f64());
        }
        self.zoom.view = None;
        self.zoom.last_tick = None;
        self.zoom.last_sent = None;
        self.ports.notifier.zoom(None);
        self.island(IslandEvent::new(IslandKind::End, ""));
        self.emit_state();
    }

    /// 어댑터가 60 Hz로 부른다(메인 스레드). 확대 중이 아니면 `false` — 틱을 멈추면 된다.
    pub fn tick_zoom(&mut self) -> bool {
        let now = Instant::now();
        let dt = self.zoom.last_tick.map_or(FRAME_SEC, |t| now.saturating_duration_since(t).as_secs_f64());
        self.zoom.last_tick = Some(now);
        self.tick_zoom_with(dt)
    }

    /// 틱 하나(시험은 `dt_sec`를 직접 준다): 커서 읽기 → 따라가기 → 돋보기 갱신 → HUD(바뀐 때만).
    pub fn tick_zoom_with(&mut self, dt_sec: f64) -> bool {
        if !self.zooming() {
            return false;
        }
        let Some(mut view) = self.zoom.view else { return false };
        let cursor = self.ports.monitors.cursor();
        // 설정을 바꾸면 바로(R-SET-0)
        let follow = FollowMode::from_setting(self.settings.str("zoom.follow"));
        // 배율 전환(커서 아래 점 고정) → 따라가기. 전환이 끝나면 HUD 글자도 목표와 같아진다
        zoom::step_scale(&mut view, cursor, dt_sec);
        zoom::step_follow(&mut view, cursor, follow, dt_sec);
        self.zoom.view = Some(view);
        self.zoom.cursor = cursor;
        self.zoom.ticks += 1;
        if !self.push_view(view) {
            return false;
        }
        self.emit_zoom();
        true
    }

    /// HUD·가짜 클라이언트 입력.
    pub fn zoom_input(&mut self, input: ZoomInput) {
        if !self.zooming() {
            return;
        }
        match input {
            ZoomInput::Wheel { dy } => self.zoom_step(dy as f64, true),
            ZoomInput::Scroll { notches } => self.zoom_step(notches, false),
            ZoomInput::Key { slot } => {
                if slot == zoom::ZOOM_EXIT_SLOT {
                    self.end_zoom();
                }
            }
            ZoomInput::Exit => self.end_zoom(),
        }
    }

    /// 잠금·절전·화면 구성 변경이면 확대를 끝낸다(02 §7). `on_system_event`가 먼저 부른다.
    pub(crate) fn zoom_system_event(&mut self, ev: SystemEvent) {
        if !self.zooming() {
            return;
        }
        match ev {
            SystemEvent::Lock | SystemEvent::Suspend => self.end_zoom(),
            SystemEvent::DisplayChange => {
                self.end_zoom();
                self.island(IslandEvent::new(IslandKind::Info, self.t("island.zoom.displayChanged")));
            }
            SystemEvent::Unlock | SystemEvent::Resume | SystemEvent::ThemeChange => {}
        }
    }

    /// 목표 배율을 휠 칸만큼 옮긴다(화면은 틱이 부드럽게 따라간다). 1×·8×에서 멈춘다(끝내지 않음).
    /// 섬은 목표 배율을 바로 보인다 — 같은 글자면 다시 띄우지 않는다(트랙패드는 한 칸 안에서도 자주 온다).
    fn zoom_step(&mut self, notches: f64, snap: bool) {
        let Some(mut view) = self.zoom_view() else { return };
        let next = zoom::wheel_target(view.target, notches, snap);
        if (next - view.target).abs() < 1e-9 {
            return;
        }
        let before = zoom::format_live_scale(view.target);
        view.target = next;
        self.zoom.view = Some(view);
        if zoom::format_live_scale(next) != before {
            self.island(self.zoom_island(next));
        }
    }

    /// 돋보기에 지금 원본·배율. 돋보기가 스스로 멈췄으면 확대를 끝내고 `false`.
    fn push_view(&mut self, view: LiveZoomView) -> bool {
        match self.zoom.magnifier.update(view.source, view.scale) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("live zoom update failed: {e}");
                self.end_zoom();
                self.island(IslandEvent::new(IslandKind::Problem, self.t("island.zoom.stopped")).detail(self.t("island.zoom.stoppedDetail")));
                false
            }
        }
    }

    /// 확대 섬(04 S13): 확대 아이콘 · **화면 확대** · 움직이는 화면 그대로 · `×2.0` / 휠 확대·축소 · Alt 1 멈추고 그리기 · Esc 끝.
    pub(crate) fn zoom_island(&self, scale: f64) -> IslandEvent {
        let lang = self.lang();
        let draw = self.settings.shortcut("draw");
        let exit = self.settings.shortcut(zoom::ZOOM_EXIT_SLOT);
        let mut hints: Vec<(String, String)> = vec![(self.t("island.hint.key.wheel"), self.t("island.zoom.hint.scale"))];
        if !draw.is_empty() {
            hints.push((spoken_for(draw, self.platform, lang), self.t("island.zoom.hint.freezeDraw")));
        }
        if !exit.is_empty() {
            hints.push((spoken_for(exit, self.platform, lang), self.t("island.zoom.hint.exit")));
        }
        let pairs: Vec<(&str, &str)> = hints.iter().map(|(k, l)| (k.as_str(), l.as_str())).collect();
        let mut ev = IslandEvent::new(IslandKind::Mode, self.t("island.zoom.title")).variant("zoom").detail(self.t("island.zoom.detail")).hints(&pairs);
        ev.mode = Some(zoom::ZOOM_SLOT.into());
        ev.value = Some(zoom::format_live_scale(scale));
        ev
    }

    /// HUD 상태(바뀐 때만 보낸다).
    pub(crate) fn emit_zoom(&mut self) {
        let (Mode::LiveZoom { session, .. }, Some(view)) = (self.mode, self.zoom.view) else { return };
        let label = zoom::monitor_label_for(self.zoom.monitor_index, self.lang());
        self.zoom.monitor_label = label.clone();
        let state = LiveZoomState {
            session,
            monitor_id: view.monitor_id,
            monitor_label: label,
            scale: view.scale,
            scale_label: zoom::format_live_scale(view.scale),
            viewport: zoom::viewport(&view),
            cursor_css: zoom::cursor_css(&view, self.zoom.cursor, self.zoom.monitor_scale),
            show_minimap: self.settings.bool("zoom.minimap"),
        };
        if self.zoom.last_sent.as_ref() == Some(&state) {
            return;
        }
        self.ports.notifier.zoom(Some(&state));
        self.zoom.last_sent = Some(state);
    }

    /// 확대 안 키(Esc 하나)를 잠시 잡는다. 비운 칸·전역 칸과 같은 키·운영체제가 거절한 키는 건너뛴다(로그만).
    fn register_zoom_keys(&mut self) {
        for id in ZOOM_KEYS {
            let combo = self.settings.shortcut(id).to_string();
            if combo.is_empty() {
                continue;
            }
            let Some(accel) = to_accelerator(&combo, self.platform) else { continue };
            if self.registered.values().any(|a| *a == accel) {
                continue;
            }
            match self.ports.hotkeys.register(id, &accel) {
                Ok(()) => {
                    self.zoom.keys.insert(id.to_string(), accel);
                }
                Err(e) => log::info!("zoom key {id} not registered: {e}"),
            }
        }
    }

    fn unregister_zoom_keys(&mut self) {
        for (_, accel) in std::mem::take(&mut self.zoom.keys) {
            self.ports.hotkeys.unregister(&accel);
        }
    }
}
