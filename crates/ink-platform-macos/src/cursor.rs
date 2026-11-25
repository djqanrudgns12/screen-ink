//! 커서 모양(M3): 지금 시스템 커서 그림 + 핫스팟 → 곧은 알파 스프라이트(물리 px).
//! `NSCursor.currentSystemCursor`는 Apple이 앞으로 nil로 바꾼다고 예고했다(사용 중단) — 그때는 None.

use crate::coords::Space;
use crate::pixels;
use ink_domain::image::CursorSprite;
use objc2_app_kit::NSCursor;
use objc2_foundation::NSRect;

pub fn sprite(space: &Space) -> Option<CursorSprite> {
    #[allow(deprecated)]
    let cursor = NSCursor::currentSystemCursor()?;
    let image = cursor.image();
    let hot = cursor.hotSpot();
    let size = image.size();
    if size.width <= 0.0 || size.height <= 0.0 {
        return None;
    }
    let (cx, cy) = crate::display::cursor_pt();
    let at = space.to_phys(cx, cy);
    let scale = space.screens.iter().find(|s| s.phys.contains(at)).map(|s| s.scale).unwrap_or(1.0);
    let (w, h) = ((size.width * scale).round().max(1.0) as usize, (size.height * scale).round().max(1.0) as usize);
    let mut rect = NSRect::new(objc2_foundation::NSPoint::new(0.0, 0.0), objc2_foundation::NSSize::new(w as f64, h as f64));
    // SAFETY: 제안 사각형은 이 호출 동안만 쓰인다.
    let cg = unsafe { image.CGImageForProposedRect_context_hints(&mut rect, None, None) }?;
    let bgra = pixels::straight_bgra(&cg, w, h)?;
    Some(CursorSprite { x: at.x - (hot.x * scale).round() as i32, y: at.y - (hot.y * scale).round() as i32, w: w as u32, h: h as u32, bgra })
}
