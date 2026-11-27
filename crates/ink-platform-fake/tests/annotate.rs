//! 캡처 주석(4.9, R-CAP-28): 주석이 있으면 결과(복사·저장·핀)에 주석 층과 모자이크가 합성되고,
//! 주석이 없으면 지금 경로 그대로(부탁 0번). 판정(키·단추·더블클릭 → 결과)은 늘 Rust 고르기 상태 기계.

use ink_app::capture::{AnnotatedCommit, CommitAction, CommitMeta, CommitRequest};
use ink_app::{CaptureInput, Frame, InkService};
use ink_domain::geometry::{PhysPoint, PhysRect};
use ink_domain::image::MosaicRect;
use ink_domain::mode::Mode;
use ink_domain::recog::{Control, ElementCand};
use ink_domain::shortcuts::Platform;
use ink_platform_fake::FakeWorld;
use std::sync::Arc;

/// 커서(420,320) 아래 강조 = 이 범위(요소 하나).
const RANGE: PhysRect = PhysRect::new(300, 300, 300, 40);

/// 화소 무늬가 있는 1번 모니터 이미지(가상 화면 (x,y) → B·G·R이 위치마다 다름).
fn pattern(x: i32, y: i32) -> [u8; 4] {
    [((x * 7) % 251) as u8, ((y * 13) % 251) as u8, 50, 255]
}

fn world() -> FakeWorld {
    let w = FakeWorld::new();
    w.elements.0.lock().unwrap().by_window.insert(1, vec![ElementCand { rect: RANGE, control: Control::DataItem, from_msaa: false }]);
    let (fw, fh) = (1920i32, 1080i32);
    let bgra: Vec<u8> = (0..fh).flat_map(|y| (0..fw).flat_map(move |x| pattern(x, y))).collect();
    w.screen.0.lock().unwrap().frames = Some(vec![Frame { monitor_id: 0, width: fw as u32, height: fh as u32, bgra: Arc::new(bgra) }]);
    w
}

fn service(w: &FakeWorld) -> InkService {
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.start();
    s
}

/// 캡처를 열고 범위를 강조까지(세션 번호).
fn open(s: &mut InkService) -> u64 {
    s.on_hotkey("region");
    s.capture_input(CaptureInput::Move { x: 420, y: 320 });
    match s.mode() {
        Mode::Capture { session, .. } => session,
        _ => panic!("캡처가 열려야 함"),
    }
}

fn cmd(name: &str) -> CaptureInput {
    CaptureInput::Command { name: name.into() }
}

/// 범위 기준 (x,y)의 원본 화소.
fn orig(x: u32, y: u32) -> [u8; 4] {
    pattern(RANGE.x + x as i32, RANGE.y + y as i32)
}

