//! 스냅숏 재생 시험(08 §5.2): 합성 사례로 틀을 확인하고, `bench/recog/cases`의 실제 사례가 기준선 아래로
//! 떨어지지 않는지 본다(사례는 시험 문서 화면에서만 뜬다).

use ink_app::case::{Case, CaseMonitor, CasePoint, CASE_SCHEMA};
use ink_app::{CaptureInput, Frame, InkService};
use ink_domain::geometry::{Monitor, PhysPoint, PhysRect};
use ink_domain::shortcuts::Platform;
use ink_domain::snapshot::RawWindow;
use ink_platform_fake::replay::{self, Loaded};
use std::path::PathBuf;
use std::sync::Arc;

const PANEL: PhysRect = PhysRect::new(100, 120, 400, 300);
const BUTTON: PhysRect = PhysRect::new(150, 200, 120, 36);

/// 800×600: 바탕(어두운 파랑) 위 흰 창, 그 안 회색 판, 그 안 짙은 단추. 접근성 정보 없음(V만).
fn synthetic() -> Loaded {
    let (w, h) = (800u32, 600u32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let mut fill = |r: PhysRect, c: u8| {
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                let o = ((y as u32 * w + x as u32) * 4) as usize;
                px[o..o + 4].copy_from_slice(&[c, c, c, 255]);
            }
        }
    };
    fill(PhysRect::new(0, 0, 800, 600), 60);
    fill(PhysRect::new(50, 50, 700, 500), 250);
    fill(PANEL, 215);
    fill(BUTTON, 140);
    let mon = Monitor { id: 0, bounds: PhysRect::new(0, 0, 800, 600), work: PhysRect::new(0, 0, 800, 560), scale: 1.0, primary: true };
    let case = Case {
        schema: CASE_SCHEMA,
        id: "synthetic-panel".into(),
        note: "합성: 창 > 판 > 단추".into(),
        monitors: vec![CaseMonitor { monitor: mon, frame: Case::frame_name("synthetic-panel", 0) }],
        cursor: PhysPoint { x: 10, y: 10 },
        windows: vec![RawWindow { handle: 1, pid: 99, exe: "testapp.exe".into(), class: "TestWindow".into(), rect: PhysRect::new(50, 50, 700, 500), visible: true, ..Default::default() }],
        elements: Default::default(),
        children: Default::default(),
        points: vec![
            CasePoint { at: PhysPoint { x: 210, y: 218 }, truth: BUTTON, label: "단추".into() },
            CasePoint { at: PhysPoint { x: 420, y: 380 }, truth: PANEL, label: "판".into() },
        ],
    };
    Loaded { case, frames: vec![Frame { monitor_id: 0, width: w, height: h, bgra: Arc::new(px) }] }
}

#[test]
fn 합성_사례는_v만으로_단추와_판을_찾는다() {
    let l = synthetic();
    let scores = replay::score(&l);
    let button = &scores[0];
    assert!(button.top1(), "단추는 첫 강조: {button:?}");
    assert_eq!(button.edge_err, Some(0));
    let panel = &scores[1];
    assert!(panel.top3(), "판은 휠 2칸 안: {panel:?}");
    let sum = replay::summarize(&scores);
    assert_eq!(sum.points, 2);
    assert!(sum.top3 >= 1.0);
    let csv = replay::to_csv(&scores);
    assert!(csv.lines().count() == 3 && csv.contains("\"단추\""), "{csv}");
}

#[test]
fn 같은_입력이면_같은_결과() {
    let l = synthetic();
    let a = replay::score(&l);
    let b = replay::score(&l);
    let pick = |v: &[replay::PointScore]| v.iter().map(|s| (s.first, s.steps, s.edge_err)).collect::<Vec<_>>();
    assert_eq!(pick(&a), pick(&b));
}

#[test]
fn 사례는_쓰고_읽어도_같다() {
    let l = synthetic();
    let dir = std::env::temp_dir().join(format!("screen-ink-case-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    replay::save(&dir, &l.case, &l.frames).unwrap();
    let back = replay::load(&dir.join("synthetic-panel.json")).unwrap();
    assert_eq!(back.case, l.case);
    assert_eq!(back.frames[0].bgra, l.frames[0].bgra, "무손실");
    assert_eq!(replay::cases_in(&dir), vec![dir.join("synthetic-panel.json")]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `bench/recog/cases/*.json` 전부 재생 — `baseline.json`(사례 id → Top-1 맞힌 점 수)보다 떨어지면 실패.
#[test]
fn 저장소_사례가_기준선_아래로_떨어지지_않는다() {
    let dir: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "bench", "recog", "cases"].iter().collect();
    let files = replay::cases_in(&dir);
    if files.is_empty() {
        eprintln!("사례 없음: {}", dir.display());
        return;
    }
    let baseline: serde_json::Value = std::fs::read_to_string(dir.join("baseline.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let mut all = Vec::new();
    for f in files {
        let l = replay::load(&f).unwrap();
        let scores = replay::score(&l);
        let top1 = scores.iter().filter(|s| s.top1()).count() as u64;
        if let Some(want) = baseline.get(&l.case.id).and_then(|v| v.as_u64()) {
            assert!(top1 >= want, "{}: Top-1 {top1} < 기준선 {want}", l.case.id);
        }
        all.extend(scores);
    }
    eprintln!("{:?}", replay::summarize(&all));
}

/// 사람 정답 도구: 앱에서 확정하면 그 범위를 정답으로 사례가 써지고, 다시 돌리면 같은 것을 고른다.
#[test]
fn 확정하면_사례가_써지고_재생하면_맞는다() {
    let l = synthetic();
    let w = replay::world(&l);
    let dir = std::env::temp_dir().join(format!("screen-ink-dump-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut s = InkService::new(w.ports(), Platform::Win, "test");
    s.start();
    s.set_case_dir(Some(dir.to_string_lossy().into_owned()));
    s.on_hotkey("region");
    s.capture_input(CaptureInput::Move { x: 220, y: 218 });
    s.capture_input(CaptureInput::Move { x: 210, y: 218 });
    s.capture_input(CaptureInput::Key { key: "Enter".into(), code: "Enter".into(), ctrl: false, shift: false, alt: false });
    let files = replay::cases_in(&dir);
    assert_eq!(files.len(), 1, "확정 한 번 = 사례 하나");
    let back = replay::load(&files[0]).unwrap();
    assert_eq!(back.case.points[0].truth, BUTTON);
    assert_eq!(back.case.points[0].at, PhysPoint { x: 210, y: 218 });
    assert!(back.case.windows.iter().all(|w| w.pid != std::process::id()), "우리 창은 빼고 뜬다");
    assert!(replay::score(&back)[0].top1());
    let _ = std::fs::remove_dir_all(&dir);
}
