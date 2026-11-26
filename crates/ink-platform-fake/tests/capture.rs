//! 캡처 진입 흐름(단계 2): 정지 스냅숏 → 오버레이 → 계측 → 끝. 모드 전환·시스템 사건.

use ink_app::{HotkeyOutcome, InkService, IslandKind, SystemEvent};
use ink_domain::geometry::PhysRect;
use ink_domain::mode::Mode;
use ink_domain::shortcuts::Platform;
use ink_domain::snapshot::{CaptureKind, RawWindow};
use ink_platform_fake::FakeWorld;
use serde_json::json;
use std::time::{Duration, Instant};

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.start();
    s
}

#[test]
fn f1은_캡처_먼저_표시_나중이고_모드_섬을_띄운다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    assert_eq!(s.on_hotkey("region"), HotkeyOutcome::Captured);
    assert_eq!(w.screen.0.lock().unwrap().captures, 1);
    let o = w.overlay.0.lock().unwrap();
    let (session, frames) = o.showing.expect("오버레이가 보임");
    assert_eq!(frames, 2, "모니터마다 이미지 한 장");
    assert_eq!(o.shown[0].monitors.len(), 2);
    drop(o);
    assert!(matches!(s.mode(), Mode::Capture { kind: CaptureKind::Region, .. }));
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.variant.as_deref(), island.mode.as_deref()), (IslandKind::Mode, Some("region"), Some("region")));
    assert_eq!(island.hints.len(), 4);
    // 스냅숏은 창 목록을 같은 순간에 담는다
    assert_eq!(s.snapshot().unwrap().windows.len(), 1);
    // 기본 강조 = 커서 모니터 전체(작업 표시줄 포함)
    let shown = w.overlay.0.lock().unwrap().shown[0].clone();
    assert_eq!(shown.highlight, Some(PhysRect::new(0, 0, 1920, 1080)));
    assert!(matches!(s.state().mode, Mode::Capture { session: x, .. } if x == session));
}

#[test]
fn 트레이_메뉴로는_단축키가_꺼져_있어도_시작한다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.set_hotkeys_enabled(false);
    s.start_capture(CaptureKind::Region);
    assert!(matches!(s.mode(), Mode::Capture { .. }));
    s.start_capture(CaptureKind::Region);
    assert!(matches!(s.mode(), Mode::Capture { .. }), "메뉴는 끝내기가 아니라 새로 시작");
}

#[test]
fn 같은_키는_끝_다른_캡처_키는_전환() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("region");
    let first = w.overlay.0.lock().unwrap().showing.unwrap().0;
    s.on_hotkey("region");
    assert_eq!(s.mode(), Mode::Idle);
    assert!(w.overlay.0.lock().unwrap().showing.is_none(), "정지 이미지를 버린다");
    assert_eq!(w.last_island().unwrap().kind, IslandKind::End);

    s.on_hotkey("region");
    s.on_hotkey("record");
    let now = w.overlay.0.lock().unwrap().showing.unwrap().0;
    assert!(now > first);
    assert!(matches!(s.mode(), Mode::Capture { kind: CaptureKind::Record, .. }));
}

#[test]
fn f2_두_번은_활성_창_요청으로_넘긴다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("window");
    // 단계 6.1부터 서비스가 활성 창을 바로 찍는다(자세한 흐름은 tests/window_record.rs)
    assert_eq!(s.on_hotkey("window"), HotkeyOutcome::Captured);
    assert_eq!(s.mode(), Mode::Idle);
}

#[test]
fn 그림이_그려지면_계측_한_줄과_통계() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey_at("region", Instant::now() - Duration::from_millis(30));
    let session = w.overlay.0.lock().unwrap().showing.unwrap().0;
    assert!(s.overlay_ready(session + 99).is_none(), "다른 세션은 무시");
    let t = s.overlay_ready(session).expect("계측");
    assert!(t.ready_ms.unwrap() >= 30.0);
    assert_eq!((t.monitors, t.windows), (2, 1));
    assert!(t.log_line().starts_with("overlay ready ["));
    assert!(s.overlay_ready(session).is_none(), "첫 모니터만 센다");
    assert_eq!(s.entry_stats().unwrap().n, 1);
}

