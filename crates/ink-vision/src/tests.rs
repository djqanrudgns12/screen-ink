use super::*;
use ink_domain::geometry::{PhysPoint, PhysRect};
use std::time::Instant;

struct Canvas {
    w: u32,
    h: u32,
    bgra: Vec<u8>,
}

impl Canvas {
    fn new(w: u32, h: u32, bg: [u8; 3]) -> Self {
        let mut bgra = Vec::with_capacity((w as usize) * (h as usize) * 4);
        for _ in 0..(w as usize * h as usize) {
            bgra.extend_from_slice(&[bg[0], bg[1], bg[2], 255]);
        }
        Self { w, h, bgra }
    }

    fn set_px(&mut self, x: i32, y: i32, bgr: [u8; 3]) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let off = (y as usize * self.w as usize + x as usize) * 4;
        self.bgra[off] = bgr[0];
        self.bgra[off + 1] = bgr[1];
        self.bgra[off + 2] = bgr[2];
        self.bgra[off + 3] = 255;
    }

    fn blend_px(&mut self, x: i32, y: i32, bgr: [u8; 3], alpha: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let off = (y as usize * self.w as usize + x as usize) * 4;
        let a = alpha.clamp(0.0, 1.0);
        for (c, &ch) in bgr.iter().enumerate() {
            let dst = self.bgra[off + c] as f32;
            let src = ch as f32;
            self.bgra[off + c] = (dst * (1.0 - a) + src * a).round() as u8;
        }
    }

    fn fill_rect(&mut self, r: PhysRect, bgr: [u8; 3]) {
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                self.set_px(x, y, bgr);
            }
        }
    }

    fn stroke_rect(&mut self, r: PhysRect, thickness: i32, bgr: [u8; 3]) {
        for t in 0..thickness {
            for x in r.x..r.right() {
                self.set_px(x, r.y + t, bgr);
                self.set_px(x, r.bottom() - 1 - t, bgr);
            }
            for y in r.y..r.bottom() {
                self.set_px(r.x + t, y, bgr);
                self.set_px(r.right() - 1 - t, y, bgr);
            }
        }
    }

    fn fill_rounded_rect(&mut self, r: PhysRect, radius: i32, bgr: [u8; 3]) {
        let rad = radius.min(r.w / 2).min(r.h / 2) as f32;
        let r2 = rad * rad;
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                let cx = if (x as f32) < r.x as f32 + rad {
                    r.x as f32 + rad - 0.5
                } else if (x as f32) >= r.right() as f32 - rad {
                    r.right() as f32 - rad - 0.5
                } else {
                    x as f32
                };
                let cy = if (y as f32) < r.y as f32 + rad {
                    r.y as f32 + rad - 0.5
                } else if (y as f32) >= r.bottom() as f32 - rad {
                    r.bottom() as f32 - rad - 0.5
                } else {
                    y as f32
                };
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                if dx * dx + dy * dy <= r2 {
                    self.set_px(x, y, bgr);
                }
            }
        }
    }

    fn draw_shadow(&mut self, r: PhysRect, spread: i32, max_alpha: f32) {
        for y in (r.y - spread)..(r.bottom() + spread) {
            for x in (r.x - spread)..(r.right() + spread) {
                if r.contains(PhysPoint { x, y }) {
                    continue;
                }
                let dx = (r.x - x).max(0).max(x - (r.right() - 1)) as f32;
                let dy = (r.y - y).max(0).max(y - (r.bottom() - 1)) as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= spread as f32 {
                    let t = 1.0 - dist / (spread as f32 + 1.0);
                    self.blend_px(x, y, [0, 0, 0], max_alpha * t * t);
                }
            }
        }
    }

    fn draw_table(&mut self, origin: (i32, i32), grid: (i32, i32), cell: (i32, i32), border: [u8; 3]) -> PhysRect {
        let (x, y) = origin;
        let (cols, rows) = grid;
        let (cw, ch) = cell;
        let tw = cols * cw + 1;
        let th = rows * ch + 1;
        let table = PhysRect::new(x, y, tw, th);
        // 가로선
        for r in 0..=rows {
            let ly = y + r * ch;
            for px in x..(x + tw) {
                self.set_px(px, ly, border);
            }
        }
        // 세로선
        for c in 0..=cols {
            let lx = x + c * cw;
            for py in y..(y + th) {
                self.set_px(lx, py, border);
            }
        }
        table
    }
}

