//! 트레이 아이콘(docs/brand/README.md 공식 아이콘): 준비 = 공식 아이콘 32px / 꺼짐 = 흐린 아이콘 + 대각선 / 녹화 = 큰 빨간 원 + 바깥 고리.
//! 모양만으로 세 상태가 갈리게 한다. 공식 아이콘은 먹색 원이라 어두운 작업 표시줄에 묻히므로, 그때만 원 둘레에 밝은 고리를 더한다.
//! 32px 한 벌(Windows가 16·20·24로 줄임). 원본 `assets/tray-app-32.png`는 `npx tauri icon`의 32x32.png와 같은 파일.

/// 공식 앱 아이콘 32×32 RGBA PNG.
const APP_ICON_32: &[u8] = include_bytes!("../assets/tray-app-32.png");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayMode {
    Ready,
    Off,
    Recording,
}

const SIZE: u32 = 32;
const RED: [u8; 3] = [0xE5, 0x48, 0x4D];

struct Canvas {
    px: Vec<u8>,
}

impl Canvas {
    fn new() -> Self {
        Canvas { px: vec![0; (SIZE * SIZE * 4) as usize] }
    }

    /// 공식 아이콘을 `a` 불투명도로 깐다. `gray`면 회색으로 바랜다(꺼짐).
    fn app_icon(&mut self, a: f32, gray: bool) {
        let rgba = app_icon_rgba();
        for y in 0..SIZE {
            for x in 0..SIZE {
                let i = ((y * SIZE + x) * 4) as usize;
                let [r, g, b, al] = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
                let rgb = if gray {
                    let l = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32).round() as u8;
                    [l, l, l]
                } else {
                    [r, g, b]
                };
                self.blend(x, y, rgb, a * al as f32 / 255.0);
            }
        }
    }

    /// 굵기 `t`의 고리(가운데 기준 반지름 `r`, 4x4 표본).
    fn ring(&mut self, r: f32, t: f32, rgb: [u8; 3], a: f32) {
        let c = SIZE as f32 / 2.0;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let mut hit = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = x as f32 + (sx as f32 + 0.5) / 4.0 - c;
                        let py = y as f32 + (sy as f32 + 0.5) / 4.0 - c;
                        if ((px * px + py * py).sqrt() - r).abs() <= t / 2.0 {
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

}

/// 내장 PNG를 RGBA 32×32로 푼다. 빌드에 박힌 파일이라 실패하면 시험이 잡는다.
fn app_icon_rgba() -> Vec<u8> {
    let mut r = png::Decoder::new(std::io::Cursor::new(APP_ICON_32)).read_info().expect("tray-app-32.png");
    let mut buf = vec![0; r.output_buffer_size().expect("tray-app-32.png size")];
    let info = r.next_frame(&mut buf).expect("tray-app-32.png frame");
    assert_eq!((info.width, info.height, info.color_type), (SIZE, SIZE, png::ColorType::Rgba));
    buf
}

/// RGBA 32×32. `light_taskbar` = 밝은 작업 표시줄이면 짙은 선, 어두우면 밝은 선과 원 둘레 고리.
pub fn render(mode: TrayMode, light_taskbar: bool) -> (Vec<u8>, u32, u32) {
    let ink = if light_taskbar { [0x1A, 0x1A, 0x1A] } else { [0xF2, 0xF2, 0xEE] };
    let mut c = Canvas::new();
    match mode {
        TrayMode::Ready => {
            c.app_icon(1.0, false);
            if !light_taskbar {
                c.ring(14.6, 1.2, ink, 0.55);
            }
        }
        TrayMode::Off => {
            c.app_icon(0.45, true);
            if !light_taskbar {
                c.ring(14.6, 1.2, ink, 0.3);
            }
            c.line((6.0, 26.0), (26.0, 6.0), 3.0, ink, 1.0);
        }
        TrayMode::Recording => {
            c.ring(13.5, 3.0, ink, 1.0);
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
        let (ready, w, h) = render(TrayMode::Ready, true);
        assert_eq!((w, h, ready.len()), (32, 32, 32 * 32 * 4));
        let (off, ..) = render(TrayMode::Off, true);
        let (rec, ..) = render(TrayMode::Recording, true);
        let center = ((16 * SIZE + 16) * 4) as usize;
        // 준비 = 공식 아이콘 그대로(원 안은 불투명, 원 바깥 모서리는 투명)
        let icon = app_icon_rgba();
        assert!((0..SIZE * SIZE).all(|p| ready[p as usize * 4 + 3] == icon[p as usize * 4 + 3]));
        assert!(alpha_at(&ready, 16, 16) > 200 && alpha_at(&ready, 0, 0) == 0);
        // 녹화 = 가운데 빨간 원
        assert_eq!(&rec[center..center + 3], &RED);
        // 꺼짐 = 흐린 아이콘 + 왼쪽 아래 → 오른쪽 위 대각선
        assert!(alpha_at(&off, 10, 21) > 200);
        assert!(alpha_at(&off, 22, 16) < alpha_at(&ready, 22, 16));
    }

    #[test]
    fn 어두운_작업_표시줄에서는_원_둘레에_밝은_고리() {
        let (light, ..) = render(TrayMode::Ready, true);
        let (dark, ..) = render(TrayMode::Ready, false);
        // 원 둘레(가운데에서 14.6px) 바로 위 점: 어두운 쪽이 더 밝다
        let i = ((SIZE + 16) * 4) as usize; // (16, 1)
        let lum = |p: &[u8]| p[i] as u32 + p[i + 1] as u32 + p[i + 2] as u32;
        assert!(lum(&dark) > lum(&light));
        // 원 안쪽은 같다
        let c = ((16 * SIZE + 16) * 4) as usize;
        assert_eq!(&dark[c..c + 4], &light[c..c + 4]);
    }
}
