//! 자동 정답(08 §5.1 RB-01·02·03·12·14·19): 시험 문서가 아는 배치(CSS px)를 정지 이미지 속 표지로 화면 좌표에 옮긴다.
//!
//! 시험 문서(`bench/recog/pages/*.html`)는 `<script type="application/json" id="layout">`에 배치를 담고,
//! 왼쪽 위에 2×2 색 표지(자홍·청록 / 노랑·검정)를 그린다. 표지 한 칸의 화소 수 ÷ CSS 크기 = 배율,
//! 표지 위치 − 표지 CSS 위치 × 배율 = 문서 원점. 브라우저 틀·확대·Windows 배율을 몰라도 정확하다.

use ink_app::case::{Case, CasePoint};
use ink_app::Frame;
use ink_domain::geometry::{PhysPoint, PhysRect};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Layout {
    pub page: String,
    pub marker: MarkerSpec,
    pub elements: Vec<Element>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct MarkerSpec {
    pub x: f64,
    pub y: f64,
    /// 표지 전체 한 변(CSS px). 한 칸은 그 반.
    pub size: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Element {
    pub label: String,
    /// [x, y, w, h] CSS px
    pub rect: [f64; 4],
    /// 정답 점(CSS px). 없으면 가운데. 그릇은 빈 자리를 준다.
    #[serde(default)]
    pub at: Option<[f64; 2]>,
    /// false면 정답에서 뺀다(글자 등 상자가 아닌 것)
    #[serde(default = "yes")]
    pub truth: bool,
}

fn yes() -> bool {
    true
}

/// 시험 문서 HTML에서 배치를 꺼낸다.
pub fn extract_layout(html: &str) -> Result<Layout, String> {
    const TAG: &str = "<script type=\"application/json\" id=\"layout\">";
    let start = html.find(TAG).map(|i| i + TAG.len()).ok_or("layout 스크립트가 없어요")?;
    let end = html[start..].find("</script>").map(|i| start + i).ok_or("layout 스크립트 끝이 없어요")?;
    serde_json::from_str(&html[start..end]).map_err(|e| format!("layout JSON: {e}"))
}

/// 이미지에서 찾은 표지: 왼쪽 위 화소(이미지 좌표)와 한 칸 화소 수.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marker {
    pub x: i32,
    pub y: i32,
    pub cell: i32,
}

const MAGENTA: [u8; 3] = [255, 0, 255];
const CYAN: [u8; 3] = [0, 255, 255];
const YELLOW: [u8; 3] = [255, 255, 0];
const BLACK: [u8; 3] = [0, 0, 0];
/// 채널별 허용 차(색 관리·압축 흔들림)
const TOL: i32 = 12;

fn px(f: &Frame, x: i32, y: i32) -> Option<[u8; 3]> {
    if x < 0 || y < 0 || x >= f.width as i32 || y >= f.height as i32 {
        return None;
    }
    let o = (y as usize * f.width as usize + x as usize) * 4;
    // BGRA → RGB
    Some([f.bgra[o + 2], f.bgra[o + 1], f.bgra[o]])
}

fn is(c: Option<[u8; 3]>, want: [u8; 3]) -> bool {
    c.is_some_and(|c| c.iter().zip(want).all(|(a, b)| (*a as i32 - b as i32).abs() <= TOL))
}

/// 2×2 색 표지를 찾는다(가장 위·왼쪽 것). 한 칸이 4 px보다 작으면 무시.
pub fn find_marker(f: &Frame) -> Option<Marker> {
    for y in 0..f.height as i32 {
        for x in 0..f.width as i32 {
            if !is(px(f, x, y), MAGENTA) || is(px(f, x - 1, y), MAGENTA) || is(px(f, x, y - 1), MAGENTA) {
                continue;
            }
            let run = (x..f.width as i32).take_while(|&xx| is(px(f, xx, y), MAGENTA)).count() as i32;
            let down = (y..f.height as i32).take_while(|&yy| is(px(f, x, yy), MAGENTA)).count() as i32;
            if run < 4 || (run - down).abs() > 1 {
                continue;
            }
            let c = run;
            let mid = c / 2;
            if is(px(f, x + c + mid, y + mid), CYAN) && is(px(f, x + mid, y + c + mid), YELLOW) && is(px(f, x + c + mid, y + c + mid), BLACK) {
                return Some(Marker { x, y, cell: c });
            }
        }
    }
    None
}

/// 표지가 있는 모니터를 찾아 배치의 모든 정답 점을 화면(가상 화면 물리 px) 좌표로 만든다.
/// 돌려주는 것: (모니터 id, 배율, 점들).
pub fn points(case: &Case, frames: &[Frame], layout: &Layout) -> Result<(u32, f64, Vec<CasePoint>), String> {
    for f in frames {
        let Some(m) = find_marker(f) else { continue };
        let mon = case.monitors.iter().find(|c| c.monitor.id == f.monitor_id).ok_or("사례에 그 모니터가 없어요")?;
        let scale = m.cell as f64 / (layout.marker.size / 2.0);
        let ox = mon.monitor.bounds.x as f64 + m.x as f64 - layout.marker.x * scale;
        let oy = mon.monitor.bounds.y as f64 + m.y as f64 - layout.marker.y * scale;
        let at = |cx: f64, cy: f64| PhysPoint { x: (ox + cx * scale).floor() as i32, y: (oy + cy * scale).floor() as i32 };
        let out = layout
            .elements
            .iter()
            .filter(|e| e.truth)
            .map(|e| {
                let [x, y, w, h] = e.rect;
                let (l, t) = ((ox + x * scale).round() as i32, (oy + y * scale).round() as i32);
                let (r, b) = ((ox + (x + w) * scale).round() as i32, (oy + (y + h) * scale).round() as i32);
                let [px, py] = e.at.unwrap_or([x + w / 2.0, y + h / 2.0]);
                CasePoint { at: at(px, py), truth: PhysRect::from_edges(l, t, r, b), label: format!("{} · {}", layout.page, e.label) }
            })
            .collect();
        return Ok((f.monitor_id, scale, out));
    }
    Err("어느 모니터에서도 표지를 찾지 못했어요(브라우저 확대 100%, 표지가 가려지지 않게)".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ink_app::case::{CaseMonitor, CASE_SCHEMA};
    use ink_domain::geometry::Monitor;
    use std::sync::Arc;

    const PAGE: &str = include_str!("../../../bench/recog/pages/rb01.html");

    fn frame_with_marker(id: u32, w: u32, h: u32, at: (i32, i32), cell: i32) -> Frame {
        let mut px = vec![255u8; (w * h * 4) as usize];
        let mut put = |x0: i32, y0: i32, rgb: [u8; 3]| {
            for y in y0..y0 + cell {
                for x in x0..x0 + cell {
                    let o = ((y as u32 * w + x as u32) * 4) as usize;
                    px[o..o + 4].copy_from_slice(&[rgb[2], rgb[1], rgb[0], 255]);
                }
            }
        };
        put(at.0, at.1, MAGENTA);
        put(at.0 + cell, at.1, CYAN);
        put(at.0, at.1 + cell, YELLOW);
        put(at.0 + cell, at.1 + cell, BLACK);
        Frame { monitor_id: id, width: w, height: h, bgra: Arc::new(px) }
    }

    fn case(monitors: &[(u32, PhysRect)]) -> Case {
        Case {
            schema: CASE_SCHEMA,
            id: "t".into(),
            note: String::new(),
            monitors: monitors
                .iter()
                .map(|(id, b)| CaseMonitor { monitor: Monitor { id: *id, bounds: *b, work: *b, scale: 1.0, primary: *id == 0 }, frame: format!("t.m{id}.png") })
                .collect(),
            cursor: PhysPoint::default(),
            windows: vec![],
            elements: Default::default(),
            children: Default::default(),
            points: vec![],
        }
    }

    #[test]
    fn 시험_문서의_배치를_읽는다() {
        let l = extract_layout(PAGE).unwrap();
        assert_eq!(l.page, "rb01");
        assert_eq!(l.marker, MarkerSpec { x: 8.0, y: 8.0, size: 16.0 });
        assert!(l.elements.iter().filter(|e| e.truth).count() >= 30);
        assert!(l.elements.iter().any(|e| e.label == "카드 제목" && !e.truth), "글자는 정답에서 뺀다");
    }

    #[test]
    fn 표지는_배율이_달라도_찾는다() {
        for cell in [8, 10, 12, 16] {
            let f = frame_with_marker(0, 300, 200, (37, 51), cell);
            assert_eq!(find_marker(&f), Some(Marker { x: 37, y: 51, cell }), "칸 {cell}");
        }
        let blank = Frame { monitor_id: 0, width: 50, height: 50, bgra: Arc::new(vec![255; 50 * 50 * 4]) };
        assert_eq!(find_marker(&blank), None);
    }

    #[test]
    fn 둘째_모니터_150퍼센트에서도_정답이_화면_좌표로() {
        // 브라우저 문서 원점 = 모니터 안 (100, 140), 배율 1.5 → 표지(CSS 8,8)는 (112, 152), 한 칸 12 px
        let f = frame_with_marker(1, 2400, 1400, (112, 152), 12);
        let c = case(&[(0, PhysRect::new(0, 0, 1920, 1080)), (1, PhysRect::new(1920, 0, 2400, 1400))]);
        let l = extract_layout(PAGE).unwrap();
        let (mon, scale, pts) = points(&c, &[f], &l).unwrap();
        assert_eq!((mon, scale), (1, 1.5));
        let card = pts.iter().find(|p| p.label == "rb01 · 카드").unwrap();
        // 카드 CSS (272,120,480,280) → 원점 (1920+100, 140) + ×1.5
        assert_eq!(card.truth, PhysRect::new(2020 + 408, 140 + 180, 720, 420));
        assert_eq!(card.at, PhysPoint { x: 2020 + 1050, y: 140 + 345 }, "그릇은 빈 자리 점");
        let btn = pts.iter().find(|p| p.label == "rb01 · 카드 단추 1").unwrap();
        assert!(btn.truth.contains(btn.at), "점은 정답 안");
    }

    #[test]
    fn rb14_캔버스_문서의_배치를_읽고_정답이_화면_좌표로() {
        let page = include_str!("../../../bench/recog/pages/rb14.html");
        let l = extract_layout(page).unwrap();
        assert_eq!(l.page, "rb14");
        assert_eq!(l.marker, MarkerSpec { x: 8.0, y: 8.0, size: 16.0 });
        assert_eq!(l.elements.len(), 33);
        for want in ["2D 캔버스", "WebGL 판", "GL 사각 1", "영상 틀", "도표 캔버스"] {
            assert!(l.elements.iter().any(|e| e.label == want && e.truth), "{want}");
        }
        // 배율 1, 문서 원점 = 모니터 (0, 0): 정답 = CSS 사각형, 점은 모두 자기 정답 안
        let f = frame_with_marker(0, 1280, 800, (8, 8), 8);
        let (_, scale, pts) = points(&case(&[(0, PhysRect::new(0, 0, 1280, 800))]), &[f], &l).unwrap();
        assert_eq!(scale, 1.0);
        assert_eq!(pts.len(), 33);
        let gl = pts.iter().find(|p| p.label == "rb14 · GL 사각 1").unwrap();
        assert_eq!(gl.truth, PhysRect::new(840, 72, 180, 120));
        let video = pts.iter().find(|p| p.label == "rb14 · 영상 틀").unwrap();
        assert_eq!((video.truth.w, video.truth.h), (368, 207), "16:9");
        assert!(pts.iter().all(|p| p.truth.contains(p.at)), "점은 정답 안");
    }

    #[test]
    fn 표지가_없으면_알려_준다() {
        let blank = Frame { monitor_id: 0, width: 50, height: 50, bgra: Arc::new(vec![255; 50 * 50 * 4]) };
        let c = case(&[(0, PhysRect::new(0, 0, 50, 50))]);
        assert!(points(&c, &[blank], &extract_layout(PAGE).unwrap()).is_err());
    }
}
