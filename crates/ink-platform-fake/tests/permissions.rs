//! 맥 권한 스마트 흐름(M10, I-mac §2.2): 업데이트 뒤 풀림 감지 · 손쉬운 사용 칩은 이번 실행 첫 캡처에 한 번.

use ink_app::{InkService, IslandKind, ACTION_PERM_AX, ACTION_PERM_SCREEN};
use ink_domain::shortcuts::Platform;
use ink_platform_fake::FakeWorld;

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Mac, "test");
    s.start();
    s
}

fn set_perm(w: &FakeWorld, screen: bool, ax: bool) {
    w.probe.0.lock().unwrap().permissions = Some((screen, ax));
}

#[test]
fn 권한_개념이_없으면_상태에_싣지_않는다() {
    let w = FakeWorld::new();
    let s = service(&w);
    assert_eq!(s.state().permissions, None);
}

#[test]
fn 업데이트_뒤_화면_기록이_풀리면_시작할_때_단추로_알린다() {
    let w = FakeWorld::new();
    set_perm(&w, true, true);
    let mut s = service(&w);
    s.startup_permission_check();
    let p = s.state().permissions.expect("맥 권한");
    assert!(p.screen && !p.screen_revoked);
    assert!(w.islands().iter().all(|i| i.kind != IslandKind::Problem), "권한이 있으면 알리지 않는다");
    // 같은 설정 파일로 다시 시작했는데 권한이 풀림(재서명·업데이트)
    drop(s);
    set_perm(&w, false, true);
    let mut s = service(&w);
    s.startup_permission_check();
    let p = s.state().permissions.unwrap();
    assert!(!p.screen && p.screen_revoked && !p.accessibility_revoked);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str()), (IslandKind::Problem, "화면 기록 권한을 다시 켜요"));
    assert_eq!(island.detail.as_deref(), Some("업데이트 뒤 권한이 풀렸어요"));
    assert_eq!(island.button.as_ref().map(|b| b.action.as_str()), Some(ACTION_PERM_SCREEN));
}

#[test]
fn 처음부터_권한이_없으면_풀림이_아니다() {
    let w = FakeWorld::new();
    set_perm(&w, false, false);
    let mut s = service(&w);
    s.startup_permission_check();
    let p = s.state().permissions.unwrap();
    assert!(!p.screen_revoked && !p.accessibility_revoked);
    assert!(w.islands().iter().all(|i| i.kind != IslandKind::Problem));
}

#[test]
fn 손쉬운_사용이_없으면_첫_캡처_모드_섬에만_칩() {
    let w = FakeWorld::new();
    set_perm(&w, true, false);
    let mut s = service(&w);
    s.on_hotkey("region");
    let first = w.islands().into_iter().rev().find(|i| i.kind == IslandKind::Mode).unwrap();
    assert_eq!(first.chip.as_ref().map(|c| c.action.as_str()), Some(ACTION_PERM_AX));
    s.end_capture();
    s.on_hotkey("region");
    let second = w.islands().into_iter().rev().find(|i| i.kind == IslandKind::Mode).unwrap();
    assert!(second.chip.is_none(), "이번 실행에 한 번만");
}

#[test]
fn 손쉬운_사용을_켜면_다음_확인부터_칩이_없다() {
    let w = FakeWorld::new();
    set_perm(&w, true, true);
    let mut s = service(&w);
    s.on_hotkey("region");
    let island = w.islands().into_iter().rev().find(|i| i.kind == IslandKind::Mode).unwrap();
    assert!(island.chip.is_none());
    assert!(s.refresh_permissions().is_some_and(|p| p.accessibility));
}

/// R2-ONB-2: 첫 실행 카드가 1초마다 다시 읽는다 — 켜지는 순간에만 상태를 보낸다(그 사이 같은 값이면 조용히).
#[test]
fn 권한_감시는_바뀔_때만_상태를_보낸다() {
    let w = FakeWorld::new();
    set_perm(&w, false, false);
    let mut s = service(&w);
    s.refresh_permissions();
    let sent = || w.notifier.0.lock().unwrap().states;
    let n = sent();
    assert!(!s.poll_permissions(), "그대로면 보내지 않는다");
    assert_eq!(sent(), n);
    set_perm(&w, false, true);
    assert!(s.poll_permissions(), "손쉬운 사용을 켠 순간");
    assert_eq!(sent(), n + 1);
    let p = s.state().permissions.unwrap();
    assert!(p.accessibility && !p.screen);
    assert!(!s.poll_permissions());
    // 권한 개념이 없는 OS는 아무 일도 없다
    let w2 = FakeWorld::new();
    let mut s2 = service(&w2);
    assert!(!s2.poll_permissions());
}

#[test]
fn 첫_실행_카드_위치는_숨은_설정으로_남는다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    assert!(s.update_setting("onboarding.step", &serde_json::json!(2)).ok);
    assert_eq!(s.state().settings["onboarding.step"], 2);
    assert!(!s.update_setting("onboarding.step", &serde_json::json!(9)).ok, "0~4만");
}
