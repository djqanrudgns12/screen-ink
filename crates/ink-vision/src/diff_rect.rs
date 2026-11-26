//! Snipaste 대조 하네스 판독(08 §5.4, C §2.3 `3f-snipaste`): Snipaste F1 전·후 화면에서 강조 사각형을 찾는다.
//!
//! Snipaste는 강조 바깥을 어둡게 덮는다 → **전·후 화소가 같은 곳 = 강조 안쪽**. 다만
//! - 아주 어두운 화소는 어둡게 해도 거의 그대로라 "같음"인지 알 수 없다(모호) — 판정에서 뺀다.
//! - 확대경·크기 꼬리표가 커서 근처에 그려진다 — 같은 화소 덩어리 중 **커서(씨앗)를 담는 것**을 고른다.
//!
//! 순서: ① 같음(채널 차 ≤ `SAME_TOL`, 모호 아님)·다름(차 > `SAME_TOL`) 표시 ② 씨앗 덩어리(4-이웃)의 바운딩 박스
//! ③ 네 변을 바깥으로 넓힌다 — 다음 줄에 다름이 거의 없으면 건너가고, "같음이 있는 마지막 줄"까지만 확정
//! (강조 가장자리의 어두운 제목 표시줄은 넘고, 강조 밖 어두운 바탕은 넘지 않는다).
//!
//! Snipaste 테두리 선이 사각형 안쪽에 그려지면 결과가 테두리 두께만큼 작다 — 하네스가 알려진 창으로 한 번 재서
//! `inset`으로 되돌린다(`bench/recog/compare-snipaste.ps1`).

use ink_domain::geometry::PhysRect;

/// 전·후 같은 화소로 칠 채널별 차.
pub const SAME_TOL: u8 = 3;
/// 이보다 어두운 화소(가장 밝은 채널)는 어둡게 덮어도 차가 작아 판정하지 않는다.
pub const DARK: u8 = 16;
/// 씨앗 자리가 모호하면 이 거리(px) 안에서 가장 가까운 같음 화소를 씨앗으로.
pub const SEED_SEARCH: i32 = 48;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Px {
    Same,
    Diff,
    Unknown,
}

fn classify(before: &[u8], after: &[u8], i: usize) -> Px {
    let (b, a) = (&before[i * 4..i * 4 + 3], &after[i * 4..i * 4 + 3]);
    let diff = b.iter().zip(a).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0);
    if diff > SAME_TOL {
        Px::Diff
    } else if b.iter().copied().max().unwrap_or(0) < DARK {
        Px::Unknown
    } else {
        Px::Same
    }
}

/// 전·후 BGRA(위→아래, 같은 크기)에서 강조 사각형(이미지 좌표). `seed` = 커서 자리(이미지 좌표).
/// 같은 화소가 없으면(전부 어두워짐) None. 씨앗이 없으면 가장 큰 같음 덩어리.
pub fn detect_snipaste_highlight(before: &[u8], after: &[u8], w: u32, h: u32, seed: Option<(i32, i32)>) -> Option<PhysRect> {
    let (w, h) = (w as i32, h as i32);
    let n = (w as usize) * (h as usize);
    if w <= 0 || h <= 0 || before.len() < n * 4 || after.len() < n * 4 {
        return None;
    }
    let mask: Vec<Px> = (0..n).map(|i| classify(before, after, i)).collect();
    let at = |x: i32, y: i32| mask[(y * w + x) as usize];

    // ② 씨앗 덩어리
    let start = match seed {
        Some((sx, sy)) if sx >= 0 && sy >= 0 && sx < w && sy < h => nearest_same(&mask, w, h, sx, sy)?,
        Some(_) => return None,
        None => largest_component_seed(&mask, w, h)?,
    };
    let mut rect = component_bbox(&mask, w, h, start);

    // ③ 변 넓히기
    let diff_limit = |span: i32| (span / 100).max(2);
    // 위
    {
        let (mut y, mut keep) = (rect.y - 1, rect.y);
        while y >= 0 {
            let (mut same, mut diff) = (0, 0);
            for x in rect.x..rect.right() {
                match at(x, y) {
                    Px::Same => same += 1,
                    Px::Diff => diff += 1,
                    Px::Unknown => {}
                }
            }
            if diff > diff_limit(rect.w) {
                break;
            }
            if same > 0 {
                keep = y;
            }
            y -= 1;
        }
        rect = PhysRect::from_edges(rect.x, keep, rect.right(), rect.bottom());
    }
    // 아래
    {
        let (mut y, mut keep) = (rect.bottom(), rect.bottom());
        while y < h {
            let (mut same, mut diff) = (0, 0);
            for x in rect.x..rect.right() {
                match at(x, y) {
                    Px::Same => same += 1,
                    Px::Diff => diff += 1,
                    Px::Unknown => {}
                }
            }
            if diff > diff_limit(rect.w) {
                break;
            }
            if same > 0 {
                keep = y + 1;
            }
            y += 1;
        }
        rect = PhysRect::from_edges(rect.x, rect.y, rect.right(), keep);
    }
    // 왼쪽
    {
        let (mut x, mut keep) = (rect.x - 1, rect.x);
        while x >= 0 {
            let (mut same, mut diff) = (0, 0);
            for y in rect.y..rect.bottom() {
                match at(x, y) {
                    Px::Same => same += 1,
                    Px::Diff => diff += 1,
                    Px::Unknown => {}
                }
            }
            if diff > diff_limit(rect.h) {
                break;
            }
            if same > 0 {
                keep = x;
            }
            x -= 1;
        }
        rect = PhysRect::from_edges(keep, rect.y, rect.right(), rect.bottom());
    }
    // 오른쪽
    {
        let (mut x, mut keep) = (rect.right(), rect.right());
        while x < w {
            let (mut same, mut diff) = (0, 0);
            for y in rect.y..rect.bottom() {
                match at(x, y) {
                    Px::Same => same += 1,
                    Px::Diff => diff += 1,
                    Px::Unknown => {}
                }
            }
            if diff > diff_limit(rect.h) {
                break;
            }
            if same > 0 {
                keep = x + 1;
            }
            x += 1;
        }
        rect = PhysRect::from_edges(rect.x, rect.y, keep, rect.bottom());
    }
    Some(rect)
}

