//! CGImage ↔ BGRA(위→아래, 헤더 없음). 계약 프레임은 알파 255, 커서 스프라이트는 곧은 알파.

use objc2_core_foundation::{CFData, CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{CGDataProvider, CGImage, CGImageAlphaInfo};
use std::ffi::c_void;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGBitmapContextCreate(data: *mut c_void, w: usize, h: usize, bpc: usize, bpr: usize, space: *const c_void, info: u32) -> *mut c_void;
    fn CGContextDrawImage(c: *mut c_void, rect: CGRect, image: *const c_void);
    fn CGContextRelease(c: *mut c_void);
    fn CGColorSpaceCreateWithName(name: *const c_void) -> *mut c_void;
    fn CGColorSpaceRelease(space: *mut c_void);
    fn CGImageCreate(
        w: usize,
        h: usize,
        bpc: usize,
        bpp: usize,
        bpr: usize,
        space: *const c_void,
        info: u32,
        provider: *const c_void,
        decode: *const f64,
        interpolate: bool,
        intent: i32,
    ) -> *mut c_void;
    fn CGDataProviderCreateWithCFData(data: *const c_void) -> *mut c_void;
    fn CGDataProviderRelease(p: *mut c_void);
    static kCGColorSpaceSRGB: *const c_void;
}

/// `kCGBitmapByteOrderMask` · `kCGBitmapFloatComponents`
const BYTE_ORDER_MASK: u32 = 0x7000;
const FLOAT_COMPONENTS: u32 = 1 << 8;

/// 메모리 순서 B,G,R,A(32비트 리틀 엔디언 + 알파 앞).
const BGRA_PREMUL: u32 = 2 << 12 | 2; // ByteOrder32Little | PremultipliedFirst
const BGRA_SKIP: u32 = 2 << 12 | 6; // ByteOrder32Little | NoneSkipFirst

/// sRGB로 칠한 BGRA 바탕에 그림을 그려 바이트로(색 공간·형식이 무엇이든).
fn draw(img: &CGImage, w: usize, h: usize, info: u32) -> Option<Vec<u8>> {
    let mut out = vec![0u8; w * h * 4];
    // SAFETY: 버퍼는 이 함수 끝까지 살아 있고, 만든 CG 객체는 모두 놓는다.
    unsafe {
        let cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
        if cs.is_null() {
            return None;
        }
        let ctx = CGBitmapContextCreate(out.as_mut_ptr().cast(), w, h, 8, w * 4, cs, info);
        CGColorSpaceRelease(cs);
        if ctx.is_null() {
            return None;
        }
        CGContextDrawImage(ctx, CGRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: w as f64, height: h as f64 } }, (img as *const CGImage).cast());
        CGContextRelease(ctx);
    }
    Some(out)
}

/// 화면 그림 → 불투명 BGRA. 이미 BGRA 32비트면 줄만 옮겨 담는다(SCK 기본 형식), 아니면 sRGB로 다시 그린다.
pub fn opaque_bgra(img: &CGImage) -> Option<(u32, u32, Vec<u8>)> {
    let (w, h) = (CGImage::width(Some(img)), CGImage::height(Some(img)));
    if w == 0 || h == 0 {
        return None;
    }
    let info = CGImage::bitmap_info(Some(img)).0;
    let alpha = CGImage::alpha_info(Some(img));
    let bgra_like = CGImage::bits_per_pixel(Some(img)) == 32
        && CGImage::bits_per_component(Some(img)) == 8
        && info & BYTE_ORDER_MASK == 2 << 12
        && (alpha == CGImageAlphaInfo::PremultipliedFirst || alpha == CGImageAlphaInfo::NoneSkipFirst || alpha == CGImageAlphaInfo::First)
        && info & FLOAT_COMPONENTS == 0;
    let mut px = if bgra_like {
        let data: CFRetained<CFData> = CGDataProvider::data(CGImage::data_provider(Some(img)).as_deref())?;
        let bytes = data.to_vec();
        let bpr = CGImage::bytes_per_row(Some(img));
        if bpr == w * 4 && bytes.len() >= w * h * 4 {
            bytes[..w * h * 4].to_vec()
        } else {
            let mut out = Vec::with_capacity(w * h * 4);
            for row in 0..h {
                let s = row * bpr;
                out.extend_from_slice(bytes.get(s..s + w * 4)?);
            }
            out
        }
    } else {
        draw(img, w, h, BGRA_SKIP)?
    };
    for a in px.iter_mut().skip(3).step_by(4) {
        *a = 255;
    }
    Some((w as u32, h as u32, px))
}

/// 그림(커서 등) → 곧은 알파 BGRA, 크기 `w × h`로 그린다.
pub fn straight_bgra(img: &CGImage, w: usize, h: usize) -> Option<Vec<u8>> {
    let mut px = draw(img, w, h, BGRA_PREMUL)?;
    for p in px.chunks_exact_mut(4) {
        let a = p[3] as u32;
        for c in &mut p[..3] {
            if let Some(v) = (*c as u32 * 255 + a / 2).checked_div(a) {
                *c = v.min(255) as u8;
            }
        }
    }
    Some(px)
}

/// 불투명 BGRA → CGImage(sRGB). 클립보드 TIFF용.
pub fn image_from_bgra(w: u32, h: u32, bgra: &[u8]) -> Option<CFRetained<CGImage>> {
    let data = CFData::from_bytes(bgra);
    // SAFETY: 데이터 제공자는 CFData를 붙잡고, 그림은 제공자를 붙잡는다. 만든 객체는 여기서 놓는다.
    unsafe {
        let provider = CGDataProviderCreateWithCFData((&*data as *const CFData).cast());
        if provider.is_null() {
            return None;
        }
        let cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
        let img = CGImageCreate(w as usize, h as usize, 8, 32, w as usize * 4, cs, BGRA_SKIP, provider, std::ptr::null(), false, 0);
        CGColorSpaceRelease(cs);
        CGDataProviderRelease(provider);
        let img = std::ptr::NonNull::new(img.cast::<CGImage>())?;
        Some(CFRetained::from_raw(img))
    }
}
