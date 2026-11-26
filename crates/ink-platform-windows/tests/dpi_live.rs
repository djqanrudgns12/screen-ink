//! D12 실측(3b-dpi·3b-msaa, 스파이크 보고서 §7 [확인 필요]) — 배율 125%·150% 모니터가 있는 PC에서 메인이 돌린다.
//!
//! 확인할 것 세 가지를 시험 하나씩으로:
//! 1. `GetWindowRect`·DWM 경계(PMv2 호출자) = 물리 px — RB-04 창 경계가 rb04-truth.json의 창 정답과 같다
//! 2. UIA `BoundingRectangle` = 물리 px — DPI를 모르는 RB-04 컨트롤이 UIA 색인에서 정답(물리)과 ±2 px
//! 3. 자체 IAccessible 서버의 `accLocation` = 논리 px → `msaa::Msaa`가 물리로 바꾼다 — RB-20 칸이 16 × 배율 px
//!
//! 준비(저장소 뿌리에서, 시험 창을 배율 125%·150% 모니터에 둔다):
//!   rb04-win32 --dpi unaware --out rb04-truth.json     (창을 옮겼으면 다시 쓰일 때까지 1초 기다림)
//!   rb20-frozen --mode slow-msaa --slow-ms 0 --life 300
//!   cargo test -p ink-platform-windows --test dpi_live -- --ignored --test-threads=1 --nocapture
//!
//! 창 제목은 읽지 않는다(실행 파일 이름으로 창을 찾음).

#![cfg(windows)]

use ink_domain::geometry::PhysRect;
use ink_domain::recog::Control;
use ink_platform_windows::dpi::{client_rect_physical, WindowDpi};
use ink_platform_windows::screen::top_windows;
use std::path::PathBuf;
use std::sync::Once;
use std::time::Duration;

fn setup() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
        // SAFETY: 프로세스 DPI 문맥을 한 번(앱과 같은 Per-Monitor V2).
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    });
}

fn find(exe: &str) -> ink_domain::snapshot::RawWindow {
    top_windows()
        .into_iter()
        .find(|w| w.exe == exe && w.visible && !w.minimized && !w.rect.is_empty())
        .unwrap_or_else(|| panic!("{exe} 창이 없어요 — 먼저 띄워요(파일 머리 주석)"))
}

fn edge(a: PhysRect, b: PhysRect) -> i32 {
    [(a.x - b.x).abs(), (a.y - b.y).abs(), (a.right() - b.right()).abs(), (a.bottom() - b.bottom()).abs()].into_iter().max().unwrap_or(0)
}

fn rect_of(v: &serde_json::Value) -> PhysRect {
    let n = |k: &str| v[k].as_i64().unwrap_or(0) as i32;
    PhysRect::new(n("x"), n("y"), n("w"), n("h"))
}

/// rb04-truth.json: 환경 변수 RB04_TRUTH 또는 저장소 뿌리.
fn rb04_truth() -> Vec<(String, PhysRect)> {
    let path = std::env::var_os("RB04_TRUTH").map(PathBuf::from).unwrap_or_else(|| [env!("CARGO_MANIFEST_DIR"), "..", "..", "rb04-truth.json"].iter().collect());
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: serde_json::Value = serde_json::from_str(&text).expect("rb04-truth.json");
    eprintln!("rb04 정답: dpi {} · 창 배율 {}", doc["dpi"], doc["windowScale"]);
    doc["points"].as_array().expect("points").iter().map(|p| (p["label"].as_str().unwrap_or("").to_string(), rect_of(&p["truth"]))).collect()
}

#[test]
#[ignore = "실제 화면 필요 — 메인이 실행(배율 125%·150% PC, rb04-win32 --dpi unaware)"]
fn dpi_창_경계는_물리_px() {
    setup();
    let w = find("rb04-win32.exe");
    let truth = rb04_truth();
    let frame = truth.iter().find(|(l, _)| l == "제목 표시줄").map(|t| t.1).expect("창 정답");
    let dpi = WindowDpi::of(w.handle);
    eprintln!("RB-04 창: 인식 {:?} · 창 DPI {} · 모니터 DPI {} · 비 {:.3}", dpi.awareness, dpi.window_dpi, dpi.monitor_dpi, dpi.ratio());
    assert!(edge(w.rect, frame) <= 1, "top_windows(DWM 경계) {:?} vs 정답 {:?}", w.rect, frame);
}

