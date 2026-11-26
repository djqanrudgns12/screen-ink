//! 유스케이스 흐름 시험(03 §7 ink-app 줄): 단축키 트랜잭션·롤백·추천·잠시 끄기, 설정 바로 반영·되돌리기.

use ink_app::{HotkeyOutcome, InkService, IslandKind, RowStatus};
use ink_domain::shortcuts::{LiveBinding, Platform};
use ink_platform_fake::FakeWorld;
use serde_json::json;

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Win, "0.1.0-test");
    s.start();
    s
}

fn row<'a>(st: &'a ink_app::InkState, id: &str) -> &'a ink_app::ShortcutRow {
    st.shortcuts.iter().find(|r| r.id == id).unwrap()
}

#[test]
fn 첫_실행은_전역_16개를_등록하고_기본값을_저장한다() {
    let w = FakeWorld::new();
    let s = service(&w);
    assert_eq!(w.registered_count(), 16);
    assert!(w.is_registered("F1") && w.is_registered("Alt+Backquote") && w.is_registered("Alt+Shift+KeyD"));
    let file = w.saved().expect("기본값 저장");
    assert_eq!(file["schemaVersion"], 3);
    let st = s.state();
    assert_eq!(st.shortcuts.len(), 59);
    assert_eq!(st.conflicts.count, 0);
    assert!(s.is_first_run());
}

#[test]
fn snipaste가_f1_계열을_쥐고_있으면_4줄이_겹침이고_앱_이름을_말한다() {
    let w = FakeWorld::new();
    w.running(&["Snipaste.exe"]).taken(&["F1", "F3", "Shift+F1", "Control+F1"]);
    let s = service(&w);
    let st = s.state();
    assert_eq!(st.conflicts.count, 4);
    assert_eq!(st.conflicts.app.as_deref(), Some("Snipaste"));
    assert_eq!(row(&st, "region").status, RowStatus::Conflict);
    assert_eq!(row(&st, "region").reason.as_deref(), Some("Snipaste에서 먼저 쓰고 있어요"));
    // F2는 탐색기 "이름 바꾸기"와 겹쳐 '주의'(선택기와 같음)지만 등록은 된다
    assert_eq!(row(&st, "window").status, RowStatus::Warn);
    assert!(w.is_registered("F2"));
}

#[test]
fn snipaste가_켜져_있어도_키를_안_쥐었으면_등록해서_확인한다() {
    let w = FakeWorld::new();
    w.running(&["Snipaste.exe"]);
    let s = service(&w);
    let st = s.state();
    assert_eq!(st.conflicts.count, 0, "{:?}", st.shortcuts.iter().filter(|r| r.status == RowStatus::Conflict).collect::<Vec<_>>());
    assert!(w.is_registered("F1"));
}

#[test]
fn 설정_파일에서_읽은_powertoys_키는_등록이_되어도_막는다() {
    let w = FakeWorld::new();
    w.running(&["PowerToys.exe"]);
    w.probe.0.lock().unwrap().live = Some(vec![LiveBinding { owner: "PowerToys 항상 위".into(), combo: "Alt+Shift+D".into(), what: "창 항상 위 고정".into() }]);
    let s = service(&w);
    let st = s.state();
    assert_eq!(row(&st, "topmost").status, RowStatus::Conflict);
    assert_eq!(row(&st, "topmost").owner.as_deref(), Some("PowerToys 항상 위"));
    assert!(!w.is_registered("Alt+Shift+KeyD"));
}

#[test]
fn 바꾸기는_새_키_등록_후_옛_키_해제_그리고_되돌리기() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let r = s.set_shortcut("active", "ctrl+alt+f2");
    assert!(r.ok, "{r:?}");
    assert!(w.is_registered("Control+Alt+F2"));
    assert!(!w.is_registered("Shift+F2"));
    let island = w.last_island().unwrap();
    assert_eq!(island.kind, IslandKind::Result);
    assert_eq!(island.text, "Ctrl Alt F2로 바꿨어요");
    assert_eq!(island.button.unwrap().action, "undo");
    assert_eq!(w.saved().unwrap()["sections"]["settings"]["data"]["shortcuts"]["active"], "Ctrl+Alt+F2");

    assert!(s.undo().ok);
    assert!(w.is_registered("Shift+F2"));
    assert!(!w.is_registered("Control+Alt+F2"));
    assert_eq!(s.settings().shortcut("active"), "Shift+F2");
}

