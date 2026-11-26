//! 이어받기(5.3 확대 → 판서 · 3e-handoff 결과 섬 [판서로]): 값 유지, 돋보기 숨김 → 화면 갱신 → 캡처 순서,
//! 프레임 재사용(캡처 0회), 다른 모니터·배율, 30초·모드 진입·화면 구성 변경 때 프레임 놓기.

use ink_app::{CaptureInput, DrawViewTransform, HotkeyOutcome, InkService, IslandKind, SystemEvent, ZoomInput};
use ink_domain::geometry::{Monitor, MonitorLayout, PhysPoint, PhysRect};
use ink_domain::mode::{DrawKind, Mode};
use ink_domain::recog::{Control, ElementCand};
use ink_domain::shortcuts::Platform;
use ink_domain::snapshot::RawWindow;
use ink_domain::zoom::fit_rect_view;
use ink_platform_fake::FakeWorld;
use serde_json::json;
use std::time::{Duration, Instant};

const DT: f64 = 1.0 / 60.0;
const FHD: PhysRect = PhysRect::new(0, 0, 1920, 1080);
/// 가짜 장면의 오른쪽 150% 모니터(물리 2880×1800 = 논리 1920×1200)
const RIGHT: PhysRect = PhysRect::new(1920, 0, 2880, 1800);
/// 모니터 0 창 1 안의 표 행(300×40) · 모니터 1 창 2 안의 표 칸(600×300 = 논리 400×200)
const ROW: PhysRect = PhysRect::new(300, 300, 300, 40);
const CELL: PhysRect = PhysRect::new(2400, 400, 600, 300);

fn world() -> FakeWorld {
    let w = FakeWorld::new();
    w.screen.0.lock().unwrap().windows.push(RawWindow {
        handle: 2,
        pid: 8,
        exe: "hwp.exe".into(),
        class: "HwpFrame".into(),
        rect: PhysRect::new(2200, 200, 2000, 1400),
        visible: true,
        ..Default::default()
    });
    let mut e = w.elements.0.lock().unwrap();
    e.by_window.insert(1, vec![ElementCand { rect: ROW, control: Control::DataItem, from_msaa: false }]);
    e.by_window.insert(2, vec![ElementCand { rect: CELL, control: Control::DataItem, from_msaa: false }]);
    drop(e);
    w
}

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.set_magnifier(w.magnifier_port());
    s.start();
    s
}

fn cursor(w: &FakeWorld, x: i32, y: i32) {
    w.screen.0.lock().unwrap().cursor = PhysPoint { x, y };
}

fn captures(w: &FakeWorld) -> usize {
    w.screen.0.lock().unwrap().captures
}

fn last_draw(w: &FakeWorld) -> (ink_app::DrawSession, Option<(u32, u32)>) {
    w.overlay.0.lock().unwrap().draws.last().cloned().expect("판서 창을 띄움")
}

fn view_of(s: f64, tx: f64, ty: f64) -> DrawViewTransform {
    DrawViewTransform { s, tx, ty }
}

/// 커서 자리에 강조를 두고 Enter(복사) — 결과 섬까지. 진입 자리에서는 모니터 전체라 손떨림 문턱보다 크게 옮긴다.
fn capture_at(s: &mut InkService, w: &FakeWorld, x: i32, y: i32) {
    cursor(w, x - 20, y - 20);
    s.on_hotkey("region");
    s.capture_input(CaptureInput::Move { x, y });
    s.capture_input(CaptureInput::Key { key: "Enter".into(), code: "Enter".into(), ctrl: false, shift: false, alt: false, meta: false });
    assert_eq!(s.mode(), Mode::Idle);
}

// ───────────────────────── 5.3 실시간 확대 → 판서 ─────────────────────────

