//! 인식 재생 보고서(08 §5.3): 사례 폴더를 모두 재생해 점별 CSV와 묶음 요약을 낸다.
//!
//!   cargo run -p ink-platform-fake --bin recog-replay -- [사례 폴더] [CSV 경로] [--holdout] [--vision-only] [--filter RB-01,RB-03] [--min-top1 80] [--no-csv]
//!   기본: bench/recog/cases → bench/results/recog-replay.csv (--vision-only면 recog-replay-vision.csv)
//!   새 장면(hold-out): --holdout → bench/recog/holdout → bench/results/recog-holdout.csv (--vision-only면 recog-holdout-vision.csv)
//!
//! - `--holdout`: 287점 기준선(`cases/baseline.json`)과 분리된 새 장면 폴더(`bench/recog/holdout/`)를
//!   기준선 검사 없이 재생해 Top-1·Top-3·경계 오차만 보고한다. 새 장면은 규칙을 고칠 때 보지 않는 시험 전용이다.
//! - `--vision-only`: A(UIA 색인)·H(자식 창) 근거를 지우고 V만으로(창 목록 W는 스냅숏이라 남김) — 관문 G3(C §2.4 `3c-g3`).
//! - `--filter`: 사례 메모(`note`)나 id에 그 글자가 들어 있는 사례만(쉼표로 여럿, 대소문자 무시).
//! - `--min-top1 N`: 전체 Top-1이 N% 미만이면 종료 코드 1(관문 판정용).
//! - `--no-csv`: CSV 파일을 쓰지 않고 콘솔 요약만 출력한다.

use ink_platform_fake::replay;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
struct Opts {
    dir: PathBuf,
    out: PathBuf,
    holdout: bool,
    vision_only: bool,
    no_csv: bool,
    filter: Vec<String>,
    min_top1: Option<f64>,
}

fn parse_opts<I: IntoIterator<Item = String>>(args: I, root: &Path) -> Result<Opts, String> {
    let mut positional = Vec::new();
    let (mut holdout, mut vision_only, mut no_csv) = (false, false, false);
    let (mut filter, mut min_top1) = (Vec::new(), None);
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--holdout" => holdout = true,
            "--vision-only" => vision_only = true,
            "--no-csv" => no_csv = true,
            "--filter" => {
                let v = args.next().ok_or("--filter 뒤에 RB-01,RB-03처럼 적어요")?;
                filter = v.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect();
            }
            "--min-top1" => {
                let v = args.next().ok_or("--min-top1 뒤에 % 숫자를 적어요")?;
                min_top1 = Some(v.parse::<f64>().map_err(|_| format!("--min-top1 숫자가 아니에요: {v}"))?);
            }
            s if s.starts_with("--") => return Err(format!("모르는 선택: {s}")),
            _ => positional.push(PathBuf::from(a)),
        }
    }
    let mut positional = positional.into_iter();
    let default_dir = if holdout { replay::HOLDOUT_REL_DIR } else { replay::CASES_REL_DIR };
    let dir = positional.next().unwrap_or_else(|| root.join(default_dir));
    let out_name = match (holdout, vision_only) {
        (true, true) => "bench/results/recog-holdout-vision.csv",
        (true, false) => "bench/results/recog-holdout.csv",
        (false, true) => "bench/results/recog-replay-vision.csv",
        (false, false) => "bench/results/recog-replay.csv",
    };
    let out = positional.next().unwrap_or_else(|| root.join(out_name));
    Ok(Opts { dir, out, holdout, vision_only, no_csv, filter, min_top1 })
}