fn near_rect(a: PhysRect, b: PhysRect, tol: i32) -> bool {
    (a.x - b.x).abs() <= tol
        && (a.y - b.y).abs() <= tol
        && (a.right() - b.right()).abs() <= tol
        && (a.bottom() - b.bottom()).abs() <= tol
}

fn assert_invariants(cands: &[VCandidate], x: i32, y: i32) {
    let p = PhysPoint { x, y };
    for c in cands {
        assert!(c.rect.contains(p), "후보 {:?}가 커서 ({},{})를 담지 않음", c.rect, x, y);
        assert!(c.support >= MIN_SUPPORT, "후보 {:?} 지지율 {:.3} < {}", c.rect, c.support, MIN_SUPPORT);
    }
    for w in cands.windows(2) {
        assert!(w[0].rect.area() <= w[1].rect.area(), "후보가 작은 것→큰 것 순이 아님: {:?}", cands);
    }
}

#[test]
fn 빈_화면은_후보_0() {
    let c = Canvas::new(640, 480, [240, 240, 240]);
    let p = prepare(&c.bgra, c.w, c.h);
    let q = p.query(320, 240);
    assert!(q.is_empty(), "빈 화면에서 후보가 나옴: {:?}", q);
    assert_eq!(p.support(PhysRect::new(100, 100, 200, 100)), 0.0);
}

#[test]
fn 저대비_패널은_모니터_끝까지_인식하고_잘린_전처리_끝은_제외한다() {
    let mut c = Canvas::new(640, 480, [248; 3]);
    let panel = PhysRect::new(0, 32, 192, 380);
    c.fill_rect(panel, [235; 3]);
    c.fill_rect(PhysRect::new(16, 80, 160, 48), [200; 3]);
    let p = prepare(&c.bgra, c.w, c.h);
    assert!(p.query(96, 270).iter().any(|v| v.rect == panel));
    let roi = prepare_roi(&c.bgra, c.w, c.h, 118, 265, 120);
    assert!(roi.query(96, 270).iter().all(|v| v.kind != VKind::Flat));
}

#[test]
fn 평평한_바탕_위_버튼() {
    let mut c = Canvas::new(800, 600, [245, 245, 245]);
    let btn = PhysRect::new(200, 180, 140, 44);
    c.fill_rect(btn, [220, 120, 40]); // 파란 버튼(BGRA)
    let p = prepare(&c.bgra, c.w, c.h);
    let q = p.query(260, 200);
    assert_invariants(&q, 260, 200);
    assert!(
        q.iter().any(|cand| near_rect(cand.rect, btn, 1)),
        "버튼 {:?}을 못 찾음: {:?}",
        btn,
        q
    );
    assert!((p.support(btn) - 1.0).abs() < 0.01, "버튼 지지율 {:.3}", p.support(btn));
}

#[test]
fn 표_칸에서_행과_표까지_사다리() {
    let mut c = Canvas::new(900, 600, [255, 255, 255]);
    // 3열 × 4행 표: 칸 180×48
    let table = c.draw_table((100, 80), (3, 4), (180, 48), [120, 120, 120]);
    let cell = PhysRect::new(100, 80 + 48, 181, 49);
    let row = PhysRect::new(100, 80 + 48, 3 * 180 + 1, 49);

    let p = prepare(&c.bgra, c.w, c.h);
    let (cx, cy) = (190, 80 + 48 + 24);
    let q = p.query(cx, cy);
    assert_invariants(&q, cx, cy);

    let i_cell = q.iter().position(|x| near_rect(x.rect, cell, 1));
    let i_row = q.iter().position(|x| near_rect(x.rect, row, 1));
    let i_table = q.iter().position(|x| near_rect(x.rect, table, 1));
    assert!(i_cell.is_some(), "표 칸 {:?} 누락: {:?}", cell, q);
    assert!(i_row.is_some(), "표 행 {:?} 누락: {:?}", row, q);
    assert!(i_table.is_some(), "표 전체 {:?} 누락: {:?}", table, q);
    assert!(i_cell.unwrap() < i_row.unwrap() && i_row.unwrap() < i_table.unwrap(), "순서가 칸→행→표가 아님: {:?}", q);
}

