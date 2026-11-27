//! F2 창 단독 캡처(M14, R-WIN-1·3): ScreenCaptureKit 창 하나 필터(`desktopIndependentWindow`)로 가려진 부분까지 한 장.
//! Windows `wgc_window.rs`와 같은 흐름 — 고르는 중 창이 정해지면 미리 준비([`prepare`]) → 확정 때 한 장([`shoot`]) →
//! 시간 안에 안 오거나 RGB가 전부 0이면 서비스가 정지 스냅숏 자르기로 대체(`windowshot::after_primary`).
//! - 그림자는 빼고(`ignoreShadowsSingleWindow`, macOS 14+). 둥근 모서리 바깥은 SCK가 투명으로 주지만, 서비스는 창 안
//!   반투명(사이드바 비침 등)을 없애려고 그림 전체를 불투명으로 바꾼 뒤 `corner_radius`만큼 귀를 깎는다(사용자 결정
//!   "둥근 창은 귀만 투명", R-OUT-3). 그래서 SCK 알파에서 귀 반지름을 재어 넘긴다([`corner_radius_from_alpha`]).
//!   0으로 넘기면 귀가 검게 칠해진다(2026-11-27 실기).
//! - 크기 = 서비스가 넘긴 창 사각형(계약 물리 px).
//! - 창 제목은 읽지 않는다(05 개인정보) — 창 번호로만 찾는다.

use crate::pixels;
use block2::RcBlock;
use ink_app::WindowShot;
use ink_domain::geometry::PhysRect;
use ink_domain::image::{blankness, Blankness};
use ink_domain::windowshot::{after_primary, ShotNext};
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::CGImage;
use objc2_foundation::NSError;
use objc2_screen_capture_kit::{SCCaptureResolutionType, SCContentFilter, SCScreenshotManager, SCShareableContent, SCStreamConfiguration, SCWindow};
use std::ptr::NonNull;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 창 목록 받기 제한.
const LIST_WAIT: Duration = Duration::from_secs(2);
/// 확정 뒤 그림을 기다리는 시간 — SCK 한 장은 보통 30~80 ms(모니터 캡처 실측), 넘기면 스냅숏 자르기로.
const SHOT_WAIT: Duration = Duration::from_millis(300);

struct Shared<T>(T);
// SAFETY: 담는 것은 만든 뒤 바뀌지 않는 SCK 값 객체(SCWindow)뿐이다.
unsafe impl<T> Send for Shared<T> {}

/// 미리 찾아 둔 창(창 번호, SCK 창).
static PREPARED: Mutex<Option<(u32, Shared<Retained<SCWindow>>)>> = Mutex::new(None);

/// SCK가 준 한 장(곧은 알파 BGRA)과 요청 → 도착 시간, 그림의 알파 정보(`CGImageAlphaInfo`).
struct Raw {
    w: u32,
    h: u32,
    bgra: Vec<u8>,
    sck_ms: f64,
    alpha_info: u32,
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// 화면에 보이는 창 목록에서 그 창 번호를 찾는다(수십 ms — 부른 스레드는 채널로 기다린다).
pub(crate) fn find(window: u32) -> Result<Retained<SCWindow>, String> {
    let (tx, rx) = mpsc::channel::<Result<Shared<Retained<SCWindow>>, String>>();
    let block = RcBlock::new(move |content: *mut SCShareableContent, err: *mut NSError| {
        // SAFETY: SCK가 넘긴 포인터는 이 호출 동안 유효하다. 찾은 창은 retain으로 붙잡는다.
        let r = match unsafe { content.as_ref() } {
            Some(c) => unsafe { c.windows() }.iter().find(|w| unsafe { w.windowID() } == window).map(Shared).ok_or_else(|| "window not shareable".to_string()),
            None => Err(unsafe { err.as_ref() }.map(|e| format!("SCShareableContent: {}", e.code())).unwrap_or_else(|| "SCShareableContent: none".into())),
        };
        let _ = tx.send(r);
    });
    // SAFETY: 처리기는 'static이고 SCK가 복사해 둔다. 바탕 화면 창 빼고, 화면에 보이는 창만.
    unsafe { SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(true, true, &block) };
    Ok(rx.recv_timeout(LIST_WAIT).map_err(|_| "SCShareableContent timeout".to_string())??.0)
}

/// F2 고르기 중 커서 아래 창이 정해졌다 — SCK 창을 뒤에서 찾아 둔다(`None` = 버리기). 부른 스레드는 기다리지 않는다.
pub fn prepare(window: Option<u64>) {
    let Some(id) = window.map(|w| w as u32) else {
        *PREPARED.lock().unwrap_or_else(|e| e.into_inner()) = None;
        return;
    };
    if PREPARED.lock().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|(w, _)| *w == id) {
        return;
    }
    std::thread::spawn(move || match find(id) {
        Ok(w) => *PREPARED.lock().unwrap_or_else(|e| e.into_inner()) = Some((id, Shared(w))),
        Err(e) => log::debug!("window capture prepare failed: {e}"),
    });
}

