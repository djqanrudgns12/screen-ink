//! 자동 정답 넣기: 시험 문서 화면으로 뜬 사례에 그 문서의 정답 점을 채운다(사례의 `points`를 바꿔 씀).
//!
//!   cargo run -p ink-platform-fake --bin recog-truth -- <사례.json> <시험 문서.html> [메모]
//!   예: ... -- bench/recog/cases/case-20261126-101500-3.json bench/recog/pages/rb01.html "RB-01 Chrome 100%"

use ink_platform_fake::{replay, truth};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("쓰는 법: recog-truth <사례.json> <시험 문서.html> [메모]");
        std::process::exit(2);
    }
    let (case_path, page_path) = (PathBuf::from(&args[0]), PathBuf::from(&args[1]));
    let run = || -> Result<String, String> {
        let mut loaded = replay::load(&case_path)?;
        let html = std::fs::read_to_string(&page_path).map_err(|e| format!("{}: {e}", page_path.display()))?;
        let layout = truth::extract_layout(&html)?;
        let (monitor, scale, points) = truth::points(&loaded.case, &loaded.frames, &layout)?;
        let n = points.len();
        loaded.case.points = points;
        if let Some(note) = args.get(2) {
            loaded.case.note = note.clone();
        }
        let json = serde_json::to_string_pretty(&loaded.case).map_err(|e| e.to_string())?;
        std::fs::write(&case_path, json).map_err(|e| e.to_string())?;
        Ok(format!("{}: 모니터 {monitor}, 배율 {scale:.3}, 정답 점 {n}개", case_path.display()))
    };
    match run() {
        Ok(msg) => println!("{msg}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
