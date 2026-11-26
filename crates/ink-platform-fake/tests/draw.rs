//! 판서·칠판(단계 4, A §4 Rust 통합 시험): 진입 → 끝 → 보관 → 복원 판정, 내보내기, 전환·시스템 사건.

use ink_app::{DrawExportMeta, DrawFinish, DrawIslandPatch, ExportAction, HotkeyOutcome, InkService, IslandKind, KeptCompat, KeptPayload, PenSample, SystemEvent};
use ink_domain::geometry::{PhysPoint, PhysRect};
use ink_domain::mode::{DrawKind, Mode};
use ink_domain::shortcuts::Platform;
use ink_domain::snapshot::CaptureKind;
use ink_platform_fake::FakeWorld;
use serde_json::{json, Value};

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.start();
    s
}

/// 항목 n개짜리 InkDocument(A §2.2 모양만).
fn doc(n: usize) -> Value {
    let items: Vec<Value> = (0..n).map(|i| json!({ "id": format!("it-{i}"), "kind": "stroke" })).collect();
    json!({ "schemaVersion": 1, "pages": [ { "id": "page-1", "items": items } ] })
}

fn finish(session: u64, n: usize) -> DrawFinish {
    DrawFinish { session, keep: true, kept: Some(KeptPayload { document: doc(n) }) }
}

fn session_of(s: &InkService) -> u64 {
    match s.mode() {
        Mode::Draw { session, .. } => session,
        other => panic!("판서 중이 아님: {other:?}"),
    }
}

fn last_text(w: &FakeWorld) -> String {
    w.last_island().map(|i| i.text).unwrap_or_default()
}

fn export(action: ExportAction, w: u32, h: u32, session: Option<u64>) -> DrawExportMeta {
    DrawExportMeta { action, w, h, session, bgra: false }
}

#[test]
fn alt_1은_커서_모니터_한_장만_찍고_그_모니터_오버레이를_띄운다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    assert_eq!(s.on_hotkey("draw"), HotkeyOutcome::Draw);
    assert_eq!(w.screen.0.lock().unwrap().captures, 1);
    let (ds, frame) = w.overlay.0.lock().unwrap().draws[0].clone();
    assert_eq!(frame, Some((1920, 1080)), "커서 모니터(0) 한 장만");
    assert!(!ds.board && ds.monitor.id == 0 && !ds.restore_on_open && !ds.has_kept);
    assert_eq!(ds.kept_compatible, KeptCompat::None);
    assert_eq!(ds.cursor, PhysPoint { x: 400, y: 300 });
    assert_eq!(w.overlay.0.lock().unwrap().showing, Some((ds.session, 1)));
    // 세션 알림에는 어댑터가 채운 배경 주소가 들어 있다
    let sent = w.notifier.0.lock().unwrap().draws.last().cloned().unwrap();
    assert_eq!(sent.monitor.frame_url.as_deref(), Some(format!("fake://frame/{}/0", ds.session).as_str()));
    assert_eq!(s.draw_session().unwrap().monitor.frame_url, sent.monitor.frame_url);
    assert_eq!(s.mode(), Mode::Draw { session: ds.session, board: false, monitor_id: 0, started_ms: match s.mode() { Mode::Draw { started_ms, .. } => started_ms, _ => 0 } });
    assert!(matches!(s.state().mode, Mode::Draw { session, .. } if session == ds.session));
    // 모드 섬: 그리기 · 펜 4px(설정 색)
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.variant.as_deref(), island.mode.as_deref(), island.text.as_str()), (IslandKind::Mode, Some("draw"), Some("draw"), "그리기 · 펜"));
    assert_eq!((island.pen_color.as_deref(), island.pen_width.as_deref()), (Some("R"), Some("4px")));
    // 진입 계측은 지금 세션에 한 번만
    assert!(s.draw_ready(ds.session + 9, Some(3.0)).is_none());
    assert!(s.draw_ready(ds.session, Some(3.0)).is_some());
    assert!(s.draw_ready(ds.session, Some(3.0)).is_none());

    // 커서가 옆 모니터(150%)면 그 모니터
    s.draw_finish(finish(ds.session, 0));
    w.screen.0.lock().unwrap().cursor = PhysPoint { x: 2500, y: 600 };
    s.on_hotkey("draw");
    let (ds, frame) = w.overlay.0.lock().unwrap().draws.last().cloned().unwrap();
    assert_eq!((ds.monitor.id, ds.monitor.scale, frame), (1, 1.5, Some((2880, 1800))));
}