fn px(img: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

/// 주석 층: (5,5) 불투명 빨강 펜, (12,5) 형광펜 노랑 알파 97, 나머지 투명.
fn layer() -> Vec<u8> {
    let (w, h) = (RANGE.w as u32, RANGE.h as u32);
    let mut v = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: u32, y: u32, rgba: [u8; 4]| {
        let i = ((y * w + x) * 4) as usize;
        v[i..i + 4].copy_from_slice(&rgba);
    };
    put(5, 5, [255, 0, 0, 255]);
    put(12, 5, [255, 214, 10, 97]);
    v
}

/// 모자이크: 범위 안 (100,0)부터 20×20, 칸 10 px → 2×2칸.
const MOSAIC: MosaicRect = MosaicRect { x: 100, y: 0, w: 20, h: 20, block: 10 };

fn answer(req: &CommitRequest) -> AnnotatedCommit {
    AnnotatedCommit {
        meta: CommitMeta { session: req.session, seq: req.seq, action: req.action, monitor_id: None, w: RANGE.w as u32, h: RANGE.h as u32, mosaics: vec![MOSAIC] },
        overlay: layer(),
    }
}

/// 주석이 합성된 결과인지(펜·형광펜·모자이크·바깥 그대로).
fn assert_annotated(img: &[u8]) {
    let w = RANGE.w as u32;
    assert_eq!(px(img, w, 5, 5), [0, 0, 255, 255], "불투명 빨강 펜(RGBA → BGRA)");
    let o = orig(12, 5);
    let a = 97.0 / 255.0;
    let src = [10.0, 214.0, 255.0];
    let got = px(img, w, 12, 5);
    for k in 0..3 {
        let e = src[k] * a + f64::from(o[k]) * (1.0 - a);
        assert!((f64::from(got[k]) - e).abs() <= 1.0, "형광펜 채널 {k}: {} vs {e:.2}", got[k]);
    }
    // 모자이크 왼쪽 위 칸(x 100..109, y 0..9) = 원본 평균
    let mut sum = [0u64; 3];
    for y in 0..10 {
        for x in 100..110 {
            let p = orig(x, y);
            for k in 0..3 {
                sum[k] += u64::from(p[k]);
            }
        }
    }
    let avg: Vec<u8> = sum.iter().map(|s| ((s + 50) / 100) as u8).collect();
    for (x, y) in [(100, 0), (109, 9), (104, 3)] {
        assert_eq!(&px(img, w, x, y)[..3], &avg[..], "모자이크 칸 ({x},{y})");
    }
    assert_ne!(px(img, w, 110, 0), px(img, w, 100, 0), "옆 칸은 다른 평균");
    // 주석 밖은 원본 그대로
    for (x, y) in [(0, 0), (299, 39), (99, 0), (120, 20), (6, 5)] {
        assert_eq!(px(img, w, x, y), orig(x, y), "바깥 ({x},{y})");
    }
}

#[test]
fn 주석이_없으면_지금_경로_그대로_부탁하지_않는다() {
    let w = world();
    let mut s = service(&w);
    open(&mut s);
    s.capture_input(cmd("copy"));
    assert_eq!(s.mode(), Mode::Idle);
    assert!(w.overlay.0.lock().unwrap().commit_requests.is_empty(), "부탁 0번");
    let d = w.desk.0.lock().unwrap();
    assert_eq!(d.images, vec![(300, 40)]);
    for (x, y) in [(5, 5), (12, 5), (100, 0)] {
        assert_eq!(px(&d.image_bgra[0], 300, x, y), orig(x, y), "원본 그대로");
    }
}

#[test]
fn 주석_합성후_복사_결과에_반영() {
    let w = world();
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session, 0, 2);
    assert!(s.capture_annotated());
    // Enter(기본 복사 키) — 판정은 Rust
    s.capture_input(CaptureInput::Key { key: "Enter".into(), code: "Enter".into(), ctrl: false, shift: false, alt: false, meta: false });
    let req = *w.overlay.0.lock().unwrap().commit_requests.last().expect("주석 층을 부탁");
    assert_eq!(req, CommitRequest { session, seq: 1, action: CommitAction::Copy, rect: RANGE });
    assert!(matches!(s.mode(), Mode::Capture { .. }), "답이 올 때까지 캡처 화면 유지");
    assert!(w.desk.0.lock().unwrap().images.is_empty(), "아직 복사 안 함");

    s.capture_commit_annotated(answer(&req));
    assert_eq!(s.mode(), Mode::Idle);
    let d = w.desk.0.lock().unwrap();
    assert_eq!(d.images, vec![(300, 40)]);
    assert_annotated(&d.image_bgra[0]);
    drop(d);
    assert_eq!(w.last_island().unwrap().text, "복사했어요");
    // 섬 [저장]도 주석이 합성된 마지막 결과
    s.save_last();
    assert_annotated(&w.desk.0.lock().unwrap().saved_bgra[0]);
}

#[test]
fn 주석_합성후_저장_결과에_반영() {
    let w = world();
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session, 0, 1);
    s.capture_input(cmd("save"));
    let req = *w.overlay.0.lock().unwrap().commit_requests.last().unwrap();
    assert_eq!(req.action, CommitAction::Save);
    s.capture_commit_annotated(answer(&req));
    let d = w.desk.0.lock().unwrap();
    assert_eq!(d.saved.len(), 1);
    assert!(d.images.is_empty(), "저장만(capture.result = copy여도 저장 단추는 저장)");
    assert_annotated(&d.saved_bgra[0]);
}

