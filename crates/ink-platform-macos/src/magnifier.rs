//! 실시간 확대 돋보기(M13, R-ZOOM-1·2·4 — 맥). Windows `magnifier.rs`(Magnification API)와 같은 포트(`Magnifier`).
//!
//! - 그 모니터를 덮는 활성화하지 않는 패널 하나(클릭 통과·창 공유 끔·모든 Space, 정지 패널과 같은 층) + CALayer.
//! - `SCStream`(그 디스플레이, **우리 앱 창은 모두 뺌** = R-ZOOM-4, 원래 해상도, 60 fps)의 프레임 IOSurface를
//!   층 내용으로 바로 붙이고, 원본 사각형은 `contentsRect`(정규화)로 고른다 — 확대는 창 서버(GPU)가 한다. 복사 없음.
//! - 커서는 Windows(`MS_SHOWMAGNIFIEDCURSOR` + 진짜 커서 숨김)처럼 **확대된 커서**를 확대 자리에 보인다: 진짜 커서는
//!   그 모니터 안에 있을 때만 숨기고, 지금 시스템 커서 그림을 층 하나로 그린다(SCK는 숨긴 커서를 찍지 않는다 —
//!   2026-11-27 실측, 그래서 스트림은 커서 없이). 뒤에 있는 앱은 공개 API로 커서를 못 숨겨서(2026-11-26 실측) 비공개 연결
//!   속성 `SetsCursorInBackground`를 켠 뒤 숨긴다. 그 함수가 없으면 진짜 커서를 그대로 두고 확대 커서는 그리지 않는다
//!   (기본 '마우스 위치대로'에서는 진짜 커서가 곧 확대 자리라 어긋나지 않는다).
//! - 패널·층은 메인 스레드 전용(thread_local). 서비스는 메인 스레드에서 부른다.

use crate::coords::{Placed, RectF};
use crate::window::{screen_frame, ALL_SPACES, OVERLAY_LEVEL};
use block2::RcBlock;
use ink_app::Frame;
use ink_domain::geometry::PhysRect;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, ProtocolObject};
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSBackingStoreType, NSColor, NSCursor, NSPanel, NSView, NSWindowAnimationBehavior, NSWindowCollectionBehavior, NSWindowSharingType, NSWindowStyleMask};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_media::{CMSampleBuffer, CMTime};
use objc2_foundation::{NSArray, NSError, NSObjectProtocol, NSPoint, NSRect, NSSize};
use objc2_quartz_core::{CALayer, CATransaction};
use objc2_screen_capture_kit::{SCContentFilter, SCRunningApplication, SCStream, SCStreamConfiguration, SCStreamOutput, SCStreamOutputType};
use std::cell::RefCell;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// 정지 패널과 같은 층(웹뷰 오버레이·HUD·섬 아래).
const MAG_LEVEL: isize = OVERLAY_LEVEL - 1;
/// 첫 그림을 기다리는 시간.
const FIRST_FRAME_WAIT: Duration = Duration::from_millis(800);
const START_WAIT: Duration = Duration::from_secs(3);
/// 확대 커서 크기 상한(배율이 커도 커서는 이 배까지만 — 8×에서 손바닥만 해지지 않게).
const CURSOR_MAX_SCALE: f64 = 3.0;
/// 커서 모양(화살표·I 자 등)을 다시 읽는 틱 간격(≈ 10 Hz).
const CURSOR_SHAPE_TICKS: u32 = 6;

type CFTypeRef = *const c_void;

