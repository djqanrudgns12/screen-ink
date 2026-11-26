//! 고르기 흐름(단계 3): 입력 → 강조·보기 → 복사·저장·기록, 느린 앱 강등.

use ink_app::{CaptureInput, InkService, IslandKind};
use ink_domain::geometry::PhysRect;
use ink_domain::mode::Mode;
use ink_domain::recog::{Control, ElementCand};
use ink_domain::shortcuts::Platform;
use ink_platform_fake::FakeWorld;
use serde_json::json;

fn world() -> FakeWorld {
    let w = FakeWorld::new();
    // 창 1(100,100,1200,800) 안의 표와 행
    w.elements.0.lock().unwrap().by_window.insert(
        1,
        vec![
            ElementCand { rect: PhysRect::new(300, 300, 300, 40), control: Control::DataItem },
            ElementCand { rect: PhysRect::new(300, 260, 600, 400), control: Control::Table },
        ],
    );
    w
}

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.start();
    s
}

fn mv(x: i32, y: i32) -> CaptureInput {
    CaptureInput::Move { x, y }
}

fn key(code: &str) -> CaptureInput {
    CaptureInput::Key { key: code.into(), code: code.into(), ctrl: false, shift: false, alt: false, meta: false }
}

#[test]
fn 진입은_모니터_전체_움직이면_uia_요소로_강조() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("region");
    assert!(w.elements.0.lock().unwrap().requested.contains(&1), "커서 아래 창을 미리 깨운다");
    s.capture_input(mv(420, 320));
    let hl = w.overlay.0.lock().unwrap().highlights.last().copied().flatten();
    assert_eq!(hl, Some(PhysRect::new(300, 300, 300, 40)), "행");
    s.capture_input(CaptureInput::Wheel { dy: -1 });
    assert_eq!(w.overlay.0.lock().unwrap().highlights.last().copied().flatten(), Some(PhysRect::new(300, 260, 600, 400)), "표");
}

#[test]
fn enter는_복사하고_기록을_남기고_결과_섬() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("region");
    s.capture_input(mv(420, 320));
    s.capture_input(key("Enter"));
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.desk.0.lock().unwrap().images, vec![(300, 40)]);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str(), island.value.as_deref()), (IslandKind::Result, "복사했어요", Some("300×40")));
    assert_eq!(island.button.unwrap().action, "capture.saveLast");
    let file = w.saved().unwrap();
    assert_eq!(file["sections"]["history"]["data"]["entries"][0]["rect"]["w"], 300);
    // 다음 진입에서 R = 직전 범위
    s.on_hotkey("region");
    s.capture_input(key("KeyR"));
    assert_eq!(w.overlay.0.lock().unwrap().highlights.last().copied().flatten(), Some(PhysRect::new(300, 300, 300, 40)));
}

#[test]
fn 단색이면_hex_칩() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("region");
    s.capture_input(key("Enter")); // 모니터 전체 — 가짜 이미지는 전부 0(검정)
    let island = w.last_island().unwrap();
    assert_eq!(island.chip.unwrap().label, "#000000 복사");
    s.copy_last_hex();
    assert_eq!(w.desk.0.lock().unwrap().texts, vec!["#000000".to_string()]);
}

#[test]
fn ctrl_s는_이름_규칙으로_저장하고_순번이_오른다() {
    let w = world();
    let mut s = service(&w);
    s.update_setting("output.template", &json!("수업_{yyyy}{MM}{dd}_{n}"));
    for _ in 0..2 {
        s.on_hotkey("region");
        s.capture_input(CaptureInput::Key { key: "s".into(), code: "KeyS".into(), ctrl: true, shift: false, alt: false, meta: false });
    }
    let saved = w.desk.0.lock().unwrap().saved.clone();
    assert_eq!(saved.iter().map(|x| x.1.as_str()).collect::<Vec<_>>(), vec!["수업_20260304_1", "수업_20260304_2"]);
    assert_eq!(w.last_island().unwrap().text, "사진 폴더에 저장했어요", "폴더를 안 정했으면 사진 폴더");
}

