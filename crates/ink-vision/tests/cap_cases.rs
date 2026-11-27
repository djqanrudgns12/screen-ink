//! R2-CAP-5: 스마트 캡처 합성 사례 14종(BGRA 버퍼 코드 생성, 실제 화면 금지).
//! 가림·여백·그림자·둥근 모서리·스크롤 목록·겹친 창·사다리 중복을 검증한다.

use ink_domain::geometry::{PhysPoint, PhysRect};
use ink_domain::recog::{
    build_chain, build_chain_with, first_level, iou, touches_occluder_edge, Control, ElementCand,
    Evidence, Node, NodeKind, RecogOptions, VisionCand, VisionKind,
};
use ink_vision::{prepare, VCandidate, VKind};

const MON: PhysRect = PhysRect::new(0, 0, 1280, 800);

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

    fn in_rounded_rect(r: PhysRect, radius: i32, x: i32, y: i32) -> bool {
        let rad = radius.min(r.w / 2).min(r.h / 2) as f32;
        let r2 = rad * rad;
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
        dx * dx + dy * dy <= r2
    }

    fn fill_rounded_rect(&mut self, r: PhysRect, radius: i32, bgr: [u8; 3]) {
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                if Self::in_rounded_rect(r, radius, x, y) {
                    self.set_px(x, y, bgr);
                }
            }
        }
    }

    /// 균일 단색이 아닌 합성 사진 무늬(그라데이션 + 색 밴드).
    /// 가장자리 대비가 `GRAD_THRESHOLD`(24)보다 작도록 부드럽게 시작해 안쪽으로 색이 깊어지는 사진도 표현할 수 있다.
    fn fill_photo_pattern(&mut self, r: PhysRect, radius: i32, base: [u8; 3], contrast: i32) {
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                if radius > 0 && !Self::in_rounded_rect(r, radius, x, y) {
                    continue;
                }
                let rx = (x - r.x) as f32 / (r.w.max(1) as f32);
                let ry = (y - r.y) as f32 / (r.h.max(1) as f32);
                let wave = (((x - r.x) / 16 + (y - r.y) / 16) % 2) * 4;
                let db = (rx * contrast as f32) as i32 - wave;
                let dg = (ry * contrast as f32) as i32 + wave;
                let dr = (((rx + ry) * 0.5) * contrast as f32) as i32;
                let b = (base[0] as i32 + db).clamp(0, 255) as u8;
                let g = (base[1] as i32 + dg).clamp(0, 255) as u8;
                let red = (base[2] as i32 + dr).clamp(0, 255) as u8;
                self.set_px(x, y, [b, g, red]);
            }
        }
    }

    /// 바깥쪽은 바탕과 색차 `> FLAT_TOLERANCE`(6)이되 `< GRAD_THRESHOLD`(24)로 부드럽게 시작해
    /// 중심부로 갈수록 풍부해지는 일러스트/사진(접근성 Image 요소 안 여백·그림자 재측정용).
    fn fill_soft_edge_photo(&mut self, r: PhysRect, radius: i32, edge_bgr: [u8; 3], center_delta: [i32; 3]) {
        let cx = r.x as f32 + r.w as f32 * 0.5;
        let cy = r.y as f32 + r.h as f32 * 0.5;
        let rx_max = (r.w as f32 * 0.5).max(1.0);
        let ry_max = (r.h as f32 * 0.5).max(1.0);
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                if radius > 0 && !Self::in_rounded_rect(r, radius, x, y) {
                    continue;
                }
                let dx = ((x as f32 - cx).abs() / rx_max).min(1.0);
                let dy = ((y as f32 - cy).abs() / ry_max).min(1.0);
                // 가장자리(t=0) → 중심(t=1)으로 부드럽게 변함
                let edge_dist = (1.0 - dx).min(1.0 - dy).clamp(0.0, 1.0);
                let t = (edge_dist * 3.0).min(1.0);
                let b = (edge_bgr[0] as f32 + center_delta[0] as f32 * t).clamp(0.0, 255.0) as u8;
                let g = (edge_bgr[1] as f32 + center_delta[1] as f32 * t).clamp(0.0, 255.0) as u8;
                let red = (edge_bgr[2] as f32 + center_delta[2] as f32 * t).clamp(0.0, 255.0) as u8;
                self.set_px(x, y, [b, g, red]);
            }
        }
    }

    fn draw_shadow(&mut self, r: PhysRect, spread: i32, max_alpha: f32) {
        self.draw_offset_shadow(r, (0, 0), spread, max_alpha);
    }

    fn draw_offset_shadow(
        &mut self,
        r: PhysRect,
        offset: (i32, i32),
        spread: i32,
        max_alpha: f32,
    ) {
        let sr = PhysRect::new(r.x + offset.0, r.y + offset.1, r.w, r.h);
        for y in (sr.y - spread)..(sr.bottom() + spread) {
            for x in (sr.x - spread)..(sr.right() + spread) {
                if r.contains(PhysPoint { x, y }) {
                    continue;
                }
                let dx = (sr.x - x).max(0).max(x - (sr.right() - 1)) as f32;
                let dy = (sr.y - y).max(0).max(y - (sr.bottom() - 1)) as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= spread as f32 {
                    let t = 1.0 - dist / (spread as f32 + 1.0);
                    self.blend_px(x, y, [0, 0, 0], max_alpha * t * t);
                }
            }
        }
    }
}