#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    fn CMSampleBufferGetImageBuffer(sbuf: &CMSampleBuffer) -> CFTypeRef;
}
#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    fn CVPixelBufferGetWidth(pb: CFTypeRef) -> usize;
    fn CVPixelBufferGetHeight(pb: CFTypeRef) -> usize;
    fn CVPixelBufferGetPixelFormatType(pb: CFTypeRef) -> u32;
    fn CVPixelBufferGetBytesPerRow(pb: CFTypeRef) -> usize;
    fn CVPixelBufferGetDataSize(pb: CFTypeRef) -> usize;
    fn CVPixelBufferLockBaseAddress(pb: CFTypeRef, flags: u64) -> i32;
    fn CVPixelBufferUnlockBaseAddress(pb: CFTypeRef, flags: u64) -> i32;
    fn CVPixelBufferGetBaseAddress(pb: CFTypeRef) -> *const u8;
    fn CVPixelBufferGetIOSurface(pb: CFTypeRef) -> CFTypeRef;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: CFTypeRef;
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn CFRelease(cf: CFTypeRef);
    fn CFStringCreateWithBytes(alloc: CFTypeRef, bytes: *const u8, len: isize, encoding: u32, external: bool) -> CFTypeRef;
}
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayHideCursor(display: u32) -> i32;
    fn CGDisplayShowCursor(display: u32) -> i32;
}
extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}
const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
const UTF8: u32 = 0x0800_0100;

/// 원본 왼쪽 위(물리 px)·배율 → 층 `contentsRect`(정규화, 원점 왼쪽 아래 — 층 좌표는 뒤집지 않는다).
/// Windows 돋보기와 같은 **고른 배율**: 크기 = 모니터 / 배율(정수로 올린 원본 크기가 아니라 — 늘이면 가로·세로 배율이
/// 조금씩 달라져 커서와 그림이 어긋난다). 그림 위쪽 = 화면 위쪽이므로 y는 아래에서 잰다: `1 − 위/높이 − 1/배율`.
pub fn contents_rect(monitor: PhysRect, source: PhysRect, scale: f64) -> RectF {
    let (mw, mh) = (monitor.w.max(1) as f64, monitor.h.max(1) as f64);
    let k = if scale.is_finite() && scale >= 1.0 { 1.0 / scale } else { 1.0 };
    let x = (source.x - monitor.x) as f64 / mw;
    let top = (source.y - monitor.y) as f64 / mh;
    RectF::new(x, 1.0 - top - k, k, k)
}

fn cg(r: RectF) -> CGRect {
    CGRect { origin: CGPoint { x: r.x, y: r.y }, size: CGSize { width: r.w, height: r.h } }
}

/// 스레드 사이로 넘기는 껍데기(SCK 객체·CVPixelBuffer 포인터). 넘긴 뒤에는 받는 쪽만 쓴다.
struct Hold<T>(T);
// SAFETY: 위 규칙.
unsafe impl<T> Send for Hold<T> {}

struct Panel {
    window: Retained<NSPanel>,
    root: Retained<CALayer>,
    image: Retained<CALayer>,
    /// 확대 커서(시스템 커서 그림)
    cursor: Retained<CALayer>,
    /// 지금 층에 붙은 프레임(CVPixelBuffer, retain) — 붙어 있는 동안 SCK 풀이 다시 쓰지 않게 쥔다
    current: Option<CFTypeRef>,
}

impl Panel {
    /// 층의 확대 사각형과 커서 층을 합성하기 전의 BGRA 버퍼를 판서에 넘긴다.
    fn original_frame(&self, screen: &Placed) -> Option<Frame> {
        let pb = self.current?;
        // SAFETY: 패널이 붙잡은 완전한 표본이다. 메인 스레드에서 빌린 동안 교체될 수 없다.
        unsafe {
            let (w, h) = (CVPixelBufferGetWidth(pb), CVPixelBufferGetHeight(pb));
            if w != screen.phys.w as usize || h != screen.phys.h as usize || CVPixelBufferGetPixelFormatType(pb) != u32::from_be_bytes(*b"BGRA") {
                return None;
            }
            const READ_ONLY: u64 = 1;
            if CVPixelBufferLockBaseAddress(pb, READ_ONLY) != 0 {
                return None;
            }
            let base = CVPixelBufferGetBaseAddress(pb);
            let size = CVPixelBufferGetDataSize(pb);
            // 잠근 동안에만 주소를 빌린다. 판서 세션은 자체 사본을 쥔다.
            let bgra = if base.is_null() || size > isize::MAX as usize {
                None
            } else {
                copy_rows(std::slice::from_raw_parts(base, size), w, h, CVPixelBufferGetBytesPerRow(pb))
            };
            CVPixelBufferUnlockBaseAddress(pb, READ_ONLY);
            Some(Frame { monitor_id: screen.id, width: w as u32, height: h as u32, bgra: Arc::new(bgra?) })
        }
    }