#[test]
fn 카드_그림자는_실제_카드_경계로_잡힌다() {
    let mut c = Canvas::new(800, 600, [242, 242, 242]);
    let card = PhysRect::new(180, 120, 360, 240);
    // 6 px 부드러운 그림자 + 카드 본체 + 1 px 연한 테두리
    c.draw_shadow(card, 6, 0.25);
    c.fill_rect(card, [255, 255, 255]);
    c.stroke_rect(card, 1, [200, 200, 200]);

    let p = prepare(&c.bgra, c.w, c.h);
    let q = p.query(350, 240);
    assert_invariants(&q, 350, 240);
    assert!(
        q.iter().any(|cand| near_rect(cand.rect, card, 1)),
        "그림자 때문에 카드 경계가 밀림 — 기대 {:?}, 결과 {:?}",
        card,
        q
    );
}

#[test]
fn 둥근_모서리_상자() {
    let mut c = Canvas::new(800, 600, [245, 245, 245]);
    let rbox = PhysRect::new(160, 150, 200, 60);
    c.fill_rounded_rect(rbox, 10, [70, 90, 200]);

    let p = prepare(&c.bgra, c.w, c.h);
    // 중앙 커서 + 모서리 근처 커서 모두 확인
    for (cx, cy) in [(260, 180), (168, 180), (260, 156)] {
        let q = p.query(cx, cy);
        assert_invariants(&q, cx, cy);
        assert!(
            q.iter().any(|cand| near_rect(cand.rect, rbox, 2)),
            "둥근 상자 {:?} 누락 (커서 {},{}): {:?}",
            rbox,
            cx,
            cy,
            q
        );
    }
}

#[test]
fn 그라데이션과_잡음_위_상자() {
    let mut c = Canvas::new(800, 600, [0, 0, 0]);
    // 가로 그라데이션 + 결정적 의사난수 잡음(±8)
    let mut seed: u32 = 0x1234_5678;
    for y in 0..c.h as i32 {
        for x in 0..c.w as i32 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let n = ((seed >> 24) as i32 % 17) - 8; // -8..+8
            let base = 60 + (x * 120) / c.w as i32;
            let v = (base + n).clamp(0, 255) as u8;
            c.set_px(x, y, [v, v, v]);
        }
    }
    let box_rect = PhysRect::new(220, 160, 260, 160);
    c.fill_rect(box_rect, [245, 245, 245]);
    c.stroke_rect(box_rect, 1, [40, 40, 40]);

    let p = prepare(&c.bgra, c.w, c.h);
    let q = p.query(350, 240);
    assert_invariants(&q, 350, 240);
    assert!(
        q.iter().any(|cand| near_rect(cand.rect, box_rect, 1)),
        "그라데이션·잡음 위 상자 {:?} 누락: {:?}",
        box_rect,
        q
    );
    // 잡음 바탕의 임의 사각형은 지지율이 낮아야 함
    let rand_sup = p.support(PhysRect::new(20, 20, 150, 100));
    assert!(rand_sup < 0.15, "잡음 바탕 임의 사각형 지지율이 너무 높음: {:.3}", rand_sup);
}

#[test]
fn 어긋난_1px_사각형의_snap() {
    let mut c = Canvas::new(640, 480, [250, 250, 250]);
    let exact = PhysRect::new(120, 100, 240, 120);
    c.fill_rect(exact, [60, 120, 180]);

    let p = prepare(&c.bgra, c.w, c.h);
    let shifted = PhysRect::new(121, 99, 238, 122);
    let snapped = p.snap(shifted, 3);
    assert_eq!(snapped, exact, "1 px 어긋난 채운 상자가 정확히 붙지 않음");

    // 1 px 테두리 상자에서도 확인
    let mut c2 = Canvas::new(640, 480, [255, 255, 255]);
    c2.stroke_rect(exact, 1, [80, 80, 80]);
    let p2 = prepare(&c2.bgra, c2.w, c2.h);
    let shifted2 = PhysRect::new(121, 101, 240, 120);
    let snapped2 = p2.snap(shifted2, 3);
    assert_eq!(snapped2, exact, "1 px 어긋난 테두리 상자가 정확히 붙지 않음");
}

