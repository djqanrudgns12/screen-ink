//! 자동 업데이트 흐름(R2-UPD) — 확인·받기는 **서비스 잠금 밖** 뒤 스레드에서, 알림·설치만 메인 스레드에서.
//!
//! - 맥: `ink_platform_macos::updater`(OS 도구 curl·codesign, 같은 인증서 서명 확인). 켠 뒤 10분 + 하루 한 번,
//!   설정 `update.auto`가 꺼져 있으면 통신 0. 그리기·녹화 같은 모드 중이거나 "오늘은 그만"이면 5분 뒤 다시 묻는다.
//! - Windows: 직접 배포 MSIX는 앱 설치 관리자에 적용을 요청하고 비패키지는 검증한 설치 파일을 안내한다.

#[cfg(not(windows))]
use crate::today;
#[cfg(windows)]
use ink_platform_windows::updater::local_date as today;
use crate::{tr, Ink};
use ink_app::{IslandEvent, IslandKind};
#[cfg(not(windows))]
use ink_domain::mode::Mode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};

pub const ACTION_CHECK: &str = "update.check";
pub const ACTION_INSTALL: &str = "update.install";
pub const ACTION_SNOOZE: &str = "update.snooze";

const FIRST_DELAY: Duration = Duration::from_secs(600);
const EVERY: Duration = Duration::from_secs(24 * 3600);
const RETRY: Duration = Duration::from_secs(300);
const OFFER_MS: u32 = 30_000;

static CHECKING: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::check;

#[cfg(target_os = "macos")]
static STAGED: std::sync::Mutex<Option<ink_platform_macos::updater::Staged>> = std::sync::Mutex::new(None);

fn auto_on<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<Ink>().lock().settings().bool("update.auto")
}

fn island<R: Runtime>(app: &AppHandle<R>, ev: IslandEvent) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || h.state::<Ink>().lock().island(ev));
}

/// 앱 시작 때 한 번: 켠 뒤 10분(시험: `SCREEN_INK_UPDATE_DELAY_SECS`)부터 하루 한 번.
pub fn schedule<R: Runtime>(app: &AppHandle<R>) {
    let first = std::env::var("SCREEN_INK_UPDATE_DELAY_SECS").ok().and_then(|v| v.parse().ok()).map(Duration::from_secs).unwrap_or(FIRST_DELAY);
    let app = app.clone();
    std::thread::Builder::new()
        .name("ink-update".into())
        .spawn(move || {
            std::thread::sleep(first);
            loop {
                if auto_on(&app) {
                    check(&app, false);
                }
                std::thread::sleep(EVERY);
            }
        })
        .ok();
}

/// 확인(뒤 스레드에서 부른다). `manual` = 설정의 [지금 확인] — 결과를 늘 섬으로 알린다.
#[cfg(not(windows))]
pub fn check<R: Runtime>(app: &AppHandle<R>, manual: bool) {
    if CHECKING.swap(true, Ordering::SeqCst) {
        return;
    }
    let result = stage(app);
    CHECKING.store(false, Ordering::SeqCst);
    match result {
        Ok(Some(version)) => offer(app, version),
        Ok(None) if manual => island(app, IslandEvent::new(IslandKind::Info, tr(app, "update.latest"))),
        Ok(None) => {}
        Err(e) => {
            log::warn!("update check failed: {e}");
            if manual {
                island(app, IslandEvent::new(IslandKind::Problem, tr(app, "update.failed")));
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn stage<R: Runtime>(app: &AppHandle<R>) -> Result<Option<String>, String> {
    use ink_platform_macos::updater;
    let Some(bundle) = updater::current_bundle() else { return Err("not running from an app bundle (dev build)".into()) };
    let cache = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    let version = app.package_info().version.to_string();
    let staged = updater::check_and_stage(updater::FEED_URL, &version, &bundle, &cache)?;
    let v = staged.as_ref().map(|s| s.version.clone());
    if staged.is_some() {
        *STAGED.lock().unwrap_or_else(|e| e.into_inner()) = staged;
    }
    Ok(v)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn stage<R: Runtime>(app: &AppHandle<R>) -> Result<Option<String>, String> {
    let ink = app.state::<Ink>();
    let r = ink.lock().check_update();
    r.map(|u| u.map(|u| u.version))
}

/// 준비된 새 판을 알린다. 모드 중·"오늘은 그만"이면 5분 뒤 다시.
#[cfg(not(windows))]
fn offer<R: Runtime>(app: &AppHandle<R>, version: String) {
    let busy = {
        let ink = app.state::<Ink>();
        let s = ink.lock();
        !matches!(s.mode(), Mode::Idle) || s.settings().str("update.snooze") == today()
    };
    if busy {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(RETRY);
            offer(&app, version);
        });
        return;
    }
    let text = ink_domain::i18n::trf_os(app.state::<Ink>().lock().lang(), ink_domain::shortcuts::Platform::current(), "update.available", &[("v", &version)]);
    let ev = IslandEvent::new(IslandKind::Info, text).detail(tr(app, "update.detail")).button(&tr(app, "update.install"), ACTION_INSTALL, None).duration_ms(OFFER_MS);
    island(app, ev);
}

/// [지금 업데이트]: 바꿔 끼우고 다시 연다(메인 스레드에서 부른다).
pub fn install<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        use ink_platform_macos::updater;
        let staged = STAGED.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let (Some(staged), Some(bundle)) = (staged, updater::current_bundle()) else { return };
        match updater::install(&staged, &bundle) {
            Ok(()) => app.exit(0),
            Err(e) => {
                log::warn!("update install failed: {e}");
                app.state::<Ink>().lock().island(IslandEvent::new(IslandKind::Problem, tr(app, "update.installFailed")));
            }
        }
    }
    #[cfg(windows)]
    windows::install(app);
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let r = app.state::<Ink>().lock().install_update();
        if let Err(e) = r {
            log::warn!("update install failed: {e}");
        }
    }
}

/// "오늘은 그만": 오늘은 다시 묻지 않는다.
pub fn snooze<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.state::<Ink>().lock().update_setting("update.snooze", &serde_json::Value::String(today()));
    #[cfg(windows)]
    windows::defer_snoozed(app);
}