    /// 프레임 하나를 붙이고 지난 것을 놓는다(None = 비우기). 암묵 애니메이션 없이.
    fn set_frame(&mut self, pb: Option<CFTypeRef>) {
        quiet(|| {
            // SAFETY: IOSurfaceRef는 IOSurface 객체와 무료 다리 — 층이 contents로 따로 쥔다(retain).
            unsafe {
                let surface = pb.map(|p| CVPixelBufferGetIOSurface(p)).filter(|s| !s.is_null());
                self.image.setContents(surface.map(|s| &*(s as *const AnyObject)));
            }
        });
        if let Some(old) = std::mem::replace(&mut self.current, pb) {
            // SAFETY: 이전에 retain한 것을 놓는다.
            unsafe { CFRelease(old) };
        }
    }
}

/// 행 끝 여백을 건너뛰고 알파를 불투명으로 맞춘다. 줄 순서와 원본 색은 유지한다.
fn copy_rows(src: &[u8], w: usize, h: usize, stride: usize) -> Option<Vec<u8>> {
    let bytes = w.checked_mul(4)?;
    if bytes == 0 || h == 0 || stride < bytes || src.len() < stride.checked_mul(h - 1)?.checked_add(bytes)? {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.checked_mul(h)?);
    for row in src.chunks(stride).take(h) {
        out.extend_from_slice(&row[..bytes]);
    }
    for pixel in out.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    Some(out)
}

thread_local! {
    static PANEL: RefCell<Option<Panel>> = const { RefCell::new(None) };
}

fn quiet(f: impl FnOnce()) {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    f();
    CATransaction::commit();
}

fn make_panel(mtm: MainThreadMarker) -> Panel {
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
    let window = NSPanel::initWithContentRect_styleMask_backing_defer(NSPanel::alloc(mtm), rect, style, NSBackingStoreType::Buffered, false);
    // SAFETY: 우리가 Retained로 쥐고 있으니 닫아도 놓이지 않게.
    unsafe { window.setReleasedWhenClosed(false) };
    window.setOpaque(true);
    window.setBackgroundColor(Some(&NSColor::blackColor()));
    window.setHasShadow(false);
    window.setIgnoresMouseEvents(true);
    window.setHidesOnDeactivate(false);
    window.setLevel(MAG_LEVEL);
    window.setCollectionBehavior(NSWindowCollectionBehavior(ALL_SPACES));
    // 확인용(SCREEN_INK_ZOOM_SHARE=1): 화면 찍기에 돋보기가 보이게 — 배율·뒤집기 실측에만 쓴다
    let share = std::env::var("SCREEN_INK_ZOOM_SHARE").is_ok_and(|v| v == "1");
    window.setSharingType(if share { NSWindowSharingType::ReadOnly } else { NSWindowSharingType::None });
    window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    // SAFETY: 창 색 공간을 SCK(sRGB)와 맞춘다(정지 패널과 같은 까닭 — 넘길 때 CPU 색 변환을 피한다).
    unsafe {
        let cs: *mut AnyObject = msg_send![objc2::class!(NSColorSpace), sRGBColorSpace];
        let _: () = msg_send![&*window, setColorSpace: cs];
    }
    let root = CALayer::layer();
    let image = CALayer::layer();
    let cursor = CALayer::layer();
    cursor.setHidden(true);
    root.addSublayer(&image);
    root.addSublayer(&cursor);
    let view = NSView::initWithFrame(NSView::alloc(mtm), rect);
    view.setLayer(Some(&root));
    view.setWantsLayer(true);
    window.setContentView(Some(&view));
    Panel { window, root, image, cursor, current: None }
}

/// SCK 표본 큐에서 프레임을 받는다. 첫 프레임은 `start`가 기다리는 채널로, 그다음은 메인 대기열로 붙인다.
/// 세대가 바뀌면(멈춤) 버린다.
struct Sink {
    excludes_self: bool,
    gen: Arc<AtomicU64>,
    mine: u64,
    first: Mutex<Option<mpsc::Sender<Hold<CFTypeRef>>>>,
}

