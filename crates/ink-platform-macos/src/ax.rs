//! A 근거: macOS 접근성(AX) 색인 일꾼(M9, 08 §4.2 A·§4.7, I-mac §2.1). Windows `uia.rs`와 같은 구조.
//!
//! - 전용 스레드 하나가 창의 하위 트리를 너비 우선으로 읽어 메모리에 둔다. 커서가 움직일 때는 이 메모리만 본다.
//! - 요소마다 역할·부역할·자리·크기·자식을 **한 번의 요청**(`AXUIElementCopyMultipleAttributeValues`)으로 받는다.
//! - 응답 없는 앱에 묶이지 않게 요청마다 400 ms 제한(`AXUIElementSetMessagingTimeout`, D17), 창 하나에 300 ms 예산.
//! - 모드가 바뀌면 세대 번호를 올려 늦게 끝난 옛 결과를 버린다.
//! - 손쉬운 사용 권한이 없으면 묻지 않고 빈 결과(W·V 근거로 그대로 캡처된다, 권한 안내는 M10).
//! - 창은 비공개 `_AXUIElementGetWindow` 대신 CG 창 경계와 AX 창 자리·크기를 견줘 찾는다(I-mac §3).
//! - 글자 값·제목은 읽지 않는다(05 개인정보) — 역할과 사각형만.

use crate::coords::{RectF, Space};
use ink_domain::geometry::{PhysPoint, PhysRect};
use ink_domain::recog::{Control, ElementCand};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 창 하나에서 방문할 요소 상한·남길 요소 상한(08 §4.7: 1500개)·깊이 상한.
const MAX_VISIT: usize = 4000;
const KEEP_ELEMENTS: usize = 1500;
const MAX_DEPTH: usize = 12;
/// 창 하나 색인 예산 — 넘으면 그때까지 모은 것으로 끝낸다(느린 앱 강등 문턱 500 ms 안).
const BUDGET: Duration = Duration::from_millis(300);
/// AX 요청 하나의 응답 제한(초).
const MESSAGING_TIMEOUT: f32 = 0.4;
/// CG 창 경계와 AX 창 자리를 같다고 볼 오차(포인트).
const MATCH_TOL: f64 = 2.0;

type CFTypeRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementSetMessagingTimeout(element: CFTypeRef, seconds: f32) -> i32;
    fn AXUIElementCopyAttributeValue(element: CFTypeRef, attribute: CFTypeRef, value: *mut CFTypeRef) -> i32;
    fn AXUIElementCopyMultipleAttributeValues(element: CFTypeRef, attributes: CFTypeRef, options: u32, values: *mut CFTypeRef) -> i32;
    fn AXValueGetTypeID() -> usize;
    fn AXValueGetValue(value: CFTypeRef, kind: u32, out: *mut c_void) -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFTypeArrayCallBacks: c_void;
    fn CFRelease(cf: CFTypeRef);
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn CFGetTypeID(cf: CFTypeRef) -> usize;
    fn CFArrayGetTypeID() -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFArrayCreate(alloc: CFTypeRef, values: *const CFTypeRef, count: isize, callbacks: *const c_void) -> CFTypeRef;
    fn CFArrayGetCount(array: CFTypeRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFTypeRef, index: isize) -> CFTypeRef;
    fn CFStringCreateWithBytes(alloc: CFTypeRef, bytes: *const u8, len: isize, encoding: u32, external: bool) -> CFTypeRef;
    fn CFStringGetCString(s: CFTypeRef, buf: *mut u8, size: isize, encoding: u32) -> bool;
}

const UTF8: u32 = 0x0800_0100;
const AX_POINT: u32 = 1;
const AX_SIZE: u32 = 2;

/// CF 객체 하나를 쥐고 있다가 놓는다(소유 규칙: Create·Copy로 받은 것).
struct Cf(CFTypeRef);

impl Cf {
    fn new(p: CFTypeRef) -> Option<Cf> {
        (!p.is_null()).then_some(Cf(p))
    }
    /// 빌린 참조(배열 원소)를 쥔다.
    fn retain(p: CFTypeRef) -> Option<Cf> {
        // SAFETY: 살아 있는 CF 객체의 참조 수만 올린다.
        (!p.is_null()).then(|| Cf(unsafe { CFRetain(p) }))
    }
}

impl Drop for Cf {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: new·retain으로 받은 소유 참조를 한 번만 놓는다.
            unsafe { CFRelease(self.0) }
        }
    }
}

// SAFETY: AX 요소·CF 문자열은 불변 객체로, 일꾼 스레드 한 곳에서만 쓴다(스레드 사이로 보내기만 함).
unsafe impl Send for Cf {}