#[test]
fn alt_3_칠판은_캡처하지_않고_빈_바탕() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    assert_eq!(s.on_hotkey("board"), HotkeyOutcome::Draw);
    assert_eq!(w.screen.0.lock().unwrap().captures, 0, "칠판은 화면을 찍지 않는다");
    let (ds, frame) = w.overlay.0.lock().unwrap().draws[0].clone();
    assert!(ds.board);
    assert_eq!(frame, None);
    assert_eq!(w.notifier.0.lock().unwrap().draws[0].monitor.frame_url, None);
    assert!(matches!(s.mode(), Mode::Draw { board: true, .. }));
    let island = w.last_island().unwrap();
    assert_eq!((island.text.as_str(), island.variant.as_deref(), island.value.as_deref(), island.mode.as_deref()), ("칠판 · 흰색", Some("board"), Some("1 / 1"), Some("board")));
    // 같은 키 = 끝
    s.on_hotkey("board");
    assert_eq!(s.mode(), Mode::Idle);
}

#[test]
fn esc로_끝내면_항목이_있을_때만_보관하고_섬으로_알린다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("draw");
    let first = session_of(&s);
    s.draw_finish(finish(first, 2));
    assert_eq!(s.mode(), Mode::Idle);
    assert!(w.overlay.0.lock().unwrap().showing.is_none(), "오버레이를 숨기고 배경을 버린다");
    let kept = s.kept_ink().cloned().expect("보관");
    assert_eq!((kept.monitor_id, kept.phys_w, kept.phys_h, kept.scale, kept.board), (0, 1920, 1080, 1.0, false));
    assert_eq!(kept.layout_fingerprint, "0,0,1920,1080@100|1920,0,2880,1800@150");
    assert_eq!(kept.document, doc(2));
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str(), island.detail.as_deref()), (IslandKind::Result, "그리기를 보관했어요", Some("Alt+Shift+1로 다시 열기")));

    // 다음 판서: 보관이 있고 같은 모니터 → 섬에 이전 그리기 안내
    s.on_hotkey("draw");
    let ds = s.draw_session().cloned().unwrap();
    assert!(ds.has_kept);
    assert_eq!(ds.kept_compatible, KeptCompat::Same);
    assert_eq!(w.last_island().unwrap().detail.as_deref(), Some("Ctrl+Shift+R 방금 그리기"));
    // 빈 판서로 끝내면 덮지 않는다 — 모드 섬만 거둔다
    s.draw_finish(finish(ds.session, 0));
    assert_eq!(s.kept_ink().unwrap().document, doc(2));
    assert_eq!(w.last_island().unwrap().kind, IslandKind::End);
    // keep=false(버리기)도 덮지 않는다
    s.on_hotkey("draw");
    s.draw_finish(DrawFinish { keep: false, ..finish(session_of(&s), 5) });
    assert_eq!(s.kept_ink().unwrap().document, doc(2));

    // 설정 "나갈 때 보관"을 끄면 보관하지 않고, 있던 것도 지운다(05 §7 D)
    assert!(s.update_setting("draw.keep", &json!(false)).ok);
    s.on_hotkey("draw");
    assert!(!s.draw_session().unwrap().has_kept);
    s.draw_finish(finish(session_of(&s), 3));
    assert!(s.kept_ink().is_none());
    assert_eq!(w.last_island().unwrap().kind, IslandKind::End);
}