impl Sink {
    fn on_frame(&self, buf: &CMSampleBuffer) {
        if self.gen.load(Ordering::SeqCst) != self.mine {
            return;
        }
        // SAFETY: SCK가 준 살아 있는 표본. 완전한 프레임만(화면이 그대로면 SCK가 이미지 없는 표본을 준다).
        let pb = unsafe {
            if !buf.is_valid() || !crate::recorder::frame_complete(buf) {
                return;
            }
            let pb = CMSampleBufferGetImageBuffer(buf);
            if pb.is_null() || CVPixelBufferGetIOSurface(pb).is_null() {
                return;
            }
            Hold(CFRetain(pb))
        };
        if let Some(tx) = self.first.lock().unwrap_or_else(|e| e.into_inner()).take() {
            if let Err(mpsc::SendError(pb)) = tx.send(pb) {
                // SAFETY: 받을 쪽이 없다 — retain한 것을 놓는다.
                unsafe { CFRelease(pb.0) };
            }
            return;
        }
        let (gen, mine) = (self.gen.clone(), self.mine);
        dispatch2::DispatchQueue::main().exec_async(move || {
            let pb = pb;
            let used = gen.load(Ordering::SeqCst) == mine
                && PANEL.with_borrow_mut(|p| match p.as_mut() {
                    Some(p) => {
                        p.set_frame(Some(pb.0));
                        true
                    }
                    None => false,
                });
            if !used {
                // SAFETY: 붙이지 않았다 — retain한 것을 놓는다.
                unsafe { CFRelease(pb.0) };
            }
        });
    }
}

define_class!(
    // SAFETY: NSObject는 하위 클래스 요구가 없고, Drop을 구현하지 않는다.
    #[unsafe(super(NSObject))]
    #[name = "ShookshokZoomOutput"]
    #[ivars = Sink]
    struct ZoomOutput;

    unsafe impl NSObjectProtocol for ZoomOutput {}

    unsafe impl SCStreamOutput for ZoomOutput {
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        fn stream_did_output(&self, _stream: &SCStream, buf: &CMSampleBuffer, kind: SCStreamOutputType) {
            if kind == SCStreamOutputType::Screen {
                self.ivars().on_frame(buf);
            }
        }
    }
);

impl ZoomOutput {
    fn new(sink: Sink) -> Retained<Self> {
        let this = Self::alloc().set_ivars(sink);
        // SAFETY: NSObject init.
        unsafe { msg_send![super(this), init] }
    }
}

fn wait_done(f: impl FnOnce(&block2::DynBlock<dyn Fn(*mut NSError)>), wait: Duration) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let block = RcBlock::new(move |err: *mut NSError| {
        // SAFETY: SCK가 넘긴 오류 포인터는 이 호출 동안 유효하다.
        let _ = tx.send(match unsafe { err.as_ref() } {
            Some(e) => Err(format!("{}", e.code())),
            None => Ok(()),
        });
    });
    f(&block);
    rx.recv_timeout(wait).map_err(|_| "timed out".to_string())?
}

/// 뒤에 있는 앱도 커서를 숨길 수 있게 하는 비공개 연결 속성. 함수가 없거나 실패하면 false.
fn allow_background_cursor() -> bool {
    type DefaultConnection = unsafe extern "C" fn() -> i32;
    type SetProperty = unsafe extern "C" fn(i32, i32, CFTypeRef, CFTypeRef) -> i32;
    // SAFETY: 이름으로 찾은 함수만 부른다(서명은 널리 쓰이는 SkyLight 선언). 문자열은 여기서 만들고 놓는다.
    unsafe {
        let conn = dlsym(RTLD_DEFAULT, c"_CGSDefaultConnection".as_ptr());
        let set = dlsym(RTLD_DEFAULT, c"CGSSetConnectionProperty".as_ptr());
        if conn.is_null() || set.is_null() {
            return false;
        }
        let conn: DefaultConnection = std::mem::transmute::<*mut c_void, DefaultConnection>(conn);
        let set: SetProperty = std::mem::transmute::<*mut c_void, SetProperty>(set);
        let name = b"SetsCursorInBackground";
        let key = CFStringCreateWithBytes(std::ptr::null(), name.as_ptr(), name.len() as isize, UTF8, false);
        if key.is_null() {
            return false;
        }
        let cid = conn();
        let r = set(cid, cid, key, kCFBooleanTrue);
        CFRelease(key);
        r == 0
    }
}

