//! 캡처 핀(R-CAP-29): 툴바 [고정] → 원래 자리·같은 크기 핀 창, 닫기·모두 닫기·복사·저장, 트레이 개수.

use ink_app::{CaptureInput, InkService, IslandKind};
use ink_domain::geometry::PhysRect;
use ink_domain::mode::Mode;
use ink_domain::recog::{Control, ElementCand};
use ink_domain::shortcuts::Platform;
use ink_platform_fake::FakeWorld;

fn world() -> FakeWorld {
    let w = FakeWorld::new();
    w.elements.0.lock().unwrap().by_window.insert(1, vec![ElementCand { rect: PhysRect::new(300, 300, 300, 40), control: Control::DataItem, from_msaa: false }]);
    w
}

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.start();
    s
}

fn pin_row(s: &mut InkService) {
    s.on_hotkey("region");
    s.capture_input(CaptureInput::Move { x: 420, y: 320 });
    s.capture_input(CaptureInput::Command { name: "pin".into() });
}

#[test]
fn 고정은_원래_자리_같은_크기로_핀을_띄우고_모드를_끝낸다() {
    let w = world();
    let mut s = service(&w);
    pin_row(&mut s);
    assert_eq!(s.mode(), Mode::Idle, "고정하면 캡처 화면은 닫힌다");
    let board = w.pins.0.lock().unwrap();
    assert_eq!(board.open.get(&1), Some(&(PhysRect::new(300, 300, 300, 40), 300, 40)));
    drop(board);
    assert_eq!(s.pin_count(), 1);
    assert_eq!(s.state().pins, 1, "트레이 \"핀 n개\"");
    assert!(w.desk.0.lock().unwrap().images.is_empty(), "고정은 클립보드를 건드리지 않는다");
}

#[test]
fn 핀_닫기_모두_닫기() {
    let w = world();
    let mut s = service(&w);
    pin_row(&mut s);
    pin_row(&mut s);
    pin_row(&mut s);
    assert_eq!(s.pin_count(), 3);
    s.pin_close(2);
    assert_eq!(w.pins.0.lock().unwrap().closed, vec![2]);
    assert_eq!(s.pin_count(), 2);
    s.pin_close(2); // 두 번 닫아도 조용히
    assert_eq!(s.pin_close_all(), 2);
    assert!(w.pins.0.lock().unwrap().open.is_empty());
    assert_eq!(s.state().pins, 0);
}

#[test]
fn 핀_메뉴_복사와_저장은_핀_그림으로() {
    let w = world();
    let mut s = service(&w);
    pin_row(&mut s);
    s.pin_copy(1);
    assert_eq!(w.desk.0.lock().unwrap().images, vec![(300, 40)]);
    assert_eq!(w.last_island().unwrap().text, "복사했어요");
    s.pin_save(1);
    assert_eq!(w.desk.0.lock().unwrap().saved.len(), 1);
    assert_eq!(w.desk.0.lock().unwrap().saved[0].2, 300);
    s.pin_copy(99); // 없는 핀은 아무 일도 없다
    assert_eq!(w.desk.0.lock().unwrap().images.len(), 1);
}

#[test]
fn 핀_창을_못_만들면_알린다() {
    let w = world();
    w.pins.0.lock().unwrap().fail_open = true;
    let mut s = service(&w);
    pin_row(&mut s);
    assert_eq!(s.pin_count(), 0);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str()), (IslandKind::Problem, "화면에 고정하지 못했어요"));
}