#[test]
fn draw_보관과_같은모니터_배율변경_다른모니터_복원() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    // 보관 없음
    s.on_hotkey("draw");
    let none = s.draw_restore(session_of(&s));
    assert_eq!((none.status, none.kept.is_none()), (KeptCompat::None, true));
    assert_eq!(last_text(&w), "불러올 그리기가 없어요", "엔진이 말하지 않는 '없음'만 Rust가 알린다");
    s.draw_finish(finish(session_of(&s), 1));

    // 같은 모니터·같은 구성 = 그대로
    s.on_hotkey("draw");
    let session = session_of(&s);
    assert_eq!(s.draw_session().unwrap().kept_compatible, KeptCompat::Same);
    let same = s.draw_restore(session);
    assert_eq!(same.status, KeptCompat::Same);
    assert_eq!(same.kept.as_ref().unwrap().document, doc(1));
    // 불러온 뒤에는 모드 섬의 "이전 판서" 안내를 거둔다(불러왔어요 문구는 엔진 restoreKept → drawIsland 안내)
    let mode_island = w.last_island().unwrap();
    assert_eq!((mode_island.kind, mode_island.detail.clone()), (IslandKind::Mode, None));
    // 지난 세션 번호로는 불러오지 않는다
    assert_eq!(s.draw_restore(session + 1).status, KeptCompat::None);
    s.draw_finish(finish(session, 1));

    // 배율만 바뀜(100% → 150%) = 비율대로 옮김
    w.screen.0.lock().unwrap().layout.monitors[0].scale = 1.5;
    s.on_system_event(SystemEvent::DisplayChange);
    s.on_hotkey("draw");
    assert_eq!(s.draw_session().unwrap().kept_compatible, KeptCompat::Rescaled);
    let r = s.draw_restore(session_of(&s));
    assert_eq!(r.status, KeptCompat::Rescaled);
    assert_eq!(r.kept.as_ref().map(|k| (k.phys_w, k.scale)), Some((1920, 1.0)), "옛 크기·배율을 넘겨 UI가 비율을 계산");
    s.draw_finish(DrawFinish { keep: false, ..finish(session_of(&s), 0) });

    // 다른 모니터 = 불러올 수 없음
    w.screen.0.lock().unwrap().cursor = PhysPoint { x: 2500, y: 600 };
    s.on_hotkey("draw");
    assert_eq!(s.draw_session().unwrap().kept_compatible, KeptCompat::DifferentMonitor);
    let before = w.notifier.0.lock().unwrap().islands.len();
    let d = s.draw_restore(session_of(&s));
    assert_eq!((d.status, d.kept.is_none()), (KeptCompat::DifferentMonitor, true));
    assert_eq!(w.notifier.0.lock().unwrap().islands.len(), before, "다른 모니터 문구는 엔진이 낸다");
    assert!(s.kept_ink().is_some(), "불러오지 못해도 보관은 그대로");
}

#[test]
fn alt_shift_1_직전판서_바로_열기() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("draw");
    s.draw_finish(finish(session_of(&s), 1));

    assert_eq!(s.on_hotkey("restore"), HotkeyOutcome::Draw);
    assert!(matches!(s.mode(), Mode::Draw { board: false, monitor_id: 0, .. }));
    let ds = w.notifier.0.lock().unwrap().draws.last().cloned().unwrap();
    assert!(ds.restore_on_open, "열자마자 복원");
    assert_eq!(ds.kept_compatible, KeptCompat::Same);
    assert_eq!(w.last_island().unwrap().detail, None, "바로 복원하니 안내는 없다");
    assert_eq!(s.draw_restore(ds.session).status, KeptCompat::Same);
    assert!(!s.draw_session().unwrap().restore_on_open);

    // 판서 중 Alt+Shift+1 = 새로 찍지 않고 같은 세션을 복원 표시로 한 번 더 알린다
    let captures = w.screen.0.lock().unwrap().captures;
    assert_eq!(s.on_hotkey("restore"), HotkeyOutcome::Draw);
    assert_eq!(w.screen.0.lock().unwrap().captures, captures);
    assert_eq!(session_of(&s), ds.session);
    let again = w.notifier.0.lock().unwrap().draws.last().cloned().unwrap();
    assert_eq!((again.session, again.restore_on_open), (ds.session, true));
    s.draw_finish(finish(ds.session, 0));

    // 칠판에서 보관했으면 칠판으로 다시 연다(화면을 찍지 않음)
    s.on_hotkey("board");
    s.draw_finish(finish(session_of(&s), 2));
    assert!(s.kept_ink().unwrap().board);
    let captures = w.screen.0.lock().unwrap().captures;
    s.start_draw(DrawKind::Restore);
    assert!(matches!(s.mode(), Mode::Draw { board: true, .. }));
    assert_eq!(w.screen.0.lock().unwrap().captures, captures);
    assert!(s.draw_session().unwrap().restore_on_open);
}

