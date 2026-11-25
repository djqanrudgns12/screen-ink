//! 스냅숏 재생 시험(08 §5.2): 떠 둔 사례(`ink_app::case::Case` + 모니터 PNG)를 OS 없이
//! 진짜 서비스·고르기 상태 기계·융합·V 엔진으로 다시 돌려 정답과 견준다(08 §5.3 지표).
//!
//! 점마다 새 진입(깊이 기억이 섞이지 않게): 점 옆에서 점으로 커서를 옮겨 첫 강조를 읽고,
//! 휠을 위·아래로 3칸씩 돌려 정답이 몇 칸 만에 나오는지 센다.

use crate::FakeWorld;
use ink_app::case::Case;
use ink_app::{CaptureInput, Frame, InkService};
use ink_domain::geometry::{MonitorLayout, PhysRect};
use ink_domain::shortcuts::Platform;
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// 정답으로 칠 경계 오차(각 변, 물리 px).
pub const HIT_TOLERANCE: i32 = 2;
/// 휠로 찾아볼 칸 수(08 §5.3 "0~3").
pub const MAX_STEPS: u32 = 3;

/// 읽어 들인 사례.
pub struct Loaded {
    pub case: Case,
    pub frames: Vec<Frame>,
}

/// `<id>.json`과 같은 폴더의 PNG를 읽는다.
pub fn load(json: &Path) -> Result<Loaded, String> {
    let text = std::fs::read_to_string(json).map_err(|e| format!("{}: {e}", json.display()))?;
    let case: Case = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", json.display()))?;
    let dir = json.parent().unwrap_or(Path::new("."));
    let mut frames = Vec::new();
    for m in &case.monitors {
        let (w, h, bgra) = read_png(&dir.join(&m.frame))?;
        frames.push(Frame { monitor_id: m.monitor.id, width: w, height: h, bgra: Arc::new(bgra) });
    }
    Ok(Loaded { case, frames })
}

/// 사례를 쓴다(합성 사례·시험용). 앱은 플러그인 어댑터가 같은 모양으로 쓴다.
pub fn save(dir: &Path, case: &Case, frames: &[Frame]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for m in &case.monitors {
        let f = frames.iter().find(|f| f.monitor_id == m.monitor.id).ok_or("frame missing")?;
        write_png(&dir.join(&m.frame), f.width, f.height, &f.bgra)?;
    }
    let json = serde_json::to_string_pretty(case).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.json", case.id)), json).map_err(|e| e.to_string())
}

fn read_png(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(file));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; r.output_buffer_size().ok_or("png too large")?];
    let info = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let px = &buf[..info.buffer_size()];
    let n = (info.width * info.height) as usize;
    let mut bgra = Vec::with_capacity(n * 4);
    match info.color_type {
        png::ColorType::Rgb => px.chunks_exact(3).for_each(|c| bgra.extend_from_slice(&[c[2], c[1], c[0], 255])),
        png::ColorType::Rgba => px.chunks_exact(4).for_each(|c| bgra.extend_from_slice(&[c[2], c[1], c[0], 255])),
        png::ColorType::Grayscale => px.iter().for_each(|&g| bgra.extend_from_slice(&[g, g, g, 255])),
        png::ColorType::GrayscaleAlpha => px.chunks_exact(2).for_each(|c| bgra.extend_from_slice(&[c[0], c[0], c[0], 255])),
        png::ColorType::Indexed => return Err("indexed png not expanded".into()),
    }
    Ok((info.width, info.height, bgra))
}

fn write_png(path: &Path, w: u32, h: u32, bgra: &[u8]) -> Result<(), String> {
    let rgb: Vec<u8> = bgra.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0]]).collect();
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(&rgb).map_err(|e| e.to_string())
}

/// 사례 그대로의 가짜 PC(진짜 V 엔진).
pub fn world(l: &Loaded) -> FakeWorld {
    let w = FakeWorld { vision: true, ..FakeWorld::new() };
    {
        let mut s = w.screen.0.lock().unwrap();
        s.layout = MonitorLayout::new(l.case.monitors.iter().map(|m| m.monitor.clone()).collect());
        s.cursor = l.case.cursor;
        s.windows = l.case.windows.clone();
        s.frames = Some(l.frames.clone());
        s.children = l.case.children.iter().map(|(k, v)| (*k, v.clone())).collect();
    }
    w.elements.0.lock().unwrap().by_window = l.case.elements.iter().map(|(k, v)| (*k, v.clone())).collect();
    w
}

/// 점 하나의 결과(08 §5.3).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PointScore {
    pub case: String,
    pub label: String,
    pub at: (i32, i32),
    pub truth: PhysRect,
    /// 점으로 옮겼을 때 첫 강조
    pub first: Option<PhysRect>,
    pub first_iou: f64,
    /// 정답까지 휠 칸 수(0 = 첫 강조가 정답). 3칸 안에 없으면 None.
    pub steps: Option<u32>,
    /// 맞힌 사각형의 가장 큰 변 오차(px)
    pub edge_err: Option<i32>,
    /// 커서를 점으로 옮긴 입력 처리 시간
    pub ms: f64,
}

impl PointScore {
    pub fn top1(&self) -> bool {
        self.steps == Some(0)
    }

    pub fn top3(&self) -> bool {
        self.steps.is_some_and(|s| s <= 2)
    }
}

/// 변마다의 오차 중 가장 큰 것.
pub fn edge_error(a: PhysRect, b: PhysRect) -> i32 {
    [(a.x - b.x).abs(), (a.y - b.y).abs(), (a.right() - b.right()).abs(), (a.bottom() - b.bottom()).abs()].into_iter().max().unwrap_or(0)
}