struct CaseSpec {
    id: &'static str,
    category: &'static str,
    canvas: Canvas,
    point: PhysPoint,
    window: Option<PhysRect>,
    children: Vec<PhysRect>,
    elements: Vec<ElementCand>,
    occluders: Vec<PhysRect>,
    truth: PhysRect,
    /// 가림 경계 ±2 px 오염을 검사할지 여부
    check_occluder_edge: bool,
}

fn to_vision_cands(vc: Vec<VCandidate>) -> Vec<VisionCand> {
    vc.into_iter()
        .map(|c| VisionCand {
            rect: c.rect,
            kind: match c.kind {
                VKind::Box => VisionKind::Box,
                VKind::Flat => VisionKind::Flat,
            },
            support: c.support,
        })
        .collect()
}

fn count_dup_pairs(chain: &[Node]) -> usize {
    let nodes: Vec<&Node> = chain
        .iter()
        .filter(|n| !matches!(n.kind, NodeKind::Window | NodeKind::Monitor | NodeKind::Virtual))
        .collect();
    let mut dups = 0;
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            if iou(&nodes[i].rect, &nodes[j].rect) >= 0.90 {
                dups += 1;
            }
        }
    }
    dups
}

fn touches_occ_edge(r: PhysRect, occs: &[PhysRect]) -> bool {
    touches_occluder_edge(r, occs)
}