#[test]
fn 결과_설정이_둘_다면_복사하고_저장() {
    let w = world();
    let mut s = service(&w);
    s.update_setting("capture.result", &json!("both"));
    s.on_hotkey("region");
    s.capture_input(key("Enter"));
    assert_eq!(w.desk.0.lock().unwrap().images.len(), 1);
    assert_eq!(w.desk.0.lock().unwrap().saved.len(), 1);
    assert_eq!(w.last_island().unwrap().text, "복사하고 저장했어요");
}

#[test]
fn 클립보드가_잠기면_알린다() {
    let w = world();
    w.desk.0.lock().unwrap().clipboard_locked = true;
    let mut s = service(&w);
    s.on_hotkey("region");
    s.capture_input(key("Enter"));
    assert_eq!(w.last_island().unwrap().text, "클립보드를 쓸 수 없어요");
}

#[test]
fn esc와_우클릭은_취소_색인은_버린다() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("region");
    s.capture_input(CaptureInput::Down { x: 10, y: 10, button: 2 });
    assert_eq!(s.mode(), Mode::Idle);
    assert!(w.elements.0.lock().unwrap().cleared >= 1);
}

#[test]
fn 색인이_늦게_오면_강조를_다시_만든다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("region");
    s.capture_input(mv(420, 320));
    assert_eq!(w.overlay.0.lock().unwrap().highlights.last().copied().flatten(), Some(PhysRect::new(100, 100, 1200, 800)), "아직 요소 없음 → 창");
    w.elements.0.lock().unwrap().by_window.insert(1, vec![ElementCand { rect: PhysRect::new(300, 300, 300, 40), control: Control::Button }]);
    s.on_elements_ready(1, "chrome.exe", 40.0);
    assert_eq!(w.overlay.0.lock().unwrap().highlights.last().copied().flatten(), Some(PhysRect::new(300, 300, 300, 40)));
}

#[test]
fn 색인이_두_번_느린_앱은_창_단위로_강등() {
    let w = world();
    let mut s = service(&w);
    s.on_elements_ready(1, "hwp.exe", 620.0);
    s.on_elements_ready(1, "hwp.exe", 710.0);
    let list = s.settings().get("capture.excluded").cloned().unwrap();
    assert_eq!(list[0]["exe"], "hwp.exe");
    assert_eq!(list[0]["mode"], "windowOnly");
    assert_eq!(list[0]["auto"], true);
    assert!(w.last_island().unwrap().text.contains("hwp.exe"));
}

#[test]
fn 보통_속도_색인은_강등하지_않는다() {
    // 학교 PC 실측 Chrome·Claude 80~210 ms — 몇 번이어도 창 단위로 떨어지지 않는다
    let w = world();
    let mut s = service(&w);
    for took in [210.0, 190.0, 160.0] {
        s.on_elements_ready(1, "chrome.exe", took);
    }
    let list = s.settings().get("capture.excluded").cloned().unwrap_or_default();
    assert!(list.as_array().cloned().unwrap_or_default().is_empty(), "{list}");
}

#[test]
fn 설정에서_바꾼_복사_키가_바로_먹는다() {
    // R-KEY-6: 설정 c-copy = Space → 다음 진입부터 Space = 복사, Enter = 아무것도 안 함
    let w = world();
    let mut s = service(&w);
    assert!(s.set_shortcut("c-copy", "Space").ok, "설정 저장");
    s.on_hotkey("region");
    s.capture_input(mv(420, 320));
    s.capture_input(key("Enter"));
    assert!(w.desk.0.lock().unwrap().images.is_empty(), "Enter는 복사 아님");
    s.capture_input(key("Space"));
    assert_eq!(w.desk.0.lock().unwrap().images, vec![(300, 40)]);
}

#[test]
fn 툴바_복사_단추는_enter와_같다() {
    let w = world();
    let mut s = service(&w);
    s.on_hotkey("region");
    s.capture_input(mv(420, 320));
    s.capture_input(CaptureInput::Command { name: "copy".into() });
    assert_eq!(w.desk.0.lock().unwrap().images, vec![(300, 40)]);
    // 모르는 단추는 아무 일도 없다
    s.on_hotkey("region");
    s.capture_input(mv(420, 320));
    s.capture_input(CaptureInput::Command { name: "launch-missiles".into() });
    assert!(matches!(s.mode(), Mode::Capture { .. }), "모드 그대로");
}
