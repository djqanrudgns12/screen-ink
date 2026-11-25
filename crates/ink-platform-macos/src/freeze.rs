//! 정지 화면 창(맥, M1 진입 시간). Windows `freeze.rs`와 같은 역할이다.
//!
//! 모니터마다 **활성화하지 않는 패널** 하나에 캡처한 그림을 CALayer 내용으로 바로 붙인다.
//! 웹뷰로 이미지를 넘기면 4K에서 60~100 ms가 더 든다(사용자 프로토콜 → `<img>` 로드 → 그리기,
//! 2026-11-25 실측 ui load 38~58 + paint 27~38 ms). 여기서는 SCK가 준 그림을 그대로 넘기므로
//! 복사도 픽셀 계산도 없다. 합성은 창 서버가 한다.
//!
//! 모양은 Windows와 같다. 바탕은 50% 어둡게, 강조만 원본 밝기다. 어둡게는 원본 위에 검은 반투명
//! 사각형 4개(강조 둘레, [`around`])를 얹어 만든다. 그래서 강조를 옮길 때도 사각형 자리만 바꾼다
//! (암묵 애니메이션 끔). 입력은 받지 않고, 위의 웹뷰 오버레이(투명, 테두리·확대경)가 받는다.
//!
//! 메인 스레드 전용이다. 패널은 메인 스레드의 thread_local에만 두고, 다른 스레드에서 부르면 아무것도 하지 않는다.

use crate::coords::{around, cocoa_to_cg, Placed, RectF, Space};
use crate::window::{screen_frame, ALL_SPACES, OVERLAY_LEVEL};
use crate::{capture, pixels};
use ink_domain::geometry::PhysRect;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSPanel, NSView, NSWindowAnimationBehavior, NSWindowCollectionBehavior, NSWindowSharingType, NSWindowStyleMask,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::{CGColor, CGImage};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::{CALayer, CATransaction};
use std::cell::RefCell;
use std::collections::HashMap;

/// 웹뷰 오버레이 바로 아래 층.
const FREEZE_LEVEL: isize = OVERLAY_LEVEL - 1;
/// Windows와 같은 50%.
const DIM: f32 = 0.5;

/// 그림 한 장 + 둘레 어둡게 4칸. 창 없이도 만들고 그릴 수 있다(시험은 비트맵에 그려 본다).
/// 층 좌표는 뒤집지 않는다(원점 왼쪽 아래 — 그림 내용이 바로 선다). 사각형만 [`Layers::hole`]에서 뒤집는다.
struct Layers {
    root: Retained<CALayer>,
    image: Retained<CALayer>,
    dims: [Retained<CALayer>; 4],
    /// 판 크기(포인트)
    size: (f64, f64),
}

impl Layers {
    fn new() -> Self {
        let root = CALayer::layer();
        let image = CALayer::layer();
        root.addSublayer(&image);
        let black = CGColor::new_generic_gray(0.0, 1.0);
        let dims = std::array::from_fn(|_| {
            let d = CALayer::layer();
            d.setBackgroundColor(Some(&black));
            d.setOpacity(DIM);
            root.addSublayer(&d);
            d
        });
        Self { root, image, dims, size: (0.0, 0.0) }
    }

    /// 판 크기와 그림(None = 비우기). 암묵 애니메이션 없이.
    fn set_image(&mut self, w: f64, h: f64, img: Option<&CGImage>) {
        self.size = (w, h);
        let full = cg(RectF::new(0.0, 0.0, w, h));
        quiet(|| {
            self.root.setFrame(full);
            self.image.setFrame(full);
            // SAFETY: CGImage는 CALayer 내용으로 받는 CF 형식이다(무료 다리).
            unsafe { self.image.setContents(img.map(|i| &*(i as *const CGImage as *const AnyObject))) };
        });
    }

    /// 밝게 둘 자리(왼쪽 위 원점 포인트). None = 전체 어둡게.
    fn hole(&self, hole: Option<RectF>) {
        let (w, h) = self.size;
        quiet(|| {
            for (d, r) in self.dims.iter().zip(around(w, h, hole)) {
                // 왼쪽 위 원점 → 층의 왼쪽 아래 원점(같은 뒤집기 식)
                d.setFrame(cg(cocoa_to_cg(r, h)));
            }
        });
    }
}

struct Panel {
    window: Retained<NSPanel>,
    layers: Layers,
    /// 지금 덮고 있는 화면(강조 px → 포인트 변환에 쓴다)
    screen: Option<Placed>,
}

thread_local! {
    static PANELS: RefCell<HashMap<u32, Panel>> = RefCell::new(HashMap::new());
}

fn cg(r: RectF) -> CGRect {
    CGRect { origin: CGPoint { x: r.x, y: r.y }, size: CGSize { width: r.w, height: r.h } }
}

/// 암묵 애니메이션 없이 층을 바꾼다.
fn quiet(f: impl FnOnce()) {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    f();
    CATransaction::commit();
}

fn make(mtm: MainThreadMarker) -> Panel {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
    let window = NSPanel::initWithContentRect_styleMask_backing_defer(NSPanel::alloc(mtm), rect, style, NSBackingStoreType::Buffered, false);
    // SAFETY: 우리가 Retained로 쥐고 있으니 닫아도 놓이지 않게 한다.
    unsafe { window.setReleasedWhenClosed(false) };
    // 그림이 붙기 전 한 순간에도 검은 틀 대신 아래 화면(정지 화면과 같음)이 보이게 투명으로 둔다.
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setHasShadow(false);
    window.setIgnoresMouseEvents(true);
    window.setHidesOnDeactivate(false);
    window.setLevel(FREEZE_LEVEL);
    window.setCollectionBehavior(NSWindowCollectionBehavior(ALL_SPACES));
    window.setSharingType(NSWindowSharingType::None);
    window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    let layers = Layers::new();
    let view = NSView::initWithFrame(NSView::alloc(mtm), rect);
    // 층을 먼저 주고 켠다 = 우리가 층 나무를 관리하는 뷰(layer-hosting)
    view.setLayer(Some(&layers.root));
    view.setWantsLayer(true);
    window.setContentView(Some(&view));
    Panel { window, layers, screen: None }
}

