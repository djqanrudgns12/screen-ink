//! 단계 6 통합 시험(가짜 세계): F2 창 캡처·대체 사슬 · 바로 캡처 키 4종 · F3 녹화 범위·자동 중지·복구·겹침.

use ink_app::{CaptureInput, Frame, HotkeyOutcome, InkService, IslandKind, SystemEvent, WindowShot, REC_WINDOW_CLOSED, WINDOW_PROTECTED};
use ink_domain::geometry::{Monitor, MonitorLayout, PhysRect};
use ink_domain::mode::Mode;
use ink_domain::recording::{Leftover, RecTarget};
use ink_domain::shortcuts::Platform;
use ink_domain::snapshot::CaptureKind;
use ink_platform_fake::FakeWorld;
use std::sync::Arc;

const WIN: PhysRect = PhysRect::new(100, 100, 1200, 800);

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.start();
    w.install_recorder(&mut s);
    s
}

fn shot(w: u32, h: u32, v: u8) -> Result<WindowShot, String> {
    Ok(WindowShot { width: w, height: h, bgra: (0..w * h).flat_map(|_| [v, v, v, 255]).collect(), method: "wgc", corner_radius: 0 })
}

fn click(s: &mut InkService, x: i32, y: i32) {
    s.capture_input(CaptureInput::Down { x, y, button: 0 });
    s.capture_input(CaptureInput::Up { x, y, button: 0 });
}

/// 첫 모니터에 색이 있는 정지 이미지(대체 경로가 "내용"으로 판정하게).
fn colored_frames(w: &FakeWorld) {
    let m = PhysRect::new(0, 0, 1920, 1080);
    let px: Vec<u8> = (0..m.w * m.h).flat_map(|_| [40u8, 80, 120, 255]).collect();
    w.screen.0.lock().unwrap().frames = Some(vec![Frame { monitor_id: 0, width: 1920, height: 1080, bgra: Arc::new(px) }]);
}

// ───────────── 6.1 F2 ─────────────

#[test]
fn f2_창_클릭은_그_창만_wgc로_찍고_고르는_동안_미리_준비한다() {
    let w = FakeWorld::new();
    w.screen.0.lock().unwrap().window_shots.insert(1, shot(120, 80, 200));
    let mut s = service(&w);
    assert_eq!(s.on_hotkey("window"), HotkeyOutcome::Captured);
    // 진입 순간 커서 아래 창을 강조하고 캡처를 미리 준비
    assert_eq!(w.screen.0.lock().unwrap().prepared_windows, vec![Some(1)]);
    assert_eq!(w.overlay.0.lock().unwrap().highlights.last().copied().flatten(), Some(WIN));
    // 창 밖(바탕 화면)으로 → 준비 버림, 다시 창으로 → 다시 준비
    s.capture_input(CaptureInput::Move { x: 1500, y: 950 });
    s.capture_input(CaptureInput::Move { x: 400, y: 300 });
    s.capture_input(CaptureInput::Move { x: 401, y: 301 });
    assert_eq!(w.screen.0.lock().unwrap().prepared_windows, vec![Some(1), None, Some(1)], "같은 창 안 이동은 다시 준비하지 않음");
    click(&mut s, 400, 300);
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.screen.0.lock().unwrap().window_captures, vec![1]);
    assert_eq!(w.desk.0.lock().unwrap().images, vec![(120, 80)], "창 내용 크기 그대로(가려진 부분 포함)");
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str()), (IslandKind::Result, "창을 복사했어요"));
    assert_eq!(island.value.as_deref(), Some("120×80"));
    assert_eq!(island.detail.as_deref(), Some("가려진 부분까지 · Ctrl+V"));
    assert_eq!(w.screen.0.lock().unwrap().prepared_windows.last(), Some(&None), "모드가 끝나면 준비를 버린다");
}