#[test]
fn 주석_합성후_핀_그림과_핀_복사에_반영() {
    let w = world();
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session, 0, 3);
    s.capture_input(cmd("pin"));
    let req = *w.overlay.0.lock().unwrap().commit_requests.last().unwrap();
    assert_eq!(req.action, CommitAction::Pin);
    s.capture_commit_annotated(answer(&req));
    assert_eq!(s.mode(), Mode::Idle);
    {
        let board = w.pins.0.lock().unwrap();
        assert_eq!(board.open.get(&1), Some(&(RANGE, 300, 40)), "원래 자리·같은 크기");
        assert_annotated(&board.bgra[&1]);
    }
    s.pin_copy(1);
    assert_annotated(&w.desk.0.lock().unwrap().image_bgra[0]);
}

#[test]
fn 본문_그대로_받아도_같은_결과() {
    let w = world();
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session, 0, 1);
    s.capture_input(CaptureInput::Dblclick { x: 420, y: 320 });
    let req = *w.overlay.0.lock().unwrap().commit_requests.last().unwrap();
    assert_eq!(req.action, CommitAction::Copy, "더블클릭 = 복사");
    let meta = format!(r#"{{"session":{session},"seq":{},"action":"copy","w":300,"h":40,"mosaics":[{{"x":100,"y":0,"w":20,"h":20,"block":10}}]}}"#, req.seq);
    let mut body = (meta.len() as u32).to_le_bytes().to_vec();
    body.extend_from_slice(meta.as_bytes());
    body.extend_from_slice(&layer());
    s.capture_commit_annotated(AnnotatedCommit::decode(&body).unwrap());
    assert_annotated(&w.desk.0.lock().unwrap().image_bgra[0]);
}

#[test]
fn 늦은_답과_다른_세션_답은_무시하고_마지막_부탁만_받는다() {
    let w = world();
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session, 0, 1);
    s.capture_input(cmd("copy"));
    s.capture_input(cmd("copy")); // 두 번 누름 → 새 부탁(seq 2)
    let reqs = w.overlay.0.lock().unwrap().commit_requests.clone();
    assert_eq!(reqs.iter().map(|r| r.seq).collect::<Vec<_>>(), vec![1, 2]);
    s.capture_commit_annotated(answer(&reqs[0]));
    assert!(matches!(s.mode(), Mode::Capture { .. }), "지난 부탁의 답은 버림");
    let mut other = answer(&reqs[1]);
    other.meta.session = session + 9;
    s.capture_commit_annotated(other);
    assert!(matches!(s.mode(), Mode::Capture { .. }), "다른 세션 답은 버림");
    s.capture_commit_annotated(answer(&reqs[1]));
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(w.desk.0.lock().unwrap().images.len(), 1, "결과는 한 번");
    // 모드가 끝난 뒤 늦게 온 답도 조용히
    s.capture_commit_annotated(answer(&reqs[1]));
    assert_eq!(w.desk.0.lock().unwrap().images.len(), 1);
}

#[test]
fn 부탁한_뒤_주석이_사라지면_기다리지_않고_주석_없이_낸다() {
    let w = world();
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session, 0, 1);
    s.capture_input(cmd("copy"));
    assert_eq!(w.overlay.0.lock().unwrap().commit_requests.len(), 1);
    s.capture_annotations(session, 0, 0); // 창이 주석을 버림(Ctrl+Z로 다 지움 등)
    assert_eq!(s.mode(), Mode::Idle);
    assert_eq!(px(&w.desk.0.lock().unwrap().image_bgra[0], 300, 5, 5), orig(5, 5));
}