/// 씨앗에서 가장 가까운(체비쇼프 거리) 같음 화소.
fn nearest_same(mask: &[Px], w: i32, h: i32, sx: i32, sy: i32) -> Option<(i32, i32)> {
    for r in 0..=SEED_SEARCH {
        for y in (sy - r).max(0)..=(sy + r).min(h - 1) {
            for x in (sx - r).max(0)..=(sx + r).min(w - 1) {
                if (x - sx).abs().max((y - sy).abs()) == r && mask[(y * w + x) as usize] == Px::Same {
                    return Some((x, y));
                }
            }
        }
    }
    None
}

/// 같음 덩어리를 4-이웃으로 채워 바운딩 박스.
fn component_bbox(mask: &[Px], w: i32, h: i32, start: (i32, i32)) -> PhysRect {
    let mut seen = vec![false; mask.len()];
    flood(mask, w, h, start, &mut seen).1
}

/// (화소 수, 바운딩 박스)
fn flood(mask: &[Px], w: i32, h: i32, start: (i32, i32), seen: &mut [bool]) -> (usize, PhysRect) {
    let (mut x0, mut y0, mut x1, mut y1) = (start.0, start.1, start.0, start.1);
    let mut count = 0usize;
    let mut stack = vec![start];
    seen[(start.1 * w + start.0) as usize] = true;
    while let Some((x, y)) = stack.pop() {
        count += 1;
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let i = (ny * w + nx) as usize;
            if !seen[i] && mask[i] == Px::Same {
                seen[i] = true;
                stack.push((nx, ny));
            }
        }
    }
    (count, PhysRect::from_edges(x0, y0, x1 + 1, y1 + 1))
}