#[test]
fn f2_wgc가_안되면_스냅숏을_자르고_보호화면은_복사하지_않는다() {
    // 1) WGC 시간 초과 → 정지 스냅숏에서 창 사각형
    let w = FakeWorld::new();
    w.screen.0.lock().unwrap().window_shots.insert(1, Err("first frame timeout".into()));
    colored_frames(&w);
    let mut s = service(&w);
    s.on_hotkey("window");
    s.capture_input(CaptureInput::Key { key: "Enter".into(), code: "Enter".into(), ctrl: false, shift: false, alt: false, meta: false });
    assert_eq!(w.desk.0.lock().unwrap().images, vec![(1200, 800)]);
    assert_eq!(w.last_island().unwrap().detail.as_deref(), Some("문서·발표 자료에 붙여 넣기 · Ctrl+V"), "자른 그림은 가린 창도 찍혀서 '가려진 부분까지'라 하지 않음");

    // 2) 표시 선호도가 걸린 창(어댑터가 WINDOW_PROTECTED) → 대체 없이 안내, 클립보드 그대로
    w.screen.0.lock().unwrap().window_shots.insert(1, Err(format!("{WINDOW_PROTECTED}: display affinity")));
    s.on_hotkey("window");
    click(&mut s, 400, 300);
    let island = w.last_island().unwrap();
    assert_eq!((island.kind, island.text.as_str()), (IslandKind::Problem, "이 창은 가져올 수 없어요"));
    assert_eq!(w.desk.0.lock().unwrap().images.len(), 1, "빈 이미지를 넣지 않는다");
    assert_eq!(s.mode(), Mode::Idle);

    // 3) WGC가 (0,0,0,255) + 스냅숏도 검정 → 보호 화면
    let w = FakeWorld::new();
    w.screen.0.lock().unwrap().window_shots.insert(1, shot(64, 48, 0));
    let mut s = service(&w);
    s.on_hotkey("window");
    click(&mut s, 400, 300);
    assert_eq!(w.last_island().unwrap().text, "이 창은 가져올 수 없어요");
    assert!(w.desk.0.lock().unwrap().images.is_empty());
}

#[test]
fn f2_창_없는_자리_클릭과_esc_우클릭() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("window");
    s.capture_input(CaptureInput::Move { x: 1500, y: 950 });
    click(&mut s, 1500, 950);
    assert_eq!(w.last_island().unwrap().text, "찍을 창이 없어요");
    assert!(matches!(s.mode(), Mode::Capture { kind: CaptureKind::Window, .. }), "모드는 그대로");
    s.capture_input(CaptureInput::Key { key: "Escape".into(), code: "Escape".into(), ctrl: false, shift: false, alt: false, meta: false });
    assert_eq!(s.mode(), Mode::Idle);
    s.on_hotkey("window");
    s.capture_input(CaptureInput::Down { x: 400, y: 300, button: 2 });
    assert_eq!(s.mode(), Mode::Idle);
    assert!(w.screen.0.lock().unwrap().window_captures.is_empty());
}

// ───────────── 6.2 바로 캡처 키 ─────────────

