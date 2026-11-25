//! 정지 캡처(M2): ScreenCaptureKit `SCScreenshotManager`로 모니터마다 한 장.
//! - **우리 앱 창은 빼고** 찍는다(`SCContentFilter` 제외 앱).
//! - 크기 = 계약 모니터 `bounds.w·h`(물리 px), 색 = sRGB, BGRA, 알파 255, 커서 없음.
//! - 공유 가능한 내용(`SCShareableContent`) 조회가 느려(수십 ms) 한 번 받아 두고, 디스플레이가 바뀌면 다시 받는다.
//! - 완료 처리기는 SCK 일꾼 큐에서 돈다 → 부른 스레드(메인)는 채널로 기다리기만 한다(교착 없음).

use crate::coords::Space;
use crate::pixels;
use block2::RcBlock;
use ink_app::Frame;
use ink_domain::geometry::MonitorLayout;
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_core_graphics::{CGImage, CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};
use objc2_foundation::{NSArray, NSError};
use objc2_screen_capture_kit::{SCCaptureResolutionType, SCContentFilter, SCDisplay, SCRunningApplication, SCScreenshotManager, SCShareableContent, SCStreamConfiguration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

/// 권한이 없을 때 돌려주는 오류(서비스가 권한 안내를 띄운다, M10).
pub use ink_app::NO_SCREEN_PERMISSION as NO_PERMISSION;

/// 한 장의 결과(폭, 높이, BGRA).
type Shot = Result<(u32, u32, Vec<u8>), String>;

const WAIT: Duration = Duration::from_secs(3);

/// SCK 객체를 스레드 사이로 넘기기 위한 껍데기. SCShareableContent·SCDisplay 등은 불변 값 객체다.
struct Shared<T>(T);
// SAFETY: 담는 것은 만든 뒤 바뀌지 않는 SCK 값 객체(내용 목록·디스플레이·앱)뿐이다.
unsafe impl<T> Send for Shared<T> {}

struct Content {
    displays: Vec<(u32, Retained<SCDisplay>)>,
    me: Option<Retained<SCRunningApplication>>,
}

static CACHE: Mutex<Option<Shared<Content>>> = Mutex::new(None);
static ASKED: AtomicBool = AtomicBool::new(false);

/// 화면 기록 권한이 있는지(묻지 않음).
pub fn has_permission() -> bool {
    CGPreflightScreenCaptureAccess()
}

/// 권한 요청 창을 한 번만 띄운다(시스템 설정에 앱을 올림). 이미 물었으면 false.
pub fn request_permission_once() -> bool {
    if ASKED.swap(true, Ordering::SeqCst) {
        return false;
    }
    CGRequestScreenCaptureAccess()
}

fn fetch_content() -> Result<Content, String> {
    let (tx, rx) = mpsc::channel::<Result<Shared<Retained<SCShareableContent>>, String>>();
    let block = RcBlock::new(move |content: *mut SCShareableContent, err: *mut NSError| {
        // SAFETY: SCK가 넘긴 포인터는 이 호출 동안 유효하다. retain으로 붙잡는다.
        let r = match unsafe { Retained::retain(content) } {
            Some(c) => Ok(Shared(c)),
            None => Err(unsafe { err.as_ref() }.map(|e| format!("SCShareableContent: {}", e.code())).unwrap_or_else(|| "SCShareableContent: none".into())),
        };
        let _ = tx.send(r);
    });
    // SAFETY: 처리기는 'static이고 SCK가 복사해 둔다.
    unsafe { SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(false, false, &block) };
    let content = rx.recv_timeout(WAIT).map_err(|_| "SCShareableContent timeout".to_string())??.0;
    let pid = std::process::id() as i32;
    // SAFETY: 읽기 전용 속성들.
    unsafe {
        let displays = content.displays().iter().map(|d| (d.displayID(), d)).collect();
        let me = content.applications().iter().find(|a| a.processID() == pid);
        Ok(Content { displays, me })
    }
}

fn capture_one(display: &SCDisplay, me: Option<&Retained<SCRunningApplication>>, w: u32, h: u32) -> mpsc::Receiver<Shot> {
    let (tx, rx) = mpsc::channel();
    // SAFETY: 필터·설정은 만든 뒤 SCK에 넘기기만 한다. 처리기는 'static.
    unsafe {
        let apps: Retained<NSArray<SCRunningApplication>> = match me {
            Some(a) => NSArray::from_retained_slice(std::slice::from_ref(a)),
            None => NSArray::new(),
        };
        let filter = SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(SCContentFilter::alloc(), display, &apps, &NSArray::new());
        let config = SCStreamConfiguration::new();
        config.setWidth(w as usize);
        config.setHeight(h as usize);
        config.setShowsCursor(false);
        config.setPixelFormat(u32::from_be_bytes(*b"BGRA"));
        config.setColorSpaceName(objc2_core_graphics::kCGColorSpaceSRGB);
        config.setCaptureResolution(SCCaptureResolutionType::Best);
        let block = RcBlock::new(move |img: *mut CGImage, err: *mut NSError| {
            let r = match img.as_ref() {
                Some(img) => pixels::opaque_bgra(img).ok_or_else(|| "unreadable CGImage".to_string()),
                None => Err(err.as_ref().map(|e| format!("SCScreenshotManager: {}", e.code())).unwrap_or_else(|| "SCScreenshotManager: none".into())),
            };
            let _ = tx.send(r);
        });
        SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(&filter, &config, Some(&block));
    }
    rx
}

/// 모니터마다 정지 이미지. 크기가 어긋나면(배율이 그 사이 바뀜) 실패로 알린다.
pub fn capture(layout: &MonitorLayout, space: &Space) -> Result<Vec<Frame>, String> {
    if !has_permission() {
        request_permission_once();
        return Err(NO_PERMISSION.into());
    }
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let need = |c: &Content| layout.monitors.iter().all(|m| space.screen_of(m.bounds).is_some_and(|s| c.displays.iter().any(|(id, _)| *id == s.display)));
    if !cache.as_ref().is_some_and(|c| need(&c.0)) {
        *cache = Some(Shared(fetch_content()?));
    }
    let content = &cache.as_ref().expect("filled above").0;
    let jobs: Vec<_> = layout
        .monitors
        .iter()
        .map(|m| {
            let s = space.screen_of(m.bounds).ok_or("monitor not in space")?;
            let (_, d) = content.displays.iter().find(|(id, _)| *id == s.display).ok_or("display not shareable")?;
            Ok::<_, String>((m.id, m.bounds, capture_one(d, content.me.as_ref(), m.bounds.w as u32, m.bounds.h as u32)))
        })
        .collect::<Result<_, _>>()?;
    let mut out = Vec::with_capacity(jobs.len());
    for (id, b, rx) in jobs {
        let (w, h, px) = rx.recv_timeout(WAIT).map_err(|_| "capture timeout".to_string())??;
        if (w, h) != (b.w as u32, b.h as u32) {
            // 다음 진입에서 내용 목록을 다시 받게
            *cache = None;
            return Err(format!("frame size {w}x{h} != monitor {}x{}", b.w, b.h));
        }
        out.push(Frame { monitor_id: id, width: w, height: h, bgra: std::sync::Arc::new(px) });
    }
    Ok(out)
}

/// 디스플레이가 바뀌면 다음 캡처에서 내용 목록을 새로 받는다.
pub fn invalidate() {
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}