#[test]
fn 커서를_안_담는_후보는_없고_경계_지지율_대조() {
    let mut c = Canvas::new(800, 600, [240, 240, 240]);
    let a = PhysRect::new(100, 100, 160, 80);
    let b = PhysRect::new(400, 300, 200, 120);
    c.fill_rect(a, [50, 50, 50]);
    c.fill_rect(b, [80, 160, 80]);

    let p = prepare(&c.bgra, c.w, c.h);
    // 상자 a 지지율 ≈ 1, 임의 사각형 ≈ 0
    assert!(p.support(a) > 0.98, "경계 사각형 지지율 {:.3}", p.support(a));
    assert!(p.support(b) > 0.98, "경계 사각형 지지율 {:.3}", p.support(b));
    assert!(p.support(PhysRect::new(280, 200, 100, 80)) < 0.05, "임의 사각형 지지율");

    // 여러 커서 위치에서 불변식 검사(빈 곳, 상자 안, 이미지 밖)
    for (x, y) in [(150, 140), (450, 350), (300, 250), (10, 10), (-5, 50), (900, 200)] {
        let q = p.query(x, y);
        assert_invariants(&q, x, y);
    }
    // 빈 공간(300, 250)은 a나 b를 담지 않으므로 후보 없음
    assert!(p.query(300, 250).is_empty());
}

#[test]
fn prepare_roi는_원본_좌표로_동작하고_자석_경계를_돌려준다() {
    let mut c = Canvas::new(1920, 1080, [240, 240, 240]);
    let box_r = PhysRect::new(1100, 600, 200, 80);
    c.fill_rect(box_r, [40, 90, 180]);

    let roi = prepare_roi(&c.bgra, c.w, c.h, 1200, 640, 300);
    let q = roi.query(1200, 640);
    assert_invariants(&q, 1200, 640);
    assert!(q.iter().any(|cand| near_rect(cand.rect, box_r, 1)), "ROI에서 상자 누락: {:?}", q);
    assert!(roi.support(box_r) > 0.98);
    assert_eq!(roi.snap(PhysRect::new(1101, 599, 198, 82), 3), box_r);

    let (xs, ys) = roi.edges_near(1104, 603, 10);
    assert!(xs.contains(&1100), "근처 세로 경계 1100 누락: {:?}", xs);
    assert!(ys.contains(&600), "근처 가로 경계 600 누락: {:?}", ys);
}

#[test]
fn 작은_평탄_버튼은_flat으로_잡힌다() {
    let mut c = Canvas::new(400, 300, [245, 245, 245]);
    // 18×18 작은 버튼(곧은 경계 24 px 미만이라 Box에는 없고 Flat으로만 잡힘)
    let small = PhysRect::new(120, 100, 18, 18);
    c.fill_rect(small, [60, 140, 220]);

    let p = prepare(&c.bgra, c.w, c.h);
    let q = p.query(128, 108);
    assert_invariants(&q, 128, 108);
    assert!(
        q.iter().any(|cand| cand.kind == VKind::Flat && cand.rect == small),
        "작은 평탄 버튼 {:?}이 Flat으로 잡히지 않음: {:?}",
        small,
        q
    );
}