fn take_prepared(id: u32) -> Option<Retained<SCWindow>> {
    let mut g = PREPARED.lock().unwrap_or_else(|e| e.into_inner());
    if g.as_ref().is_some_and(|(w, _)| *w == id) {
        g.take().map(|(_, w)| w.0)
    } else {
        None
    }
}

/// 창 한 장을 요청한다(그림·걸린 ms를 채널로).
fn request(window: &SCWindow, w: u32, h: u32) -> mpsc::Receiver<Result<Raw, String>> {
    let (tx, rx) = mpsc::channel();
    // SAFETY: 필터·설정은 만든 뒤 SCK에 넘기기만 한다. 처리기는 'static.
    unsafe {
        let filter = SCContentFilter::initWithDesktopIndependentWindow(SCContentFilter::alloc(), window);
        let config = SCStreamConfiguration::new();
        config.setWidth(w as usize);
        config.setHeight(h as usize);
        config.setShowsCursor(false);
        config.setIgnoreShadowsSingleWindow(true);
        config.setPixelFormat(u32::from_be_bytes(*b"BGRA"));
        config.setColorSpaceName(objc2_core_graphics::kCGColorSpaceSRGB);
        config.setCaptureResolution(SCCaptureResolutionType::Best);
        let asked = Instant::now();
        let block = RcBlock::new(move |img: *mut CGImage, err: *mut NSError| {
            let r = match NonNull::new(img) {
                Some(img) => {
                    let sck = ms(asked.elapsed());
                    // SAFETY: SCK가 넘긴 그림은 이 호출 동안 유효하다. retain으로 붙잡는다.
                    let image: CFRetained<CGImage> = CFRetained::retain(img);
                    let (iw, ih) = (CGImage::width(Some(&image)), CGImage::height(Some(&image)));
                    let alpha_info = CGImage::alpha_info(Some(&image)).0;
                    pixels::straight_bgra(&image, iw, ih)
                        .map(|bgra| Raw { w: iw as u32, h: ih as u32, bgra, sck_ms: sck, alpha_info })
                        .ok_or_else(|| "unreadable CGImage".to_string())
                }
                None => Err(err.as_ref().map(|e| format!("SCScreenshotManager: {}", e.code())).unwrap_or_else(|| "SCScreenshotManager: none".into())),
            };
            let _ = tx.send(r);
        });
        SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(&filter, &config, Some(&block));
    }
    rx
}

/// 확정: 창 한 장(가려진 부분 포함). 시간 안에 안 오거나 RGB가 전부 0이면 오류 → 서비스가 스냅숏 자르기로 대체.
/// 맥에는 Windows 표시 선호도(WDA) 같은 값이 없어 보호 화면 판정은 대체 경로 결과(검정)로 한다.
pub fn shoot(window: u64, bounds: PhysRect) -> Result<WindowShot, String> {
    if !crate::permissions::screen() {
        return Err(ink_app::NO_SCREEN_PERMISSION.into());
    }
    let id = window as u32;
    let t = Instant::now();
    let prepared = take_prepared(id);
    let warm = prepared.is_some();
    let sc = match prepared {
        Some(w) => w,
        None => find(id)?,
    };
    let (w, h) = (bounds.w.max(1) as u32, bounds.h.max(1) as u32);
    let rx = request(&sc, w, h);
    let left = SHOT_WAIT.saturating_sub(t.elapsed()).max(Duration::from_millis(50));
    let raw = rx.recv_timeout(left).map_err(|_| "window capture timed out".to_string())??;
    let b = blankness(&raw.bgra);
    log::debug!(
        "window shot {}x{}: {} {:.0} ms (sck {:.0}), alpha info {}, {b:?}",
        raw.w,
        raw.h,
        if warm { "prepared" } else { "cold" },
        ms(t.elapsed()),
        raw.sck_ms,
        raw.alpha_info
    );
    match after_primary(Some(b), 0) {
        ShotNext::Use => {
            let corner_radius = corner_radius_from_alpha(&raw.bgra, raw.w, raw.h);
            log::debug!("window shot corner radius {corner_radius} px");
            Ok(WindowShot { width: raw.w, height: raw.h, bgra: raw.bgra, method: "sck", corner_radius })
        }
        _ if b == Blankness::Unpainted => Err("window capture empty".into()),
        _ => Err("window capture black".into()),
    }
}

