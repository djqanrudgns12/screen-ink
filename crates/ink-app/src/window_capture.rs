//! F2 창 캡처(단계 6.1, R-WIN-1~3, 02 §2): 커서 아래 창 하나만 강조 → 클릭 = 그 창만(가려진 부분 포함).
//!
//! 사슬: 어댑터 WGC(첫 프레임 150 ms, 보호 화면이면 `WINDOW_PROTECTED`) → 검정·실패면 정지 스냅숏에서 창 사각형 자르기 →
//! 그것도 RGB 전부 0이면 "이 창은 가져올 수 없어요"(빈 이미지는 클립보드에 넣지 않는다). 고르는 동안 커서 아래 창이
//! 바뀔 때마다 어댑터가 item·pool·session을 미리 만들어 둔다(spikes §1: ≈15 ms 숨김).

use crate::capture::{CaptureInput, LastCapture};
use crate::ports::{Frame, WINDOW_PROTECTED};
use crate::service::InkService;
use crate::state::{CaptureView, IslandEvent, IslandKind};
use ink_domain::geometry::PhysPoint;
use ink_domain::image::{self, Blankness, FrameRef};
use ink_domain::mode::Mode;
use ink_domain::modekeys::{self, CaptureKeys};
use ink_domain::selection::{Button, Command, Input, Phase, SelectionView};
use ink_domain::settings::ExcludedApp;
use ink_domain::snapshot::{filter_windows, CaptureKind, WindowInfo};
use ink_domain::windowshot::{self, ShotNext};
use std::sync::Arc;
use std::time::Instant;

impl InkService {
    pub(crate) fn window_picking(&self) -> bool {
        matches!(self.mode, Mode::Capture { kind: CaptureKind::Window, .. }) && self.snapshot.is_some()
    }

    /// F2 오버레이 입력: 이동 = 창 강조, 왼쪽 클릭·Enter·툴바 = 찍기, 오른쪽 클릭·Esc = 취소.
    pub(crate) fn window_input(&mut self, input: CaptureInput) {
        let keys = CaptureKeys::from_lookup(|id| self.settings.shortcut(id).to_string());
        match Input::from(input) {
            Input::Move(p) => self.hover_window(p, false),
            Input::Down(_, Button::Right) => self.end_capture(),
            Input::Up(p, Button::Left) => {
                self.hover_window(p, false);
                self.shoot_hovered(false);
            }
            Input::Key(k) => {
                if modekeys::matches(&keys.cancel, &k) {
                    self.end_capture();
                } else if modekeys::matches(&keys.copy, &k) {
                    self.shoot_hovered(false);
                } else if modekeys::matches(&keys.save, &k) || modekeys::matches(&keys.quick, &k) {
                    self.shoot_hovered(true);
                }
            }
            Input::Command(Command::Cancel) => self.end_capture(),
            Input::Command(Command::Save | Command::QuickSave) => self.shoot_hovered(true),
            Input::Command(_) => self.shoot_hovered(false),
            _ => {}
        }
    }

    /// 커서 아래 창(덮개 의심 창 건너뜀)을 강조하고 창 캡처를 미리 준비한다. 바뀔 때만(`force` = 진입 직후).
    pub(crate) fn hover_window(&mut self, p: PhysPoint, force: bool) {
        let Mode::Capture { session, .. } = self.mode else { return };
        let Some(snap) = self.snapshot.as_ref() else { return };
        let hit = snap.window_at(p, true).map(|w| (w.handle, w.rect));
        let handle = hit.map(|(h, _)| h);
        if !force && handle == self.rec.hovered {
            return;
        }
        self.rec.hovered = handle;
        self.ports.screen.prepare_window(handle);
        let rect = hit.map(|(_, r)| r);
        self.ports.overlay.highlight(session, rect);
        let view = SelectionView { phase: Phase::Hover, rect, path: vec![self.t("island.window.pathWindow")], level: 0, guides: Vec::new(), magnet: true, cursor_included: false, help: false, evidence: Vec::new() };
        self.ports.notifier.capture(&CaptureView { session, selection: view });
    }

    fn shoot_hovered(&mut self, save: bool) {
        let Some(snap) = self.snapshot.as_ref() else { return };
        let Some(w) = self.rec.hovered.and_then(|h| snap.windows.iter().find(|w| w.handle == h)).cloned() else {
            self.island(IslandEvent::new(IslandKind::Info, self.t("island.window.none")));
            return;
        };
        self.shoot_window(w, save);
    }

    /// `Shift+F2`·F2 두 번(R-WIN-2): 지금 활성 창을 고르기 없이 바로.
    pub fn capture_active_window(&mut self) {
        if self.secure_blocked() {
            return;
        }
        let Some(handle) = self.ports.probe.foreground_window() else {
            self.island(IslandEvent::new(IslandKind::Info, self.t("island.window.noActive")));
            return;
        };
        let layout = self.ports.monitors.layout();
        let raw = self.ports.windows.windows();
        let excluded: Vec<ExcludedApp> = self.settings.get("capture.excluded").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        // 최소화·숨김·우리 창은 거른다(F2 목록과 같은 규칙)
        let Some(w) = filter_windows(&raw, &layout, std::process::id(), &excluded).into_iter().find(|w| w.handle == handle) else {
            self.island(IslandEvent::new(IslandKind::Info, self.t("island.window.noActive")));
            return;
        };
        self.shoot_window(w, false);
    }