/// 씨앗이 없을 때: 가장 큰 같음 덩어리의 한 화소.
fn largest_component_seed(mask: &[Px], w: i32, h: i32) -> Option<(i32, i32)> {
    let mut seen = vec![false; mask.len()];
    let mut best: Option<(usize, (i32, i32))> = None;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if seen[i] || mask[i] != Px::Same {
                continue;
            }
            let (count, _) = flood(mask, w, h, (x, y), &mut seen);
            if best.map_or(true, |(c, _)| count > c) {
                best = Some((count, (x, y)));
            }
        }
    }
    best.map(|b| b.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: u32 = 400;
    const H: u32 = 300;

    /// 어둡지 않은 결이 있는 바탕(경사가 있어 "우연히 같음"이 생기지 않게).
    fn screen() -> Vec<u8> {
        let mut px = vec![0u8; (W * H * 4) as usize];
        for y in 0..H {
            for x in 0..W {
                let o = ((y * W + x) * 4) as usize;
                px[o] = 60 + ((x * 3 + y) % 150) as u8;
                px[o + 1] = 80 + ((x + y * 5) % 120) as u8;
                px[o + 2] = 100 + ((x * 7 + y * 2) % 100) as u8;
                px[o + 3] = 255;
            }
        }
        px
    }

    fn fill(px: &mut [u8], r: PhysRect, c: [u8; 3]) {
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                let o = ((y as u32 * W + x as u32) * 4) as usize;
                px[o..o + 3].copy_from_slice(&c);
            }
        }
    }

    /// Snipaste처럼: 강조 밖을 반쯤 어둡게.
    fn dim_outside(before: &[u8], keep: PhysRect) -> Vec<u8> {
        let mut after = before.to_vec();
        for y in 0..H as i32 {
            for x in 0..W as i32 {
                if keep.contains(ink_domain::geometry::PhysPoint { x, y }) {
                    continue;
                }
                let o = ((y as u32 * W + x as u32) * 4) as usize;
                for c in 0..3 {
                    after[o + c] /= 2;
                }
            }
        }
        after
    }

    const R: PhysRect = PhysRect::new(120, 80, 160, 110);

    #[test]
    fn diff_rect_어두운덮개_밖_강조사각형_추출() {
        let before = screen();
        let after = dim_outside(&before, R);
        assert_eq!(detect_snipaste_highlight(&before, &after, W, H, Some((200, 130))), Some(R));
        assert_eq!(detect_snipaste_highlight(&before, &after, W, H, None), Some(R), "씨앗 없이도 가장 큰 덩어리");
    }

    #[test]
    fn diff_rect_강조_가장자리의_어두운_띠는_넘는다() {
        // 강조된 창: 1 px 회색 테두리 안쪽 위 20 px이 검은 제목 표시줄(모호) — 그래도 창 전체
        let mut before = screen();
        fill(&mut before, R, [60, 60, 60]);
        fill(&mut before, PhysRect::new(R.x + 1, R.y + 1, R.w - 2, 20), [8, 8, 8]);
        fill(&mut before, PhysRect::new(R.x + 1, R.y + 21, R.w - 2, R.h - 22), [200, 210, 220]);
        fill(&mut before, PhysRect::new(R.x + 10, R.y + 8, 30, 3), [240, 240, 240]); // 제목 글자
        let after = dim_outside(&before, R);
        assert_eq!(detect_snipaste_highlight(&before, &after, W, H, Some((200, 150))), Some(R));
    }

    #[test]
    fn diff_rect_테두리_없이_가장자리까지_검으면_같음이_있는_마지막_줄까지() {
        // 한계: 강조 맨 위 줄들이 모두 아주 어두우면 안인지 밖인지 모른다 → 같음이 보이는 줄(글자)까지만.
        // 실제 창은 1 px 테두리가 있어 대부분 위 시험처럼 맞는다.
        let mut before = screen();
        fill(&mut before, PhysRect::new(R.x, R.y, R.w, 20), [8, 8, 8]);
        fill(&mut before, PhysRect::new(R.x + 10, R.y + 2, 30, 3), [240, 240, 240]);
        let after = dim_outside(&before, R);
        assert_eq!(detect_snipaste_highlight(&before, &after, W, H, Some((200, 150))), Some(PhysRect::from_edges(R.x, R.y + 2, R.right(), R.bottom())));
    }

    #[test]
    fn diff_rect_강조_밖_어두운_바탕으로는_새지_않는다() {
        // 강조 왼쪽에 붙은 검은 바탕(모호)이 넓게 있어도 강조 왼쪽 변에서 멈춘다
        let mut before = screen();
        fill(&mut before, PhysRect::new(0, 0, R.x, H as i32), [4, 4, 4]);
        let after = dim_outside(&before, R);
        assert_eq!(detect_snipaste_highlight(&before, &after, W, H, Some((200, 130))), Some(R));
    }

    #[test]
    fn diff_rect_확대경이_밖에_같은_화소를_그려도_씨앗_덩어리만() {
        let before = screen();
        let mut after = dim_outside(&before, R);
        // 강조 오른쪽 아래 밖에 확대경: 테두리(다름) 안에 원본과 같은 화소
        let mag = PhysRect::new(300, 200, 80, 80);
        for y in mag.y..mag.bottom() {
            for x in mag.x..mag.right() {
                let o = ((y as u32 * W + x as u32) * 4) as usize;
                let edge = x == mag.x || y == mag.y || x == mag.right() - 1 || y == mag.bottom() - 1;
                for c in 0..3 {
                    after[o + c] = if edge { 255 } else { before[o + c] };
                }
            }
        }
        assert_eq!(detect_snipaste_highlight(&before, &after, W, H, Some((200, 130))), Some(R));
    }

    #[test]
    fn diff_rect_테두리_선이_안쪽이면_그만큼_작다() {
        // Snipaste 테두리 2 px이 강조 안쪽에 그려지면 결과는 2 px 안쪽(하네스가 inset으로 되돌림)
        let before = screen();
        let mut after = dim_outside(&before, R);
        for y in R.y..R.bottom() {
            for x in R.x..R.right() {
                let inner = x >= R.x + 2 && y >= R.y + 2 && x < R.right() - 2 && y < R.bottom() - 2;
                if !inner {
                    let o = ((y as u32 * W + x as u32) * 4) as usize;
                    after[o..o + 3].copy_from_slice(&[255, 144, 30]);
                }
            }
        }
        assert_eq!(detect_snipaste_highlight(&before, &after, W, H, Some((200, 130))), Some(PhysRect::new(R.x + 2, R.y + 2, R.w - 4, R.h - 4)));
    }

    #[test]
    fn diff_rect_전부_어두워졌으면_없음_안_어두워졌으면_화면_전체() {
        let before = screen();
        let all_dim = dim_outside(&before, PhysRect::new(0, 0, 0, 0));
        assert_eq!(detect_snipaste_highlight(&before, &all_dim, W, H, Some((10, 10))), None);
        assert_eq!(detect_snipaste_highlight(&before, &before, W, H, Some((10, 10))), Some(PhysRect::new(0, 0, W as i32, H as i32)));
        assert_eq!(detect_snipaste_highlight(&before, &before, W, H, Some((-1, 10))), None, "씨앗이 화면 밖");
        assert_eq!(detect_snipaste_highlight(&before[..8], &before, W, H, None), None, "크기가 안 맞음");
    }
}
