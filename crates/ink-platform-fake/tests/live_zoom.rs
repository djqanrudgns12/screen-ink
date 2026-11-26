//! 실시간 확대(단계 5, Alt+2): 진입 → 틱(따라가기) → 휠·확대 안 키 → 끝. 캡처와 전환·시스템 사건·실패.

use ink_app::{HotkeyOutcome, InkService, IslandKind, SystemEvent, ZoomInput};
use ink_domain::geometry::{PhysPoint, PhysRect};
use ink_domain::mode::Mode;
use ink_domain::shortcuts::Platform;
use ink_domain::snapshot::CaptureKind;
use ink_platform_fake::FakeWorld;
use serde_json::json;

const DT: f64 = 1.0 / 60.0;
const FHD: PhysRect = PhysRect::new(0, 0, 1920, 1080);
/// 확대 중 잡는 전역 키는 Esc(끝) 하나 — 방향키는 아래 앱(PPT 넘기기)으로 가야 한다(사용자 결정)
const ZOOM_KEYS: [&str; 1] = ["Escape"];
const ARROWS: [&str; 2] = ["ArrowUp", "ArrowDown"];

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.set_magnifier(w.magnifier_port());
    s.start();
    s
}

fn cursor(w: &FakeWorld, x: i32, y: i32) {
    w.screen.0.lock().unwrap().cursor = PhysPoint { x, y };
}

fn running(w: &FakeWorld) -> Option<(PhysRect, PhysRect, f64)> {
    w.magnifier.0.lock().unwrap().running
}

fn zooming(s: &InkService) -> bool {
    matches!(s.mode(), Mode::LiveZoom { .. })
}

#[test]
fn 쉬다가_alt2는_커서_모니터에_돋보기와_확대_섬을_띄우고_esc만_잠시_잡는다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let globals = w.registered_count();
    assert!(ZOOM_KEYS.iter().all(|k| !w.is_registered(k)), "쉬는 동안 방향키·Esc는 다른 앱 것");

    assert_eq!(s.on_hotkey("zoom"), HotkeyOutcome::Zoom);
    assert!(matches!(s.mode(), Mode::LiveZoom { monitor_id: 0, .. }));
    assert!(matches!(s.state().mode, Mode::LiveZoom { .. }));
    // 커서 (400, 300), ×2 → 원본 960×540, 왼쪽 가장자리에 가둠
    assert_eq!(running(&w), Some((FHD, PhysRect::new(0, 30, 960, 540), 2.0)));

    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.variant.as_deref(), island.mode.as_deref()), (IslandKind::Mode, Some("zoom"), Some("zoom")));
    assert_eq!(island.text, "실시간 확대");
    assert_eq!(island.detail.as_deref(), Some("움직이는 화면 그대로"));
    assert_eq!(island.value.as_deref(), Some("×2.0"));
    let keys: Vec<&str> = island.hints.iter().map(|h| h.key.as_str()).collect();
    assert_eq!(keys, ["휠", "Alt 1", "Esc"]);

    assert!(ZOOM_KEYS.iter().all(|k| w.is_registered(k)));
    assert!(ARROWS.iter().all(|k| !w.is_registered(k)), "방향키는 잡지 않는다");
    assert_eq!(w.registered_count(), globals + 1);

    let hud = w.last_zoom().flatten().expect("HUD 상태");
    assert_eq!((hud.monitor_label.as_str(), hud.scale_label.as_str(), hud.show_minimap), ("모니터 1", "×2.0", true));
    assert_eq!((hud.viewport.x, hud.viewport.w, hud.viewport.h), (0.0, 0.5, 0.5));
    let c = hud.cursor_css.expect("커서가 확대 화면 안");
    assert_eq!((c.x, c.y), (801.0, 541.0));
}

#[test]
fn 같은_키나_esc로_끝내면_돋보기와_확대_키를_푼다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let globals = w.registered_count();
    s.on_hotkey("zoom");
    assert_eq!(s.on_hotkey("zoom"), HotkeyOutcome::Zoom);
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(running(&w), None);
    assert_eq!(w.magnifier.0.lock().unwrap().stops, 1);
    assert_eq!(w.registered_count(), globals);
    assert!(ZOOM_KEYS.iter().all(|k| !w.is_registered(k)));
    assert_eq!(w.last_island().unwrap().kind, IslandKind::End);
    assert_eq!(w.last_zoom(), Some(None), "HUD를 거둔다");

    s.on_hotkey("zoom");
    assert_eq!(s.on_hotkey("z-exit"), HotkeyOutcome::Zoom);
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.magnifier.0.lock().unwrap().stops, 2);
    // 확대가 끝난 뒤의 Esc는 우리 것이 아니다
    assert_eq!(s.on_hotkey("z-exit"), HotkeyOutcome::Ignored);
    assert_eq!(s.zoom_handoff(), None);
}

