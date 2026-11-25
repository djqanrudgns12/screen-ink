//! 트레이 아이콘을 코드로 그린다(10 §4): 조준틀 네 귀 + 가운데 점 / 꺼짐 = 흐림 + 대각선 / 녹화 = 큰 빨간 원 + 짧은 귀.
//! 모양만으로 세 상태가 갈리게 하고, 작업 표시줄 밝기에 따라 선 색만 바꾼다. 32px 한 벌(Windows가 16·20·24로 줄임).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayMode {
    Ready,
    Off,
    Recording,
}

const SIZE: u32 = 32;
const AMBER: [u8; 3] = [0xFF, 0x9F, 0x0A];
const RED: [u8; 3] = [0xE5, 0x48, 0x4D];

struct Canvas {
    px: Vec<u8>,
}

impl Canvas {
    fn new() -> Self {
        Canvas { px: vec![0; (SIZE * SIZE * 4) as usize] }
    }

    /// 덮어 그리기(알파 합성).
    fn blend(&mut self, x: u32, y: u32, rgb: [u8; 3], a: f32) {
        if x >= SIZE || y >= SIZE || a <= 0.0 {
            return;
        }
        let i = ((y * SIZE + x) * 4) as usize;
        let dst_a = self.px[i + 3] as f32 / 255.0;
        let out_a = a + dst_a * (1.0 - a);
        for (c, &src) in rgb.iter().enumerate() {
            let d = self.px[i + c] as f32;
            self.px[i + c] = ((src as f32 * a + d * dst_a * (1.0 - a)) / out_a.max(1e-6)).round() as u8;
        }
        self.px[i + 3] = (out_a * 255.0).round() as u8;
    }

    fn rect(&mut self, x0: u32, y0: u32, w: u32, h: u32, rgb: [u8; 3], a: f32) {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                self.blend(x, y, rgb, a);
            }
        }
    }

    /// 경계를 4x4 표본으로 부드럽게 한 원.
    fn disc(&mut self, cx: f32, cy: f32, r: f32, rgb: [u8; 3], a: f32) {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let mut hit = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = x as f32 + (sx as f32 + 0.5) / 4.0 - cx;
                        let py = y as f32 + (sy as f32 + 0.5) / 4.0 - cy;
                        if px * px + py * py <= r * r {
                            hit += 1;
                        }
                    }
                }
                if hit > 0 {
                    self.blend(x, y, rgb, a * hit as f32 / 16.0);
                }
            }
        }
    }

    /// 굵기 `t`의 선분(표본 방식).
    fn line(&mut self, (x0, y0): (f32, f32), (x1, y1): (f32, f32), t: f32, rgb: [u8; 3], a: f32) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = dx * dx + dy * dy;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let mut hit = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                        let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                        let k = (((px - x0) * dx + (py - y0) * dy) / len2).clamp(0.0, 1.0);
                        let (qx, qy) = (x0 + k * dx - px, y0 + k * dy - py);
                        if qx * qx + qy * qy <= (t / 2.0) * (t / 2.0) {
                            hit += 1;
                        }
                    }
                }
                if hit > 0 {
                    self.blend(x, y, rgb, a * hit as f32 / 16.0);
                }
            }
        }
    }

    /// 네 귀 꺾쇠: 바깥 여백 `m`, 길이 `len`, 굵기 `t`(정수 맞춤).
    fn corners(&mut self, m: u32, len: u32, t: u32, rgb: [u8; 3], a: f32) {
        let far = SIZE - m;
        // 왼쪽 위 · 오른쪽 위 · 왼쪽 아래 · 오른쪽 아래
        self.rect(m, m, len, t, rgb, a);
        self.rect(m, m + t, t, len - t, rgb, a);
        self.rect(far - len, m, len, t, rgb, a);
        self.rect(far - t, m + t, t, len - t, rgb, a);
        self.rect(m, far - t, len, t, rgb, a);
        self.rect(m, far - len, t, len - t, rgb, a);
        self.rect(far - len, far - t, len, t, rgb, a);
        self.rect(far - t, far - len, t, len - t, rgb, a);
    }
}

/// RGBA 32×32. `light_taskbar` = 밝은 작업 표시줄이면 짙은 선.
pub fn render(mode: TrayMode, light_taskbar: bool) -> (Vec<u8>, u32, u32) {
    let ink = if light_taskbar { [0x1A, 0x1A, 0x1A] } else { [0xF2, 0xF2, 0xEE] };
    let mut c = Canvas::new();
    match mode {
        TrayMode::Ready => {
            c.corners(3, 9, 3, ink, 1.0);
            c.disc(16.0, 16.0, 4.0, AMBER, 1.0);
        }
        TrayMode::Off => {
            c.corners(3, 9, 3, ink, 0.45);
            c.disc(16.0, 16.0, 3.0, ink, 0.45);
            c.line((6.0, 26.0), (26.0, 6.0), 3.0, ink, 1.0);
        }
        TrayMode::Recording => {
            c.corners(2, 6, 3, ink, 1.0);
            c.disc(16.0, 16.0, 9.0, RED, 1.0);
        }
    }
    (c.px, SIZE, SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(px: &[u8], x: u32, y: u32) -> u8 {
        px[((y * SIZE + x) * 4 + 3) as usize]
    }

    #[test]
    fn 세_상태는_모양으로_갈린다() {
        let (ready, w, h) = render(TrayMode::Ready, false);
        assert_eq!((w, h, ready.len()), (32, 32, 32 * 32 * 4));
        let (off, ..) = render(TrayMode::Off, false);
        let (rec, ..) = render(TrayMode::Recording, true);
        // 가운데: 준비 = 주황 점, 녹화 = 빨간 원, 꺼짐 = 흐림
        let center = ((16 * SIZE + 16) * 4) as usize;
        assert_eq!(&ready[center..center + 3], &AMBER);
        assert_eq!(&rec[center..center + 3], &RED);
        // 녹화 원은 반지름 9까지, 준비 점은 4까지
        assert!(alpha_at(&rec, 16, 9) > 200 && alpha_at(&ready, 16, 9) == 0);
        // 꺼짐 대각선은 왼쪽 아래 → 오른쪽 위
        assert!(alpha_at(&off, 10, 21) > 200 && alpha_at(&ready, 10, 21) == 0);
        // 귀는 바깥 여백 안
        assert_eq!(alpha_at(&ready, 0, 0), 0);
        assert!(alpha_at(&ready, 4, 4) > 200);
    }
}