pub fn iou(a: PhysRect, b: PhysRect) -> f64 {
    let inter = a.intersect(&b).map_or(0, |r| r.area()) as f64;
    let union = (a.area() + b.area()) as f64 - inter;
    if union <= 0.0 {
        0.0
    } else {
        inter / union
    }
}

fn hit(r: Option<PhysRect>, truth: PhysRect) -> Option<i32> {
    r.map(|r| edge_error(r, truth)).filter(|e| *e <= HIT_TOLERANCE)
}

fn key(code: &str) -> CaptureInput {
    CaptureInput::Key { key: code.into(), code: code.into(), ctrl: false, shift: false, alt: false }
}

/// 진입 → 점 옆 → 점: 첫 강조와 처리 시간. 이어서 휠 `dy`로 `MAX_STEPS`칸 돌린 강조들.
fn probe(l: &Loaded, at: (i32, i32), dy: i32) -> (Option<PhysRect>, f64, Vec<Option<PhysRect>>) {
    let w = world(l);
    let mut s = InkService::new(w.ports(), Platform::Win, "replay");
    s.start();
    s.on_hotkey("region");
    let last = |w: &FakeWorld| w.overlay.0.lock().unwrap().highlights.last().copied().flatten();
    // 진입 직후는 모니터 전체(움직이기 전) — 점 옆에서 들어온다
    let mon = l.case.monitors.iter().map(|m| m.monitor.bounds).find(|b| b.contains(ink_domain::geometry::PhysPoint { x: at.0, y: at.1 }));
    let side = if mon.is_some_and(|b| at.0 + 8 < b.right()) { at.0 + 8 } else { at.0 - 8 };
    s.capture_input(CaptureInput::Move { x: side, y: at.1 });
    let t = Instant::now();
    s.capture_input(CaptureInput::Move { x: at.0, y: at.1 });
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    let first = last(&w);
    let mut wheel = Vec::new();
    for _ in 0..MAX_STEPS {
        s.capture_input(CaptureInput::Wheel { dy });
        wheel.push(last(&w));
    }
    s.capture_input(key("Escape"));
    (first, ms, wheel)
}

/// 사례의 모든 점을 잰다.
pub fn score(l: &Loaded) -> Vec<PointScore> {
    l.case
        .points
        .iter()
        .map(|p| {
            let at = (p.at.x, p.at.y);
            let (first, ms, up) = probe(l, at, 1);
            let (_, _, down) = probe(l, at, -1);
            let mut best: Option<(u32, i32)> = hit(first, p.truth).map(|e| (0, e));
            if best.is_none() {
                for k in 0..MAX_STEPS as usize {
                    if let Some(e) = hit(up[k], p.truth).or_else(|| hit(down[k], p.truth)) {
                        best = Some((k as u32 + 1, e));
                        break;
                    }
                }
            }
            PointScore {
                case: l.case.id.clone(),
                label: p.label.clone(),
                at,
                truth: p.truth,
                first,
                first_iou: first.map_or(0.0, |r| iou(r, p.truth)),
                steps: best.map(|b| b.0),
                edge_err: best.map(|b| b.1),
                ms,
            }
        })
        .collect()
}

/// 묶음 요약(08 §5.3).
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub points: usize,
    pub top1: f64,
    pub top3: f64,
    /// 맞힌 점들의 평균 경계 오차(px)
    pub mean_edge: f64,
    pub p95_ms: f64,
}

pub fn summarize(scores: &[PointScore]) -> Summary {
    if scores.is_empty() {
        return Summary::default();
    }
    let n = scores.len() as f64;
    let hits: Vec<i32> = scores.iter().filter_map(|s| s.edge_err).collect();
    let mut ms: Vec<f64> = scores.iter().map(|s| s.ms).collect();
    ms.sort_by(|a, b| a.total_cmp(b));
    let rank = ((0.95 * ms.len() as f64).ceil() as usize).clamp(1, ms.len());
    Summary {
        points: scores.len(),
        top1: scores.iter().filter(|s| s.top1()).count() as f64 / n,
        top3: scores.iter().filter(|s| s.top3()).count() as f64 / n,
        mean_edge: if hits.is_empty() { 0.0 } else { hits.iter().sum::<i32>() as f64 / hits.len() as f64 },
        p95_ms: ms[rank - 1],
    }
}

/// `bench/results/recog-*.csv` 한 줄씩.
pub fn to_csv(scores: &[PointScore]) -> String {
    let mut out = String::from("case,label,x,y,truth_x,truth_y,truth_w,truth_h,first_iou,steps,edge_err,ms\n");
    for s in scores {
        let esc = |t: &str| format!("\"{}\"", t.replace('"', "\"\""));
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{:.3},{},{},{:.2}\n",
            esc(&s.case),
            esc(&s.label),
            s.at.0,
            s.at.1,
            s.truth.x,
            s.truth.y,
            s.truth.w,
            s.truth.h,
            s.first_iou,
            s.steps.map_or("fail".into(), |v| v.to_string()),
            s.edge_err.map_or(String::new(), |v| v.to_string()),
            s.ms
        ));
    }
    out
}

/// 폴더의 모든 사례(`*.json`, 이름순).
pub fn cases_in(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "json") && p.file_name().is_some_and(|n| n != "baseline.json")).collect())
        .unwrap_or_default();
    v.sort();
    v
}