#[test]
fn 안티에일리어싱과_등휘도_색_버튼() {
    let mut c = Canvas::new(400, 300, [250, 250, 250]);
    // 대비 34(단일 반단계 17 < 24이지만 1 px AA 전환 합 34 >= 24)
    let aa_box = PhysRect::new(60, 60, 120, 60);
    c.fill_rect(aa_box, [216, 216, 216]);
    for x in aa_box.x..aa_box.right() {
        c.set_px(x, aa_box.y, [233, 233, 233]);
        c.set_px(x, aa_box.bottom() - 1, [233, 233, 233]);
    }
    for y in aa_box.y..aa_box.bottom() {
        c.set_px(aa_box.x, y, [233, 233, 233]);
        c.set_px(aa_box.right() - 1, y, [233, 233, 233]);
    }

    let p = prepare(&c.bgra, c.w, c.h);
    let q = p.query(120, 90);
    assert_invariants(&q, 120, 90);
    assert!(q.iter().any(|cand| near_rect(cand.rect, aa_box, 2)), "AA 상자 누락: {:?}", q);

    // 등휘도 색 버튼: 회색(120,120,120) 바탕 위 파란 버튼(220,120,40, Y≈117)
    let mut c2 = Canvas::new(400, 300, [120, 120, 120]);
    let iso = PhysRect::new(100, 100, 140, 50);
    c2.fill_rect(iso, [220, 120, 40]);
    let p2 = prepare(&c2.bgra, c2.w, c2.h);
    let q2 = p2.query(170, 125);
    assert!(q2.iter().any(|cand| near_rect(cand.rect, iso, 1)), "등휘도 색 버튼 누락: {:?}", q2);
}

fn make_realistic_ui(w: u32, h: u32) -> Canvas {
    let mut c = Canvas::new(w, h, [242, 244, 248]);
    let s = (w / 1920).max(1) as i32;
    // 사이드바 + 상단 바 + 카드 + 표 + 버튼 여러 개
    c.fill_rect(PhysRect::new(0, 0, 260 * s, h as i32), [230, 233, 240]);
    c.fill_rect(PhysRect::new(260 * s, 0, w as i32 - 260 * s, 56 * s), [255, 255, 255]);
    let card = PhysRect::new(300 * s, 90 * s, 1200 * s, 720 * s);
    c.draw_shadow(card, 6 * s, 0.2);
    c.fill_rect(card, [255, 255, 255]);
    c.stroke_rect(card, 1, [200, 204, 212]);
    c.draw_table((340 * s, 140 * s), (5, 8), (200 * s, 52 * s), [175, 180, 190]);
    for i in 0..5 {
        c.fill_rounded_rect(PhysRect::new(340 * s + i * 160 * s, 620 * s, 130 * s, 44 * s), 8 * s, [215, 115, 45]);
    }
    c
}

#[test]
fn 성능_측정_1080p와_4k() {
    for (label, w, h, max_prep_ms) in [("1080p", 1920u32, 1080u32, 250.0f64), ("4K", 3840u32, 2160u32, 800.0f64)] {
        let c = make_realistic_ui(w, h);
        let t0 = Instant::now();
        let p = prepare(&c.bgra, w, h);
        let prep_ms = t0.elapsed().as_secs_f64() * 1000.0;

        let s = (w / 1920).max(1) as i32;
        let qx = 420 * s;
        let qy = 220 * s;
        let t1 = Instant::now();
        let res = p.query(qx, qy);
        let query_ms = t1.elapsed().as_secs_f64() * 1000.0;

        let t2 = Instant::now();
        let roi = prepare_roi(&c.bgra, w, h, qx as u32, qy as u32, 512);
        let roi_ms = t2.elapsed().as_secs_f64() * 1000.0;
        let roi_res = roi.query(qx, qy);

        eprintln!(
            "[ink-vision bench] {label} ({w}x{h}): prepare={prep_ms:.2} ms, prepare_roi(1024^2)={roi_ms:.2} ms, query={query_ms:.3} ms (cands={}, roi_cands={})",
            res.len(),
            roi_res.len()
        );
        assert!(!res.is_empty(), "{label} 표 위에서 후보가 비어 있음");
        assert!(!roi_res.is_empty(), "{label} ROI 표 위에서 후보가 비어 있음");
        assert_invariants(&res, qx, qy);
        assert!(prep_ms < max_prep_ms, "{label} prepare 너무 느림: {prep_ms:.2} ms");
        assert!(query_ms < 50.0, "{label} query 너무 느림: {query_ms:.3} ms");
    }
}