/// 지금 메인 스레드인지(아니면 부르는 쪽이 메인 스레드로 넘긴다).
pub fn on_main_thread() -> bool {
    MainThreadMarker::new().is_some()
}

/// 50% 어둡게 한 정지 화면(+ 처음 강조만 밝게)을 그 모니터 위에 바로 보인다. 성공하면 true.
/// 그림은 방금 캡처한 SCK 그림을 쓰고(복사 없음), 없으면 `bgra`로 만든다. `highlight`는 모니터 기준 물리 px.
pub fn show(monitor: u32, space: &Space, bounds: PhysRect, w: u32, h: u32, bgra: &[u8], highlight: Option<PhysRect>) -> bool {
    let Some(mtm) = MainThreadMarker::new() else { return false };
    let Some(s) = space.screen_of(bounds).copied() else { return false };
    let Some(frame) = screen_frame(mtm, &s) else { return false };
    let Some(img) = capture::take_image(monitor, w, h).or_else(|| pixels::image_from_bgra(w, h, bgra)) else { return false };
    PANELS.with_borrow_mut(|panels| {
        let p = panels.entry(monitor).or_insert_with(|| make(mtm));
        p.screen = Some(s);
        p.window.setFrame_display(frame, false);
        p.layers.set_image(s.pt.w, s.pt.h, Some(&img));
        p.layers.hole(highlight.map(|r| s.local_to_pt(r)));
        // 층 변경을 지금 창 서버로 보낸 뒤 앞으로(첫 그림부터 정지 화면이 보이게)
        CATransaction::flush();
        p.window.orderFrontRegardless();
    });
    true
}

/// 강조(밝게 뚫기)를 옮긴다. `rect`는 모니터 기준 물리 px.
pub fn set_highlight(monitor: u32, rect: Option<PhysRect>) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    PANELS.with_borrow(|panels| {
        let Some(p) = panels.get(&monitor) else { return };
        let Some(s) = p.screen else { return };
        p.layers.hole(rect.map(|r| s.local_to_pt(r)));
    });
}

/// 모두 숨기고 그림을 놓는다.
pub fn hide_all() {
    capture::clear_images();
    if MainThreadMarker::new().is_none() {
        return;
    }
    PANELS.with_borrow_mut(|panels| {
        for p in panels.values_mut() {
            p.window.orderOut(None);
            let (w, h) = p.layers.size;
            p.layers.set_image(w, h, None);
        }
    });
}

/// 배치에 없는 모니터의 창을 닫는다.
pub fn retain(monitors: &[u32]) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    PANELS.with_borrow_mut(|panels| {
        panels.retain(|id, p| {
            let keep = monitors.contains(id);
            if !keep {
                p.window.close();
            }
            keep
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 층 나무를 비트맵(BGRA, 위→아래)에 그려 본다 — 창·화면 기록 권한 없이.
    #[test]
    fn 그림은_바로_서고_강조만_밝다() {
        let (w, h) = (40u32, 30u32);
        // 위 절반 빨강, 아래 절반 파랑(BGRA)
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        for (i, p) in bgra.chunks_exact_mut(4).enumerate() {
            let y = i as u32 / w;
            p.copy_from_slice(if y < h / 2 { &[0, 0, 200, 255] } else { &[200, 0, 0, 255] });
        }
        let img = pixels::image_from_bgra(w, h, &bgra).expect("CGImage");
        let mut l = Layers::new();
        l.set_image(w as f64, h as f64, Some(&img));
        // 왼쪽 위 원점 (5,2) 10x6 만 밝게
        l.hole(Some(RectF::new(5.0, 2.0, 10.0, 6.0)));
        let out = pixels::render_bgra(w as usize, h as usize, |ctx| l.root.renderInContext(ctx)).expect("render");
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            [out[i], out[i + 1], out[i + 2]]
        };
        // 강조 안: 원본 그대로(위쪽이라 빨강) → 그림이 뒤집히지 않았다
        assert_eq!(px(6, 3), [0, 0, 200], "강조 안 = 원본 빨강");
        // 강조 밖 위쪽: 빨강 50%
        let dim_red = px(30, 3);
        assert!(dim_red[2].abs_diff(100) <= 2 && dim_red[0] <= 2, "위쪽 어둡게 {dim_red:?}");
        // 아래쪽: 파랑 50%
        let dim_blue = px(30, 25);
        assert!(dim_blue[0].abs_diff(100) <= 2 && dim_blue[2] <= 2, "아래쪽 어둡게 {dim_blue:?}");
        // 강조 경계 바로 밖(왼쪽·아래)은 어둡다
        assert!(px(4, 3)[2] < 150 && px(6, 8)[2] < 150, "구멍은 (5,2)~(15,8) 안만");
        // 강조 없음 = 전체 어둡게
        l.hole(None);
        let out = pixels::render_bgra(w as usize, h as usize, |ctx| l.root.renderInContext(ctx)).expect("render");
        assert!(out[((3 * w + 6) * 4 + 2) as usize].abs_diff(100) <= 2);
    }
}