#[test]
fn f2_400ms_두번_활성창_및_바로캡처_4종() {
    let w = FakeWorld::new();
    w.screen.0.lock().unwrap().window_shots.insert(1, shot(120, 80, 9));
    let mut s = service(&w);

    // Shift+F1: 기록이 없으면 안내
    assert_eq!(s.on_hotkey("last"), HotkeyOutcome::Captured);
    assert_eq!(w.last_island().unwrap().text, "이전 범위가 없어요");

    // Ctrl+F1: 커서 모니터 전체
    s.on_hotkey("full");
    assert_eq!(w.desk.0.lock().unwrap().images.last(), Some(&(1920, 1080)));
    assert_eq!(s.mode(), Mode::Idle, "고르기 화면 없이");
    assert!(w.overlay.0.lock().unwrap().showing.is_none());

    // Shift+F1: 방금 범위(모니터 전체)를 다시
    s.on_hotkey("last");
    assert_eq!(w.desk.0.lock().unwrap().images.last(), Some(&(1920, 1080)));

    // Shift+F2: 지금 창이 없으면 안내 → 있으면 그 창
    s.on_hotkey("active");
    assert_eq!(w.last_island().unwrap().text, "지금 창이 없어요");
    w.probe.0.lock().unwrap().foreground = Some(1);
    s.on_hotkey("active");
    assert_eq!(w.desk.0.lock().unwrap().images.last(), Some(&(120, 80)));

    // F2 두 번(400 ms 안) = 활성 창(강조 없이 바로)
    let before = w.desk.0.lock().unwrap().images.len();
    s.on_hotkey("window");
    assert_eq!(s.on_hotkey("window"), HotkeyOutcome::Captured);
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.desk.0.lock().unwrap().images.len(), before + 1);
    assert_eq!(w.screen.0.lock().unwrap().window_captures, vec![1, 1]);

    // F2로 창을 찍으면 Shift+F1의 "직전 범위"는 그 창 자리
    s.on_hotkey("window");
    click(&mut s, 400, 300);
    s.on_hotkey("last");
    assert_eq!(w.desk.0.lock().unwrap().images.last(), Some(&(1200, 800)));

    // Alt+F1: 3초 카운트다운 → 0이면 F1
    let t0 = s.clock_ms();
    s.on_hotkey("delay");
    let island = w.last_island().unwrap();
    assert_eq!((island.variant.as_deref(), island.countdown, island.text.as_str()), (Some("delay"), Some(3), "초 뒤 범위 캡처"));
    assert_eq!(island.hints[0].key, "Esc", "카운트다운 동안만 Esc로 취소");
    let esc = ink_domain::shortcuts::to_accelerator("Esc", Platform::Win).unwrap();
    assert!(w.is_registered(&esc), "Esc 임시 전역 등록");
    s.tick_at(t0 + 1_100);
    assert_eq!(w.last_island().unwrap().countdown, Some(2));
    let n = w.islands().len();
    s.tick_at(t0 + 1_200);
    assert_eq!(w.islands().len(), n, "초가 바뀔 때만 다시 그림");
    s.tick_at(t0 + 3_500);
    assert!(matches!(s.mode(), Mode::Capture { kind: CaptureKind::Region, .. }));
    assert!(s.delay_pending().is_none());
    assert!(!w.is_registered(&esc), "끝나면 Esc를 놓는다");
    s.end_capture();

    // 카운트다운 중 Esc(어댑터가 칸 delay로 넘김)·같은 키 = 취소
    s.on_hotkey("delay");
    assert_eq!(w.hotkeys.0.lock().unwrap().registered.get(&esc).map(String::as_str), Some("delay"));
    s.on_hotkey("delay");
    assert_eq!(w.last_island().unwrap().text, "지연 캡처를 취소했어요");
    assert!(!w.is_registered(&esc));
    s.tick_at(s.clock_ms() + 10_000);
    assert_eq!(s.mode(), Mode::Idle);
}

#[test]
fn 지연_초는_설정을_따르고_보안_화면에서는_시작하지_않는다() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    assert!(s.update_setting("capture.delay", &serde_json::json!(5)).ok);
    assert!(!s.update_setting("capture.delay", &serde_json::json!(4)).ok, "3·5·10만");
    s.on_hotkey("delay");
    assert_eq!(w.last_island().unwrap().countdown, Some(5));
    s.on_hotkey("delay");
    w.probe.0.lock().unwrap().secure_desktop = true;
    s.on_hotkey("full");
    assert_eq!(w.last_island().unwrap().variant.as_deref(), Some("security"));
    assert!(w.desk.0.lock().unwrap().images.is_empty());
}

// ───────────── 6.3 F3 녹화 ─────────────

fn last_target(w: &FakeWorld) -> RecTarget {
    w.recorder.0.lock().unwrap().started.last().expect("녹화 시작").target.clone()
}