#[test]
fn 잠금_모니터변경시_판서_안전종료_및_보관() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("draw");
    let locked = session_of(&s);
    s.on_system_event(SystemEvent::Lock);
    assert_eq!(s.mode(), Mode::Idle);
    assert!(w.overlay.0.lock().unwrap().showing.is_none());
    assert_eq!(w.last_island().unwrap().kind, IslandKind::End);
    // 문서는 웹뷰에 있다 — 뒤이어 오는 늦은 끝 한 번을 받아 보관(잠금이라 섬은 띄우지 않음)
    s.draw_finish(finish(locked, 3));
    assert_eq!(s.kept_ink().unwrap().document, doc(3));
    assert_eq!(w.last_island().unwrap().kind, IslandKind::End);
    // 같은 세션의 두 번째 끝은 무시
    s.draw_finish(finish(locked, 7));
    assert_eq!(s.kept_ink().unwrap().document, doc(3));

    // 화면 구성 변경: 판서를 닫았다고 알리고(캡처 문구 아님) 늦은 끝으로 보관
    s.on_hotkey("draw");
    let changed = session_of(&s);
    w.screen.0.lock().unwrap().layout.monitors.pop();
    s.on_system_event(SystemEvent::DisplayChange);
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(last_text(&w), "화면이 바뀌어 그리기를 껐어요");
    s.draw_finish(finish(changed, 4));
    let kept = s.kept_ink().unwrap();
    assert_eq!((kept.document.clone(), kept.layout_fingerprint.as_str()), (doc(4), "0,0,1920,1080@100|1920,0,2880,1800@150"));
    assert_eq!(last_text(&w), "화면이 바뀌어 그리기를 껐어요");

    // 절전도 같은 규칙
    s.on_hotkey("draw");
    s.on_system_event(SystemEvent::Suspend);
    assert_eq!(s.mode(), Mode::Idle);
}

#[test]
fn 내보내기_복사와_저장은_원본_바이트를_포트로() {
    let w = FakeWorld::new();
    // 내보내기는 늘 모니터 전체 — 시험은 4×2 모니터로 작게
    {
        let mut scr = w.screen.0.lock().unwrap();
        scr.layout.monitors[0].bounds = PhysRect::new(0, 0, 4, 2);
        scr.layout.monitors[0].work = PhysRect::new(0, 0, 4, 2);
        scr.cursor = PhysPoint { x: 1, y: 1 };
    }
    let mut s = service(&w);
    let px = |w: u32, h: u32| vec![7u8; (w * h * 4) as usize];
    // 판서 중이 아니면 받지 않는다
    assert!(!s.draw_export(export(ExportAction::Copy, 4, 2, None), px(4, 2)).ok);

    s.on_hotkey("draw");
    let session = session_of(&s);
    let r = s.draw_export(export(ExportAction::Copy, 4, 2, Some(session)), px(4, 2));
    assert!(r.ok && r.name.is_none());
    assert_eq!(w.desk.0.lock().unwrap().images, vec![(4, 2)]);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str(), island.value.as_deref()), (IslandKind::Result, "복사했어요", Some("4×2")));
    assert!(matches!(s.mode(), Mode::Draw { .. }), "복사·저장은 판서를 끝내지 않는다");

    // 저장: 캡처 저장과 같은 이름 규칙(설정 템플릿·오늘 순번)
    let r = s.draw_export(export(ExportAction::Save, 4, 2, None), px(4, 2));
    // 이름 = 설정 템플릿(기본값은 브랜드 이름 접두) + 고정 시계 2026-03-04 09:05:07
    let template = ink_domain::settings::Settings::defaults(Platform::Win).str("output.template").to_string();
    let stem = template.replace("{yyyy}{MM}{dd}_{HH}{mm}{ss}", "20260304_090507");
    assert!(!stem.contains('{'), "기본 템플릿 모양이 바뀌면 이 시험도 고친다: {template}");
    assert_eq!((r.ok, r.name.clone()), (true, Some(format!("{stem}.png"))));
    assert_eq!(w.desk.0.lock().unwrap().saved, vec![(String::new(), stem.clone(), 4, 2)]);
    assert_eq!(last_text(&w), "사진 폴더에 저장했어요", "저장 폴더 설정이 비면 사진 폴더");

    // 바이트 수가 틀리거나 지난 세션이거나 모니터 전체 크기가 아니면(확대 중 보이는 부분 등) 아무것도 쓰지 않는다
    assert!(!s.draw_export(export(ExportAction::Copy, 4, 2, None), px(4, 1)).ok);
    assert!(!s.draw_export(export(ExportAction::Save, 4, 2, Some(session + 1)), px(4, 2)).ok);
    let part = s.draw_export(export(ExportAction::Copy, 2, 2, None), px(2, 2));
    assert_eq!((part.ok, part.error.as_deref()), (false, Some("not monitor size")));
    assert!(!s.draw_export(export(ExportAction::Save, 2, 4, None), px(2, 4)).ok, "가로세로가 바뀌어도 거절");
    let written = { let d = w.desk.0.lock().unwrap(); (d.images.len(), d.saved.len()) };
    assert_eq!(written, (1, 1));

    // 클립보드가 잠겨 있으면 문제 섬
    w.desk.0.lock().unwrap().clipboard_locked = true;
    let r = s.draw_export(export(ExportAction::Copy, 4, 2, None), px(4, 2));
    assert!(!r.ok);
    assert_eq!((w.last_island().unwrap().kind, last_text(&w)), (IslandKind::Problem, "복사하지 못했어요".to_string()));
}