#[test]
fn 확대중_alt1은_그_순간_배율과_위치를_그대로_판서로_이어받는다() {
    let w = world();
    cursor(&w, 960, 540);
    let mut s = service(&w);
    s.on_hotkey("zoom");
    // 휠 네 칸(2 × 2^(4/8) = 2√2배), 커서를 옮겨 따라가게 한 뒤의 값(배율 전환도 끝남)
    s.zoom_input(ZoomInput::Wheel { dy: -4 });
    cursor(&w, 1500, 700);
    for _ in 0..30 {
        s.tick_zoom_with(DT);
    }
    let h = s.zoom_handoff().expect("확대 중");
    assert!((h.scale - 2.0 * 2f64.sqrt()).abs() < 1e-9, "{}", h.scale);
    assert!(h.tx < 0.0 && h.ty < 0.0, "가운데가 아닌 곳: {h:?}");
    let before = captures(&w);

    assert_eq!(s.on_hotkey("draw"), HotkeyOutcome::Draw);
    assert!(matches!(s.mode(), Mode::Draw { board: false, monitor_id: 0, .. }));
    let (ds, frame) = last_draw(&w);
    assert_eq!(ds.initial_view, view_of(h.s, h.tx, h.ty), "배율·이동 유지(R-ZOOM-3)");
    assert_eq!(frame, Some((1920, 1080)), "그 모니터 한 장");
    assert_eq!(captures(&w), before + 1);
    // 세션 알림도 같은 값(오버레이 엔진이 이것으로 시작)
    let sent = w.notifier.0.lock().unwrap().draws.last().cloned().unwrap();
    assert_eq!(sent.initial_view, ds.initial_view);
    // 확대는 깨끗이 끝: 돋보기·HUD·확대 키(Esc)
    assert_eq!(w.magnifier.0.lock().unwrap().running, None);
    assert_eq!(w.last_zoom(), Some(None));
    assert!(!w.is_registered("Escape"));
    // 판서 섬은 처음부터 이어받은 배율
    let island = w.last_island().unwrap();
    assert_eq!((island.variant.as_deref(), island.value.as_deref()), (Some("draw"), Some("×2.83")));
}

#[test]
fn 돋보기를_숨기고_화면_갱신을_기다린_뒤에_그_모니터를_찍는다() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("zoom");
    w.timeline.clear();
    s.on_hotkey("draw");
    assert_eq!(w.timeline.events(), ["magnifier.stop", "magnifier.wait_hidden", "screen.capture", "overlay.show_draw"]);
    let m = w.magnifier.0.lock().unwrap();
    assert_eq!((m.stops, m.waits, m.on_screen), (1, 1, false), "찍을 때 돋보기는 화면에 없다");
}

#[test]
fn 진입_계측에_출처_zoom과_배율이_남는다() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("zoom");
    s.on_hotkey("draw");
    let line = s.draw_entry_line(Some(4.0)).expect("그리기 전");
    assert!(line.starts_with("mode enter draw ["), "{line}");
    assert!(line.contains("(from zoom, scale 2, hide "), "{line}");
    assert!(line.contains(", capture ") && line.ends_with(", ui paint 4)"), "{line}");
    let Mode::Draw { session, .. } = s.mode() else { panic!("판서 중") };
    assert!(s.draw_ready(session, Some(4.0)).is_some());
    assert_eq!(s.draw_entry_line(None), None, "한 번만");

    // 키로 연 판서는 예전 줄 그대로(출처 없음)
    s.on_hotkey("draw");
    s.on_hotkey("draw");
    let line = s.draw_entry_line(None).unwrap();
    assert!(line.contains("] (capture ") && !line.contains("from"), "{line}");
}

#[test]
fn 오른쪽_150퍼센트_모니터_확대는_커서가_옮겨가도_그_모니터로_이어받는다() {
    let w = world();
    cursor(&w, 1920 + 1440, 900);
    let mut s = service(&w);
    s.on_hotkey("zoom");
    let h = s.zoom_handoff().unwrap();
    assert_eq!((h.monitor_id, h.s, h.tx, h.ty), (1, 2.0, -960.0, -600.0));
    // 확대 중 커서가 왼쪽 모니터로 — 확대는 그 모니터에 머문다
    cursor(&w, 400, 300);
    s.tick_zoom_with(DT);
    s.on_hotkey("draw");
    let (ds, frame) = last_draw(&w);
    assert_eq!((ds.monitor.id, ds.monitor.scale, frame), (1, 1.5, Some((2880, 1800))));
    assert_eq!(ds.initial_view, view_of(2.0, -960.0, -600.0));
    assert_eq!(ds.cursor, PhysPoint { x: 400, y: 300 }, "펜 고리는 지금 커서");
}

