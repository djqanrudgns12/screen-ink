use super::*;
use crate::{tr_for, IslandGen, EVT_ISLAND, ISLAND, ISLAND_SIZE};
use ink_app::InkService;
use ink_domain::mode::Mode;
use ink_domain::package::{should_defer_update, UpdateDeferInput};
use ink_domain::shortcuts::Platform;
use ink_platform_windows::{
    events, package,
    updater::{self, Payload, Ready},
};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

struct Pending {
    update: Arc<Ready>,
    generation: Option<u64>,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn busy(s: &InkService) -> bool {
    should_defer_update(&UpdateDeferInput {
        mode_idle: matches!(s.mode(), Mode::Idle),
        recording_active: s.recording_active(),
        fullscreen_app: package::fullscreen_app_active(),
        secure_desktop: events::secure_desktop_active(),
        snooze_date: Some(s.settings().str("update.snooze")),
        today_local: &today(),
    })
}

pub fn check<R: Runtime>(app: &AppHandle<R>, manual: bool) {
    if !auto_on(app) {
        if manual {
            island(app, IslandEvent::new(IslandKind::Info, "자동으로 새 판 확인을 켜면 확인할 수 있어요"));
        }
        return;
    }
    if CHECKING.swap(true, Ordering::SeqCst) {
        return;
    }
    let result = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())
        .and_then(|cache| updater::check(&app.package_info().version.to_string(), &cache.join("updates"), || auto_on(app)));
    if auto_on(app) {
        match result {
            Ok(ready) => {
                let ready = ready.map(Arc::new);
                *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = ready.as_ref().map(|r| Pending {
                    update: r.clone(),
                    generation: None,
                });
                if let Some(ready) = ready {
                    offer(app, ready);
                } else if manual {
                    island(app, IslandEvent::new(IslandKind::Info, tr(app, "update.latest")));
                }
            }
            Err(e) => {
                log::warn!("업데이트 확인 실패: {e}");
                if manual {
                    island(app, IslandEvent::new(IslandKind::Problem, tr(app, "update.failed")));
                }
            }
        }
    }
    CHECKING.store(false, Ordering::SeqCst);
}

fn offer<R: Runtime>(app: &AppHandle<R>, ready: Arc<Ready>) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        let mut pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
        let Some(item) = pending.as_mut().filter(|p| Arc::ptr_eq(&p.update, &ready)) else {
            return;
        };
        let ink = h.state::<Ink>();
        let s = ink.lock();
        if !s.settings().bool("update.auto") {
            *pending = None;
            return;
        }
        if busy(&s) {
            let h = h.clone();
            std::thread::spawn(move || {
                std::thread::sleep(RETRY);
                offer(&h, ready);
            });
            return;
        }
        let lang = s.lang();
        let text = ink_domain::i18n::trf_os(lang, Platform::Win, "update.available", &[("v", &ready.version)]);
        let mut event = IslandEvent::new(IslandKind::Info, text)
            .button(&tr_for(lang, "update.install"), ACTION_INSTALL, None)
            .secondary(&tr_for(lang, "updateUi.snoozeBtn"), ACTION_SNOOZE, None)
            .duration_ms(OFFER_MS);
        if matches!(ready.payload, Payload::VerifiedFile(_)) {
            event = event.detail("설치 파일 보기");
        }
        drop(s);
        // 공통 섬 창은 8초 뒤에 숨는다. 업데이트 단추는 같은 창에서 30초 동안 기다린다.
        let generation = h.state::<IslandGen>().0.fetch_add(1, Ordering::SeqCst) + 1;
        item.generation = Some(generation);
        drop(pending);
        let Some(window) = h.get_webview_window(ISLAND) else { return };
        if let Ok(hwnd) = window.hwnd() {
            use ink_platform_windows::screen;
            screen::show_island(hwnd.0 as isize, screen::cursor_pos(), ISLAND_SIZE);
        }
        let _ = window.show();
        let _ = h.emit(EVT_ISLAND, event);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(OFFER_MS.into()));
            let app = h.clone();
            let _ = h.run_on_main_thread(move || {
                // 다른 섬이 그 자리에 뜨면 해당 섬의 표시 시간을 따른다.
                if app
                    .state::<IslandGen>()
                    .0
                    .compare_exchange(generation, generation + 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    let _ = window.hide();
                }
            });
        });
    });
}

pub(super) fn defer_snoozed<R: Runtime>(app: &AppHandle<R>) {
    let mut pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    let Some(item) = pending.as_mut() else { return };
    if item.generation.take().is_some() {
        let ready = item.update.clone();
        drop(pending);
        offer(app, ready);
    }
}

pub(super) fn install<R: Runtime>(app: &AppHandle<R>) {
    let mut pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    let generation = app.state::<IslandGen>().0.load(Ordering::SeqCst);
    let Some(item) = pending.as_mut().filter(|p| p.generation == Some(generation)) else {
        return;
    };
    if !auto_on(app) {
        *pending = None;
        return;
    }
    if busy(&app.state::<Ink>().lock()) {
        item.generation = None;
        let ready = item.update.clone();
        drop(pending);
        offer(app, ready);
        return;
    }
    let ready = item.update.clone();
    *pending = None;
    drop(pending);
    match &ready.payload {
        Payload::VerifiedFile(path) => {
            crate::reveal_path(path);
            island(app, IslandEvent::new(IslandKind::Info, "설치하려면 탐색기에서 선택된 MSIX 파일을 열어 주세요"));
        }
        Payload::AppInstaller(uri) => {
            let uri = uri.clone();
            let h = app.clone();
            island(app, IslandEvent::new(IslandKind::Info, "업데이트 중이에요 · 곧 다시 켜져요"));
            std::thread::spawn(move || {
                if !auto_on(&h) {
                    return;
                }
                if let Err(e) = updater::apply(&uri) {
                    log::warn!("업데이트 적용 실패: {e}");
                    island(&h, IslandEvent::new(IslandKind::Problem, tr(&h, "update.installFailed")));
                }
            });
        }
    }
}