/// 커서 화소(실수 물리 px)가 확대 화면에서 놓이는 자리 — 그 화소 가운데를 맞춘다(도메인 `screen_of`와 같은 고른 배율).
pub fn magnified_point(monitor: PhysRect, source: PhysRect, scale: f64, cursor: (f64, f64)) -> (f64, f64) {
    let f = |m: i32, s: i32, c: f64| m as f64 + (c + 0.5 - s as f64) * scale - 0.5;
    (f(monitor.x, source.x, cursor.0), f(monitor.y, source.y, cursor.1))
}

/// 지금 시스템 커서 그림·핫스팟·크기(포인트). Apple이 `currentSystemCursor`를 nil로 바꾸면 None.
struct Shape {
    image: Retained<objc2_core_graphics::CGImage>,
    hot: (f64, f64),
    size: (f64, f64),
}

fn cursor_shape() -> Option<Shape> {
    #[allow(deprecated)]
    let c = NSCursor::currentSystemCursor()?;
    let image = c.image();
    let size = image.size();
    if size.width <= 0.0 || size.height <= 0.0 {
        return None;
    }
    let hot = c.hotSpot();
    let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), size);
    // SAFETY: 제안 사각형은 이 호출 동안만 쓰인다.
    let cg = unsafe { image.CGImageForProposedRect_context_hints(&mut rect, None, None) }?;
    Some(Shape { image: cg, hot: (hot.x, hot.y), size: (size.width, size.height) })
}

/// 진짜 커서: 그 모니터 안이면 숨기고 확대 커서를 그린다, 밖이면 보인다(숨긴 만큼만 되돌린다).
struct CursorState {
    allowed: bool,
    hidden: bool,
    ticks: u32,
    shape: Option<Shape>,
}

impl CursorState {
    fn sync(&mut self, placed: &Placed, monitor: PhysRect, source: PhysRect, scale: f64) {
        if !self.allowed {
            return;
        }
        let (x, y) = crate::display::cursor_pt();
        // 실수 물리 px(그 화면 기준) — 정수로 자르지 않아 확대 커서가 화소 단위로 튀지 않는다
        let c = (placed.phys.x as f64 + (x - placed.pt.x) * placed.scale, placed.phys.y as f64 + (y - placed.pt.y) * placed.scale);
        let inside = x >= placed.pt.x && x < placed.pt.right() && y >= placed.pt.y && y < placed.pt.bottom();
        if inside != self.hidden {
            // SAFETY: 숨김·보임은 짝을 맞춘다(hidden 상태로 센다).
            let ok = unsafe { if inside { CGDisplayHideCursor(CGMainDisplayID()) } else { CGDisplayShowCursor(CGMainDisplayID()) } } == 0;
            if ok {
                self.hidden = inside;
            }
        }
        if self.ticks % CURSOR_SHAPE_TICKS == 0 || self.shape.is_none() {
            self.shape = cursor_shape();
        }
        self.ticks = self.ticks.wrapping_add(1);
        let (qx, qy) = magnified_point(monitor, source, scale, c);
        PANEL.with_borrow(|p| {
            let Some(p) = p.as_ref() else { return };
            quiet(|| match (&self.shape, self.hidden) {
                (Some(shape), true) => {
                    let k = scale.clamp(1.0, CURSOR_MAX_SCALE);
                    let (w, h) = (shape.size.0 * k, shape.size.1 * k);
                    // 확대 자리(물리 px) → 이 화면 포인트(왼쪽 위 원점) → 층(왼쪽 아래 원점)
                    let left = (qx - monitor.x as f64) / placed.scale - shape.hot.0 * k;
                    let top = (qy - monitor.y as f64) / placed.scale - shape.hot.1 * k;
                    // SAFETY: CGImage는 CALayer 내용으로 받는 CF 형식이다(무료 다리).
                    unsafe { p.cursor.setContents(Some(&*(&*shape.image as *const objc2_core_graphics::CGImage as *const AnyObject))) };
                    p.cursor.setFrame(cg(RectF::new(left, placed.pt.h - top - h, w, h)));
                    p.cursor.setHidden(false);
                }
                _ => p.cursor.setHidden(true),
            });
        });
    }