#[test]
fn 확대중_트레이_판서도_이어받고_칠판_직전판서는_1배로_연다() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("zoom");
    s.zoom_input(ZoomInput::Wheel { dy: -2 });
    let h = s.zoom_handoff().unwrap();
    s.start_draw(DrawKind::Draw);
    assert_eq!(last_draw(&w).0.initial_view, view_of(h.s, h.tx, h.ty));

    // 칠판(Alt+3)은 확대를 끝내고 1× 빈 바탕
    let Mode::Draw { session, .. } = s.mode() else { panic!() };
    s.draw_finish(ink_app::DrawFinish { session, keep: false, kept: None });
    s.on_hotkey("zoom");
    w.timeline.clear();
    s.on_hotkey("board");
    assert!(matches!(s.mode(), Mode::Draw { board: true, .. }));
    let (ds, frame) = last_draw(&w);
    assert_eq!((ds.initial_view, frame), (DrawViewTransform::default(), None));
    assert!(!w.timeline.events().contains(&"screen.capture"));
}

#[test]
fn 확대중_캡처_키와_트레이_캡처도_돋보기가_사라진_뒤에_찍는다() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("zoom");
    w.timeline.clear();
    assert_eq!(s.on_hotkey("region"), HotkeyOutcome::Captured);
    assert_eq!(w.timeline.events(), ["magnifier.stop", "magnifier.wait_hidden", "screen.capture", "overlay.show"]);

    s.on_hotkey("region");
    s.on_hotkey("zoom");
    w.timeline.clear();
    s.start_capture(ink_domain::snapshot::CaptureKind::Window);
    assert_eq!(w.timeline.events(), ["magnifier.stop", "magnifier.wait_hidden", "screen.capture", "overlay.show"]);
    // 그냥 끝낼 때(Esc·Alt+2)는 기다리지 않는다
    s.end_capture();
    s.on_hotkey("zoom");
    w.timeline.clear();
    s.on_hotkey("z-exit");
    assert_eq!(w.timeline.events(), ["magnifier.stop"]);
}

#[test]
fn 확대중_alt1_캡처가_실패하면_확대는_끝나고_문제_섬() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("zoom");
    w.screen.0.lock().unwrap().capture_fails = true;
    s.on_hotkey("draw");
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.magnifier.0.lock().unwrap().running, None);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str()), (IslandKind::Problem, "화면을 가져오지 못했어요"));
    assert!(!w.is_registered("Escape"));
}

// ───────────────────────── 3e-handoff 결과 섬 [판서로] ─────────────────────────

#[test]
fn 결과_섬에_판서로_단추와_키캡() {
    let w = world();
    let mut s = service(&w);
    capture_at(&mut s, &w, 420, 320);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str()), (IslandKind::Result, "복사했어요"));
    assert_eq!(island.button.as_ref().unwrap().action, "capture.saveLast", "주 단추는 그대로 [저장]");
    let second = island.secondary_button.clone().expect("[그리기로]");
    assert_eq!((second.label.as_str(), second.action.as_str()), ("그리기로", "capture.toDraw"));
    assert_eq!(island.secondary_button_key.as_deref(), Some("Alt+1"));
    let v = serde_json::to_value(&island).unwrap();
    assert_eq!(v["secondaryButton"], json!({ "label": "그리기로", "action": "capture.toDraw" }));
    assert_eq!(v["secondaryButtonKey"], "Alt+1");
    assert!(s.capture_frame_held());

    // 판서 키를 비우면 키캡 없이 단추만
    s.set_shortcut("draw", "");
    capture_at(&mut s, &w, 420, 320);
    let island = w.last_island().unwrap();
    assert!(island.secondary_button.is_some() && island.secondary_button_key.is_none());
    assert!(serde_json::to_value(&island).unwrap().get("secondaryButtonKey").is_none());

    // 저장 결과 섬도 [폴더 열기] 옆에 [판서로]
    s.update_setting("capture.result", &json!("save"));
    capture_at(&mut s, &w, 420, 320);
    let island = w.last_island().unwrap();
    assert_eq!(island.button.as_ref().unwrap().action, "folder.open");
    assert_eq!(island.secondary_button.unwrap().action, "capture.toDraw");
}

