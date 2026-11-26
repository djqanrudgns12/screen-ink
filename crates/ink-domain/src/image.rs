//! 정지 이미지 다루기: 모니터를 넘는 범위 자르기 · 섬 썸네일 · 단색 판정(02 §1.6 추천 칩).
//! 입력·출력 모두 BGRA(위→아래). 결과의 알파는 255(R-OUT-3: 투명 픽셀이 없으면 알파를 빼서 저장).

use crate::geometry::PhysRect;

/// 모니터 한 장(가상 화면 물리 px 경계 + 픽셀).
#[derive(Clone, Copy)]
pub struct FrameRef<'a> {
    pub bounds: PhysRect,
    pub w: u32,
    pub h: u32,
    pub bgra: &'a [u8],
}

/// 범위 `r`을 잘라 한 장으로. 모니터 사이 틈은 검정(불투명).
pub fn crop(frames: &[FrameRef], r: PhysRect) -> (u32, u32, Vec<u8>) {
    let (w, h) = (r.w.max(0) as usize, r.h.max(0) as usize);
    let mut out = vec![0u8; w * h * 4];
    for px in out.chunks_exact_mut(4) {
        px[3] = 255;
    }
    for f in frames {
        let Some(part) = f.bounds.intersect(&r) else { continue };
        let row_len = part.w as usize * 4;
        for y in 0..part.h {
            let sy = (part.y - f.bounds.y + y) as usize;
            let sx = (part.x - f.bounds.x) as usize;
            let src = (sy * f.w as usize + sx) * 4;
            let dy = (part.y - r.y + y) as usize;
            let dx = (part.x - r.x) as usize;
            let dst = (dy * w + dx) * 4;
            if let (Some(s), Some(d)) = (f.bgra.get(src..src + row_len), out.get_mut(dst..dst + row_len)) {
                d.copy_from_slice(s);
            }
        }
        // GDI가 알파 바이트를 0으로 남길 수 있다 → 그 부분만 불투명으로
        for y in 0..part.h {
            let dy = (part.y - r.y + y) as usize;
            let dx = (part.x - r.x) as usize;
            let start = (dy * w + dx) * 4;
            for px in out[start..start + row_len].chunks_exact_mut(4) {
                px[3] = 255;
            }
        }
    }
    (w as u32, h as u32, out)
}

/// 캡처 순간의 커서 모양(곧은 알파 BGRA)과 화면 위치(왼쪽 위, 물리 px).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorSprite {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub bgra: Vec<u8>,
}

/// 잘라 낸 이미지(`origin` = 그 범위의 왼쪽 위)에 커서를 알파 합성한다. 범위 밖 부분은 버린다.
pub fn composite_cursor(dst: &mut [u8], dw: u32, dh: u32, origin: (i32, i32), c: &CursorSprite) {
    for cy in 0..c.h as i32 {
        let y = c.y + cy - origin.1;
        if y < 0 || y >= dh as i32 {
            continue;
        }
        for cx in 0..c.w as i32 {
            let x = c.x + cx - origin.0;
            if x < 0 || x >= dw as i32 {
                continue;
            }
            let s = ((cy as u32 * c.w + cx as u32) * 4) as usize;
            let a = c.bgra[s + 3] as u32;
            if a == 0 {
                continue;
            }
            let d = ((y as u32 * dw + x as u32) * 4) as usize;
            for k in 0..3 {
                dst[d + k] = ((c.bgra[s + k] as u32 * a + dst[d + k] as u32 * (255 - a)) / 255) as u8;
            }
            dst[d + 3] = 255;
        }
    }
}

/// 검은 바탕·흰 바탕에 같은 커서를 그린 두 장 → 곧은 알파 BGRA(투명도·그림자 살림).
pub fn alpha_from_two(black: &[u8], white: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; black.len()];
    for ((o, b), w) in out.chunks_exact_mut(4).zip(black.chunks_exact(4)).zip(white.chunks_exact(4)) {
        // 바탕 차이만큼 투명: alpha = 255 - (흰 - 검) (채널 평균)
        let diff = (0..3).map(|k| w[k] as i32 - b[k] as i32).sum::<i32>() / 3;
        let a = (255 - diff).clamp(0, 255) as u32;
        for k in 0..3 {
            // a = 0이면 None → 색 0 그대로(완전 투명)
            if let Some(v) = (b[k] as u32 * 255 + a / 2).checked_div(a) {
                o[k] = v.min(255) as u8;
            }
        }
        o[3] = a as u8;
    }
    out
}

