//! 인식 재생 보고서(08 §5.3): 사례 폴더를 모두 재생해 점별 CSV와 묶음 요약을 낸다.
//!
//!   cargo run -p ink-platform-fake --bin recog-replay -- [사례 폴더] [CSV 경로] [--vision-only] [--filter RB-01,RB-03] [--min-top1 80]
//!   기본: bench/recog/cases → bench/results/recog-replay.csv (--vision-only면 recog-replay-vision.csv)
//!
//! - `--vision-only`: A(UIA 색인)·H(자식 창) 근거를 지우고 V만으로(창 목록 W는 스냅숏이라 남김) — 관문 G3(C §2.4 `3c-g3`).
//! - `--filter`: 사례 메모(`note`)나 id에 그 글자가 들어 있는 사례만(쉼표로 여럿, 대소문자 무시).
//! - `--min-top1 N`: 전체 Top-1이 N% 미만이면 종료 코드 1(관문 판정용).

use ink_platform_fake::replay;
use std::path::PathBuf;

struct Opts {
    dir: PathBuf,
    out: PathBuf,
    vision_only: bool,
    filter: Vec<String>,
    min_top1: Option<f64>,
}

fn opts() -> Result<Opts, String> {
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", ".."].iter().collect();
    let mut positional = Vec::new();
    let (mut vision_only, mut filter, mut min_top1) = (false, Vec::new(), None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--vision-only" => vision_only = true,
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
    let dir = positional.next().unwrap_or_else(|| root.join("bench/recog/cases"));
    let name = if vision_only { "bench/results/recog-replay-vision.csv" } else { "bench/results/recog-replay.csv" };
    let out = positional.next().unwrap_or_else(|| root.join(name));
    Ok(Opts { dir, out, vision_only, filter, min_top1 })
}

fn main() {
    let o = match opts() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let files = replay::cases_in(&o.dir);
    if files.is_empty() {
        eprintln!("사례가 없어요: {}", o.dir.display());
        std::process::exit(1);
    }
    let mut all = Vec::new();
    let mut matched = vec![false; o.filter.len()];
    for f in &files {
        match replay::load(f) {
            Ok(mut l) => {
                let hay = format!("{} {}", l.case.id, l.case.note).to_lowercase();
                let mut selected = o.filter.is_empty();
                for (term, seen) in o.filter.iter().zip(&mut matched) {
                    if hay.contains(term.as_str()) {
                        *seen |= !l.case.points.is_empty();
                        selected = true;
                    }
                }
                if !selected {
                    continue;
                }
                if o.vision_only {
                    l.case.elements.clear();
                    l.case.children.clear();
                }
                let s = replay::score(&l);
                let sum = replay::summarize(&s);
                println!("{:<40} 점 {:>3}  Top-1 {:>5.1}%  Top-3 {:>5.1}%  경계 {:.2}px", l.case.id, sum.points, sum.top1 * 100.0, sum.top3 * 100.0, sum.mean_edge);
                all.extend(s);
            }
            Err(e) => {
                eprintln!("사례를 읽지 못했어요: {e}");
                std::process::exit(1);
            }
        }
    }
    for (term, seen) in o.filter.iter().zip(matched) {
        if !seen {
            eprintln!("필터에 맞는 정답 점이 없어요: {term}");
            std::process::exit(1);
        }
    }
    if all.is_empty() {
        eprintln!("고른 사례에 점이 없어요(--filter 확인)");
        std::process::exit(1);
    }
    let sum = replay::summarize(&all);
    let mode = if o.vision_only { " (V만)" } else { "" };
    println!("전체{mode}: 점 {}  Top-1 {:.1}%  Top-3 {:.1}%  경계 {:.2}px  p95 {:.1} ms", sum.points, sum.top1 * 100.0, sum.top3 * 100.0, sum.mean_edge, sum.p95_ms);
    if let Some(p) = o.out.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    match std::fs::write(&o.out, replay::to_csv(&all)) {
        Ok(()) => println!("CSV: {}", o.out.display()),
        Err(e) => {
            eprintln!("CSV를 쓰지 못했어요: {e}");
            std::process::exit(1);
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