#[test]
fn 네이티브_정지_화면이면_그_순간이_진입_시간이고_ui가_늦어도_남는다() {
    let w = FakeWorld::new();
    w.overlay.0.lock().unwrap().native = true;
    let mut s = service(&w);
    s.on_hotkey_at("region", Instant::now() - Duration::from_millis(40));
    s.on_hotkey("region"); // UI가 알리기 전에 끝냄
    let st = s.entry_stats().expect("표본 하나");
    assert_eq!(st.n, 1);
    assert!(st.p50 >= 40.0 && st.p50 < 1_000.0);
}

#[test]
fn 감지_제외_앱과_자기_창은_스냅숏에_없다() {
    let w = FakeWorld::new();
    w.screen.0.lock().unwrap().windows.push(RawWindow {
        handle: 2,
        pid: 8,
        exe: "LICECAP.EXE".into(),
        rect: PhysRect::new(0, 0, 300, 300),
        visible: true,
        ..Default::default()
    });
    w.screen.0.lock().unwrap().windows.push(RawWindow { handle: 3, pid: std::process::id(), rect: PhysRect::new(0, 0, 300, 300), visible: true, ..Default::default() });
    let mut s = service(&w);
    s.update_setting("capture.excluded", &json!([{ "exe": "licecap.exe", "mode": "exclude" }]));
    s.on_hotkey("region");
    let handles: Vec<u64> = s.snapshot().unwrap().windows.iter().map(|x| x.handle).collect();
    assert_eq!(handles, vec![1]);
}

#[test]
fn 캡처나_표시가_실패하면_모드에_들어가지_않고_알린다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    w.screen.0.lock().unwrap().capture_fails = true;
    s.on_hotkey("region");
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.last_island().unwrap().text, "화면을 가져오지 못했어요");
    w.screen.0.lock().unwrap().capture_fails = false;
    w.overlay.0.lock().unwrap().fail_show = true;
    s.on_hotkey("region");
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.last_island().unwrap().kind, IslandKind::Problem);
}

#[test]
fn 화면_기록_권한이_없으면_어디서_켜는지_알린다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    {
        let mut sc = w.screen.0.lock().unwrap();
        sc.capture_fails = true;
        sc.fail_reason = Some(ink_app::NO_SCREEN_PERMISSION.into());
    }
    s.on_hotkey("region");
    assert_eq!(s.mode(), Mode::Idle);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str()), (IslandKind::Problem, "화면 기록 권한이 필요해요"));
    assert!(island.detail.as_deref().is_some_and(|d| d.contains("화면 기록")));
}

#[test]
fn 보안_화면에서는_조용히_한_번만_알린다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    w.probe.0.lock().unwrap().secure_desktop = true;
    s.on_hotkey("region");
    s.on_hotkey("region");
    assert_eq!(s.mode(), Mode::Idle);
    let islands = w.notifier.0.lock().unwrap().islands.iter().filter(|i| i.variant.as_deref() == Some("security")).count();
    assert_eq!(islands, 1);
    assert_eq!(w.screen.0.lock().unwrap().captures, 0);
}

#[test]
fn 잠금은_모드를_끝내고_배치_변경은_오버레이를_다시_맞춘다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let prepared = w.overlay.0.lock().unwrap().prepared;
    assert_eq!(prepared, 1, "시작할 때 한 번 맞춘다");
    s.on_hotkey("region");
    assert_eq!(w.overlay.0.lock().unwrap().prepared, 1, "배치가 같으면 다시 맞추지 않는다");
    s.on_system_event(SystemEvent::Lock);
    assert_eq!(s.mode(), Mode::Idle);

    w.screen.0.lock().unwrap().layout.monitors.pop();
    s.on_system_event(SystemEvent::DisplayChange);
    assert_eq!(w.overlay.0.lock().unwrap().prepared, 2);
    s.on_system_event(SystemEvent::Resume);
    assert_eq!(w.overlay.0.lock().unwrap().prepared, 3, "깨어나면 지문이 같아도 다시");
    s.on_hotkey("region");
    assert_eq!(w.overlay.0.lock().unwrap().showing.unwrap().1, 1);
}

#[test]
fn 단축키를_꺼도_캡처는_트레이에서_시작할_수_있게_모드만_막는다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.set_hotkeys_enabled(false);
    assert_eq!(s.on_hotkey("region"), HotkeyOutcome::Ignored);
    assert_eq!(s.mode(), Mode::Idle);
}