#[test]
fn 결과_섬이_떠_있을_때_alt1은_판서로와_같고_지나면_보통_판서() {
    let w = world();
    let mut s = service(&w);
    capture_at(&mut s, &w, 420, 320);
    let before = captures(&w);
    let now = std::time::Instant::now();
    assert_eq!(s.on_hotkey_at("draw", now), HotkeyOutcome::Draw);
    assert_eq!(captures(&w), before, "재캡처 0회 — [판서로]와 같다");
    let (ds, _) = last_draw(&w);
    let (vs, tx, ty) = fit_rect_view(FHD, 1.0, ROW);
    assert_eq!(ds.initial_view, view_of(vs, tx, ty), "고른 범위가 꽉 차게");
    // 판서를 끝내고 다시: 결과 섬 시간이 지나면 보통 판서(1×, 새로 찍음)
    s.on_hotkey_at("draw", now);
    capture_at(&mut s, &w, 420, 320);
    let later = std::time::Instant::now() + ink_app::RESULT_ISLAND_TO_DRAW + std::time::Duration::from_millis(10);
    assert_eq!(s.on_hotkey_at("draw", later), HotkeyOutcome::Draw);
    let (ds, _) = last_draw(&w);
    assert_eq!(ds.initial_view.s, 1.0, "보통 판서는 1×");
}

#[test]
fn 판서로는_다시_찍지_않고_고른_범위가_꽉_차게_그_모니터_판서로() {
    let w = world();
    let mut s = service(&w);
    capture_at(&mut s, &w, 420, 320);
    let before = captures(&w);
    w.timeline.clear();

    s.capture_to_draw();
    assert_eq!(captures(&w), before, "재캡처 0회");
    assert!(!w.timeline.events().contains(&"screen.capture"));
    assert_eq!(w.timeline.events(), ["overlay.show_draw"]);
    assert!(matches!(s.mode(), Mode::Draw { board: false, monitor_id: 0, .. }));
    let (ds, frame) = last_draw(&w);
    assert_eq!(frame, Some((1920, 1080)), "그 모니터 전체 정지 프레임");
    let (vs, tx, ty) = fit_rect_view(FHD, 1.0, ROW);
    assert_eq!(ds.initial_view, view_of(vs, tx, ty));
    assert!(vs > 1.0, "{vs}");
    // 범위 가운데가 화면 가운데(가둘 필요 없는 자리)
    let cx = (450.0 * vs + tx, 320.0 * vs + ty);
    assert!((cx.0 - 960.0).abs() < 1e-6 && (cx.1 - 540.0).abs() < 1e-6, "{cx:?}");
    let island = w.last_island().unwrap();
    assert_eq!(island.variant.as_deref(), Some("draw"));
    assert_eq!(island.value.as_deref(), Some(format!("×{vs:.2}").as_str()));
    let line = s.draw_entry_line(None).unwrap();
    assert!(line.contains(&format!("(from capture, scale {vs}, frame reused, capture 0, ")), "{line}");
    // 한 번 쓰면 놓는다(섬은 판서 섬으로 바뀌었다)
    assert!(!s.capture_frame_held());
}

#[test]
fn 다른_모니터_150퍼센트_범위는_그_모니터_프레임과_배율로() {
    let w = world();
    let mut s = service(&w);
    capture_at(&mut s, &w, 2500, 500);
    assert_eq!(w.desk.0.lock().unwrap().images.last(), Some(&(600, 300)), "표 칸이 골라졌다");
    let before = captures(&w);
    s.capture_to_draw();
    assert_eq!(captures(&w), before);
    let (ds, frame) = last_draw(&w);
    assert_eq!((ds.monitor.id, ds.monitor.scale, frame), (1, 1.5, Some((2880, 1800))));
    let (vs, tx, ty) = fit_rect_view(RIGHT, 1.5, CELL);
    assert_eq!(vs, 4.22, "논리 400×200 → 1920×1200의 88%");
    assert_eq!(ds.initial_view, view_of(vs, tx, ty));
}