#[test]
fn 여러_창_개수는_모니터별로_세고_다른_세션_개수는_무시() {
    let w = world();
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session + 1, 0, 4);
    assert!(!s.capture_annotated(), "다른 세션");
    s.capture_annotations(session, 0, 1);
    s.capture_annotations(session, 1, 2);
    s.capture_annotations(session, 0, 0);
    assert!(s.capture_annotated(), "1번 모니터 창에 아직 2개");
    s.capture_annotations(session, 1, 0);
    assert!(!s.capture_annotated());
    s.capture_input(cmd("copy"));
    assert!(w.overlay.0.lock().unwrap().commit_requests.is_empty());
    assert_eq!(w.desk.0.lock().unwrap().images.len(), 1);
}

#[test]
fn 부탁할_길이_없거나_층_크기가_틀리면_주석_없이_낸다() {
    // 어댑터가 부탁을 못 보냄
    let w = world();
    w.overlay.0.lock().unwrap().no_commit = true;
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session, 0, 1);
    s.capture_input(cmd("copy"));
    assert_eq!(s.mode(), Mode::Idle, "기다리지 않음");
    assert_eq!(w.desk.0.lock().unwrap().images.len(), 1);

    // 층 크기가 범위와 다름 → 잘못 겹치지 않게 주석 없이
    let w = world();
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session, 0, 1);
    s.capture_input(cmd("copy"));
    let req = *w.overlay.0.lock().unwrap().commit_requests.last().unwrap();
    let mut bad = answer(&req);
    bad.overlay.truncate(400);
    s.capture_commit_annotated(bad);
    assert_eq!(s.mode(), Mode::Idle);
    let d = w.desk.0.lock().unwrap();
    assert_eq!(px(&d.image_bgra[0], 300, 5, 5), orig(5, 5));
    assert_eq!(px(&d.image_bgra[0], 300, 100, 0), orig(100, 0), "모자이크도 빼고");
}

#[test]
fn 모자이크만_있는_주석도_합성된다() {
    let w = world();
    let mut s = service(&w);
    let session = open(&mut s);
    s.capture_annotations(session, 0, 1);
    s.capture_input(cmd("copy"));
    let req = *w.overlay.0.lock().unwrap().commit_requests.last().unwrap();
    let mut only = answer(&req);
    only.overlay.clear();
    only.meta.w = 0;
    only.meta.h = 0;
    s.capture_commit_annotated(only);
    let d = w.desk.0.lock().unwrap();
    assert_eq!(px(&d.image_bgra[0], 300, 5, 5), orig(5, 5), "층 없음 = 펜 없음");
    assert_ne!(px(&d.image_bgra[0], 300, 100, 0), orig(100, 0), "모자이크는 적용");
}

fn dual_monitor_world() -> FakeWorld {
    use ink_domain::geometry::Monitor;
    let w = FakeWorld::new();
    let mut scr = w.screen.0.lock().unwrap();
    scr.layout = ink_domain::geometry::MonitorLayout::new(vec![
        Monitor { id: 0, bounds: PhysRect::new(0, 0, 3840, 2160), work: PhysRect::new(0, 0, 3840, 2080), scale: 2.0, primary: true },
        Monitor { id: 1, bounds: PhysRect::new(3840, 0, 1920, 1080), work: PhysRect::new(3840, 0, 1920, 1040), scale: 1.0, primary: false },
    ]);
    scr.windows.clear();
    scr.cursor = PhysPoint { x: 4000, y: 500 };
    scr.frames = Some(vec![
        Frame { monitor_id: 0, width: 3840, height: 2160, bgra: Arc::new(vec![40; 3840 * 2160 * 4]) },
        Frame { monitor_id: 1, width: 1920, height: 1080, bgra: Arc::new(vec![80; 1920 * 1080 * 4]) },
    ]);
    drop(scr);
    w
}