    fn restore(&mut self) {
        if self.hidden {
            // SAFETY: 숨긴 것 한 번을 되돌린다.
            unsafe {
                let _ = CGDisplayShowCursor(CGMainDisplayID());
            }
            self.hidden = false;
        }
        PANEL.with_borrow(|p| {
            if let Some(p) = p.as_ref() {
                quiet(|| p.cursor.setHidden(true));
            }
        });
    }
}

struct Live {
    stream: Hold<Retained<SCStream>>,
    output: Hold<Retained<ZoomOutput>>,
    monitor: PhysRect,
    placed: Placed,
    cursor: CursorState,
    _wheel: Option<crate::wheel::WheelTap>,
}

type InputFn = Arc<dyn Fn(ink_app::ZoomInput) + Send + Sync>;

/// `Magnifier`(맥). `on_input`(휠 칸·트랙패드·Esc)은 가로채기 스레드에서 불린다(잠금 없이 메인 대기열로만 넘길 것).
pub struct MacMagnifier {
    live: Option<Live>,
    gen: Arc<AtomicU64>,
    on_input: InputFn,
}

/// 가로챈 사건 → 서비스 입력.
pub fn to_input(ev: crate::wheel::TapEvent) -> ink_app::ZoomInput {
    match ev {
        crate::wheel::TapEvent::Notches(dy) => ink_app::ZoomInput::Wheel { dy },
        crate::wheel::TapEvent::Scroll(notches) => ink_app::ZoomInput::Scroll { notches },
        crate::wheel::TapEvent::Escape => ink_app::ZoomInput::Exit,
    }
}

impl MacMagnifier {
    pub fn new(on_input: impl Fn(ink_app::ZoomInput) + Send + Sync + 'static) -> Self {
        MacMagnifier { live: None, gen: Arc::new(AtomicU64::new(0)), on_input: Arc::new(on_input) }
    }

    fn open_stream(&self, monitor: PhysRect, display: u32, mine: u64, first: mpsc::Sender<Hold<CFTypeRef>>) -> Result<(Retained<SCStream>, Retained<ZoomOutput>), String> {
        let display = crate::capture::shareable_display(display)?;
        let me = crate::capture::shareable_me();
        let output = ZoomOutput::new(Sink { excludes_self: me.is_some(), gen: self.gen.clone(), mine, first: Mutex::new(Some(first)) });
        // SAFETY: 필터·설정·스트림은 만든 뒤 SCK에 넘긴다. 표본 큐는 직렬.
        let stream = unsafe {
            let apps: Retained<NSArray<SCRunningApplication>> = match &me {
                Some(a) => NSArray::from_retained_slice(std::slice::from_ref(a)),
                None => NSArray::new(),
            };
            let filter = SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(SCContentFilter::alloc(), &display, &apps, &NSArray::new());
            let config = SCStreamConfiguration::new();
            config.setCaptureResolution(objc2_screen_capture_kit::SCCaptureResolutionType::Best);
            config.setWidth(monitor.w.max(2) as usize);
            config.setHeight(monitor.h.max(2) as usize);
            config.setMinimumFrameInterval(CMTime::new(1, 60));
            // 커서는 우리가 그린다(숨긴 커서는 SCK가 찍지 않는다)
            config.setShowsCursor(false);
            config.setPixelFormat(u32::from_be_bytes(*b"BGRA"));
            config.setColorSpaceName(objc2_core_graphics::kCGColorSpaceSRGB);
            config.setQueueDepth(5);
            let stream = SCStream::initWithFilter_configuration_delegate(SCStream::alloc(), &filter, &config, None);
            let queue = dispatch2::DispatchQueue::new("kr.shookshok.zoom", None);
            stream
                .addStreamOutput_type_sampleHandlerQueue_error(ProtocolObject::from_ref(&*output), SCStreamOutputType::Screen, Some(&queue))
                .map_err(|e| format!("stream output: {}", e.code()))?;
            stream
        };
        // SAFETY: 살아 있는 스트림.
        wait_done(|b| unsafe { stream.startCaptureWithCompletionHandler(Some(b)) }, START_WAIT).map_err(|e| format!("zoom stream did not start: {e}"))?;
        Ok((stream, output))
    }
}