/// 상자 필터로 `max_w×max_h` 안에 들어가게 줄인다(키우지는 않음).
pub fn thumbnail(w: u32, h: u32, bgra: &[u8], max_w: u32, max_h: u32) -> (u32, u32, Vec<u8>) {
    if w == 0 || h == 0 {
        return (0, 0, Vec::new());
    }
    let s = (max_w as f64 / w as f64).min(max_h as f64 / h as f64).min(1.0);
    let (tw, th) = (((w as f64 * s).round() as u32).max(1), ((h as f64 * s).round() as u32).max(1));
    let mut out = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        let (y0, y1) = ((ty * h / th) as usize, (((ty + 1) * h / th).max(ty * h / th + 1)) as usize);
        for tx in 0..tw {
            let (x0, x1) = ((tx * w / tw) as usize, (((tx + 1) * w / tw).max(tx * w / tw + 1)) as usize);
            let mut acc = [0u32; 4];
            let mut n = 0u32;
            for y in y0..y1.min(h as usize) {
                for x in x0..x1.min(w as usize) {
                    let i = (y * w as usize + x) * 4;
                    for c in 0..4 {
                        acc[c] += bgra[i + c] as u32;
                    }
                    n += 1;
                }
            }
            let o = ((ty * tw + tx) * 4) as usize;
            for c in 0..4 {
                out[o + c] = (acc[c] / n.max(1)) as u8;
            }
        }
    }
    (tw, th, out)
}

/// 92% 이상이 한 색(채널마다 ±6)이면 그 색(RGB). 큰 범위는 간격을 두고 본다(50 ms 예산 안).
pub fn uniform_color(w: u32, h: u32, bgra: &[u8]) -> Option<[u8; 3]> {
    let total = w as usize * h as usize;
    if total == 0 {
        return None;
    }
    let step = (total / 200_000).max(1);
    let px = |i: usize| [bgra[i * 4 + 2], bgra[i * 4 + 1], bgra[i * 4]];
    // 가운데 픽셀을 기준 후보로, 맞으면 그 색
    let base = px(total / 2);
    let (mut same, mut seen) = (0usize, 0usize);
    let mut i = 0;
    while i < total {
        let p = px(i);
        if (0..3).all(|c| (p[c] as i32 - base[c] as i32).abs() <= 6) {
            same += 1;
        }
        seen += 1;
        i += step;
    }
    (same as f64 >= seen as f64 * 0.92).then_some(base)
}

// ─────────────── 캡처 주석 합성(4.9, R-CAP-28 · B §2.1 "합성 결과 전달 계약") ───────────────

/// 모자이크 칸 크기 한도(물리 px). UI는 논리 6~24 px × 배율을 보낸다(사용자 결정 #1).
pub const MOSAIC_BLOCK_MIN: u32 = 1;
pub const MOSAIC_BLOCK_MAX: u32 = 256;

/// 모자이크 사각형 하나(결과 그림 기준 물리 px — 왼쪽 위 = 고른 범위의 왼쪽 위). 그림 밖 부분은 버린다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MosaicRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// 칸 크기(물리 px). 칸 수 = ceil(w / block) — UI 미리보기(canvasRender.drawMosaic)와 같은 나눔
    pub block: u32,
}