#[test]
fn 두_모니터_주석_도구_동기화_및_선택_범위_잠금() {
    let w = dual_monitor_world();
    let mut s = service(&w);
    s.on_hotkey("region");
    // 경계를 넘는 600×400 범위 (3500, 320, 600, 400) 드래그 확정
    s.capture_input(CaptureInput::Down { x: 3500, y: 320, button: 0 });
    s.capture_input(CaptureInput::Move { x: 4100, y: 720 });
    s.capture_input(CaptureInput::Up { x: 4100, y: 720, button: 0 });
    let last_view = w.notifier.0.lock().unwrap().captures.last().cloned().expect("CaptureView 발행");
    assert_eq!(last_view.selection.phase, ink_domain::selection::Phase::Confirmed);
    let expected_rect = PhysRect::new(3500, 320, 600, 400);
    assert_eq!(last_view.selection.rect, Some(expected_rect));

    // 오른쪽 도구 막대에서 펜 선택
    s.capture_input(CaptureInput::AnnotTool { tool: Some("pen".into()) });
    let after_tool = w.notifier.0.lock().unwrap().captures.last().cloned().unwrap();
    assert_eq!(after_tool.annot_tool.as_deref(), Some("pen"));

    // 왼쪽 모니터에서 드래그·더블클릭·휠·방향키·우클릭을 보내도 확정 범위가 바뀌지 않는다
    s.capture_input(CaptureInput::Down { x: 3590, y: 430, button: 0 });
    s.capture_input(CaptureInput::Move { x: 3700, y: 450 });
    s.capture_input(CaptureInput::Up { x: 3700, y: 450, button: 0 });
    s.capture_input(CaptureInput::Dblclick { x: 3600, y: 440 });
    s.capture_input(CaptureInput::Wheel { dy: -1 });
    s.capture_input(CaptureInput::Key { key: "ArrowLeft".into(), code: "ArrowLeft".into(), ctrl: false, shift: false, alt: false, meta: false });
    s.capture_input(CaptureInput::Down { x: 3600, y: 440, button: 2 });
    let locked_view = w.notifier.0.lock().unwrap().captures.last().cloned().unwrap();
    assert_eq!(locked_view.selection.rect, Some(expected_rect));
    assert!(matches!(s.mode(), Mode::Capture { .. }));

    // Esc → 캡처 취소 대신 도구만 해제
    s.capture_input(CaptureInput::Key { key: "Escape".into(), code: "Escape".into(), ctrl: false, shift: false, alt: false, meta: false });
    assert!(matches!(s.mode(), Mode::Capture { .. }));
    let released_view = w.notifier.0.lock().unwrap().captures.last().cloned().unwrap();
    assert_eq!(released_view.annot_tool, None);

    // 도구 해제 후에는 왼쪽 모니터 드래그가 다시 범위를 바꿈
    s.capture_input(CaptureInput::Down { x: 3590, y: 430, button: 0 });
    s.capture_input(CaptureInput::Move { x: 3700, y: 450 });
    s.capture_input(CaptureInput::Up { x: 3700, y: 450, button: 0 });
    let redragged = w.notifier.0.lock().unwrap().captures.last().cloned().unwrap();
    assert_eq!(redragged.selection.rect, Some(PhysRect::new(3590, 430, 110, 20)));
}