impl ink_app::Magnifier for MacMagnifier {
    fn start(&mut self, monitor: PhysRect, source: PhysRect, scale: f64) -> Result<(), String> {
        let mtm = MainThreadMarker::new().ok_or("magnifier must start on the main thread")?;
        ink_app::Magnifier::stop(self);
        if !crate::permissions::screen() {
            return Err(ink_app::NO_SCREEN_PERMISSION.into());
        }
        let t = Instant::now();
        let space = crate::space();
        let placed = *space.screen_of(monitor).ok_or("monitor not in space")?;
        let frame = screen_frame(mtm, &placed).ok_or("no screen frame")?;
        let mine = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, rx) = mpsc::channel();
        let (stream, output) = self.open_stream(monitor, placed.display, mine, tx)?;
        let Ok(first) = rx.recv_timeout(FIRST_FRAME_WAIT) else {
            self.gen.fetch_add(1, Ordering::SeqCst);
            // SAFETY: 살아 있는 스트림을 멈춘다(실패 길).
            unsafe { stream.stopCaptureWithCompletionHandler(None) };
            return Err("zoom first frame timed out".into());
        };
        let streamed = t.elapsed();
        PANEL.with_borrow_mut(|p| {
            let p = p.get_or_insert_with(|| make_panel(mtm));
            p.window.setFrame_display(frame, false);
            let full = cg(RectF::new(0.0, 0.0, placed.pt.w, placed.pt.h));
            let r = contents_rect(monitor, source, scale);
            quiet(|| {
                p.root.setFrame(full);
                p.image.setFrame(full);
                p.image.setContentsRect(cg(r));
            });
            p.set_frame(Some(first.0));
            CATransaction::flush();
            p.window.orderFrontRegardless();
        });
        let mut cursor = CursorState { allowed: allow_background_cursor(), hidden: false, ticks: 0, shape: None };
        cursor.sync(&placed, monitor, source, scale);
        let cb = self.on_input.clone();
        let wheel = crate::wheel::start(monitor, move |ev| cb(to_input(ev)));
        log::info!(
            "zoom started: first frame {:.0} ms, shown {:.0} ms (cursor hide {}, wheel tap {})",
            streamed.as_secs_f64() * 1000.0,
            t.elapsed().as_secs_f64() * 1000.0,
            if cursor.allowed { "on" } else { "off" },
            if wheel.is_some() { "on" } else { "off" }
        );
        self.live = Some(Live { stream: Hold(stream), output: Hold(output), monitor, placed, cursor, _wheel: wheel });
        Ok(())
    }

    fn update(&mut self, source: PhysRect, scale: f64) -> Result<(), String> {
        let Some(live) = self.live.as_mut() else { return Err("zoom not running".into()) };
        let r = contents_rect(live.monitor, source, scale);
        PANEL.with_borrow(|p| {
            if let Some(p) = p.as_ref() {
                quiet(|| p.image.setContentsRect(cg(r)));
            }
        });
        live.cursor.sync(&live.placed, live.monitor, source, scale);
        Ok(())
    }

    fn original_frame(&self) -> Option<Frame> {
        let live = self.live.as_ref()?;
        if !live.output.0.ivars().excludes_self || crate::refresh_space().screen_of(live.monitor) != Some(&live.placed) {
            return None;
        }
        PANEL.with_borrow(|p| p.as_ref()?.original_frame(&live.placed))
    }

    fn stop(&mut self) {
        let Some(mut live) = self.live.take() else { return };
        self.gen.fetch_add(1, Ordering::SeqCst);
        live.cursor.restore();
        PANEL.with_borrow_mut(|p| {
            if let Some(p) = p.as_mut() {
                p.window.orderOut(None);
                p.set_frame(None);
            }
        });
        // SAFETY: 살아 있는 스트림을 멈춘다(완료를 기다리지 않는다 — 세대가 바뀌어 늦은 프레임은 버려진다).
        unsafe { live.stream.0.stopCaptureWithCompletionHandler(None) };
        // 휠 탭은 live와 함께 놓인다(Drop = 실행 고리 멈춤)
    }

    fn wait_hidden(&mut self) {
        // 패널을 내린 것을 창 서버로 보내고 화면 갱신 한 번(60 Hz ≈ 17 ms)의 두 배를 기다린다
        CATransaction::flush();
        std::thread::sleep(Duration::from_millis(34));
    }
}