/// 원본 BGRA에 모자이크(칸마다 평균색)를 적용한다(순수 함수). 칸 나눔은 사각형 왼쪽 위 기준이고,
/// 화소 x가 속한 칸 = floor((x − rect.x) × 칸 수 / rect.w)로 UI의 축소→계단 확대와 맞춘다.
/// 평균은 **원본**에서 잰다(겹친 모자이크끼리 서로를 다시 흐리지 않음 — 화면 미리보기와 같음).
/// 사각형 밖 화소는 바꾸지 않는다. 크기가 맞지 않는 버퍼는 그대로 둔다.
pub fn apply_mosaic(bgra: &mut [u8], w: u32, h: u32, rects: &[MosaicRect]) {
    let (wi, hi) = (w as i64, h as i64);
    if rects.is_empty() || bgra.len() as u64 != w as u64 * h as u64 * 4 {
        return;
    }
    // 둘 이상이면 원본을 한 번 떠 둔다(하나면 읽고 나서 쓰므로 필요 없음)
    let source: Option<Vec<u8>> = (rects.len() > 1).then(|| bgra.to_vec());
    for r in rects {
        if r.w <= 0 || r.h <= 0 {
            continue;
        }
        let (rx, ry, rw, rh) = (r.x as i64, r.y as i64, r.w as i64, r.h as i64);
        let x0 = rx.max(0);
        let y0 = ry.max(0);
        let x1 = (rx + rw).min(wi);
        let y1 = (ry + rh).min(hi);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let block = i64::from(r.block.clamp(MOSAIC_BLOCK_MIN, MOSAIC_BLOCK_MAX));
        let cols = ((rw + block - 1) / block).max(1);
        let rows = ((rh + block - 1) / block).max(1);
        let col_of = |x: i64| (((x - rx) * cols) / rw).clamp(0, cols - 1) as usize;
        let row_of = |y: i64| (((y - ry) * rows) / rh).clamp(0, rows - 1) as usize;
        let mut acc = vec![[0u64; 5]; (cols * rows) as usize];
        {
            let src: &[u8] = source.as_deref().unwrap_or(bgra);
            for y in y0..y1 {
                let row = row_of(y) * cols as usize;
                let base = (y * wi) as usize * 4;
                for x in x0..x1 {
                    let i = base + x as usize * 4;
                    let a = &mut acc[row + col_of(x)];
                    for (sum, &v) in a.iter_mut().zip(&src[i..i + 4]) {
                        *sum += u64::from(v);
                    }
                    a[4] += 1;
                }
            }
        }
        for y in y0..y1 {
            let row = row_of(y) * cols as usize;
            let base = (y * wi) as usize * 4;
            for x in x0..x1 {
                let a = &acc[row + col_of(x)];
                let n = a[4].max(1);
                let i = base + x as usize * 4;
                for (px, &sum) in bgra[i..i + 3].iter_mut().zip(&a[..3]) {
                    *px = ((sum + n / 2) / n) as u8;
                }
                bgra[i + 3] = 255;
            }
        }
    }
}