fn cfstr(s: &str) -> Cf {
    // SAFETY: UTF-8 바이트로 새 CFString을 만든다(소유 참조, 실패하면 null — Drop이 거른다).
    Cf(unsafe { CFStringCreateWithBytes(std::ptr::null(), s.as_ptr(), s.len() as isize, UTF8, false) })
}

fn string_of(v: CFTypeRef) -> Option<String> {
    // SAFETY: 형 확인 뒤 CFString에서 UTF-8로 복사.
    unsafe {
        if v.is_null() || CFGetTypeID(v) != CFStringGetTypeID() {
            return None;
        }
        let mut buf = [0u8; 64];
        CFStringGetCString(v, buf.as_mut_ptr(), buf.len() as isize, UTF8).then(|| {
            let n = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            String::from_utf8_lossy(&buf[..n]).into_owned()
        })
    }
}

fn point_of(v: CFTypeRef) -> Option<(f64, f64)> {
    let mut p = [0f64; 2];
    // SAFETY: 형 확인 뒤 AXValue에서 CGPoint(f64 둘)를 꺼낸다.
    unsafe { (!v.is_null() && CFGetTypeID(v) == AXValueGetTypeID() && AXValueGetValue(v, AX_POINT, p.as_mut_ptr().cast())).then_some((p[0], p[1])) }
}

fn size_of(v: CFTypeRef) -> Option<(f64, f64)> {
    let mut s = [0f64; 2];
    // SAFETY: 형 확인 뒤 AXValue에서 CGSize(f64 둘)를 꺼낸다.
    unsafe { (!v.is_null() && CFGetTypeID(v) == AXValueGetTypeID() && AXValueGetValue(v, AX_SIZE, s.as_mut_ptr().cast())).then_some((s[0], s[1])) }
}

/// 배열 원소를 쥔 목록으로.
fn array_items(v: CFTypeRef) -> Vec<Cf> {
    // SAFETY: 형 확인 뒤 원소를 하나씩 retain.
    unsafe {
        if v.is_null() || CFGetTypeID(v) != CFArrayGetTypeID() {
            return Vec::new();
        }
        (0..CFArrayGetCount(v)).filter_map(|i| Cf::retain(CFArrayGetValueAtIndex(v, i))).collect()
    }
}

/// 한 요소의 사실값(역할·자리·자식).
struct Node {
    role: String,
    subrole: String,
    rect: Option<RectF>,
    children: Vec<Cf>,
}

/// 요청에 쓰는 속성 이름 배열(역할·부역할·자리·크기·자식) — 일꾼마다 한 번 만든다.
struct Attrs {
    many: Cf,
    _names: Vec<Cf>,
    windows: Cf,
    position: Cf,
    size: Cf,
}

impl Attrs {
    fn new() -> Attrs {
        let names: Vec<Cf> = ["AXRole", "AXSubrole", "AXPosition", "AXSize", "AXChildren"].iter().map(|s| cfstr(s)).collect();
        let ptrs: Vec<CFTypeRef> = names.iter().map(|c| c.0).collect();
        // SAFETY: 위 문자열들을 담는 CFArray(원소 retain 콜백).
        let many = Cf(unsafe { CFArrayCreate(std::ptr::null(), ptrs.as_ptr(), ptrs.len() as isize, &kCFTypeArrayCallBacks) });
        Attrs { many, _names: names, windows: cfstr("AXWindows"), position: cfstr("AXPosition"), size: cfstr("AXSize") }
    }

    fn copy(&self, e: &Cf, attr: &Cf) -> Option<Cf> {
        let mut out: CFTypeRef = std::ptr::null();
        // SAFETY: 살아 있는 AX 요소·속성 이름, 결과는 우리가 소유.
        let err = unsafe { AXUIElementCopyAttributeValue(e.0, attr.0, &mut out) };
        if err == 0 {
            Cf::new(out)
        } else {
            None
        }
    }

    fn node(&self, e: &Cf) -> Option<Node> {
        let mut out: CFTypeRef = std::ptr::null();
        // SAFETY: 살아 있는 AX 요소·속성 이름 배열, 결과 배열은 우리가 소유(없는 속성은 AXValue 오류 값으로 온다).
        let err = unsafe { AXUIElementCopyMultipleAttributeValues(e.0, self.many.0, 0, &mut out) };
        let vals = Cf::new(out).filter(|_| err == 0)?;
        let at = |i: isize| {
            // SAFETY: 요청한 속성 수(5)만큼의 배열 — 범위를 확인한다.
            unsafe { if i < CFArrayGetCount(vals.0) { CFArrayGetValueAtIndex(vals.0, i) } else { std::ptr::null() } }
        };
        let rect = match (point_of(at(2)), size_of(at(3))) {
            (Some((x, y)), Some((w, h))) => Some(RectF::new(x, y, w, h)),
            _ => None,
        };
        Some(Node { role: string_of(at(0)).unwrap_or_default(), subrole: string_of(at(1)).unwrap_or_default(), rect, children: array_items(at(4)) })
    }