#[test]
#[ignore = "실제 화면 필요 — 메인이 실행(배율 125%·150% PC, rb04-win32 --dpi unaware)"]
fn dpi_uia_사각형은_물리_px() {
    use ink_app::ElementSource;
    setup();
    let w = find("rb04-win32.exe");
    let truth = rb04_truth();
    let (tx, rx) = std::sync::mpsc::channel();
    let idx = ink_platform_windows::uia::UiaIndex::spawn(move |win, _exe, ms| {
        let _ = tx.send((win, ms));
    });
    idx.request(w.handle, &w.exe);
    let (_, ms) = rx.recv_timeout(Duration::from_secs(10)).expect("UIA 색인이 10초 안에 안 끝남");
    let els = ElementSource::elements(&idx, w.handle);
    eprintln!("UIA 색인 {} 개, {ms:.0} ms", els.len());
    // 컨트롤 단위 정답(창·항목 점은 빼고)
    let controls: Vec<&(String, PhysRect)> = truth.iter().filter(|(l, _)| !matches!(l.as_str(), "제목 표시줄" | "창 빈 곳" | "목록 첫 행" | "트리 뿌리 항목" | "첫 탭")).collect();
    let mut hit = 0;
    for (label, t) in &controls {
        let best = els.iter().map(|e| edge(e.rect, *t)).min().unwrap_or(i32::MAX);
        if best <= 2 {
            hit += 1;
        } else {
            eprintln!("  어긋남 {label}: 정답 {t:?}, 가장 가까운 UIA 변 오차 {best} px");
        }
    }
    eprintln!("UIA가 정답(물리) ±2 px로 맞힌 컨트롤 {hit}/{}", controls.len());
    assert!(hit * 10 >= controls.len() * 8, "UIA 사각형이 물리 px가 아닐 수 있음: {hit}/{}", controls.len());
}

#[test]
#[ignore = "실제 화면 필요 — 메인이 실행(배율 125%·150% PC, rb20-frozen --mode slow-msaa --slow-ms 0)"]
fn dpi_자체_msaa_서버의_논리_좌표를_물리로() {
    setup();
    let w = find("rb20-frozen.exe");
    let dpi = WindowDpi::of(w.handle);
    let client = client_rect_physical(w.handle).expect("클라이언트 영역");
    let els = ink_platform_windows::msaa::Msaa::new().index(w.handle, &w.exe);
    let cells: Vec<PhysRect> = els.iter().filter(|e| e.control == Control::Button).map(|e| e.rect).collect();
    let want = (16.0 * dpi.ratio()).round() as i32;
    eprintln!("RB-20: 인식 {:?} · 창 DPI {} · 모니터 DPI {} · 비 {:.3} · MSAA 칸 {}개 · 기대 칸 크기 {want} px", dpi.awareness, dpi.window_dpi, dpi.monitor_dpi, dpi.ratio(), cells.len());
    if !dpi.virtualized() {
        eprintln!("  배율 100%(또는 PM 창) — 보정 경로는 이 PC에서 검증되지 않음");
    }
    assert!(!cells.is_empty(), "MSAA 칸이 없음(모드 확인: --mode slow-msaa --slow-ms 0)");
    let grown = PhysRect::new(client.x - 2, client.y - 2, client.w + 4, client.h + 4);
    for c in &cells {
        assert!(c.intersect(&grown) == Some(*c), "칸 {c:?}이 클라이언트 {client:?} 밖 — 논리 좌표를 그대로 쓴 듯");
        assert!((c.w - want).abs() <= 1 && (c.h - want).abs() <= 1, "칸 크기 {}×{} ≠ {want} — 보정 안 됨", c.w, c.h);
    }
}