#[test]
fn f3_모니터_클릭은_모니터_창_클릭은_창_추적_드래그는_짝수_범위() {
    let w = FakeWorld::new();
    let mut s = service(&w);

    // 모니터 한 번 클릭(진입 그대로) → 모니터 전체
    s.on_hotkey("record");
    assert_eq!(w.recorder.0.lock().unwrap().prepared, 1, "고르기 진입 때 녹화기를 미리 준비");
    click(&mut s, 400, 300);
    assert_eq!(last_target(&w), RecTarget::Monitor { monitor_id: 0, rect: PhysRect::new(0, 0, 1920, 1080) });
    assert!(matches!(s.mode(), Mode::Recording { .. }));
    assert!(w.overlay.0.lock().unwrap().showing.is_none(), "고르기 화면을 먼저 숨긴다");
    let st = s.state();
    assert!(st.recording.active);
    assert_eq!((st.recording.width, st.recording.height, st.recording.fps, st.recording.target), (1920, 1080, 30, Some("monitor")));
    let opts = w.recorder.0.lock().unwrap().started[0].clone();
    assert!(opts.temp_path.to_string_lossy().starts_with(ink_platform_fake::FAKE_TEMP_DIR));
    assert!(opts.temp_path.to_string_lossy().ends_with(".mp4"));
    assert_eq!((opts.fps, opts.mic_device, opts.capture_cursor), (30, None, true));
    s.on_hotkey("record");
    assert_eq!(s.mode(), Mode::Idle);
    assert!(!s.state().recording.active);

    // 창 위에서 클릭 → 창 추적(사용자 결정 #6)
    s.on_hotkey("record");
    s.capture_input(CaptureInput::Move { x: 420, y: 320 });
    click(&mut s, 420, 320);
    assert_eq!(last_target(&w), RecTarget::Window { handle: 1, rect: WIN });
    s.stop_recording();

    // 드래그 → 한 모니터 안 짝수 범위
    s.on_hotkey("record");
    s.capture_input(CaptureInput::Down { x: 30, y: 30, button: 0 });
    s.capture_input(CaptureInput::Move { x: 200, y: 150 });
    s.capture_input(CaptureInput::Move { x: 331, y: 232 });
    s.capture_input(CaptureInput::Up { x: 331, y: 232, button: 0 });
    match last_target(&w) {
        RecTarget::Region { monitor_id: 0, rect } => assert!(rect.w % 2 == 0 && rect.h % 2 == 0 && rect.w >= 290 && rect.h >= 190, "{rect:?}"),
        t => panic!("범위여야 함: {t:?}"),
    }
    s.stop_recording();
    assert_eq!(w.recorder.0.lock().unwrap().stopped, 3);
}

#[test]
fn 녹화_섬은_1초마다_경과를_보이고_중지하면_이름_규칙으로_저장() {
    let w = FakeWorld::new();
    w.recorder.0.lock().unwrap().bytes = 18 * 1_048_576;
    w.recorder.0.lock().unwrap().elapsed_ms = 42_300;
    let mut s = service(&w);
    // 첫 녹화: 안내 한 줄 → 기록(설정 되돌리기에는 안 남김)
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    let first = w.last_island().unwrap();
    assert_eq!((first.variant.as_deref(), first.text.as_str()), (Some("record"), "그리기도 녹화 · 안내는 제외"));
    assert_eq!(first.button.as_ref().map(|b| b.action.as_str()), Some("record.stop"));
    assert_eq!(first.button_key.as_deref(), Some("F3"));
    assert!(first.mic_off);
    assert!(s.settings().bool("record.firstHintDone"));
    assert!(!s.can_undo());
    let t0 = s.clock_ms();
    let n = w.islands().len();
    s.tick_at(t0 + 1_500);
    assert_eq!(w.islands().len(), n, "첫 안내 2초 동안은 덮지 않음");
    s.tick_at(t0 + 2_600);
    let ev = w.last_island().unwrap();
    assert_eq!(ev.text, "녹화 중");
    assert!(ev.value.as_deref().unwrap().starts_with("00:02 · 1920×1080 · 30fps"), "{:?}", ev.value);
    s.stop_recording();
    let ev = w.last_island().unwrap();
    assert_eq!((ev.kind, ev.value.as_deref()), (IslandKind::Result, Some("00:42 · 18 MB")));
    // 저장 폴더를 비워 두면(기본) 사진 폴더 — 그 자리를 알린다
    assert_eq!(ev.text, "사진 폴더에 저장했어요");
    assert_eq!(ev.button.as_ref().map(|b| b.action.as_str()), Some("folder.open"));
    let b = w.recorder.0.lock().unwrap();
    assert_eq!(b.kept.len(), 1);
    assert_eq!(b.kept[0].2, "Shookshok_20260304_090507", "캡처와 같은 이름 규칙");
    assert!(b.temp_files.is_empty(), "임시 파일은 옮겨짐");
}