#[test]
fn 이중_진입_막기() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("draw");
    let first = session_of(&s);
    // 같은 키 = 끝(두 번째 판서를 열지 않는다)
    assert_eq!(s.on_hotkey("draw"), HotkeyOutcome::Draw);
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.overlay.0.lock().unwrap().draws.len(), 1);
    // 사용자가 끝냈으니 늦은 끝으로 보관되면 섬으로 알린다
    s.draw_finish(finish(first, 1));
    assert_eq!(last_text(&w), "그리기를 보관했어요");
    assert_eq!(w.last_island().unwrap().detail.as_deref(), Some("Alt+Shift+1로 다시 열기"));

    // 트레이에서 두 번: 앞 세션을 끝내고 하나만 보인다
    s.start_draw(DrawKind::Draw);
    let a = session_of(&s);
    s.start_draw(DrawKind::Draw);
    let b = session_of(&s);
    assert!(b > a);
    assert_eq!(w.overlay.0.lock().unwrap().showing.map(|(x, _)| x), Some(b));
    // 끝난 세션의 늦은 명령: 계측·복원은 무시, 끝은 보관만(조용히)
    assert!(s.draw_ready(a, None).is_none());
    assert_eq!(s.draw_restore(a).status, KeptCompat::None);
    // 모르는 세션의 끝이 먼저 와도 기다리던 늦은 끝은 남아 있다
    s.draw_finish(finish(b + 7, 8));
    s.draw_finish(finish(a, 6));
    assert_eq!(s.kept_ink().unwrap().document, doc(6));
    assert_eq!(session_of(&s), b, "지금 판서는 그대로");
    // 모르는 세션의 끝은 아무 일도 없다
    s.draw_finish(finish(a, 9));
    s.draw_finish(finish(b + 5, 9));
    assert_eq!(s.kept_ink().unwrap().document, doc(6));
    assert_eq!(session_of(&s), b);

    // 판서 중 칠판 키 = 판서를 끝내고 칠판으로(창은 하나)
    assert_eq!(s.on_hotkey("board"), HotkeyOutcome::Draw);
    assert!(matches!(s.mode(), Mode::Draw { board: true, .. }));
    assert_eq!(w.overlay.0.lock().unwrap().showing.map(|(x, _)| x), Some(session_of(&s)));
}

