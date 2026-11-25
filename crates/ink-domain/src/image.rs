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
        if a > 0 {
            for k in 0..3 {
                o[k] = ((b[k] as u32 * 255 + a / 2) / a).min(255) as u8;
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
}