#[test]
fn 녹화중_잠금_절전_모니터해제_자동중지_저장() {
    for (ev, reason) in [(SystemEvent::Lock, "화면 잠김"), (SystemEvent::Suspend, "절전 전환")] {
        let w = FakeWorld::new();
        let mut s = service(&w);
        s.on_hotkey("record");
        click(&mut s, 400, 300);
        s.on_system_event(ev);
        assert!(!s.recording_active(), "{ev:?}");
        assert_eq!(w.recorder.0.lock().unwrap().kept.len(), 1, "{ev:?}: 임시 파일을 저장 폴더로");
        let island = w.last_island().unwrap();
        assert_eq!(island.kind, IslandKind::Result);
        assert_eq!(island.text, "사진 폴더에 저장했어요");
        assert!(island.detail.as_deref().is_some_and(|d| d.starts_with(reason)), "{:?}", island.detail);
    }

    // 배치 변경: 녹화하던 모니터가 그대로면(설정 바뀜 알림 등) 계속, 사라지면 저장
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    let n = w.islands().len();
    s.on_system_event(SystemEvent::DisplayChange);
    assert!(s.recording_active(), "같은 배치 → 계속");
    assert_eq!(w.islands().len(), n, "'캡처를 닫았어요'를 띄우지 않음");
    w.screen.0.lock().unwrap().layout = MonitorLayout::new(vec![Monitor { id: 0, bounds: PhysRect::new(1920, 0, 2880, 1800), work: PhysRect::new(1920, 0, 2880, 1728), scale: 1.5, primary: true }]);
    s.on_system_event(SystemEvent::DisplayChange);
    assert!(!s.recording_active());
    let island = w.last_island().unwrap();
    assert_eq!(island.text, "사진 폴더에 저장했어요");
    assert!(island.detail.as_deref().is_some_and(|d| d.starts_with("화면이 바뀜")), "{:?}", island.detail);
}

#[test]
fn 녹화중_f1_캡처_겹침_허용_및_임시파일_복구제안() {
    let w = FakeWorld::new();
    w.recorder.0.lock().unwrap().leftovers = vec![
        Leftover { path: format!("{}/rec-1.mp4", ink_platform_fake::FAKE_TEMP_DIR), bytes: 10, playable: true },
        Leftover { path: format!("{}/rec-2.mp4", ink_platform_fake::FAKE_TEMP_DIR), bytes: 10, playable: false },
    ];
    let mut s = service(&w);
    assert_eq!(s.leftover_recordings().len(), 2);
    // 첫 틱에 복구 제안(창이 다 만들어진 뒤)
    s.tick_at(s.clock_ms());
    let ev = w.last_island().unwrap();
    assert_eq!(ev.text, "저장 못 한 녹화가 2개예요");
    assert_eq!(ev.button.as_ref().map(|b| b.action.as_str()), Some("record.recoverSave"));
    assert_eq!(ev.chip.as_ref().map(|b| b.action.as_str()), Some("record.recoverDelete"));
    assert_eq!(ev.detail.as_deref(), Some("재생되지 않을 수도 있어요"));
    let n = w.islands().len();
    s.tick_at(s.clock_ms());
    assert_eq!(w.islands().len(), n, "한 번만");
    s.recover_leftovers(true);
    assert_eq!(w.recorder.0.lock().unwrap().kept.len(), 2);
    assert_eq!(w.last_island().unwrap().text, "녹화 2개를 저장했어요");
    assert!(s.leftover_recordings().is_empty());

    // 녹화 중 F1 → 캡처 정상(녹화 계속), 끝나면 녹화 상태로
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    assert_eq!(s.on_hotkey("region"), HotkeyOutcome::Captured);
    assert!(matches!(s.mode(), Mode::Capture { kind: CaptureKind::Region, .. }));
    assert!(s.recording_active());
    s.capture_input(CaptureInput::Command { name: "copy".into() });
    assert!(matches!(s.mode(), Mode::Recording { .. }), "캡처가 끝나면 녹화로 돌아옴");
    assert!(s.recording_active());
    assert_eq!(w.recorder.0.lock().unwrap().stopped, 0);
    // 다른 모드가 섬을 쓰는 중에는 녹화 섬으로 덮지 않음 → 돌아오면 다시
    let t = s.clock_ms() + 5_000;
    s.tick_at(t);
    assert_eq!(w.last_island().unwrap().text, "녹화 중");
    // 녹화 중 F3 고르기를 띄우지 않고 중지
    assert_eq!(s.on_hotkey("record"), HotkeyOutcome::Captured);
    assert!(!s.recording_active());
    assert_eq!(s.mode(), Mode::Idle);
}