#[test]
fn 판서와_캡처는_서로_끝내고_전환() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    // 캡처 중 Alt+1 = 캡처를 끝내고 판서
    s.on_hotkey("region");
    assert!(matches!(s.mode(), Mode::Capture { .. }));
    assert_eq!(s.on_hotkey("draw"), HotkeyOutcome::Draw);
    assert!(matches!(s.mode(), Mode::Draw { .. }));
    assert!(s.snapshot().is_none(), "캡처 스냅숏을 버린다");
    let drawing = session_of(&s);

    // 판서 중 F1 = 판서를 끝내고 캡처(늦은 끝은 조용히 보관 — 캡처 섬을 덮지 않음)
    assert_eq!(s.on_hotkey("region"), HotkeyOutcome::Captured);
    assert!(matches!(s.mode(), Mode::Capture { kind: CaptureKind::Region, .. }));
    let capture_island = w.last_island().unwrap();
    s.draw_finish(finish(drawing, 2));
    assert_eq!(s.kept_ink().unwrap().document, doc(2));
    assert_eq!(w.last_island().unwrap(), capture_island);
    // 판서 중 F2는 활성 창이 아니라 그냥 창 캡처로
    s.end_capture();
    s.on_hotkey("draw");
    assert_eq!(s.on_hotkey("window"), HotkeyOutcome::Captured);
    assert!(matches!(s.mode(), Mode::Capture { kind: CaptureKind::Window, .. }));
}

#[test]
fn 보안_화면_캡처_실패_창_실패는_판서를_열지_않는다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    w.probe.0.lock().unwrap().secure_desktop = true;
    s.on_hotkey("draw");
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.last_island().unwrap().variant.as_deref(), Some("security"));
    w.probe.0.lock().unwrap().secure_desktop = false;

    w.screen.0.lock().unwrap().capture_fails = true;
    s.on_hotkey("draw");
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!((w.last_island().unwrap().kind, last_text(&w)), (IslandKind::Problem, "화면을 가져오지 못했어요".to_string()));
    w.screen.0.lock().unwrap().capture_fails = false;

    w.overlay.0.lock().unwrap().fail_show = true;
    s.on_hotkey("draw");
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(last_text(&w), "그리기 화면을 열지 못했어요");
    assert!(w.overlay.0.lock().unwrap().draws.is_empty());
}

#[test]
fn 판서_섬_갱신과_펜_지연_계측() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    // 판서 중이 아니면 섬을 바꾸지 않는다
    let before = w.notifier.0.lock().unwrap().islands.len();
    s.draw_island(DrawIslandPatch { zoom: Some(2.0), ..Default::default() });
    assert_eq!(w.notifier.0.lock().unwrap().islands.len(), before);

    s.on_hotkey("draw");
    let session = session_of(&s);
    s.draw_island(DrawIslandPatch { session: Some(session), color: Some("G".into()), zoom: Some(2.0), notice: Some("수평으로 맞췄어요".into()), ..Default::default() });
    let islands = w.notifier.0.lock().unwrap().islands.clone();
    let (mode, notice) = (&islands[islands.len() - 2], &islands[islands.len() - 1]);
    assert_eq!((mode.kind, mode.pen_color.as_deref(), mode.value.as_deref(), mode.pen_width.as_deref()), (IslandKind::Mode, Some("G"), Some("×2.0"), Some("4px")));
    assert_eq!((notice.kind, notice.text.as_str()), (IslandKind::Info, "수평으로 맞췄어요"));
    // 조각은 보낸 것만 바꾼다(색은 그대로)
    s.draw_island(DrawIslandPatch { tool: Some("eraser".into()), ..Default::default() });
    let ev = w.last_island().unwrap();
    assert_eq!((ev.text.as_str(), ev.pen_color.as_deref()), ("그리기 · 지우개", Some("G")));
    // 지난 세션 조각은 무시
    let n = w.notifier.0.lock().unwrap().islands.len();
    s.draw_island(DrawIslandPatch { session: Some(session + 1), tool: Some("pen".into()), ..Default::default() });
    assert_eq!(w.notifier.0.lock().unwrap().islands.len(), n);

    assert_eq!(s.draw_pen_metrics(PenSample { n: 120, p50: 7.5, p95: 13.25, max: 40.0 }).as_deref(), Some("pen latency n=120 p50=7.5 p95=13.2 max=40.0 ms"));
    assert_eq!(s.draw_pen_metrics(PenSample::default()), None);

    // 멈춤 도형 제안: 섬 글이 제안이 되고 [바꾸기] 단추 → 섬 명령으로 엔진에
    s.draw_island(DrawIslandPatch { suggestion: Some("원으로 바꿀까요? · Enter".into()), ..Default::default() });
    let ev = w.last_island().unwrap();
    assert_eq!((ev.kind, ev.text.as_str()), (IslandKind::Mode, "원으로 바꿀까요? · Enter"));
    assert_eq!(ev.button.as_ref().map(|b| (b.label.as_str(), b.action.as_str())), Some(("바꾸기", "draw.acceptSuggestion")));
    assert!(s.draw_command("acceptSuggestion"));
    // 제안 없이 오면 걷힌다
    s.draw_island(DrawIslandPatch::default());
    assert_eq!((w.last_island().unwrap().text.as_str(), w.last_island().unwrap().button.is_none()), ("그리기 · 지우개", true));
    // 모두 지우기 안내에는 [되돌리기] — 설정 되돌리기(undo)와 다른 동작 이름
    s.draw_island(DrawIslandPatch { notice: Some("판서를 모두 지웠어요".into()), notice_action: Some("undo".into()), ..Default::default() });
    assert_eq!(w.last_island().unwrap().button.map(|b| b.action), Some("draw.undo".to_string()));
    assert!(s.draw_command("undo"));
    assert!(!s.draw_command("format-disk"), "모르는 명령은 넘기지 않는다");
    assert_eq!(w.notifier.0.lock().unwrap().draw_commands, vec![(session, "acceptSuggestion".to_string()), (session, "undo".to_string())]);
    s.draw_finish(finish(session, 0));
    assert!(!s.draw_command("undo"), "판서 중이 아니면 무시");
}