#[test]
fn 휠은_단계를_오르내리고_커서_아래_점이_제자리() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("zoom");
    let at = |w: &FakeWorld| w.last_zoom().flatten().unwrap().cursor_css.unwrap();
    let before = at(&w);

    s.zoom_input(ZoomInput::Wheel { dy: -1 });
    assert_eq!(running(&w).unwrap().2, 2.5);
    assert_eq!(w.last_island().unwrap().value.as_deref(), Some("×2.5"));
    let after = at(&w);
    assert!((after.x - before.x).abs() <= 2.5 && (after.y - before.y).abs() <= 2.5, "{before:?} → {after:?}");

    s.zoom_input(ZoomInput::Wheel { dy: -1 });
    assert_eq!(running(&w).unwrap().2, 3.0);
    let after = at(&w);
    assert!((after.x - before.x).abs() <= 3.0 && (after.y - before.y).abs() <= 3.0, "{before:?} → {after:?}");
    s.zoom_input(ZoomInput::Wheel { dy: 1 });
    assert_eq!(running(&w).unwrap().2, 2.5);
    // 없어진 칸(z-in·z-out)과 확대 화면 안 다른 키 이름은 아무 일도 안 한다
    assert_eq!(s.on_hotkey("z-in"), HotkeyOutcome::Ignored);
    s.zoom_input(ZoomInput::Key { slot: "z-in".into() });
    assert_eq!(running(&w).unwrap().2, 2.5);

    for _ in 0..10 {
        s.zoom_input(ZoomInput::Wheel { dy: 1 });
    }
    assert_eq!(running(&w).unwrap().2, 1.25, "가장 작은 단계에서 멈춘다(끝내지 않음)");
    assert_eq!(w.last_island().unwrap().value.as_deref(), Some("×1.25"));
    assert!(zooming(&s));
    for _ in 0..10 {
        s.zoom_input(ZoomInput::Wheel { dy: -1 });
    }
    assert_eq!(running(&w).unwrap().2, 8.0);
    let src = running(&w).unwrap().1;
    assert!(FHD.intersect(&src) == Some(src), "원본은 모니터 안: {src:?}");

    s.zoom_input(ZoomInput::Key { slot: "z-exit".into() });
    assert_eq!(s.mode(), Mode::Idle);
    // 확대가 아닐 때의 입력은 무시
    s.zoom_input(ZoomInput::Wheel { dy: -1 });
    assert_eq!(w.magnifier.0.lock().unwrap().starts, 1);
}

#[test]
fn 틱은_부드럽게_따라가고_죽은_영역에서는_멈추고_설정은_바로_반영() {
    let w = FakeWorld::new();
    cursor(&w, 960, 540);
    let mut s = service(&w);
    s.on_hotkey("zoom");
    assert_eq!(running(&w).unwrap().1, PhysRect::new(480, 270, 960, 540));
    let sent = || w.notifier.0.lock().unwrap().zooms.len();

    // 죽은 영역(±48) 안 — 원본은 그대로, 틱마다 돋보기에 같은 값을 알린다(멈춤 감시 신호)
    cursor(&w, 1000, 540);
    assert!(s.tick_zoom_with(DT));
    assert_eq!(running(&w).unwrap().1, PhysRect::new(480, 270, 960, 540));
    assert_eq!(w.magnifier.0.lock().unwrap().updates.len(), 1);
    let n = sent();
    assert!(s.tick_zoom_with(DT));
    assert_eq!(sent(), n, "바뀐 것이 없으면 HUD에 다시 보내지 않는다");

    // 바깥 — 죽은 영역을 넘은 만큼(440 − 48)의 0.35
    cursor(&w, 1400, 540);
    s.tick_zoom_with(DT);
    let v = s.zoom_view().unwrap();
    assert!((v.center_x - (960.0 + 392.0 * 0.35)).abs() < 1e-9, "{}", v.center_x);
    assert_eq!(running(&w).unwrap().1.x, 617);

    // 바로 따라가기로 바꾸면 다음 틱부터
    s.update_setting("zoom.follow", &json!("instant"));
    s.tick_zoom_with(DT);
    assert_eq!(s.zoom_view().unwrap().center_x, 1400.0);
    assert_eq!(running(&w).unwrap().1.x, 920);

    // 위치 지도 끄기도 다음 틱에 HUD로
    s.update_setting("zoom.minimap", &json!(false));
    s.tick_zoom_with(DT);
    assert!(!w.last_zoom().flatten().unwrap().show_minimap);

    s.on_hotkey("zoom");
    assert!(!s.tick_zoom_with(DT), "끝났으면 틱을 멈추라고 알린다");
}