#[test]
fn 시작_실패_스스로_멈춤_버리기_마이크_공간_안내() {
    let w = FakeWorld::new();
    w.recorder.0.lock().unwrap().fail_start = Some("MF_E_TOPO_CODEC_NOT_FOUND".into());
    let mut s = service(&w);
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    assert_eq!(w.last_island().unwrap().text, "녹화를 시작하지 못했어요");
    assert!(!s.state().recording.active);
    assert_eq!(s.mode(), Mode::Idle);

    // 창이 닫혀 녹화기가 스스로 멈춤 → 저장
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    w.recorder.0.lock().unwrap().failure = Some(REC_WINDOW_CLOSED.into());
    s.tick();
    assert!(!s.recording_active());
    let island = w.last_island().unwrap();
    assert_eq!(island.text, "사진 폴더에 저장했어요");
    assert!(island.detail.as_deref().is_some_and(|d| d.starts_with("창 닫힘")), "{:?}", island.detail);

    // 버리기 → 임시 파일 지움
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    s.discard_recording();
    assert_eq!(w.last_island().unwrap().text, "녹화를 버렸어요");
    assert_eq!(w.recorder.0.lock().unwrap().aborted, 1);

    // 마무리 실패 → 임시 파일 보존 안내
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    w.recorder.0.lock().unwrap().fail_stop = Some("finalize".into());
    s.stop_recording();
    assert_eq!(w.last_island().unwrap().text, "녹화를 마무리하지 못했어요");
    assert!(!w.recorder.0.lock().unwrap().temp_files.is_empty(), "다음 시작 때 복구");

    // 마이크를 켰지만 녹화기가 못 하면 끄고 한 번 알림 · 남은 공간이 10분을 못 채우면 알리고 그대로 시작
    assert!(s.update_setting("record.mic", &serde_json::json!(true)).ok);
    w.recorder.0.lock().unwrap().free = Some(100_000_000);
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    let texts: Vec<String> = w.islands().iter().rev().take(3).map(|e| e.text.clone()).collect();
    assert!(texts.contains(&"마이크 녹음은 준비 중이에요".to_string()), "{texts:?}");
    assert!(texts.contains(&"남은 공간이 2분 정도예요".to_string()), "{texts:?}");
    assert!(s.recording_active());
    assert_eq!(w.recorder.0.lock().unwrap().started.last().unwrap().mic_device, None);
    s.stop_recording();
}

#[test]
fn 녹화기가_없는_곳은_안내만() {
    let w = FakeWorld::new();
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.start();
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    assert_eq!(w.last_island().unwrap().text, "화면 녹화를 지원하지 않아요");
    assert!(!s.recording_active());
}

#[test]
fn 녹화중_alt1_판서_겹침_허용_끝나면_녹화로() {
    let w = FakeWorld::new();
    let mut s = service(&w);
    s.on_hotkey("record");
    click(&mut s, 400, 300);
    assert!(matches!(s.mode(), Mode::Recording { .. }));
    // Alt+1 = 녹화를 끊지 않고 그 위에서 판서
    s.on_hotkey("draw");
    assert!(matches!(s.mode(), Mode::Draw { .. }), "{:?}", s.mode());
    assert!(s.recording_active());
    // 판서를 끝내면 녹화 상태로 돌아온다
    s.on_hotkey("draw");
    assert!(matches!(s.mode(), Mode::Recording { .. }), "{:?}", s.mode());
    assert!(s.recording_active());
    assert_eq!(w.recorder.0.lock().unwrap().stopped, 0);
    s.on_hotkey("record");
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.recorder.0.lock().unwrap().stopped, 1);
}