fn main() {
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", ".."].iter().collect();
    let o = match parse_opts(std::env::args().skip(1), &root) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if o.holdout && replay::cases_in(&o.dir).is_empty() {
        eprintln!("새 장면(hold-out) 사례가 없어요: {} (bench/recog/holdout/에 사례를 둔 뒤 실행하세요)", o.dir.display());
        std::process::exit(1);
    }
    let report = match replay::evaluate_dir(&o.dir, o.vision_only, &o.filter) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    for c in &report.cases {
        println!("{:<40} 점 {:>3}  Top-1 {:>5.1}%  Top-3 {:>5.1}%  경계 {:.2}px", c.id, c.summary.points, c.summary.top1 * 100.0, c.summary.top3 * 100.0, c.summary.mean_edge);
    }
    let sum = &report.summary;
    let mode = match (o.holdout, o.vision_only) {
        (false, false) => "",
        (false, true) => " (V만)",
        (true, false) => " (새 장면)",
        (true, true) => " (새 장면, V만)",
    };
    println!("전체{mode}: 점 {}  Top-1 {:.1}%  Top-3 {:.1}%  경계 {:.2}px  p95 {:.1} ms", sum.points, sum.top1 * 100.0, sum.top3 * 100.0, sum.mean_edge, sum.p95_ms);
    if !o.no_csv {
        if let Some(p) = o.out.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        match std::fs::write(&o.out, replay::to_csv(&report.all)) {
            Ok(()) => println!("CSV: {}", o.out.display()),
            Err(e) => {
                eprintln!("CSV를 쓰지 못했어요: {e}");
                std::process::exit(1);
            }
        }
    }
    if let Some(min) = o.min_top1 {
        let got = sum.top1 * 100.0;
        if got + 1e-9 < min {
            println!("판정: Top-1 {got:.1}% < {min:.1}% — 미달");
            std::process::exit(1);
        }
        println!("판정: Top-1 {got:.1}% ≥ {min:.1}% — 통과");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| (*x).to_string()).collect()
    }

    #[test]
    fn 기본_인자를_해석한다() {
        let root = Path::new("/repo");
        let o = parse_opts(s(&[]), root).unwrap();
        assert_eq!(o.dir, root.join("bench/recog/cases"));
        assert_eq!(o.out, root.join("bench/results/recog-replay.csv"));
        assert!(!o.holdout);
        assert!(!o.vision_only);
        assert!(!o.no_csv);
        assert!(o.filter.is_empty());
        assert_eq!(o.min_top1, None);
    }

    #[test]
    fn holdout_단독과_조합_인자를_해석한다() {
        let root = Path::new("/repo");
        let o = parse_opts(s(&["--holdout"]), root).unwrap();
        assert_eq!(o.dir, root.join("bench/recog/holdout"));
        assert_eq!(o.out, root.join("bench/results/recog-holdout.csv"));
        assert!(o.holdout);

        let o2 = parse_opts(s(&["--holdout", "--vision-only", "--filter", "RB-12, rb-14", "--min-top1", "80", "--no-csv"]), root).unwrap();
        assert_eq!(o2.dir, root.join("bench/recog/holdout"));
        assert_eq!(o2.out, root.join("bench/results/recog-holdout-vision.csv"));
        assert!(o2.holdout && o2.vision_only && o2.no_csv);
        assert_eq!(o2.filter, s(&["rb-12", "rb-14"]));
        assert_eq!(o2.min_top1, Some(80.0));
    }

    #[test]
    fn holdout에서도_위치_인자로_경로를_바꿀_수_있다() {
        let root = Path::new("/repo");
        let o = parse_opts(s(&["--holdout", "/tmp/custom", "/tmp/out.csv"]), root).unwrap();
        assert!(o.holdout);
        assert_eq!(o.dir, PathBuf::from("/tmp/custom"));
        assert_eq!(o.out, PathBuf::from("/tmp/out.csv"));
    }

    #[test]
    fn 잘못된_선택은_오류를_돌려준다() {
        let root = Path::new("/repo");
        assert!(parse_opts(s(&["--unknown"]), root).is_err());
        assert!(parse_opts(s(&["--min-top1", "abc"]), root).is_err());
        assert!(parse_opts(s(&["--filter"]), root).is_err());
    }
}