#[test]
fn 다른_앱이_쥔_키는_옛_키를_지키고_등록_가능한_추천_3개() {
    let w = FakeWorld::new();
    w.taken(&["Control+Alt+F2", "Alt+Digit4", "Alt+Digit5"]);
    let mut s = service(&w);
    let r = s.set_shortcut("active", "Ctrl+Alt+F2");
    assert!(!r.ok);
    assert_eq!(r.reason.as_deref(), Some("다른 앱이 먼저 쓰고 있어요"));
    assert_eq!(r.suggestions.len(), 3);
    assert!(!r.suggestions.iter().any(|c| c == "Alt+4" || c == "Alt+5"), "등록 안 되는 키는 추천하지 않는다: {:?}", r.suggestions);
    assert!(w.is_registered("Shift+F2"), "옛 키 유지");
    assert_eq!(s.settings().shortcut("active"), "Shift+F2");
    assert!(!s.can_undo());
}

#[test]
fn 함께_실패한_줄은_첫_추천이_서로_다르다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let a = s.set_shortcut("active", "Q");
    let b = s.set_shortcut("delay", "W");
    assert!(!a.ok && !b.ok);
    assert_ne!(a.suggestions[0], b.suggestions[0]);
}

#[test]
fn 이_표_안_겹침과_글자_하나는_등록을_시도하지도_않는다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let before = w.registered_count();
    let r = s.set_shortcut("active", "Alt+1");
    assert!(!r.ok);
    assert!(r.reason.unwrap().contains("판서"));
    let r = s.set_shortcut("active", "Shift+Q");
    assert!(!r.ok);
    assert_eq!(w.registered_count(), before);
}

#[test]
fn 비우면_해제되고_트레이로_시작할_수_있다고_알린다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    assert!(s.set_shortcut("delay", "").ok);
    assert!(!w.is_registered("Alt+F1"));
    assert_eq!(row(&s.state(), "delay").status, RowStatus::Off);
    assert!(w.last_island().unwrap().text.starts_with("비웠어요"));
}

#[test]
fn 잠시_끄면_alt_백틱만_남고_다시_누르면_켜진다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.set_hotkeys_enabled(false);
    assert_eq!(w.registered_count(), 1);
    assert!(w.is_registered("Alt+Backquote"));
    assert_eq!(w.last_island().unwrap().kind, IslandKind::Off);
    assert_eq!(s.on_hotkey("region"), HotkeyOutcome::Ignored);
    assert_eq!(s.on_hotkey("pause"), HotkeyOutcome::Toggled(true));
    assert_eq!(w.registered_count(), 16);
    assert_eq!(s.on_hotkey("region"), HotkeyOutcome::Captured);
    assert_eq!(s.on_hotkey("draw"), HotkeyOutcome::Draw, "캡처 중 Alt+1 = 캡처를 끝내고 판서로");
}

#[test]
fn 설정은_바로_반영되고_되돌릴_수_있다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let bad = s.update_setting("draw.width", &json!(99));
    assert!(!bad.ok);
    assert_eq!(bad.error.as_deref(), Some("1~30 사이로 정해 주세요"));
    assert!(s.update_setting("draw.width", &json!(9)).ok);
    assert_eq!(w.saved().unwrap()["sections"]["settings"]["data"]["values"]["draw.width"], 9);
    assert!(s.update_setting("general.theme", &json!("dark")).ok);
    assert_eq!(s.state().theme, "dark");
    s.undo();
    s.undo();
    assert_eq!(s.settings().get("draw.width"), Some(&json!(4)));
    assert_eq!(s.state().theme, "system");
}

#[test]
fn 저장이_실패하면_값을_되돌리고_섬으로_알린다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    w.store.0.lock().unwrap().fail_writes = true;
    let r = s.update_setting("zoom.minimap", &json!(false));
    assert!(!r.ok);
    assert_eq!(s.settings().get("zoom.minimap"), Some(&json!(true)));
    assert_eq!(w.last_island().unwrap().text, "저장하지 못했어요");
    let r = s.set_shortcut("active", "Ctrl+Alt+F2");
    assert!(!r.ok);
    assert!(w.is_registered("Shift+F2") && !w.is_registered("Control+Alt+F2"), "등록도 되돌린다");
}