#[test]
fn 이중_진입_트레이로_다시_시작해도_돋보기와_확대_키는_하나() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let globals = w.registered_count();
    s.start_zoom();
    let Mode::LiveZoom { session: first, .. } = s.mode() else { panic!("확대 중이어야") };
    // 가짜 돋보기는 멈추지 않고 다시 띄우면 실패한다 → 서비스가 언제나 먼저 멈추는지 본다
    s.start_zoom();
    let Mode::LiveZoom { session: second, .. } = s.mode() else { panic!("다시 시작") };
    assert!(second > first);
    let m = w.magnifier.0.lock().unwrap();
    assert_eq!((m.starts, m.stops, m.running.is_some()), (2, 1, true));
    drop(m);
    assert_eq!(w.registered_count(), globals + 1, "확대 키(Esc)는 한 벌만");

    // 단축키로 끄고 켜기 반복
    for _ in 0..3 {
        s.on_hotkey("zoom");
        s.on_hotkey("zoom");
    }
    assert!(zooming(&s));
    assert_eq!(w.registered_count(), globals + 1);
}

#[test]
fn 확대_중_캡처_키는_확대를_끝내고_캡처로_캡처_중_alt2는_캡처를_끝내고_확대로() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("zoom");
    assert_eq!(s.on_hotkey("region"), HotkeyOutcome::Captured);
    assert!(matches!(s.mode(), Mode::Capture { kind: CaptureKind::Region, .. }));
    assert_eq!(running(&w), None, "캡처 전에 돋보기를 내린다(확대 화면이 찍히지 않게)");
    assert!(ZOOM_KEYS.iter().all(|k| !w.is_registered(k)));
    assert!(w.overlay.0.lock().unwrap().showing.is_some());

    assert_eq!(s.on_hotkey("zoom"), HotkeyOutcome::Zoom);
    assert!(zooming(&s));
    assert!(w.overlay.0.lock().unwrap().showing.is_none(), "캡처 정지 이미지를 버린다");
    assert!(running(&w).is_some());

    // 트레이 캡처도 확대를 끝낸다
    s.start_capture(CaptureKind::Window);
    assert!(matches!(s.mode(), Mode::Capture { kind: CaptureKind::Window, .. }));
    assert_eq!(running(&w), None);
}

#[test]
fn 잠금_절전_화면_구성_변경이면_확대를_끝낸다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    for ev in [SystemEvent::Lock, SystemEvent::Suspend] {
        s.on_hotkey("zoom");
        s.on_system_event(ev);
        assert_eq!(s.mode(), Mode::Idle, "{ev:?}");
        assert_eq!(running(&w), None);
    }
    s.on_hotkey("zoom");
    s.on_system_event(SystemEvent::Unlock);
    s.on_system_event(SystemEvent::ThemeChange);
    assert!(zooming(&s), "풀림·테마는 확대를 그대로 둔다");
    s.on_system_event(SystemEvent::DisplayChange);
    assert_eq!(s.mode(), Mode::Idle);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str()), (IslandKind::Info, "화면 구성이 바뀌어 확대를 끝냈어요"));
    assert_eq!(w.magnifier.0.lock().unwrap().stops, 3);
}