    /// 찍기 → 결과(복사·저장·섬). 보호 화면이면 안내만.
    pub(crate) fn shoot_window(&mut self, w: WindowInfo, save: bool) {
        let t = Instant::now();
        match self.grab_window(&w) {
            Some((cw, ch, px, method)) => {
                // 진단: 방식·크기·시간만(창 제목 없음, R-LOG-1)
                log::info!("window shot {method} {cw}x{ch} in {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
                self.deliver_window(&w, cw, ch, px, method, save);
            }
            None => {
                log::info!("window shot refused: protected or blank");
                self.end_capture();
                self.island(IslandEvent::new(IslandKind::Problem, self.t("island.window.protected")).detail(self.t("island.window.protectedDetail")));
            }
        }
    }

    /// WGC → 정지 스냅숏(F2) 또는 지금 화면(Shift+F2) 자르기. 보호 화면이면 `None`.
    fn grab_window(&mut self, w: &WindowInfo) -> Option<(u32, u32, Vec<u8>, &'static str)> {
        match self.ports.screen.capture_window(w.handle, w.rect) {
            Ok(mut s) if s.width > 0 && s.height > 0 && image::blankness(&s.bgra) == Blankness::Content => {
                // 사용자 결정: 귀만 투명 — 창 안 반투명(아크릴 등)은 불투명으로, 둥근 모서리 바깥만 알파 0
                image::make_opaque(&mut s.bgra);
                if s.corner_radius > 0 {
                    image::mask_round_corners(&mut s.bgra, s.width, s.height, s.corner_radius);
                }
                return Some((s.width, s.height, s.bgra, s.method));
            }
            Err(e) if e.starts_with(WINDOW_PROTECTED) => return None,
            Ok(s) => log::info!("window capture {} gave a blank frame, cropping the snapshot", s.method),
            Err(e) => log::info!("window capture unavailable ({e}), cropping the snapshot"),
        }
        let layout = self.snapshot.as_ref().map(|s| s.layout.clone()).unwrap_or_else(|| self.ports.monitors.layout());
        let fresh: Vec<Frame>;
        let frames: &[Frame] = if self.cap.frames.is_empty() {
            fresh = self.ports.screen.capture(&layout).ok()?;
            &fresh
        } else {
            &self.cap.frames
        };
        let refs: Vec<FrameRef> = frames
            .iter()
            .filter_map(|f| layout.monitors.iter().find(|m| m.id == f.monitor_id).map(|m| FrameRef { bounds: m.bounds, w: f.width, h: f.height, bgra: &f.bgra }))
            .collect();
        let (cw, ch, px) = image::crop(&refs, w.rect);
        match windowshot::after_fallback(image::blankness(&px)) {
            ShotNext::Use => Some((cw, ch, px, "snapshot")),
            _ => None,
        }
    }

    /// F1 확정과 같은 결과 길(R-OUT): 화면을 먼저 돌려주고 → 클립보드·저장 → 결과 섬.
    fn deliver_window(&mut self, w: &WindowInfo, cw: u32, ch: u32, px: Vec<u8>, method: &str, save: bool) {
        let app = Some(w.exe.trim_end_matches(".exe").to_string()).filter(|s| !s.is_empty());
        let (tw, th, tpx) = image::thumbnail(cw, ch, &px, 160, 96);
        let thumb = self.ports.overlay.thumbnail(tw, th, tpx);
        let afterimage = match self.mode {
            Mode::Capture { session, .. } => self.snapshot.as_ref().map(|s| (session, s.layout.clone())),
            _ => None,
        };
        if self.snapshot.is_some() {
            // Shift+F1 "직전 범위"가 이 창 자리를 다시 찍을 수 있게
            self.remember_range(w.rect);
        }
        self.cap.last = Some(LastCapture { w: cw, h: ch, bgra: Arc::new(px), app, hex: None });
        self.end_capture();
        if let Some((session, layout)) = afterimage {
            self.ports.overlay.flash_afterimage(session, w.rect, &layout);
        }
        let result = self.settings.str("capture.result").to_string();
        let do_copy = !save && result != "save";
        let do_save = save || result == "save" || result == "both";
        let mut copied = false;
        if do_copy {
            let Some(last) = self.cap.last.clone() else { return };
            match self.ports.clipboard.set_image(last.w, last.h, &last.bgra) {
                Ok(()) => copied = true,
                Err(e) => {
                    log::warn!("clipboard: {e}");
                    self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.copyError")));
                    return;
                }
            }
        }
        if do_save {
            if let Some(mut ev) = self.save_last_inner() {
                if copied {
                    ev.text = self.t("island.capture.copiedAndSaved");
                }
                ev.thumb_url = thumb;
                self.island(ev);
            }
            return;
        }
        let save_key = self.key_text("c-save");
        let mut ev = IslandEvent::new(IslandKind::Result, self.t("island.window.copied"))
            .variant("result")
            .button(&self.t("island.action.save"), "capture.saveLast", save_key.as_deref());
        ev.value = Some(format!("{cw}×{ch}"));
        // 스냅숏 자르기는 가린 창까지 함께 찍힌다 — 그때는 "가려진 부분까지"라고 하지 않는다
        ev.detail = Some(if method == "snapshot" { self.t("island.capture.pasteHint") } else { self.t("island.window.occludedPasteHint") });
        ev.thumb_url = thumb;
        self.island(ev);
    }
}