/// 투명 주석 층(곧은 알파 RGBA — 캔버스 `getImageData` 그대로)을 원본 BGRA 위에 source-over 합성한다(순수 함수).
/// 결과 알파는 255(R-OUT-3). 크기가 맞지 않으면 아무것도 하지 않는다.
pub fn composite_rgba_over_bgra(dst_bgra: &mut [u8], src_rgba: &[u8], w: u32, h: u32) {
    let n = w as u64 * h as u64 * 4;
    if dst_bgra.len() as u64 != n || src_rgba.len() as u64 != n {
        return;
    }
    for (d, s) in dst_bgra.chunks_exact_mut(4).zip(src_rgba.chunks_exact(4)) {
        let a = u32::from(s[3]);
        if a == 0 {
            continue;
        }
        // RGBA → BGRA
        let sc = [s[2], s[1], s[0]];
        if a == 255 {
            d[..3].copy_from_slice(&sc);
        } else {
            for (dc, &s) in d[..3].iter_mut().zip(&sc) {
                *dc = ((u32::from(s) * a + u32::from(*dc) * (255 - a) + 127) / 255) as u8;
            }
        }
        d[3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, b: u8, g: u8, r: u8) -> Vec<u8> {
        (0..w * h).flat_map(|_| [b, g, r, 0]).collect()
    }

    #[test]
    fn 두_모니터에_걸친_범위를_한_장으로() {
        let left = solid(4, 4, 10, 20, 30);
        let right = solid(4, 4, 40, 50, 60);
        let frames = [
            FrameRef { bounds: PhysRect::new(0, 0, 4, 4), w: 4, h: 4, bgra: &left },
            FrameRef { bounds: PhysRect::new(4, 0, 4, 4), w: 4, h: 4, bgra: &right },
        ];
        let (w, h, out) = crop(&frames, PhysRect::new(2, 1, 4, 2));
        assert_eq!((w, h), (4, 2));
        assert_eq!(&out[0..4], &[10, 20, 30, 255], "왼쪽 모니터, 알파는 불투명으로");
        assert_eq!(&out[12..16], &[40, 50, 60, 255], "오른쪽 모니터");
        // 모니터 밖은 검정 불투명
        let (_, _, gap) = crop(&frames, PhysRect::new(6, 3, 4, 2));
        let i = (4 + 3) * 4; // 둘째 줄 넷째 칸 = (9,4), 두 모니터 밖
        assert_eq!(&gap[i..i + 4], &[0, 0, 0, 255]);
    }

    #[test]
    fn 썸네일은_비율을_지키고_평균색() {
        let img = solid(400, 100, 0, 0, 200);
        let (w, h, t) = thumbnail(400, 100, &img, 120, 48);
        assert_eq!((w, h), (120, 30));
        assert_eq!(&t[0..3], &[0, 0, 200]);
        assert_eq!(thumbnail(10, 10, &solid(10, 10, 1, 1, 1), 120, 48).0, 10, "키우지 않는다");
    }

    #[test]
    fn 커서는_알파로_합성되고_범위_밖은_버린다() {
        // 검은 바탕 그림 [불투명 흰, 반투명 회색(= 128·128/255), 투명]
        let black = [255, 255, 255, 0, 64, 64, 64, 0, 0, 0, 0, 0];
        let white = [255, 255, 255, 0, 191, 191, 191, 0, 255, 255, 255, 0];
        let a = alpha_from_two(&black, &white);
        assert_eq!(a[3], 255);
        assert!((a[7] as i32 - 128).abs() <= 1);
        assert_eq!(a[11], 0);
        let sprite = CursorSprite { x: 9, y: 10, w: 3, h: 1, bgra: a };
        let mut dst = solid(4, 2, 0, 0, 0);
        composite_cursor(&mut dst, 4, 2, (8, 10), &sprite);
        assert_eq!(&dst[4..8], &[255, 255, 255, 255], "(9,10) → 범위 안 x=1");
        assert!(dst[8] > 50 && dst[8] < 80, "반투명은 섞임");
        assert_eq!(&dst[12..15], &[0, 0, 0], "투명은 그대로");
    }

    #[test]
    fn 단색_판정() {
        let mut img = solid(100, 100, 255, 255, 255);
        assert_eq!(uniform_color(100, 100, &img), Some([255, 255, 255]));
        for i in 0..1000 {
            img[i * 4] = 0; // 10%가 다른 색
        }
        assert_eq!(uniform_color(100, 100, &img), None);
    }

    /// (x, y) 화소 BGRA.
    fn at(img: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        [img[i], img[i + 1], img[i + 2], img[i + 3]]
    }

    /// 가로 줄무늬: x가 짝수면 검정, 홀수면 흰색(B=G=R).
    fn stripes(w: u32, h: u32) -> Vec<u8> {
        (0..w * h).flat_map(|i| if (i % w) % 2 == 0 { [0, 0, 0, 255] } else { [200, 200, 200, 255] }).collect()
    }

    #[test]
    fn 모자이크_블록_평균_및_경계_클리핑() {
        let (w, h) = (12u32, 6u32);
        let mut img = stripes(w, h);
        let before = img.clone();
        // 칸 4 px: 가로 2칸·세로 1칸(8×4 → 칸마다 검·흰 반반 = 100)
        apply_mosaic(&mut img, w, h, &[MosaicRect { x: 2, y: 1, w: 8, h: 4, block: 4 }]);
        for y in 0..h {
            for x in 0..w {
                let inside = (2..10).contains(&x) && (1..5).contains(&y);
                if inside {
                    assert_eq!(at(&img, w, x, y), [100, 100, 100, 255], "안쪽 ({x},{y})는 칸 평균");
                } else {
                    assert_eq!(at(&img, w, x, y), at(&before, w, x, y), "바깥 ({x},{y})은 그대로");
                }
            }
        }
        // 그림 밖으로 나간 사각형: 안쪽 부분만, 평균도 안쪽 화소로만
        let mut img = stripes(w, h);
        apply_mosaic(&mut img, w, h, &[MosaicRect { x: 10, y: -3, w: 6, h: 6, block: 6 }]);
        assert_eq!(at(&img, w, 10, 0), [100, 100, 100, 255], "x 10·11만 들어옴 → 검·흰 평균");
        assert_eq!(at(&img, w, 11, 2), [100, 100, 100, 255]);
        assert_eq!(at(&img, w, 9, 0), at(&stripes(w, h), w, 9, 0), "왼쪽 이웃은 그대로");
        assert_eq!(at(&img, w, 10, 3), at(&stripes(w, h), w, 10, 3), "아래로는 y 0..3까지만");
        // 완전히 밖·빈 사각형·크기 틀린 버퍼는 아무 일도 없다
        let mut img = stripes(w, h);
        apply_mosaic(&mut img, w, h, &[MosaicRect { x: 40, y: 40, w: 5, h: 5, block: 2 }, MosaicRect { x: 0, y: 0, w: 0, h: 3, block: 2 }]);
        assert_eq!(img, stripes(w, h));
        let mut short = vec![0u8; 8];
        apply_mosaic(&mut short, w, h, &[MosaicRect { x: 0, y: 0, w: 2, h: 2, block: 1 }]);
        assert_eq!(short, vec![0u8; 8]);
    }

    #[test]
    fn 모자이크_칸_수는_올림_나눔이고_겹쳐도_원본_평균() {
        // 10 px 폭을 칸 4 px → 3칸(UI drawMosaic의 ceil(w/block)과 같음), 화소 x의 칸 = floor(x·3/10)
        let (w, h) = (10u32, 1u32);
        let mut img: Vec<u8> = (0..w).flat_map(|x| [x as u8 * 10, 0, 0, 255]).collect();
        apply_mosaic(&mut img, w, h, &[MosaicRect { x: 0, y: 0, w: 10, h: 1, block: 4 }]);
        // 칸 0 = x 0..3 (0,10,20,30) → 15, 칸 1 = x 4..6 (40,50,60) → 50, 칸 2 = x 7..9 (70,80,90) → 80
        let blues: Vec<u8> = img.chunks_exact(4).map(|p| p[0]).collect();
        assert_eq!(blues, vec![15, 15, 15, 15, 50, 50, 50, 80, 80, 80]);
        // 겹친 두 모자이크: 둘째는 첫째 결과가 아니라 원본에서 평균
        let mut img: Vec<u8> = (0..w).flat_map(|x| [x as u8 * 10, 0, 0, 255]).collect();
        apply_mosaic(&mut img, w, h, &[MosaicRect { x: 0, y: 0, w: 10, h: 1, block: 10 }, MosaicRect { x: 0, y: 0, w: 2, h: 1, block: 2 }]);
        assert_eq!(img[0], 5, "둘째(x 0..1) = 원본 0·10의 평균, 첫째 평균(45)이 아님");
        assert_eq!(img[4 * 5], 45, "둘째 밖은 첫째 평균");
    }

    #[test]
    fn 투명_주석_rgba_원본_bgra_알파합성() {
        let (w, h) = (3u32, 1u32);
        // 원본: 파랑 바탕(B=200)
        let mut dst: Vec<u8> = (0..w * h).flat_map(|_| [200, 0, 0, 0]).collect();
        // [불투명 빨강, 형광펜 노랑 알파 0.38(97), 투명]
        let src = [255, 0, 0, 255, 255, 214, 10, 97, 9, 9, 9, 0];
        composite_rgba_over_bgra(&mut dst, &src, w, h);
        assert_eq!(at(&dst, w, 0, 0), [0, 0, 255, 255], "불투명 빨강 펜은 그대로(RGBA → BGRA)");
        // 반투명: c = s·a + d·(1−a), 기대값은 실수로 계산해 1/255 안
        let a = 97.0 / 255.0;
        let expect = [10.0 * a + 200.0 * (1.0 - a), 214.0 * a, 255.0 * a];
        let got = at(&dst, w, 1, 0);
        for k in 0..3 {
            assert!((f64::from(got[k]) - expect[k]).abs() <= 1.0, "채널 {k}: {} vs {:.2}", got[k], expect[k]);
        }
        assert_eq!(got[3], 255, "결과는 불투명");
        assert_eq!(at(&dst, w, 2, 0), [200, 0, 0, 0], "투명 화소는 원본 그대로");
        // 크기가 틀리면 아무것도 안 함
        let mut d2 = vec![1u8; 12];
        composite_rgba_over_bgra(&mut d2, &[0, 0, 0, 255], w, h);
        assert_eq!(d2, vec![1u8; 12]);
    }

    #[test]
    fn 모자이크_사각형은_카멜_json() {
        let r: MosaicRect = serde_json::from_str(r#"{"x":1,"y":-2,"w":30,"h":40,"block":15}"#).unwrap();
        assert_eq!(r, MosaicRect { x: 1, y: -2, w: 30, h: 40, block: 15 });
    }
}