#[test]
fn 공유_원본의_반복_행을_건너뛰어도_경계와_후보가_같다() {
    use std::sync::Arc;
    let c = make_realistic_ui(640, 480);
    let pixels = Arc::new(c.bgra);
    let shared = prepare_shared(pixels.clone(), c.w, c.h);
    assert!(Arc::ptr_eq(&pixels, &shared.data.bgra));
    // 알파만 줄마다 바꾸면 경계 판정의 색은 같지만 모든 줄을 개별 검사한다.
    let mut distinct = pixels.as_ref().clone();
    for (y, row) in distinct.chunks_exact_mut(c.w as usize * 4).enumerate() {
        row[3] = y as u8;
    }
    let reference = prepare(&distinct, c.w, c.h);
    assert_eq!(shared.data.h_prefix, reference.data.h_prefix);
    assert_eq!(shared.data.v_prefix, reference.data.v_prefix);
    assert_eq!(shared.data.h_segs, reference.data.h_segs);
    assert_eq!(shared.data.v_segs, reference.data.v_segs);
    for (x, y) in [(0, 0), (180, 90), (440, 125), (639, 479)] {
        assert_eq!(shared.query(x, y), reference.query(x, y));
    }
}

#[test]
fn 병렬_전처리는_단일_스레드와_완전히_동일하다() {
    let c = make_realistic_ui(640, 480);
    let seq = prep::build_roi_with_threads(&c.bgra, (c.w, c.h), (0, 0), (c.w, c.h), 1);
    let par = prep::build_roi_with_threads(&c.bgra, (c.w, c.h), (0, 0), (c.w, c.h), 4);
    assert_eq!(seq.h_prefix, par.h_prefix, "h_prefix 불일치");
    assert_eq!(seq.v_prefix, par.v_prefix, "v_prefix 불일치");
    assert_eq!(seq.h_segs, par.h_segs, "h_segs 불일치");
    assert_eq!(seq.h_row_offsets, par.h_row_offsets, "h_row_offsets 불일치");
    assert_eq!(seq.v_segs, par.v_segs, "v_segs 불일치");
    assert_eq!(seq.v_col_offsets, par.v_col_offsets, "v_col_offsets 불일치");
}

#[test]
fn 가림_창_경계_2px_v_후보와_snap_제외() {
    let mut c = Canvas::new(800, 600, [245, 245, 245]);
    // 가린 창(0..800 × 0..160) 바로 아래 2 px에 연한 윗변 사진
    let photo = PhysRect::new(160, 162, 320, 220);
    c.fill_rect(photo, [232, 232, 232]); // 배경 245와 차이 13 < 24라 윗변 경계 없음
    c.fill_rect(PhysRect::new(160, 170, 320, 212), [80, 120, 170]);
    let occ = [PhysRect::new(0, 0, 800, 160)];
    c.fill_rect(occ[0], [45, 50, 60]);

    let p = prepare(&c.bgra, c.w, c.h);
    // 일반 snap은 강한 앞 창 모서리 y=160으로 끌리지만, snap_with_occluders는 y=162를 지킨다
    assert_eq!(p.snap(photo, 3).y, 160);
    assert_eq!(p.snap_with_occluders(photo, 3, &occ), photo);

    // query_with_occluders는 앞 창 경계 ±2 px에 붙은 후보를 내지 않는다
    let q = p.query_with_occluders(320, 270, &occ);
    assert!(!q.iter().any(|cand| cand.rect.y <= 162 && cand.rect.bottom() >= 160), "앞 창 경계 후보 금지: {:?}", q);
    assert!(p.support_with_occluders(PhysRect::new(160, 160, 320, 222), &occ) < p.support(PhysRect::new(160, 160, 320, 222)));
}

