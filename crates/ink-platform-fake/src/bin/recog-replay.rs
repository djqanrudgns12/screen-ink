//! 인식 재생 보고서(08 §5.3): 사례 폴더를 모두 재생해 점별 CSV와 묶음 요약을 낸다.
//!
//!   cargo run -p ink-platform-fake --bin recog-replay -- [사례 폴더] [CSV 경로]
//!   기본: bench/recog/cases → bench/results/recog-replay.csv

use ink_platform_fake::replay;
use std::path::PathBuf;

fn main() {
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", ".."].iter().collect();
    let mut args = std::env::args().skip(1);
    let dir = args.next().map(PathBuf::from).unwrap_or_else(|| root.join("bench/recog/cases"));
    let out = args.next().map(PathBuf::from).unwrap_or_else(|| root.join("bench/results/recog-replay.csv"));
    let files = replay::cases_in(&dir);
    if files.is_empty() {
        eprintln!("사례가 없어요: {}", dir.display());
        std::process::exit(1);
    }
    let mut all = Vec::new();
    for f in &files {
        match replay::load(f) {
            Ok(l) => {
                let s = replay::score(&l);
                let sum = replay::summarize(&s);
                println!("{:<40} 점 {:>3}  Top-1 {:>5.1}%  Top-3 {:>5.1}%  경계 {:.2}px", l.case.id, sum.points, sum.top1 * 100.0, sum.top3 * 100.0, sum.mean_edge);
                all.extend(s);
            }
            Err(e) => eprintln!("건너뜀 {e}"),
        }
    }
    let sum = replay::summarize(&all);
    println!("전체: 점 {}  Top-1 {:.1}%  Top-3 {:.1}%  경계 {:.2}px  p95 {:.1} ms", sum.points, sum.top1 * 100.0, sum.top3 * 100.0, sum.mean_edge, sum.p95_ms);
    if let Some(p) = out.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    match std::fs::write(&out, replay::to_csv(&all)) {
        Ok(()) => println!("CSV: {}", out.display()),
        Err(e) => eprintln!("CSV를 쓰지 못했어요: {e}"),
    }
}