#[test]
fn 두_모니터_각각_그린_주석을_모두_합성한다() {
    let w = dual_monitor_world();
    let mut s = service(&w);
    s.on_hotkey("region");
    let session = match s.mode() {
        Mode::Capture { session, .. } => session,
        _ => panic!("캡처가 열려야 함"),
    };
    s.capture_input(CaptureInput::Down { x: 3500, y: 320, button: 0 });
    s.capture_input(CaptureInput::Move { x: 4100, y: 720 });
    s.capture_input(CaptureInput::Up { x: 4100, y: 720, button: 0 });

    s.capture_annotations(session, 1, 1);
    s.capture_annotations(session, 0, 1);
    s.capture_input(cmd("save"));

    let reqs = w.overlay.0.lock().unwrap().commit_requests.clone();
    assert_eq!(reqs.len(), 1);
    let req = reqs[0];
    assert_eq!((req.rect.w, req.rect.h), (600, 400));

    // 모니터 1 답: (450, 160)에 빨강
    let mut layer1 = vec![0u8; 600 * 400 * 4];
    let idx1 = (160 * 600 + 450) * 4;
    layer1[idx1..idx1 + 4].copy_from_slice(&[255, 0, 0, 255]);
    s.capture_commit_annotated(AnnotatedCommit {
        meta: CommitMeta {
            session,
            seq: req.seq,
            action: CommitAction::Save,
            monitor_id: Some(1),
            w: 600,
            h: 400,
            mosaics: vec![MosaicRect { x: 400, y: 20, w: 20, h: 20, block: 10 }],
        },
        overlay: layer1.clone(),
    });
    assert!(matches!(s.mode(), Mode::Capture { .. }), "모니터 0 답을 기다려야 함");

    // 모니터 1 중복 답은 무시
    s.capture_commit_annotated(AnnotatedCommit {
        meta: CommitMeta { session, seq: req.seq, action: CommitAction::Save, monitor_id: Some(1), w: 600, h: 400, mosaics: vec![] },
        overlay: layer1,
    });
    assert!(matches!(s.mode(), Mode::Capture { .. }));

    // 모니터 0 답: (125, 160)에 파랑
    let mut layer0 = vec![0u8; 600 * 400 * 4];
    let idx0 = (160 * 600 + 125) * 4;
    layer0[idx0..idx0 + 4].copy_from_slice(&[0, 0, 255, 255]);
    s.capture_commit_annotated(AnnotatedCommit {
        meta: CommitMeta {
            session,
            seq: req.seq,
            action: CommitAction::Save,
            monitor_id: Some(0),
            w: 600,
            h: 400,
            mosaics: vec![MosaicRect { x: 20, y: 20, w: 20, h: 20, block: 10 }],
        },
        overlay: layer0,
    });
    assert_eq!(s.mode(), Mode::Idle);
    let saved = &w.desk.0.lock().unwrap().saved_bgra[0];
    assert_eq!(px(saved, 600, 450, 160), [0, 0, 255, 255], "모니터 1 빨강(BGRA)");
    assert_eq!(px(saved, 600, 125, 160), [255, 0, 0, 255], "모니터 0 파랑(BGRA)");
}

#[test]
fn 여러_모니터_실행_취소_다시_실행은_편집_순서대로_창에_전달한다() {
    let w = dual_monitor_world();
    let mut s = service(&w);
    s.on_hotkey("region");
    let session = match s.mode() {
        Mode::Capture { session, .. } => session,
        _ => panic!("캡처가 열려야 함"),
    };
    s.capture_input(CaptureInput::Down { x: 3500, y: 320, button: 0 });
    s.capture_input(CaptureInput::Move { x: 4099, y: 719 });
    s.capture_input(CaptureInput::Up { x: 4099, y: 719, button: 0 });

    s.capture_input(CaptureInput::AnnotHistorySync { monitor_id: 1, undo_depth: 1, redo_depth: 0 });
    s.capture_input(CaptureInput::AnnotHistorySync { monitor_id: 0, undo_depth: 1, redo_depth: 0 });
    let v = w.notifier.0.lock().unwrap().captures.last().cloned().unwrap();
    assert!(v.annot_can_undo);
    assert!(!v.annot_can_redo);

    s.capture_input(CaptureInput::AnnotHistory { dir: "undo".into() });
    s.capture_input(CaptureInput::AnnotHistory { dir: "undo".into() });
    let v2 = w.notifier.0.lock().unwrap().captures.last().cloned().unwrap();
    assert!(!v2.annot_can_undo);
    assert!(v2.annot_can_redo);

    s.capture_input(CaptureInput::AnnotHistory { dir: "redo".into() });
    let v3 = w.notifier.0.lock().unwrap().captures.last().cloned().unwrap();
    assert!(v3.annot_can_undo);

    let cmds = w.notifier.0.lock().unwrap().capture_commands.clone();
    assert_eq!(
        cmds,
        vec![
            (session, 0, "undo".to_string()),
            (session, 1, "undo".to_string()),
            (session, 1, "redo".to_string()),
        ]
    );
}
