//! 클립보드(M6): `NSPasteboard` 일반 판에 PNG + TIFF(이미지), 글.
//! PNG는 플러그인이 만든 것을 받는다(Windows와 같은 인코더). TIFF는 앱마다 붙여 넣기 형식이 달라 함께 둔다.

use crate::pixels;
use objc2::AnyThread;
use objc2_app_kit::{NSBitmapImageRep, NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeString, NSPasteboardTypeTIFF};
use objc2_foundation::{NSData, NSString};

pub fn set_image(w: u32, h: u32, bgra: &[u8], png: Option<&[u8]>) -> Result<(), String> {
    write_image(&NSPasteboard::generalPasteboard(), w, h, bgra, png)
}

fn write_image(pb: &NSPasteboard, w: u32, h: u32, bgra: &[u8], png: Option<&[u8]>) -> Result<(), String> {
    if w == 0 || h == 0 || bgra.len() < (w * h * 4) as usize {
        return Err("empty image".into());
    }
    let tiff = pixels::image_from_bgra(w, h, bgra).and_then(|cg| NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg).TIFFRepresentation());
    pb.clearContents();
    let mut wrote = false;
    // SAFETY: 형식 이름은 AppKit 상수.
    unsafe {
        if let Some(png) = png {
            wrote |= pb.setData_forType(Some(&NSData::with_bytes(png)), NSPasteboardTypePNG);
        }
        if let Some(t) = tiff {
            wrote |= pb.setData_forType(Some(&t), NSPasteboardTypeTIFF);
        }
    }
    if wrote {
        Ok(())
    } else {
        Err("pasteboard refused image".into())
    }
}

pub fn set_text(text: &str) -> Result<(), String> {
    write_text(&NSPasteboard::generalPasteboard(), text)
}

fn write_text(pb: &NSPasteboard, text: &str) -> Result<(), String> {
    pb.clearContents();
    // SAFETY: 형식 이름은 AppKit 상수.
    if unsafe { pb.setString_forType(&NSString::from_str(text), NSPasteboardTypeString) } {
        Ok(())
    } else {
        Err("pasteboard refused text".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 사용자 클립보드는 건드리지 않는다 — 이름 없는 임시 판에 쓰고 읽어 본 뒤 지운다.
    #[test]
    fn 이미지는_tiff와_png로_크기_그대로_글은_그대로() {
        let pb = NSPasteboard::pasteboardWithUniqueName();
        let (w, h) = (7u32, 3u32);
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        for (i, px) in bgra.chunks_exact_mut(4).enumerate() {
            px.copy_from_slice(&[(i * 10) as u8, 0x40, 0xC0, 255]);
        }
        write_image(&pb, w, h, &bgra, Some(&[0x89, b'P', b'N', b'G'])).unwrap();
        // SAFETY: 형식 이름은 AppKit 상수.
        let (tiff, png) = unsafe { (pb.dataForType(NSPasteboardTypeTIFF), pb.dataForType(NSPasteboardTypePNG)) };
        let rep = NSBitmapImageRep::imageRepWithData(&tiff.expect("TIFF")).expect("TIFF 읽기");
        assert_eq!((rep.pixelsWide(), rep.pixelsHigh()), (w as isize, h as isize));
        // 첫 화소 색이 그대로(sRGB, 알파 없음): B=0 G=0x40 R=0xC0
        let p = rep.bitmapData();
        let first = unsafe { std::slice::from_raw_parts(p, 4) };
        assert_eq!(&first[..3], &[0xC0, 0x40, 0x00], "RGB 순서로 저장");
        assert_eq!(png.expect("PNG").len(), 4);
        write_text(&pb, "#C04000").unwrap();
        // SAFETY: 형식 이름은 AppKit 상수.
        let text = unsafe { pb.stringForType(NSPasteboardTypeString) }.map(|s| s.to_string());
        assert_eq!(text.as_deref(), Some("#C04000"));
        // SAFETY: NSPasteboard 공개 메서드(임시 판을 지운다).
        unsafe {
            let _: () = objc2::msg_send![&*pb, releaseGlobally];
        }
        assert!(write_image(&pb, 0, 0, &[], None).is_err());
    }
}