fn build_cases() -> Vec<CaseSpec> {
    let mut cases = Vec::new();

    // 1. 위 15% 가린 사진 + 앞 창 아래 2px 그림자 테두리
    {
        let mut c = Canvas::new(1280, 800, [245, 245, 245]);
        let win = PhysRect::new(100, 80, 1000, 620);
        c.fill_rect(win, [252, 252, 252]);
        let photo = PhysRect::new(200, 160, 400, 300);
        c.fill_photo_pattern(photo, 0, [140, 170, 200], 35);
        let occ = PhysRect::new(0, 0, 1200, 205);
        c.fill_rect(occ, [50, 55, 65]);
        // 앞 창 바로 아래 2px 그림자
        c.fill_rect(PhysRect::new(0, 205, 1200, 2), [190, 190, 190]);
        cases.push(CaseSpec {
            id: "cap01_top15_occluded_photo",
            category: "가림(위 15%)",
            canvas: c,
            point: PhysPoint { x: 400, y: 350 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: photo, control: Control::Image, from_msaa: false }],
            occluders: vec![occ],
            truth: PhysRect::new(200, 205, 400, 255),
            check_occluder_edge: true,
        });
    }

    // 2. 앞 창 경계 ±2 px 바로 아래 놓인 연한 윗변 사진
    {
        let mut c = Canvas::new(1280, 800, [248, 248, 248]);
        let win = PhysRect::new(100, 80, 1000, 620);
        c.fill_rect(win, [248, 248, 248]);
        let photo = PhysRect::new(200, 202, 400, 260);
        // 윗변 대비를 약하게(248 vs 232, 차이 16 < 24) 두고 나머지 3면은 선명하게
        c.fill_photo_pattern(photo, 0, [110, 160, 200], 45);
        c.fill_rect(PhysRect::new(200, 202, 400, 4), [232, 232, 232]);
        let occ = PhysRect::new(0, 0, 1200, 200);
        c.fill_rect(occ, [45, 50, 60]);
        cases.push(CaseSpec {
            id: "cap02_photo_near_occluder_2px",
            category: "가림 경계 ±2px",
            canvas: c,
            point: PhysPoint { x: 400, y: 330 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: photo, control: Control::Image, from_msaa: false }],
            occluders: vec![occ],
            truth: photo,
            check_occluder_edge: false, // 사진 자체가 y=202라 앞 창과 2px 거리에 놓임(snap만 안 붙으면 됨)
        });
    }

    // 3. 가림 + V 전용(접근성 없음) — 앞 창 아래 모서리를 윗변으로 쓰는 큰 가짜 상자 방지
    {
        let mut c = Canvas::new(1280, 800, [242, 242, 242]);
        let win = PhysRect::new(100, 100, 1000, 600);
        c.fill_rect(win, [250, 250, 250]);
        c.stroke_rect(win, 1, [160, 160, 160]);
        let card = PhysRect::new(220, 220, 320, 200);
        c.fill_rect(card, [225, 235, 245]);
        c.stroke_rect(card, 1, [140, 160, 190]);
        let occ = PhysRect::new(0, 0, 1200, 180);
        c.fill_rect(occ, [40, 44, 52]);
        cases.push(CaseSpec {
            id: "cap03_occluded_v_only",
            category: "가림(V 전용)",
            canvas: c,
            point: PhysPoint { x: 380, y: 320 },
            window: Some(win),
            children: vec![],
            elements: vec![],
            occluders: vec![occ],
            truth: card,
            check_occluder_edge: true,
        });
    }

    // 4. 왼쪽 위 모서리 L자 가림
    {
        let mut c = Canvas::new(1280, 800, [245, 245, 245]);
        let win = PhysRect::new(50, 50, 1100, 700);
        c.fill_rect(win, [255, 255, 255]);
        let photo = PhysRect::new(100, 100, 400, 300);
        c.fill_photo_pattern(photo, 0, [130, 160, 190], 40);
        let occ = PhysRect::new(0, 0, 250, 200);
        c.fill_rect(occ, [55, 60, 70]);
        cases.push(CaseSpec {
            id: "cap04_corner_l_occluded",
            category: "가림(L자 모서리)",
            canvas: c,
            point: PhysPoint { x: 350, y: 300 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: photo, control: Control::Image, from_msaa: false }],
            occluders: vec![occ],
            truth: PhysRect::new(100, 200, 400, 200),
            check_occluder_edge: true,
        });
    }

    // 5. 겹친 앞 창 2개
    {
        let mut c = Canvas::new(1280, 800, [245, 245, 245]);
        let win = PhysRect::new(50, 50, 1100, 700);
        c.fill_rect(win, [255, 255, 255]);
        let photo = PhysRect::new(100, 100, 400, 300);
        c.fill_photo_pattern(photo, 0, [120, 175, 160], 40);
        let occ1 = PhysRect::new(0, 0, 1200, 150);
        let occ2 = PhysRect::new(0, 0, 180, 750);
        c.fill_rect(occ1, [48, 52, 60]);
        c.fill_rect(occ2, [64, 68, 76]);
        cases.push(CaseSpec {
            id: "cap05_two_overlapping_windows",
            category: "겹친 창 2개",
            canvas: c,
            point: PhysPoint { x: 350, y: 300 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: photo, control: Control::Image, from_msaa: false }],
            occluders: vec![occ1, occ2],
            truth: PhysRect::new(180, 150, 320, 250),
            check_occluder_edge: true,
        });
    }

    // 6. 이미지 4면 여백(연한 파스텔 가장자리 — V Box가 못 잡고 평탄도 재측정 필요)
    {
        let mut c = Canvas::new(1280, 800, [255, 255, 255]);
        let win = PhysRect::new(50, 50, 1100, 700);
        let elem = PhysRect::new(100, 100, 400, 300);
        let inner = PhysRect::new(130, 125, 340, 250);
        // 바깥 여백 [255,255,255], 가장자리는 [241, 243, 245](차이 10~14 < 24, > 6)에서 중심부로 진해짐
        c.fill_soft_edge_photo(inner, 0, [241, 243, 245], [-90, -60, -25]);
        cases.push(CaseSpec {
            id: "cap06_image_padding_4sides",
            category: "이미지 여백(4면)",
            canvas: c,
            point: PhysPoint { x: 300, y: 250 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: elem, control: Control::Image, from_msaa: false }],
            occluders: vec![],
            truth: inner,
            check_occluder_edge: false,
        });
    }

    // 7. 좌우 레터박스 여백(상하는 꽉 참)
    {
        let mut c = Canvas::new(1280, 800, [255, 255, 255]);
        let win = PhysRect::new(50, 50, 1100, 700);
        let elem = PhysRect::new(100, 100, 480, 270);
        c.fill_rect(elem, [24, 24, 28]);
        let inner = PhysRect::new(220, 100, 240, 270);
        // 어두운 여백과 차이 14(<24, >6)로 시작해 중앙이 밝아지는 사진
        c.fill_soft_edge_photo(inner, 0, [38, 40, 44], [90, 110, 130]);
        cases.push(CaseSpec {
            id: "cap07_image_letterbox",
            category: "이미지 여백(좌우)",
            canvas: c,
            point: PhysPoint { x: 340, y: 235 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: elem, control: Control::Image, from_msaa: false }],
            occluders: vec![],
            truth: inner,
            check_occluder_edge: false,
        });
    }

    // 8. 이미지 부드러운 그림자 + 여백
    {
        let mut c = Canvas::new(1280, 800, [246, 246, 246]);
        let win = PhysRect::new(50, 50, 1100, 700);
        let elem = PhysRect::new(140, 100, 420, 300);
        let inner = PhysRect::new(165, 125, 370, 250);
        c.draw_shadow(inner, 14, 0.12);
        // 은은한 그림자 끝(~220)과 밝은 사진 가장자리(234)의 차이가 < 24라 V Box가 못 잡고,
        // 단조 그림자 BFS는 그림자를 걷어내어 inner를 정확히 찾는다.
        c.fill_soft_edge_photo(inner, 0, [234, 236, 238], [-80, -50, -30]);
        cases.push(CaseSpec {
            id: "cap08_image_drop_shadow",
            category: "이미지 그림자",
            canvas: c,
            point: PhysPoint { x: 350, y: 250 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: elem, control: Control::Image, from_msaa: false }],
            occluders: vec![],
            truth: inner,
            check_occluder_edge: false,
        });
    }

    // 9. 오프셋 그림자 + 둥근 모서리 + 여백
    {
        let mut c = Canvas::new(1280, 800, [244, 244, 244]);
        let win = PhysRect::new(50, 50, 1100, 700);
        let elem = PhysRect::new(120, 100, 380, 260);
        let inner = PhysRect::new(142, 118, 324, 208);
        c.draw_offset_shadow(inner, (6, 8), 14, 0.24);
        c.fill_soft_edge_photo(inner, 20, [230, 232, 236], [-70, -40, -20]);
        cases.push(CaseSpec {
            id: "cap09_image_offset_shadow_rounded",
            category: "그림자+둥근모서리",
            canvas: c,
            point: PhysPoint { x: 300, y: 220 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: elem, control: Control::Image, from_msaa: false }],
            occluders: vec![],
            truth: inner,
            check_occluder_edge: false,
        });
    }

    // 10. 둥근 모서리 + 여백(반지름 20)
    {
        let mut c = Canvas::new(1280, 800, [248, 248, 248]);
        let win = PhysRect::new(50, 50, 1100, 700);
        let elem = PhysRect::new(140, 120, 300, 220);
        let inner = PhysRect::new(165, 145, 250, 170);
        c.fill_soft_edge_photo(inner, 24, [234, 236, 238], [-85, -55, -20]);
        cases.push(CaseSpec {
            id: "cap10_image_rounded_padded",
            category: "둥근 모서리+여백",
            canvas: c,
            point: PhysPoint { x: 290, y: 230 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: elem, control: Control::Image, from_msaa: false }],
            occluders: vec![],
            truth: inner,
            check_occluder_edge: false,
        });
    }

    // 11. 여백 없는 둥근 모서리 사진(회귀 방지)
    {
        let mut c = Canvas::new(1280, 800, [245, 245, 245]);
        let win = PhysRect::new(50, 50, 1100, 700);
        let elem = PhysRect::new(160, 140, 260, 180);
        c.fill_rounded_rect(elem, 18, [120, 155, 205]);
        c.fill_soft_edge_photo(PhysRect::new(178, 158, 224, 144), 0, [120, 155, 205], [-30, -20, -10]);
        cases.push(CaseSpec {
            id: "cap11_image_rounded_tight",
            category: "둥근 모서리(여백없음)",
            canvas: c,
            point: PhysPoint { x: 290, y: 230 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: elem, control: Control::Image, from_msaa: false }],
            occluders: vec![],
            truth: elem,
            check_occluder_edge: false,
        });
    }

    // 12. 스크롤 목록(사다리 중복 제거: List/Pane IoU≈0.96, ListItem/Group IoU≈0.9052, 5px 차이라 snap 회피)
    {
        let mut c = Canvas::new(1280, 800, [245, 245, 245]);
        let win = PhysRect::new(50, 50, 1100, 700);
        c.fill_rect(win, [252, 252, 252]);
        let list = PhysRect::new(100, 100, 840, 480);
        let viewport = PhysRect::new(100, 100, 820, 480);
        let item = PhysRect::new(110, 160, 800, 120);
        let wrapper = PhysRect::new(115, 165, 790, 110);
        c.stroke_rect(list, 1, [200, 200, 200]);
        c.fill_rect(item, [235, 242, 250]);
        c.stroke_rect(item, 1, [160, 175, 195]);
        cases.push(CaseSpec {
            id: "cap12_scroll_list_items",
            category: "스크롤 목록(사다리)",
            canvas: c,
            point: PhysPoint { x: 300, y: 210 },
            window: Some(win),
            children: vec![],
            elements: vec![
                ElementCand { rect: wrapper, control: Control::Group, from_msaa: false },
                ElementCand { rect: item, control: Control::ListItem, from_msaa: false },
                ElementCand { rect: viewport, control: Control::Pane, from_msaa: false },
                ElementCand { rect: list, control: Control::List, from_msaa: false },
            ],
            occluders: vec![],
            truth: item,
            check_occluder_edge: false,
        });
    }

    // 13. 이중 테두리 카드 사다리(IoU ≈ 0.902 겹침 정리)
    {
        let mut c = Canvas::new(1280, 800, [242, 244, 248]);
        let win = PhysRect::new(50, 50, 1100, 700);
        c.fill_rect(win, [242, 244, 248]);
        let outer = PhysRect::new(160, 120, 300, 200);
        let inner = PhysRect::new(166, 126, 288, 188);
        c.fill_rect(outer, [255, 255, 255]);
        c.stroke_rect(outer, 1, [120, 130, 145]);
        c.stroke_rect(inner, 1, [180, 190, 205]);
        cases.push(CaseSpec {
            id: "cap13_nested_card_ladder_dedup",
            category: "이중 테두리 사다리",
            canvas: c,
            point: PhysPoint { x: 310, y: 220 },
            window: Some(win),
            children: vec![],
            elements: vec![
                ElementCand { rect: inner, control: Control::Group, from_msaa: false },
                ElementCand { rect: outer, control: Control::Group, from_msaa: false },
            ],
            occluders: vec![],
            truth: outer,
            check_occluder_edge: false,
        });
    }

    // 14. 여백 없는 하늘 풍경 사진(균일 상단 하늘을 여백으로 오인하지 않음)
    {
        let mut c = Canvas::new(1280, 800, [255, 255, 255]);
        let win = PhysRect::new(50, 50, 1100, 700);
        let photo = PhysRect::new(120, 100, 400, 300);
        // 상단 30% 균일 하늘색 [230, 180, 120]에서 하단 초록 풍경으로 부드럽게 이어짐
        for y in photo.y..photo.bottom() {
            let t = ((y - (photo.y + 90)) as f32 / 120.0).clamp(0.0, 1.0);
            let b = (230.0 * (1.0 - t) + 70.0 * t).round() as u8;
            let g = (180.0 * (1.0 - t) + 155.0 * t).round() as u8;
            let r = (120.0 * (1.0 - t) + 80.0 * t).round() as u8;
            for x in photo.x..photo.right() {
                c.set_px(x, y, [b, g, r]);
            }
        }
        cases.push(CaseSpec {
            id: "cap14_fullbleed_sky_photo",
            category: "여백없는 하늘 사진(비오탐)",
            canvas: c,
            point: PhysPoint { x: 320, y: 250 },
            window: Some(win),
            children: vec![],
            elements: vec![ElementCand { rect: photo, control: Control::Image, from_msaa: false }],
            occluders: vec![],
            truth: photo,
            check_occluder_edge: false,
        });
    }

    cases
}