impl Drop for MacMagnifier {
    fn drop(&mut self) {
        ink_app::Magnifier::stop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 판서_원본은_행_여백을_빼고_색을_보존하며_알파를_불투명하게_한다() {
        let rows = [1, 2, 3, 0, 4, 5, 6, 7, 99, 99, 8, 9, 10, 11, 12, 13, 14, 15];
        assert_eq!(copy_rows(&rows, 2, 2, 10).unwrap(), [1, 2, 3, 255, 4, 5, 6, 255, 8, 9, 10, 255, 12, 13, 14, 255]);
        assert!(copy_rows(&rows[..17], 2, 2, 10).is_none());
        assert!(copy_rows(&rows, 2, 2, 7).is_none());
    }

    #[test]
    fn 원본_사각형은_아래에서_잰_정규화_사각형() {
        let mon = PhysRect::new(0, 0, 3840, 2160);
        // 2배: 가운데 1920×1080
        assert_eq!(contents_rect(mon, PhysRect::new(960, 540, 1920, 1080), 2.0), RectF::new(0.25, 0.25, 0.5, 0.5));
        // 왼쪽 위 1/4 = 층 좌표로는 위쪽(y 0.5부터)
        assert_eq!(contents_rect(mon, PhysRect::new(0, 0, 1920, 1080), 2.0), RectF::new(0.0, 0.5, 0.5, 0.5));
        // 오른쪽 모니터(3840,0 1920×1080)의 오른쪽 아래 1/4
        let right = PhysRect::new(3840, 0, 1920, 1080);
        assert_eq!(contents_rect(right, PhysRect::new(3840 + 960, 540, 960, 540), 2.0), RectF::new(0.5, 0.0, 0.5, 0.5));
        // 나누어떨어지지 않는 배율: 크기는 올린 원본이 아니라 1/배율(가로·세로 같은 배율)
        let r = contents_rect(mon, PhysRect::new(0, 0, 1763, 992), 2.18);
        assert!((r.w - 1.0 / 2.18).abs() < 1e-12 && (r.h - 1.0 / 2.18).abs() < 1e-12, "{r:?}");
        // 1×: 그림 전체
        assert_eq!(contents_rect(mon, mon, 1.0), RectF::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn 확대_커서_자리는_커서_화소_가운데를_맞춘다() {
        let mon = PhysRect::new(0, 0, 3840, 2160);
        // 1×: 제자리
        assert_eq!(magnified_point(mon, mon, 1.0, (100.0, 200.0)), (100.0, 200.0));
        // 2×, 원본 왼쪽 위(960, 540): (1000, 600) 화소 → (40.5×2 − 0.5, 60.5×2 − 0.5)
        assert_eq!(magnified_point(mon, PhysRect::new(960, 540, 1920, 1080), 2.0, (1000.0, 600.0)), (80.5, 120.5));
        // 오른쪽 모니터도 그 모니터 기준
        let right = PhysRect::new(3840, 0, 1920, 1080);
        assert_eq!(magnified_point(right, PhysRect::new(3840, 0, 960, 540), 2.0, (3840.0, 0.0)), (3840.5, 0.5));
    }
}