#[test]
fn 처음_상태로_되돌리면_보관_판서도_지운다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("draw");
    s.draw_finish(finish(session_of(&s), 1));
    assert!(s.kept_ink().is_some());
    s.reset_all();
    assert!(s.kept_ink().is_none());
}

#[test]
fn 판서와_실시간_확대는_서로_끝내고_전환() {
    let w = FakeWorld::new();
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.set_magnifier(w.magnifier_port());
    s.start();
    // 확대 중 Alt+1 = 확대를 끝내고 그 배율 그대로 판서(5.3 — 자세한 값은 tests/handoff.rs)
    s.on_hotkey("zoom");
    assert!(matches!(s.mode(), Mode::LiveZoom { .. }));
    let h = s.zoom_handoff().unwrap();
    assert_eq!(s.on_hotkey("draw"), HotkeyOutcome::Draw);
    assert!(matches!(s.mode(), Mode::Draw { board: false, .. }));
    assert!(w.magnifier.0.lock().unwrap().running.is_none(), "돋보기를 끈다");
    assert_eq!(s.draw_session().unwrap().initial_view.s, h.s);
    let drawing = session_of(&s);
    // 판서 중 Alt+2 = 판서를 끝내고(늦은 끝으로 보관) 확대
    s.on_hotkey("zoom");
    assert!(matches!(s.mode(), Mode::LiveZoom { .. }));
    s.draw_finish(finish(drawing, 1));
    assert_eq!(s.kept_ink().unwrap().document, doc(1));
    // 트레이 판서도 확대를 끝내고 연다
    s.start_draw(DrawKind::Board);
    assert!(matches!(s.mode(), Mode::Draw { board: true, .. }));
    // 판서 중 트레이 [실시간 확대]도 Alt+2처럼 판서를 끝내고(늦은 끝으로 보관) 확대
    let board = session_of(&s);
    s.start_zoom();
    assert!(matches!(s.mode(), Mode::LiveZoom { .. }), "트레이 확대는 판서를 끝낸다");
    assert!(w.overlay.0.lock().unwrap().showing.is_none(), "판서 오버레이를 숨긴다");
    s.draw_finish(finish(board, 2));
    assert_eq!(s.kept_ink().unwrap().document, doc(2), "늦은 끝으로 보관");
}
