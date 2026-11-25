//! 클립보드(M6): `NSPasteboard` 일반 판에 PNG + TIFF(이미지), 글.
//! PNG는 플러그인이 만든 것을 받는다(Windows와 같은 인코더). TIFF는 앱마다 붙여 넣기 형식이 달라 함께 둔다.

use crate::pixels;
use objc2::AnyThread;
use objc2_app_kit::{NSBitmapImageRep, NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeString, NSPasteboardTypeTIFF};
use objc2_foundation::{NSData, NSString};

pub fn set_image(w: u32, h: u32, bgra: &[u8], png: Option<&[u8]>) -> Result<(), String> {
    if w == 0 || h == 0 || bgra.len() < (w * h * 4) as usize {
        return Err("empty image".into());
    }
    let tiff = pixels::image_from_bgra(w, h, bgra).and_then(|cg| NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg).TIFFRepresentation());
    let pb = NSPasteboard::generalPasteboard();
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
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    // SAFETY: 형식 이름은 AppKit 상수.
    if unsafe { pb.setString_forType(&NSString::from_str(text), NSPasteboardTypeString) } {
        Ok(())
    } else {
        Err("pasteboard refused text".into())
    }
}