#[test]
fn 삼십초가_지나면_프레임을_놓고_판서로는_새로_찍어_같은_배율로() {
    let w = world();
    let mut s = service(&w);
    capture_at(&mut s, &w, 420, 320);
    assert!(!s.expire_capture_frame(), "아직 30초 전");
    let later = Instant::now() + ink_app::capture::CAPTURE_FRAME_TTL + Duration::from_secs(1);
    assert!(s.expire_capture_frame_at(later));
    assert!(!s.capture_frame_held());
    assert!(!s.expire_capture_frame_at(later), "한 번만");

    let before = captures(&w);
    s.capture_to_draw();
    assert_eq!(captures(&w), before + 1, "프레임이 없으면 지금 화면을 찍는다");
    let (vs, tx, ty) = fit_rect_view(FHD, 1.0, ROW);
    assert_eq!(last_draw(&w).0.initial_view, view_of(vs, tx, ty));
    let line = s.draw_entry_line(None).unwrap();
    assert!(line.contains("from capture,") && !line.contains("frame reused"), "{line}");

    // 놓기 타이머가 늦어도 30초 뒤의 클릭은 새로 찍는다
    let Mode::Draw { session, .. } = s.mode() else { panic!() };
    s.draw_finish(ink_app::DrawFinish { session, keep: false, kept: None });
    capture_at(&mut s, &w, 420, 320);
    let before = captures(&w);
    s.capture_to_draw_at(later);
    assert_eq!(captures(&w), before + 1);
}

#[test]
fn 다른_모드로_가거나_잠그면_판서로_프레임을_놓는다() {
    let w = world();
    let mut s = service(&w);
    let gone = |s: &mut InkService, w: &FakeWorld| {
        assert!(!s.capture_frame_held());
        let n = w.overlay.0.lock().unwrap().draws.len();
        s.capture_to_draw();
        assert_eq!(w.overlay.0.lock().unwrap().draws.len(), n, "판서를 열지 않는다");
        assert_eq!(w.last_island().unwrap().text, "그리기로 넘길 캡처가 없어요");
    };

    // 확대가 결과 섬을 덮는다 — 그 뒤 [판서로]는 아무것도 열지 않고 확대도 그대로
    capture_at(&mut s, &w, 420, 320);
    s.on_hotkey("zoom");
    gone(&mut s, &w);
    assert!(matches!(s.mode(), Mode::LiveZoom { .. }));
    s.on_hotkey("zoom");

    capture_at(&mut s, &w, 420, 320);
    s.on_hotkey("draw");
    let Mode::Draw { session, .. } = s.mode() else { panic!() };
    s.draw_finish(ink_app::DrawFinish { session, keep: false, kept: None });
    gone(&mut s, &w);

    capture_at(&mut s, &w, 420, 320);
    s.on_hotkey("region");
    assert!(!s.capture_frame_held(), "새 캡처가 덮는다");
    s.end_capture();

    for ev in [SystemEvent::Lock, SystemEvent::Suspend, SystemEvent::DisplayChange] {
        capture_at(&mut s, &w, 420, 320);
        s.on_system_event(ev);
        gone(&mut s, &w);
        s.on_system_event(SystemEvent::Unlock);
    }
}

#[test]
fn 모니터_구성이_바뀌었으면_지금_구성에서_새로_찍는다() {
    let w = world();
    let mut s = service(&w);
    capture_at(&mut s, &w, 420, 320);
    // 왼쪽 모니터 배율만 125%로(시스템 사건 전에 단추를 누른 경우)
    w.screen.0.lock().unwrap().layout = MonitorLayout::new(vec![
        Monitor { id: 0, bounds: FHD, work: PhysRect::new(0, 0, 1920, 1040), scale: 1.25, primary: true },
        Monitor { id: 1, bounds: RIGHT, work: PhysRect::new(1920, 0, 2880, 1728), scale: 1.5, primary: false },
    ]);
    let before = captures(&w);
    s.capture_to_draw();
    assert_eq!(captures(&w), before + 1, "옛 구성의 프레임은 쓰지 않는다");
    let (ds, _) = last_draw(&w);
    assert_eq!((ds.monitor.id, ds.monitor.scale), (0, 1.25));
    let (vs, tx, ty) = fit_rect_view(FHD, 1.25, ROW);
    assert_eq!(ds.initial_view, view_of(vs, tx, ty));
}