#[test]
fn 묶음_기본값은_한_번에_되돌리기_한_건() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.set_shortcut("active", "Ctrl+Alt+F2");
    s.set_shortcut("delay", "");
    s.begin_settings_session(); // 기록을 비우고 시작
    s.reset_shortcut_group("capture");
    assert_eq!(s.settings().shortcut("active"), "Shift+F2");
    assert_eq!(s.settings().shortcut("delay"), "Alt+F1");
    assert!(w.is_registered("Alt+F1"));
    s.undo();
    assert_eq!(s.settings().shortcut("active"), "Ctrl+Alt+F2");
    assert_eq!(s.settings().shortcut("delay"), "");
    assert!(!s.can_undo());
}

#[test]
fn 더_새_스키마_파일은_읽기만_하고_저장하지_않는다() {
    let w = FakeWorld::new();
    let newer = json!({ "schemaVersion": 4, "sections": { "settings": { "revision": 1, "data": { "values": { "draw.width": 7 } } } } });
    w.store.0.lock().unwrap().file = Some(newer.clone());
    let mut s = service(&w);
    assert!(s.state().readonly);
    assert_eq!(s.settings().get("draw.width"), Some(&json!(7)));
    s.update_setting("draw.width", &json!(8));
    assert_eq!(w.saved().unwrap(), newer);
}

#[test]
fn snipaste를_끄고_다시_보면_실패했던_줄이_살아난다() {
    let w = FakeWorld::new();
    w.running(&["Snipaste.exe"]).taken(&["F1", "F3", "Shift+F1", "Control+F1"]);
    let mut s = service(&w);
    assert_eq!(s.state().conflicts.count, 4);
    w.running(&[]).taken(&[]);
    s.begin_settings_session();
    assert_eq!(s.state().conflicts.count, 0);
    assert!(w.is_registered("F1"));
}

#[test]
fn 가져오기는_미리_보기로_바뀌는_항목만_세고_반영은_되돌리기_한_건() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    let mut other = InkService::new(FakeWorld::new().ports(), Platform::Win, "x");
    other.update_setting("draw.width", &json!(12));
    other.set_shortcut("active", "Ctrl+Alt+F2");
    let file = other.export_file();

    let writes = w.store.0.lock().unwrap().writes;
    let p = s.preview_import(&file);
    assert!(p.ok);
    assert_eq!(p.changes.len(), 2, "{:?}", p.changes);
    assert_eq!(w.store.0.lock().unwrap().writes, writes, "미리 보기는 저장하지 않는다");
    assert!(!s.preview_import(&json!({ "hello": 1 })).ok);

    assert!(s.apply_import(&file).ok);
    assert_eq!(s.settings().get("draw.width"), Some(&json!(12)));
    assert!(w.is_registered("Control+Alt+F2") && !w.is_registered("Shift+F2"));
    s.undo();
    assert_eq!(s.settings().get("draw.width"), Some(&json!(4)));
    assert!(w.is_registered("Shift+F2") && !w.is_registered("Control+Alt+F2"));
}

#[test]
fn 모두_처음으로는_기본값과_등록을_되살린다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.update_setting("onboarding.done", &json!(true));
    s.update_setting("zoom.minimap", &json!(false));
    s.set_shortcut("delay", "");
    let st = s.reset_all();
    assert_eq!(st.settings["zoom.minimap"], json!(true));
    assert!(w.is_registered("Alt+F1"));
    assert_eq!(w.registered_count(), 16);
    assert!(!s.can_undo());
    assert!(!s.is_first_run(), "첫 실행 안내는 다시 띄우지 않는다");
}

#[test]
fn 맥은_58칸이고_창_항상_위가_없다() {
    let w = FakeWorld::new();
    let mut s = InkService::new(w.ports(), Platform::Mac, "0.1.0-test");
    s.start();
    let st = s.state();
    assert_eq!(st.platform, "macos");
    assert_eq!(st.shortcuts.len(), 58);
    assert!(st.shortcuts.iter().all(|r| r.id != "topmost"));
    assert_eq!(w.registered_count(), 15);
}