#[test]
fn 평탄도_이미지_경계_재측정_여백_그림자_둥근모서리_3종() {
    // 1. 여백 4면
    let mut c1 = Canvas::new(640, 480, [255, 255, 255]);
    let elem1 = PhysRect::new(80, 80, 320, 220);
    let inner1 = PhysRect::new(110, 105, 260, 170);
    c1.fill_rect(inner1, [238, 235, 230]);
    let p1 = prepare(&c1.bgra, c1.w, c1.h);
    assert_eq!(p1.refine_image(elem1), inner1);

    // 2. 부드러운 그림자 + 여백
    let mut c2 = Canvas::new(640, 480, [246, 246, 246]);
    let elem2 = PhysRect::new(100, 80, 340, 240);
    let inner2 = PhysRect::new(125, 105, 290, 190);
    c2.draw_shadow(inner2, 14, 0.14);
    c2.fill_rect(inner2, [234, 236, 238]);
    let p2 = prepare(&c2.bgra, c2.w, c2.h);
    assert_eq!(p2.refine_image(elem2), inner2);

    // 3. 둥근 모서리 + 여백
    let mut c3 = Canvas::new(640, 480, [248, 248, 248]);
    let elem3 = PhysRect::new(100, 100, 280, 200);
    let inner3 = PhysRect::new(124, 120, 232, 160);
    c3.fill_rounded_rect(inner3, 20, [225, 210, 200]);
    let p3 = prepare(&c3.bgra, c3.w, c3.h);
    assert_eq!(p3.refine_image(elem3), inner3);

    // 4. 여백 없는 둥근 모서리 그림 & 상단 하늘 사진은 줄이지 않음(비오탐)
    let mut c4 = Canvas::new(640, 480, [245, 245, 245]);
    let tight = PhysRect::new(120, 100, 240, 160);
    c4.fill_rounded_rect(tight, 18, [120, 155, 205]);
    let p4 = prepare(&c4.bgra, c4.w, c4.h);
    assert_eq!(p4.refine_image(tight), tight);
}

#[test]
fn 앞_창에_맞닿은_요소는_앞_창_안쪽_3px_경계로_끌리지_않는다() {
    // 가로 방향: 앞 창 아래 y=160에 맞닿은 요소가 앞 창 안쪽 3 px(y=157) 경계로 끌려 들어가면 안 된다
    let mut c = Canvas::new(800, 600, [245, 245, 245]);
    let occ = [PhysRect::new(0, 0, 800, 160)];
    c.fill_rect(occ[0], [50, 55, 65]);
    c.fill_rect(PhysRect::new(0, 157, 800, 3), [220, 220, 220]);
    let photo = PhysRect::new(160, 160, 320, 220);
    c.fill_rect(PhysRect::new(160, 160, 320, 10), [235, 235, 235]);
    c.fill_rect(PhysRect::new(160, 170, 320, 210), [80, 120, 170]);

    let p = prepare(&c.bgra, c.w, c.h);
    let snapped = p.snap_with_occluders(photo, 3, &occ);
    assert_eq!(snapped.y, 160, "앞 창 안쪽 3 px 경계로 끌려 들어가면 안 된다: {snapped:?}");
    assert_eq!(snapped.intersect(&occ[0]).map_or(0, |i| i.area()), 0);

    // 세로 방향: 왼쪽 앞 창 x=160에 맞닿은 요소가 앞 창 안쪽 3 px(x=157) 경계로 끌려 들어가면 안 된다
    let mut c_l = Canvas::new(800, 600, [245, 245, 245]);
    let occ_l = [PhysRect::new(0, 0, 160, 600)];
    c_l.fill_rect(occ_l[0], [50, 55, 65]);
    c_l.fill_rect(PhysRect::new(157, 0, 3, 600), [220, 220, 220]);
    let photo_l = PhysRect::new(160, 140, 320, 220);
    c_l.fill_rect(PhysRect::new(160, 140, 10, 220), [235, 235, 235]);
    c_l.fill_rect(PhysRect::new(170, 140, 310, 220), [80, 120, 170]);

    let p_l = prepare(&c_l.bgra, c_l.w, c_l.h);
    let snapped_l = p_l.snap_with_occluders(photo_l, 3, &occ_l);
    assert_eq!(snapped_l.x, 160, "왼쪽 앞 창 안쪽 3 px 경계로 끌려 들어가면 안 된다: {snapped_l:?}");
    assert_eq!(snapped_l.intersect(&occ_l[0]).map_or(0, |i| i.area()), 0);
}

