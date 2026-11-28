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
    let mut baseline: std::collections::BTreeMap<String, usize> =
        serde_json::from_slice(&std::fs::read(dir.join("baseline.json")).expect("기준선 읽기")).expect("사례별 정답 수");
    assert!(!baseline.is_empty(), "저장소 기준선이 비었어요");
    let mut all = Vec::new();
    for f in files {
        let l = replay::load(&f).unwrap();
        let want = baseline.remove(&l.case.id).expect("기준선에 없는 사례 또는 중복 ID");
        assert!(!l.case.points.is_empty(), "{}: 정답 점이 없어요", l.case.id);
        for p in &l.case.points {
            assert!(p.truth.contains(p.at) && l.case.monitors.iter().any(|m| m.monitor.bounds.contains(p.at)), "{}: 잘못된 정답 점 {}", l.case.id, p.label);
        }
        let scores = replay::score(&l);
        let top1 = scores.iter().filter(|s| s.top1()).count();
        assert!(top1 >= want, "{}: Top-1 {top1} < 기준선 {want}", l.case.id);
        all.extend(scores);
    }
    assert!(baseline.is_empty(), "재생하지 못한 기준선 사례: {:?}", baseline.keys());
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
    s.capture_input(CaptureInput::Key { key: "Enter".into(), code: "Enter".into(), ctrl: false, shift: false, alt: false, meta: false });
    let files = replay::cases_in(&dir);
    assert_eq!(files.len(), 1, "확정 한 번 = 사례 하나");
    let back = replay::load(&files[0]).unwrap();
    assert_eq!(back.case.points[0].truth, BUTTON);
    assert_eq!(back.case.points[0].at, PhysPoint { x: 210, y: 218 });
    assert!(back.case.windows.iter().all(|w| w.pid != std::process::id()), "우리 창은 빼고 뜬다");
    assert!(replay::score(&back)[0].top1());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 새_장면_폴더는_기준선_없이_보고만_낸다() {
    let dir = std::env::temp_dir().join(format!("screen-ink-holdout-eval-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    assert!(replay::evaluate_dir(&dir, false, &[]).is_err(), "빈 폴더는 오류");

    let l = synthetic();
    replay::save(&dir, &l.case, &l.frames).unwrap();
    std::fs::write(dir.join("README.md"), "# holdout\n").unwrap();

    let report = replay::evaluate_dir(&dir, false, &[]).unwrap();
    assert_eq!(report.cases.len(), 1);
    assert_eq!(report.cases[0].id, "synthetic-panel");
    assert_eq!(report.summary.points, 2);
    assert!(report.summary.top3 >= 1.0);
    assert!(report.summary.p95_ms >= 0.0);

    let filtered = replay::evaluate_dir(&dir, true, &["synthetic".into()]).unwrap();
    assert_eq!(filtered.summary.points, 2);

    assert!(replay::evaluate_dir(&dir, false, &["missing-case".into()]).is_err());

    // 사례 ID 중복 차단
    let dup = dir.join("dup.json");
    let raw = std::fs::read_to_string(dir.join("synthetic-panel.json")).unwrap();
    std::fs::write(&dup, raw).unwrap();
    assert!(replay::evaluate_dir(&dir, false, &[]).is_err());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn recog_replay_바이너리_holdout_실행() {
    let base = std::env::temp_dir().join(format!("screen-ink-holdout-bin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let empty_dir = base.join("empty");
    let ho_dir = base.join("holdout");
    let csv_path = base.join("out.csv");
    std::fs::create_dir_all(&empty_dir).unwrap();

    let bin = env!("CARGO_BIN_EXE_recog-replay");

    // 1) 빈 holdout 폴더는 안내와 함께 종료 코드 1
    let empty_out = std::process::Command::new(bin).args(["--holdout"]).arg(&empty_dir).output().unwrap();
    assert!(!empty_out.status.success());
    let stderr = String::from_utf8_lossy(&empty_out.stderr);
    assert!(stderr.contains("새 장면(hold-out)"), "stderr: {stderr}");

    // 2) 맞히는 점 1개(BUTTON) + 빗나가는 점 1개(흰 바탕 위 존재하지 않는 사각형) → Top-1 50.0%
    let mut l = synthetic();
    l.case.id = "ho-synth".into();
    l.case.monitors[0].frame = Case::frame_name("ho-synth", 0);
    l.case.points = vec![
        CasePoint { at: PhysPoint { x: 210, y: 218 }, truth: BUTTON, label: "맞는 단추".into() },
        CasePoint { at: PhysPoint { x: 640, y: 470 }, truth: PhysRect::new(615, 455, 50, 30), label: "없는 상자".into() },
    ];
    replay::save(&ho_dir, &l.case, &l.frames).unwrap();

    let ok_out = std::process::Command::new(bin).arg("--holdout").arg(&ho_dir).arg(&csv_path).args(["--min-top1", "50"]).output().unwrap();
    assert!(ok_out.status.success(), "stderr: {}", String::from_utf8_lossy(&ok_out.stderr));
    let stdout = String::from_utf8_lossy(&ok_out.stdout);
    assert!(stdout.contains("전체 (새 장면): 점 2  Top-1 50.0%"), "stdout: {stdout}");
    assert!(stdout.contains("판정: Top-1 50.0% ≥ 50.0% — 통과"), "stdout: {stdout}");
    assert!(csv_path.exists());

    // 3) --no-csv + --min-top1 80 미달 검증
    let fail_out = std::process::Command::new(bin).arg("--holdout").arg(&ho_dir).args(["--no-csv", "--min-top1", "80"]).output().unwrap();
    assert!(!fail_out.status.success());
    let fail_stdout = String::from_utf8_lossy(&fail_out.stdout);
    assert!(fail_stdout.contains("판정: Top-1 50.0% < 80.0% — 미달"), "stdout: {fail_stdout}");

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn 저장소_holdout_폴더가_기준선과_분리되어_있다() {
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", ".."].iter().collect();
    let cases = root.join(replay::CASES_REL_DIR);
    let holdout = root.join(replay::HOLDOUT_REL_DIR);
    assert_ne!(cases, holdout);
    assert!(holdout.join("README.md").is_file(), "holdout README.md가 있어야 해요");
}