#[test]
fn 돋보기를_못_띄우면_문제_섬이고_모드와_키는_그대로() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let globals = w.registered_count();
    w.magnifier.0.lock().unwrap().fail_start = Some("MagInitialize failed".into());
    s.on_hotkey("zoom");
    assert_eq!(s.mode(), Mode::Idle);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str()), (IslandKind::Problem, "실시간 확대를 시작하지 못했어요"));
    assert_eq!(w.registered_count(), globals);

    // 돋보기 어댑터가 없는 곳(맥 1차) = 기본 NoMagnifier
    let w2 = FakeWorld::new();
    let mut plain = InkService::new(w2.ports(), Platform::Win, "test");
    plain.start();
    plain.on_hotkey("zoom");
    assert_eq!(plain.mode(), Mode::Idle);
    assert_eq!(w2.last_island().unwrap().text, "이 컴퓨터에서는 실시간 확대를 아직 쓸 수 없어요");

    // 보안 화면
    let w3 = FakeWorld::new();
    let mut sec = service(&w3);
    w3.probe.0.lock().unwrap().secure_desktop = true;
    sec.on_hotkey("zoom");
    assert_eq!(sec.mode(), Mode::Idle);
    assert_eq!(w3.magnifier.0.lock().unwrap().starts, 0);
}

#[test]
fn 돋보기가_스스로_멈추면_틱에서_확대를_끝낸다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let globals = w.registered_count();
    s.on_hotkey("zoom");
    w.magnifier.0.lock().unwrap().fail_update = true;
    assert!(!s.tick_zoom_with(DT));
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.registered_count(), globals);
    let islands = w.notifier.0.lock().unwrap().islands.clone();
    let tail: Vec<IslandKind> = islands.iter().rev().take(2).map(|i| i.kind).collect();
    assert_eq!(tail, [IslandKind::Problem, IslandKind::End]);
    assert_eq!(islands.last().unwrap().text, "실시간 확대를 끝냈어요");
}

#[test]
fn 단축키를_잠시_끄면_alt2는_무시하지만_트레이로_연_확대는_esc로_끝난다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.set_hotkeys_enabled(false);
    assert_eq!(s.on_hotkey("zoom"), HotkeyOutcome::Ignored);
    assert_eq!(s.mode(), Mode::Idle);
    s.start_zoom();
    assert!(zooming(&s));
    assert_eq!(s.on_hotkey("z-exit"), HotkeyOutcome::Zoom);
    assert_eq!(s.mode(), Mode::Idle);
}

#[test]
fn 확대_중_alt1은_확대를_끝내고_넘기고_핀_키는_확대를_둔다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("zoom");
    s.on_hotkey("pin");
    assert!(zooming(&s), "핀·타이머 같은 키는 확대를 그대로 둔다");
    // 5.3 전: Alt+1은 확대를 끝내고 판서 처리(지금은 어댑터의 "준비 중")로 넘긴다
    assert_ne!(s.on_hotkey("draw"), HotkeyOutcome::Zoom);
    assert!(!zooming(&s));
    assert_eq!(running(&w), None);
}

#[test]
fn 이어받기_값은_그_모니터_판서_초기_변환과_같다() {
    let w = FakeWorld::new();
    // 오른쪽 150% 모니터(1920, 0, 2880×1800) 가운데
    cursor(&w, 1920 + 1440, 900);
    let mut s = service(&w);
    s.on_hotkey("zoom");
    let h = s.zoom_handoff().expect("확대 중");
    assert_eq!((h.monitor_id, h.scale), (1, 2.0));
    assert_eq!(h.source, PhysRect::new(1920 + 720, 450, 1440, 900));
    assert_eq!((h.s, h.tx, h.ty), (2.0, -960.0, -600.0));
    let hud = w.last_zoom().flatten().unwrap();
    assert_eq!(hud.monitor_label, "모니터 2");
    // 후광 = CSS px(물리 / 1.5)
    let c = hud.cursor_css.unwrap();
    assert!((c.x - 1441.0 / 1.5).abs() < 1e-9, "{c:?}");
}

#[test]
fn 다른_앱이_esc를_잡고_있어도_확대는_되고_alt2로_끝낸다() {
    let w = FakeWorld::new();
    w.taken(&["Escape"]);
    let mut s = service(&w);
    let globals = w.registered_count();
    s.on_hotkey("zoom");
    assert!(zooming(&s));
    assert!(!w.is_registered("Escape") && ARROWS.iter().all(|k| !w.is_registered(k)));
    assert_eq!(w.registered_count(), globals);
    assert_eq!(s.on_hotkey("z-exit"), HotkeyOutcome::Ignored, "잡지 못한 키는 우리 것이 아니다");
    assert!(zooming(&s));
    s.on_hotkey("zoom");
    assert_eq!(w.registered_count(), globals);
}