    fn rect_of(&self, e: &Cf) -> Option<RectF> {
        let p = self.copy(e, &self.position).and_then(|v| point_of(v.0))?;
        let s = self.copy(e, &self.size).and_then(|v| size_of(v.0))?;
        Some(RectF::new(p.0, p.1, s.0, s.1))
    }
}

/// CG 창 경계(포인트)에 맞는 AX 창 — 자리·크기가 오차 안이면 그것, 없으면 겹침이 가장 큰 것.
fn find_window(attrs: &Attrs, app: &Cf, cg: RectF) -> Option<Cf> {
    let list = attrs.copy(app, &attrs.windows)?;
    let mut best: Option<(f64, Cf)> = None;
    for w in array_items(list.0) {
        let Some(r) = attrs.rect_of(&w) else { continue };
        if (r.x - cg.x).abs() <= MATCH_TOL && (r.y - cg.y).abs() <= MATCH_TOL && (r.w - cg.w).abs() <= MATCH_TOL && (r.h - cg.h).abs() <= MATCH_TOL {
            return Some(w);
        }
        let ix = (r.right().min(cg.right()) - r.x.max(cg.x)).max(0.0);
        let iy = (r.bottom().min(cg.bottom()) - r.y.max(cg.y)).max(0.0);
        let area = ix * iy;
        if area > 0.0 && best.as_ref().map_or(true, |(a, _)| area > *a) {
            best = Some((area, w));
        }
    }
    best.map(|(_, w)| w)
}

/// AX 역할 → 종류. 탭 단추는 부역할로 가린다.
fn control_of(role: &str, subrole: &str) -> Control {
    if subrole == "AXTabButton" {
        return Control::TabItem;
    }
    Control::from_ax(role)
}

/// 창 하나를 너비 우선으로 색인(창 자신은 빼고 자손만, 08 §4.2 A). 사각형은 창 안으로 자른 물리 px.
fn index_window(attrs: &Attrs, space: &Space, pid: i32, cg: RectF) -> Vec<ElementCand> {
    // SAFETY: pid로 앱 AX 요소를 만들고 요청 제한을 건다(소유 참조).
    let Some(app) = Cf::new(unsafe { AXUIElementCreateApplication(pid) }) else { return Vec::new() };
    // SAFETY: 살아 있는 AX 요소.
    unsafe { AXUIElementSetMessagingTimeout(app.0, MESSAGING_TIMEOUT) };
    let Some(win) = find_window(attrs, &app, cg) else { return Vec::new() };
    let win_phys = space.rect_to_phys(cg);
    let started = Instant::now();
    let mut out = Vec::new();
    let mut visits = 0usize;
    let mut queue: VecDeque<(Cf, usize)> = VecDeque::new();
    if let Some(root) = attrs.node(&win) {
        queue.extend(root.children.into_iter().map(|c| (c, 1)));
    }
    while let Some((e, depth)) = queue.pop_front() {
        visits += 1;
        if visits > MAX_VISIT || out.len() >= KEEP_ELEMENTS || started.elapsed() > BUDGET {
            break;
        }
        let Some(n) = attrs.node(&e) else { continue };
        if let Some(r) = n.rect {
            // 화면 밖(스크롤로 가려진 것)은 창과 겹치는 부분만, 너무 작은 것은 뺀다
            if let Some(rect) = space.rect_to_phys(r).intersect(&win_phys) {
                if rect.w >= 4 && rect.h >= 4 {
                    out.push(ElementCand { rect, control: control_of(&n.role, &n.subrole), from_msaa: false });
                }
            }
        }
        if depth < MAX_DEPTH {
            queue.extend(n.children.into_iter().map(|c| (c, depth + 1)));
        }
    }
    out
}

type Ready = Box<dyn Fn(u64, String, f64) + Send + Sync>;

struct Job {
    window: u64,
    exe: String,
    gen: u64,
}

#[derive(Clone)]
pub struct AxIndex {
    tx: mpsc::Sender<Job>,
    cache: Arc<Mutex<HashMap<u64, Vec<ElementCand>>>>,
    pending: Arc<Mutex<HashSet<u64>>>,
    gen: Arc<AtomicU64>,
}