#[test]
fn 앞_창에_가린_이미지_여백_재측정은_잘린_변만_건너뛰고_나머지_세_변을_다시_잰다() {
    // (a) 위 15% 가림 + 4면 흰 여백 + 둥근 모서리 + 앞 창 2 px 경계·10 px 그림자(W5 실기 치수)
    let mut ca = Canvas::new(900, 600, [238, 238, 238]);
    let elem_a = PhysRect::new(175, 160, 540, 319);
    ca.fill_rect(elem_a, [255, 255, 255]);
    let inner_a = PhysRect::new(216, 200, 459, 239);
    ca.fill_rounded_rect(inner_a, 24, [150, 175, 210]);
    let occ_a = [PhysRect::new(0, 0, 900, 236)];
    ca.draw_shadow(occ_a[0], 10, 0.25);
    ca.fill_rect(occ_a[0], [50, 55, 65]);
    ca.fill_rect(PhysRect::new(0, 236, 900, 2), [190, 190, 190]);
    let pa = prepare(&ca.bgra, ca.w, ca.h);
    let vis_a = PhysRect::new(175, 236, 540, 243);
    assert_eq!(pa.refine_image_with_occluders(vis_a, &occ_a), PhysRect::new(216, 236, 459, 203));

    // (b) 왼쪽 위 L자 가림 + 4면 여백 → 우·하 여백만 걷어내고 좌·상 잘린 경계 유지
    let mut cb = Canvas::new(800, 600, [238, 238, 238]);
    let elem_b = PhysRect::new(120, 100, 400, 300);
    cb.fill_rect(elem_b, [255, 255, 255]);
    let inner_b = PhysRect::new(150, 130, 340, 240);
    cb.fill_rect(inner_b, [120, 160, 200]);
    let occ_b = [PhysRect::new(0, 0, 200, 180)];
    cb.fill_rect(occ_b[0], [45, 50, 60]);
    cb.fill_rect(PhysRect::new(0, 180, 202, 2), [185, 185, 185]);
    cb.fill_rect(PhysRect::new(200, 0, 2, 182), [185, 185, 185]);
    let pb = prepare(&cb.bgra, cb.w, cb.h);
    // 왼쪽 띠 또는 아래 띠 중 커서를 담는 아래 띠(120, 180, 400, 220): 위쪽 일부만 가린 L자 아래 띠
    let vis_b = PhysRect::new(120, 180, 400, 220);
    assert_eq!(pb.refine_image_with_occluders(vis_b, &occ_b), PhysRect::new(150, 180, 340, 190));

    // (c) 앞 창이 윗여백 일부만 가리고 그림 위 여백이 남은 경우 → 남은 윗여백까지 걷어냄
    let mut cc = Canvas::new(800, 600, [238, 238, 238]);
    let elem_c = PhysRect::new(120, 100, 400, 300);
    cc.fill_rect(elem_c, [255, 255, 255]);
    let inner_c = PhysRect::new(160, 160, 320, 190);
    cc.fill_rect(inner_c, [110, 150, 195]);
    let occ_c = [PhysRect::new(0, 0, 800, 125)];
    cc.fill_rect(occ_c[0], [48, 52, 60]);
    cc.fill_rect(PhysRect::new(0, 125, 800, 2), [190, 190, 190]);
    let pc = prepare(&cc.bgra, cc.w, cc.h);
    let vis_c = PhysRect::new(120, 125, 400, 275);
    assert_eq!(pc.refine_image_with_occluders(vis_c, &occ_c), inner_c);

    // (d) 위 15% 가림 + 여백 없는 하늘 사진 → 비오탐(원래 잘린 사각형 유지)
    let mut cd = Canvas::new(800, 600, [255, 255, 255]);
    let photo_d = PhysRect::new(120, 100, 400, 300);
    for y in photo_d.y..photo_d.bottom() {
        let t = ((y - (photo_d.y + 90)) as f32 / 120.0).clamp(0.0, 1.0);
        let b = (230.0 * (1.0 - t) + 70.0 * t).round() as u8;
        let g = (180.0 * (1.0 - t) + 155.0 * t).round() as u8;
        let r = (120.0 * (1.0 - t) + 80.0 * t).round() as u8;
        for x in photo_d.x..photo_d.right() {
            cd.set_px(x, y, [b, g, r]);
        }
    }
    let occ_d = [PhysRect::new(0, 0, 800, 145)];
    cd.fill_rect(occ_d[0], [48, 52, 60]);
    cd.fill_rect(PhysRect::new(0, 145, 800, 2), [190, 190, 190]);
    let pd = prepare(&cd.bgra, cd.w, cd.h);
    let vis_d = PhysRect::new(120, 145, 400, 255);
    assert_eq!(pd.refine_image_with_occluders(vis_d, &occ_d), vis_d);
}
