//! 창 목록(M4): `CGWindowListCopyWindowInfo(화면에 보이는 것만)` → `RawWindow`(앞 → 뒤).
//! 창 제목(`kCGWindowName`)은 **읽지 않는다**(05 개인정보). 소유 프로세스·경계·층·투명도만.
//!
//! 맥 사실값 → 계약 필드(결정 기록: 인수인계 §4)
//! - `handle` = `kCGWindowNumber`(CGWindowID), `pid` = `kCGWindowOwnerPID`
//! - `exe` = 실행 파일 이름(소문자, 예: `safari`), 없으면 소유자 이름(소문자)
//! - `class` = 번들 ID(예: `com.apple.Safari`), 없으면 빈 문자열(앱 가족 판정용)
//! - `topmost` = 층 > 0(떠 있는 패널·메뉴·알림), `tool` = 같음(Dock·앱 전환기에 없는 보조 창)
//! - `layered_alpha` = 창 알파 < 1일 때만 `Some(알파 × 255)`(불투명 창은 Windows의 일반 창처럼 None)
//! - 화면에 보이는 것만 받으므로 `minimized`·`cloaked`(다른 Space)는 false
//! - WindowServer(메뉴 막대·커서)·Dock 소유 창은 앱 창이 아니라 화면 장식이라 목록에서 뺀다

use crate::coords::{RectF, Space};
use ink_domain::geometry::PhysRect;
use ink_domain::snapshot::RawWindow;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::NSRunningApplication;
use objc2_core_foundation::CFArray;
use objc2_core_graphics::{kCGNullWindowID, CGWindowListCopyWindowInfo, CGWindowListOption};
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString};
use std::collections::HashMap;

/// 앱 창이 아닌 화면 장식의 소유자.
const CHROME_OWNERS: [&str; 1] = ["Window Server"];
const CHROME_BUNDLES: [&str; 1] = ["com.apple.dock"];

fn num(d: &NSDictionary<NSString, AnyObject>, key: &str) -> Option<Retained<NSNumber>> {
    d.objectForKey(&NSString::from_str(key)).and_then(|o| o.downcast::<NSNumber>().ok())
}

fn bounds(d: &NSDictionary<NSString, AnyObject>) -> Option<RectF> {
    let b = d.objectForKey(&NSString::from_str("kCGWindowBounds"))?.downcast::<NSDictionary>().ok()?;
    // SAFETY: CG가 만든 경계 사전은 NSString → NSNumber.
    let b: &NSDictionary<NSString, AnyObject> = unsafe { &*(&*b as *const NSDictionary as *const NSDictionary<NSString, AnyObject>) };
    let f = |k: &str| num(b, k).map(|n| n.doubleValue());
    Some(RectF::new(f("X")?, f("Y")?, f("Width")?, f("Height")?))
}

#[derive(Clone, Default)]
struct App {
    exe: String,
    bundle: String,
}

fn app_of(pid: i32, owner: &str) -> App {
    let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);
    let exe = app
        .as_ref()
        .and_then(|a| a.executableURL())
        .and_then(|u| u.lastPathComponent())
        .map(|s| s.to_string().to_lowercase())
        .unwrap_or_else(|| owner.to_lowercase());
    let bundle = app.as_ref().and_then(|a| a.bundleIdentifier()).map(|s| s.to_string()).unwrap_or_default();
    App { exe, bundle }
}

/// 창 하나(CGWindowID)의 소유 pid와 경계(CG 전역 포인트) — AX 색인이 그 앱의 AX 창을 찾을 때(M9).
pub(crate) fn owner_and_bounds(window: u32) -> Option<(i32, RectF)> {
    let list = CGWindowListCopyWindowInfo(CGWindowListOption::OptionIncludingWindow, window)?;
    // SAFETY: 위 `top_windows`와 같은 무료 다리.
    let list: &NSArray<NSDictionary<NSString, AnyObject>> = unsafe { &*(&*list as *const CFArray as *const NSArray<NSDictionary<NSString, AnyObject>>) };
    let d = list.iter().next()?;
    Some((num(&d, "kCGWindowOwnerPID")?.intValue(), bounds(&d)?))
}

/// 창 추적 녹화용: 대상 창이 지금 화면에 보이면 물리 사각형, 최소화·다른 Space·숨김이면 `None`.
pub(crate) fn visible_rect(window: u32, space: &Space) -> Option<PhysRect> {
    let list = CGWindowListCopyWindowInfo(CGWindowListOption::OptionIncludingWindow, window)?;
    // SAFETY: 위 `owner_and_bounds`와 같은 무료 다리.
    let list: &NSArray<NSDictionary<NSString, AnyObject>> = unsafe { &*(&*list as *const CFArray as *const NSArray<NSDictionary<NSString, AnyObject>>) };
    let d = list.iter().next()?;
    if !num(&d, "kCGWindowIsOnscreen").is_some_and(|n| n.boolValue()) {
        return None;
    }
    let alpha = num(&d, "kCGWindowAlpha").map(|n| n.doubleValue()).unwrap_or(1.0);
    if alpha <= 0.0 {
        return None;
    }
    let b = bounds(&d)?;
    if b.w <= 0.0 || b.h <= 0.0 {
        return None;
    }
    Some(space.rect_to_phys(b))
}

/// Z 순서(앞 → 뒤) 화면에 보이는 창 전부. 거르기는 도메인(`snapshot::filter_windows`)이 한다.
pub fn top_windows(space: &Space) -> Vec<RawWindow> {
    let opts = CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements;
    let Some(list) = CGWindowListCopyWindowInfo(opts, kCGNullWindowID) else { return Vec::new() };
    // SAFETY: CFArray와 NSArray는 무료 다리(toll-free bridged)이고, 원소는 CG가 만든 사전이다.
    let list: &NSArray<NSDictionary<NSString, AnyObject>> = unsafe { &*(&*list as *const CFArray as *const NSArray<NSDictionary<NSString, AnyObject>>) };
    let mut apps: HashMap<i32, App> = HashMap::new();
    let mut out = Vec::with_capacity(list.count());
    for d in list.iter() {
        let (Some(id), Some(pid)) = (num(&d, "kCGWindowNumber"), num(&d, "kCGWindowOwnerPID")) else { continue };
        let pid = pid.intValue();
        let owner = d.objectForKey(&NSString::from_str("kCGWindowOwnerName")).and_then(|o| o.downcast::<NSString>().ok()).map(|s| s.to_string()).unwrap_or_default();
        if CHROME_OWNERS.contains(&owner.as_str()) {
            continue;
        }
        let app = apps.entry(pid).or_insert_with(|| app_of(pid, &owner)).clone();
        if CHROME_BUNDLES.contains(&app.bundle.as_str()) {
            continue;
        }
        let layer = num(&d, "kCGWindowLayer").map(|n| n.intValue()).unwrap_or(0);
        let alpha = num(&d, "kCGWindowAlpha").map(|n| n.doubleValue()).unwrap_or(1.0);
        let rect = bounds(&d).map(|r| space.rect_to_phys(r)).unwrap_or_default();
        out.push(RawWindow {
            handle: id.unsignedIntValue() as u64,
            pid: pid.max(0) as u32,
            exe: app.exe,
            class: app.bundle,
            rect,
            visible: true,
            minimized: false,
            maximized: false,
            cloaked: false,
            hung: false,
            click_through: false,
            layered_alpha: (alpha < 1.0).then(|| (alpha.clamp(0.0, 1.0) * 255.0).round() as u8),
            topmost: layer > 0,
            tool: layer > 0,
        });
    }
    out
}