#[test]
fn 합성_사례_14종_r2_cap_2_5_모두_맞춤() {
    let cases = build_cases();
    assert!(cases.len() >= 12, "합성 사례는 12개 이상이어야 한다");

    let mut pass = 0usize;
    let mut total_dups = 0usize;
    let mut total_occ_hits = 0usize;
    eprintln!("=== AFTER (R2-CAP-2·3·4 적용 후) ===");
    for cs in &cases {
        let prep = prepare(&cs.canvas.bgra, cs.canvas.w, cs.canvas.h);
        let vc = prep.query_with_occluders(cs.point.x, cs.point.y, &cs.occluders);
        let vis = to_vision_cands(vc);
        let sup = |r: PhysRect| prep.support_with_occluders(r, &cs.occluders);
        let snap = |r: PhysRect| prep.snap_with_occluders(r, 3, &cs.occluders);
        let refine = |r: PhysRect| prep.refine_image(r);
        let ev = Evidence {
            point: cs.point,
            window: cs.window,
            children: &cs.children,
            elements: &cs.elements,
            vision: &vis,
            monitor: MON,
            virtual_bounds: MON,
            occluders: &cs.occluders,
            support: Some(&sup),
            snap: Some(&snap),
        };
        let opts = RecogOptions {
            refine_image: Some(&refine),
        };
        let chain = build_chain_with(&ev, &opts);
        let lvl = first_level(&chain, None);
        let picked = chain[lvl].rect;
        let score_iou = iou(&picked, &cs.truth);
        let ok = score_iou >= 0.90;
        if ok {
            pass += 1;
        }
        let dups = count_dup_pairs(&chain);
        total_dups += dups;
        let occ_hits = if cs.check_occluder_edge {
            vis.iter().filter(|v| touches_occ_edge(v.rect, &cs.occluders)).count()
                + chain
                    .iter()
                    .filter(|n| matches!(n.kind, NodeKind::Vision(_)) && touches_occ_edge(n.rect, &cs.occluders))
                    .count()
        } else {
            0
        };
        total_occ_hits += occ_hits;
        eprintln!(
            "AFTER  | {:<32} | {} | truth={:?} | picked={:?} | iou={:.4} | ok={} | dups={} | occ_hits={}",
            cs.id, cs.category, cs.truth, picked, score_iou, ok, dups, occ_hits
        );
        assert!(ok, "{} 실패: truth={:?}, picked={:?}, iou={:.4}", cs.id, cs.truth, picked, score_iou);
        assert_eq!(dups, 0, "{} 사다리 중복 쌍 존재: {:?}", cs.id, chain);
        assert_eq!(occ_hits, 0, "{} 앞 창 경계 오염 존재: vis={:?}, picked={:?}", cs.id, vis, picked);
    }
    eprintln!(
        "AFTER SUMMARY: pass={}/{} ({:.1}%), total_dups={}, total_occ_hits={}",
        pass,
        cases.len(),
        pass as f64 * 100.0 / cases.len() as f64,
        total_dups,
        total_occ_hits
    );

    // 기본 build_chain(&ev) 호출(recog.rs 측 방어)도 가림 경계 V 제외와 사다리 중복 정리를 보장한다
    for cs in &cases {
        let prep = prepare(&cs.canvas.bgra, cs.canvas.w, cs.canvas.h);
        let vc = prep.query(cs.point.x, cs.point.y);
        let vis = to_vision_cands(vc);
        let sup = |r: PhysRect| prep.support(r);
        let snap = |r: PhysRect| prep.snap(r, 3);
        let ev = Evidence {
            point: cs.point,
            window: cs.window,
            children: &cs.children,
            elements: &cs.elements,
            vision: &vis,
            monitor: MON,
            virtual_bounds: MON,
            occluders: &cs.occluders,
            support: Some(&sup),
            snap: Some(&snap),
        };
        let chain = build_chain(&ev);
        assert_eq!(count_dup_pairs(&chain), 0, "{} 기본 경로 중복: {:?}", cs.id, chain);
        if cs.check_occluder_edge {
            assert!(
                !chain
                    .iter()
                    .any(|n| matches!(n.kind, NodeKind::Vision(_)) && touches_occluder_edge(n.rect, &cs.occluders)),
                "{} 기본 경로에서도 앞 창 경계 V 후보가 사슬에 남음: {:?}",
                cs.id,
                chain
            );
        }
    }
}
