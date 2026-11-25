//! A 근거: UIA 색인 일꾼(08 §4.2 A, §4.7, D11·D17·D18·D19).
//!
//! - 전용 MTA 스레드 하나가 창의 하위 트리를 **캐시 요청 한 번**(사각형·종류·화면 밖)으로 받아 메모리에 둔다.
//! - 커서가 움직일 때는 이 메모리만 본다 — 느린 앱이 있어도 화면이 굳지 않는다.
//! - 응답 없는 창은 묻지 않고(SendMessageTimeout SMTO_ABORTIFHUNG 50 ms), 연결·처리 시간 제한을 짧게 둔다.
//! - 모드가 바뀌면 세대 번호를 올려 늦게 끝난 옛 결과를 버린다.

use ink_domain::geometry::{PhysPoint, PhysRect};
use ink_domain::recog::{Control, ElementCand};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uiautomation::types::{Handle, TreeScope, UIProperty};
use uiautomation::UIAutomation;

/// 창 하나에서 받을 요소 상한(08 §4.7: 1500개 — 화면 밖·너무 작은 것을 빼기 전 넉넉히).
const MAX_ELEMENTS: usize = 4000;
const KEEP_ELEMENTS: usize = 1500;

type Ready = Box<dyn Fn(u64, String, f64) + Send + Sync>;

struct Job {
    window: u64,
    exe: String,
    gen: u64,
}

#[derive(Clone)]
pub struct UiaIndex {
    tx: mpsc::Sender<Job>,
    cache: Arc<Mutex<HashMap<u64, Vec<ElementCand>>>>,
    pending: Arc<Mutex<HashSet<u64>>>,
    gen: Arc<AtomicU64>,
}

fn responsive(hwnd: u64) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_NULL};
    let mut out = 0usize;
    // SAFETY: WM_NULL은 아무 일도 하지 않는 메시지, 50 ms 안에 돌아온다.
    unsafe { SendMessageTimeoutW(hwnd as _, WM_NULL, 0, 0, SMTO_ABORTIFHUNG, 50, &mut out) != 0 }
}

fn index_window(auto: &UIAutomation, hwnd: u64) -> uiautomation::Result<Vec<ElementCand>> {
    let root = auto.element_from_handle(Handle::from(hwnd as isize))?;
    let cr = auto.create_cache_request()?;
    cr.add_property(UIProperty::BoundingRectangle)?;
    cr.add_property(UIProperty::ControlType)?;
    cr.add_property(UIProperty::IsOffscreen)?;
    let all = auto.create_true_condition()?;
    let found = root.find_all_build_cache(TreeScope::Descendants, &all, &cr)?;
    let mut out = Vec::new();
    for e in found.iter().take(MAX_ELEMENTS) {
        if e.is_cached_offscreen().unwrap_or(false) {
            continue;
        }
        let Ok(r) = e.get_cached_bounding_rectangle() else { continue };
        let rect = PhysRect::from_edges(r.get_left(), r.get_top(), r.get_right(), r.get_bottom());
        if rect.w < 4 || rect.h < 4 {
            continue;
        }
        let control = e.get_cached_control_type().map(|c| Control::from_uia(c as i32)).unwrap_or(Control::Other);
        out.push(ElementCand { rect, control });
        if out.len() >= KEEP_ELEMENTS {
            break;
        }
    }
    Ok(out)
}

/// 연결·처리 시간 제한(D17): 기본 2초를 짧게.
fn set_timeouts(auto: &UIAutomation) {
    use windows::core::Interface;
    use windows::Win32::UI::Accessibility::{IUIAutomation, IUIAutomation2};
    let base: &IUIAutomation = auto.as_ref();
    if let Ok(a2) = base.cast::<IUIAutomation2>() {
        // SAFETY: COM 메서드 호출, 값만 넘긴다.
        unsafe {
            let _ = a2.SetConnectionTimeout(400);
            let _ = a2.SetTransactionTimeout(1500);
        }
    }
}

impl UiaIndex {
    /// 일꾼을 띄운다. `on_ready(창, 실행 파일, 걸린 ms)`는 일꾼 스레드에서 불린다.
    pub fn spawn(on_ready: impl Fn(u64, String, f64) + Send + Sync + 'static) -> UiaIndex {
        let (tx, rx) = mpsc::channel::<Job>();
        let cache: Arc<Mutex<HashMap<u64, Vec<ElementCand>>>> = Arc::default();
        let pending: Arc<Mutex<HashSet<u64>>> = Arc::default();
        let gen = Arc::new(AtomicU64::new(1));
        let (c, p, g) = (cache.clone(), pending.clone(), gen.clone());
        let on_ready: Ready = Box::new(on_ready);
        let _ = std::thread::Builder::new().name("screen-ink-uia".into()).spawn(move || {
            // UIAutomation::new = CoInitializeEx(MTA) + CUIAutomation (D11: MTA를 명시)
            let auto = match UIAutomation::new() {
                Ok(a) => a,
                Err(e) => {
                    log::warn!("uia unavailable: {e}");
                    return;
                }
            };
            set_timeouts(&auto);
            while let Ok(job) = rx.recv() {
                if job.gen != g.load(Ordering::SeqCst) {
                    p.lock().unwrap_or_else(|e| e.into_inner()).remove(&job.window);
                    continue;
                }
                let t = Instant::now();
                let found = if responsive(job.window) { index_window(&auto, job.window).unwrap_or_default() } else { Vec::new() };
                let took = t.elapsed().as_secs_f64() * 1000.0;
                p.lock().unwrap_or_else(|e| e.into_inner()).remove(&job.window);
                if job.gen != g.load(Ordering::SeqCst) {
                    continue;
                }
                log::info!("uia index {} elements in {} ms", found.len(), took.round() as i64);
                c.lock().unwrap_or_else(|e| e.into_inner()).insert(job.window, found);
                on_ready(job.window, job.exe, took);
            }
        });
        UiaIndex { tx, cache, pending, gen }
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

    pub fn rects(&self, window: u64) -> Vec<PhysRect> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(&window).map(|v| v.iter().map(|e| e.rect).collect()).unwrap_or_default()
    }

    /// 모드 끝: 색인을 버리고 세대를 올린다(늦게 끝난 결과는 버려짐).
    pub fn clear(&self) {
        self.gen.fetch_add(1, Ordering::SeqCst);
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

impl ink_app::ElementSource for UiaIndex {
    fn request(&self, window: u64, exe: &str) {
        UiaIndex::request(self, window, exe)
    }

    fn elements_at(&self, window: u64, p: PhysPoint) -> Vec<ElementCand> {
        UiaIndex::elements_at(self, window, p)
    }

    fn rects(&self, window: u64) -> Vec<PhysRect> {
        UiaIndex::rects(self, window)
    }

    fn clear(&self) {
        UiaIndex::clear(self)
    }
}
