//! 정지 이미지 전송 형식 — BMP(BITMAPV4HEADER, 32bpp BI_BITFIELDS, 알파 마스크 0, 위→아래).
//! 인코딩이 없어(헤더만 붙임) 4K도 복사 한 번이면 끝나고, 웹뷰가 바로 그린다. 알파 마스크 0 = 불투명으로 읽힘
//! (GDI가 알파 바이트에 무엇을 남기든 반투명으로 보이지 않게).

pub const FILE_HEADER: usize = 14;
pub const INFO_HEADER: usize = 108;
pub const HEADER_LEN: usize = FILE_HEADER + INFO_HEADER;

/// `w×h` BGRA(4바이트) 픽셀 앞에 붙일 헤더.
pub fn header(w: u32, h: u32) -> [u8; HEADER_LEN] {
    let image = w as u64 * h as u64 * 4;
    let mut b = [0u8; HEADER_LEN];
    let mut put = |at: usize, bytes: &[u8]| b[at..at + bytes.len()].copy_from_slice(bytes);
    put(0, b"BM");
    put(2, &((HEADER_LEN as u64 + image) as u32).to_le_bytes());
    put(10, &(HEADER_LEN as u32).to_le_bytes());
    let i = FILE_HEADER;
    put(i, &(INFO_HEADER as u32).to_le_bytes());
    put(i + 4, &(w as i32).to_le_bytes());
    put(i + 8, &(-(h as i32)).to_le_bytes()); // 음수 = 위→아래
    put(i + 12, &1u16.to_le_bytes());
    put(i + 14, &32u16.to_le_bytes());
    put(i + 16, &3u32.to_le_bytes()); // BI_BITFIELDS
    put(i + 20, &(image as u32).to_le_bytes());
    put(i + 24, &2835i32.to_le_bytes()); // 72 dpi
    put(i + 28, &2835i32.to_le_bytes());
    put(i + 40, &0x00FF_0000u32.to_le_bytes()); // R
    put(i + 44, &0x0000_FF00u32.to_le_bytes()); // G
    put(i + 48, &0x0000_00FFu32.to_le_bytes()); // B
    put(i + 52, &0u32.to_le_bytes()); // A = 없음 → 불투명
    put(i + 56, &0x7352_4742u32.to_le_bytes()); // LCS_sRGB
    b
}

/// 헤더 + 픽셀 한 덩어리(복사 한 번).
pub fn encode(w: u32, h: u32, bgra: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + bgra.len());
    out.extend_from_slice(&header(w, h));
    out.extend_from_slice(bgra);
    out
}

/// BMP 덩어리에서 (x, y) 픽셀의 RGB(시험·확대경 검증용).
pub fn pixel(bmp: &[u8], w: u32, x: u32, y: u32) -> Option<[u8; 3]> {
    let i = HEADER_LEN + ((y as usize * w as usize) + x as usize) * 4;
    let p = bmp.get(i..i + 4)?;
    Some([p[2], p[1], p[0]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 헤더_필드와_픽셀_자리() {
        let h = header(1920, 1080);
        assert_eq!(&h[0..2], b"BM");
        assert_eq!(u32::from_le_bytes(h[2..6].try_into().unwrap()) as usize, HEADER_LEN + 1920 * 1080 * 4);
        assert_eq!(u32::from_le_bytes(h[10..14].try_into().unwrap()) as usize, HEADER_LEN);
        assert_eq!(i32::from_le_bytes(h[22..26].try_into().unwrap()), -1080, "위→아래");
        assert_eq!(u32::from_le_bytes(h[30..34].try_into().unwrap()), 3, "BI_BITFIELDS");
        let bgra = [10, 20, 30, 0, 40, 50, 60, 0];
        let bmp = encode(2, 1, &bgra);
        assert_eq!(pixel(&bmp, 2, 1, 0), Some([60, 50, 40]));
        assert_eq!(pixel(&bmp, 2, 2, 0), None);
    }
}