impl AxIndex {
    /// 일꾼을 띄운다. `on_ready(창, 실행 파일, 걸린 ms)`는 일꾼 스레드에서 불린다.
    pub fn spawn(on_ready: impl Fn(u64, String, f64) + Send + Sync + 'static) -> AxIndex {
        let (tx, rx) = mpsc::channel::<Job>();
        let cache: Arc<Mutex<HashMap<u64, Vec<ElementCand>>>> = Arc::default();
        let pending: Arc<Mutex<HashSet<u64>>> = Arc::default();
        let gen = Arc::new(AtomicU64::new(1));
        let (c, p, g) = (cache.clone(), pending.clone(), gen.clone());
        let on_ready: Ready = Box::new(on_ready);
        let _ = std::thread::Builder::new().name("screen-ink-ax".into()).spawn(move || {
            let attrs = Attrs::new();
            let warned = AtomicBool::new(false);
            while let Ok(job) = rx.recv() {
                if job.gen != g.load(Ordering::SeqCst) {
                    p.lock().unwrap_or_else(|e| e.into_inner()).remove(&job.window);
                    continue;
                }
                let t = Instant::now();
                let trusted = crate::permissions::accessibility();
                if !trusted && !warned.swap(true, Ordering::SeqCst) {
                    log::info!("ax index skipped: accessibility not granted");
                }
                let found = match (trusted, crate::windows::owner_and_bounds(job.window as u32)) {
                    (true, Some((pid, cg))) => index_window(&attrs, &crate::space(), pid, cg),
                    _ => Vec::new(),
                };
                let took = t.elapsed().as_secs_f64() * 1000.0;
                p.lock().unwrap_or_else(|e| e.into_inner()).remove(&job.window);
                if job.gen != g.load(Ordering::SeqCst) {
                    continue;
                }
                if trusted {
                    log::info!("ax index {} elements in {} ms", found.len(), took.round() as i64);
                }
                c.lock().unwrap_or_else(|e| e.into_inner()).insert(job.window, found);
                on_ready(job.window, job.exe, took);
            }
        });
        AxIndex { tx, cache, pending, gen }
    }

    pub fn request(&self, window: u64, exe: &str) {
        if self.cache.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&window) {
            return;
        }
        if !self.pending.lock().unwrap_or_else(|e| e.into_inner()).insert(window) {
            return;
        }
        let _ = self.tx.send(Job { window, exe: exe.to_string(), gen: self.gen.load(Ordering::SeqCst) });
    }

    pub fn elements_at(&self, window: u64, p: PhysPoint) -> Vec<ElementCand> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(&window).map(|v| v.iter().filter(|e| e.rect.contains(p)).copied().collect()).unwrap_or_default()
    }

    /// 모드 끝: 색인을 버리고 세대를 올린다(늦게 끝난 결과는 버려짐).
    pub fn clear(&self) {
        self.gen.fetch_add(1, Ordering::SeqCst);
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

impl ink_app::ElementSource for AxIndex {
    fn request(&self, window: u64, exe: &str) {
        AxIndex::request(self, window, exe)
    }

    fn elements_at(&self, window: u64, p: PhysPoint) -> Vec<ElementCand> {
        AxIndex::elements_at(self, window, p)
    }

    fn rects(&self, window: u64) -> Vec<PhysRect> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(&window).map(|v| v.iter().map(|e| e.rect).collect()).unwrap_or_default()
    }

    fn elements(&self, window: u64) -> Vec<ElementCand> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(&window).cloned().unwrap_or_default()
    }

    fn clear(&self) {
        AxIndex::clear(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 탭_단추는_부역할로() {
        assert_eq!(control_of("AXRadioButton", "AXTabButton"), Control::TabItem);
        assert_eq!(control_of("AXRadioButton", ""), Control::Button);
        assert_eq!(control_of("AXButton", "AXCloseButton"), Control::Button);
    }

    /// 실제 창 색인(손쉬운 사용 권한이 있는 셸에서 손으로): 맨 앞 앱 창 하나를 색인해 개수·시간·종류를 찍는다.
    /// `cargo test -p ink-platform-macos ax::tests::실제_맨앞_창_색인 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn 실제_맨앞_창_색인() {
        assert!(crate::permissions::accessibility(), "손쉬운 사용 권한이 필요하다");
        let space = crate::space();
        let windows = crate::windows::top_windows(&space);
        let w = windows.iter().find(|w| !w.topmost && w.rect.w > 200 && w.rect.h > 200).expect("앱 창");
        let (pid, cg) = crate::windows::owner_and_bounds(w.handle as u32).expect("창 정보");
        let attrs = Attrs::new();
        let t = Instant::now();
        let found = index_window(&attrs, &space, pid, cg);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let mut kinds: HashMap<String, usize> = HashMap::new();
        for e in &found {
            *kinds.entry(format!("{:?}", e.control)).or_default() += 1;
        }
        eprintln!("{} ({}): {} elements in {ms:.0} ms, kinds {kinds:?}", w.exe, w.handle, found.len());
        assert!(found.iter().all(|e| e.rect.intersect(&w.rect).is_some()), "모두 창 안");
    }
}