/// SCK가 준 곧은 알파에서 둥근 귀 반지름(px)을 잰다. 네 귀마다 맨 끝 줄에서 알파가 절반을 넘는 첫 화소까지 거리 x를 재고,
/// 원 r에서 그 줄 가운데(0.5 px) 높이의 경계는 x = r − √(r − ¼) → r = (½ + √x)² + ¼ 이다. 네 귀의 가운데 값을 쓰고,
/// 모자라면 검은 테가 남으므로 올림한다. 귀 화소가 불투명하면(전체 화면·각진 창) 0.
fn corner_radius_from_alpha(bgra: &[u8], w: u32, h: u32) -> u32 {
    if w < 4 || h < 4 || bgra.len() < (w as usize * h as usize * 4) {
        return 0;
    }
    let alpha = |x: u32, y: u32| bgra[(y as usize * w as usize + x as usize) * 4 + 3];
    let limit = w.min(h) / 2;
    let edge = |y: u32, from_right: bool| -> Option<u32> {
        let at = |i: u32| if from_right { w - 1 - i } else { i };
        if alpha(at(0), y) >= 128 {
            return Some(0);
        }
        (1..limit).find(|&i| alpha(at(i), y) >= 128)
    };
    let mut radii: Vec<f64> = [(0, false), (0, true), (h - 1, false), (h - 1, true)]
        .into_iter()
        .filter_map(|(y, right)| edge(y, right))
        .map(|x| if x == 0 { 0.0 } else { (0.5 + (x as f64).sqrt()).powi(2) + 0.25 })
        .collect();
    if radii.is_empty() {
        return 0;
    }
    radii.sort_by(|a, b| a.total_cmp(b));
    let mid = (radii[(radii.len() - 1) / 2] + radii[radii.len() / 2]) / 2.0;
    (mid.ceil() as u32).min(limit)
}

#[cfg(test)]
mod radius_tests {
    use super::corner_radius_from_alpha;
    use ink_domain::image::mask_round_corners;

    fn rounded(w: u32, h: u32, r: u32) -> Vec<u8> {
        let mut px = vec![255u8; (w * h * 4) as usize];
        mask_round_corners(&mut px, w, h, r);
        px
    }

    #[test]
    fn 둥근_귀_반지름을_알파에서_잰다() {
        for r in [8u32, 12, 20, 26, 40] {
            let got = corner_radius_from_alpha(&rounded(300, 200, r), 300, 200);
            assert!(got >= r && got <= r + 2, "r {r} → {got}");
        }
    }

    #[test]
    fn 각진_창과_잘못된_크기는_0() {
        assert_eq!(corner_radius_from_alpha(&vec![255u8; 100 * 80 * 4], 100, 80), 0);
        assert_eq!(corner_radius_from_alpha(&[0u8; 8], 100, 80), 0);
    }

    #[test]
    fn 잰_반지름으로_깎으면_귀_밖에_불투명이_남지_않는다() {
        let (w, h, r) = (240u32, 160u32, 22u32);
        let sck = rounded(w, h, r);
        let est = corner_radius_from_alpha(&sck, w, h);
        // 서비스처럼: 전부 불투명 → 잰 반지름으로 깎기
        let mut out = sck.clone();
        out.chunks_exact_mut(4).for_each(|p| p[3] = 255);
        mask_round_corners(&mut out, w, h, est);
        for (o, s) in out.chunks_exact(4).zip(sck.chunks_exact(4)) {
            assert!(o[3] <= s[3].saturating_add(8), "SCK가 투명하게 준 곳이 불투명해짐");
        }
    }
}

#[cfg(test)]
mod tests {
    /// 실제 맨 앞 앱 창 한 장(화면 기록 권한이 있는 셸에서 손으로). 그림은 저장하지 않는다(개인정보) — 크기·모서리 알파만.
    /// `cargo test -p ink-platform-macos window_shot::tests -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn 실제_맨앞_창_한_장() {
        let space = crate::space();
        let list: Vec<_> = crate::windows::top_windows(&space).into_iter().filter(|w| !w.topmost && w.rect.w > 200 && w.rect.h > 200).take(3).collect();
        assert!(!list.is_empty(), "앱 창");
        for w in list {
            super::prepare(Some(w.handle));
            std::thread::sleep(std::time::Duration::from_millis(300));
            let t = std::time::Instant::now();
            let shot = super::shoot(w.handle, w.rect).expect("창 한 장");
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            let a = |x: u32, y: u32| shot.bgra[((y * shot.width + x) * 4 + 3) as usize];
            eprintln!(
                "{} {}x{} in {ms:.0} ms, alpha corner {} (4,4) {} center {}, radius {}",
                w.exe,
                shot.width,
                shot.height,
                a(0, 0),
                a(4, 4),
                a(shot.width / 2, shot.height / 2),
                shot.corner_radius
            );
            assert_eq!((shot.width, shot.height), (w.rect.w as u32, w.rect.h as u32));
            assert_eq!(a(shot.width / 2, shot.height / 2), 255, "창 가운데는 불투명");
        }
    }
}
