//! 캡처 핀 창(R-CAP-29) 어댑터: Tauri 창 `ink-pin-<id>` + 그림 주소 `ink://…/pin/<id>`.
//!
//! 서비스가 목록·그림을 쥐고(복사·저장), 이 어댑터는 창을 만들고·없애고, 휠 확대(창 크기·자리)와
//! 끌기·오른쪽 클릭 메뉴를 맡는다. 투명도는 핀 화면(UI)이 그림에 바로 건다.

use ink_domain::geometry::PhysRect;
use ink_domain::pin::{next_zoom, zoom_rect};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const PREFIX: &str = "ink-pin-";
/// 메뉴 항목 id 앞머리: `pin:<동작>:<핀 id>`
pub const MENU_PREFIX: &str = "pin:";

pub fn label(id: u64) -> String {
    format!("{PREFIX}{id}")
}

/// 그림 주소(핀 화면의 `<img>`).
pub fn pin_url(id: u64) -> String {
    if cfg!(windows) || cfg!(target_os = "android") {
        format!("http://{}.localhost/pin/{id}", crate::overlay::SCHEME)
    } else {
        format!("{}://localhost/pin/{id}", crate::overlay::SCHEME)
    }
}

#[derive(Clone)]
pub struct PinEntry {
    pub w: u32,
    pub h: u32,
    pub bgra: Arc<Vec<u8>>,
    /// 지금 배율(%)
    pub zoom: u32,
}

/// 핀 그림(주소로 내주기)과 배율.
#[derive(Clone, Default)]
pub struct PinStore(Arc<Mutex<HashMap<u64, PinEntry>>>);

impl PinStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u64, PinEntry>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn put(&self, id: u64, e: PinEntry) {
        self.lock().insert(id, e);
    }

    pub fn get(&self, id: u64) -> Option<PinEntry> {
        self.lock().get(&id).cloned()
    }

    pub fn remove(&self, id: u64) {
        self.lock().remove(&id);
    }

    fn set_zoom(&self, id: u64, zoom: u32) {
        if let Some(e) = self.lock().get_mut(&id) {
            e.zoom = zoom;
        }
    }
}

/// 핀 화면이 처음 묻는 것.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinView {
    pub w: u32,
    pub h: u32,
    pub url: String,
    pub zoom: u32,
}

pub fn view(store: &PinStore, id: u64) -> Option<PinView> {
    store.get(id).map(|e| PinView { w: e.w, h: e.h, url: pin_url(id), zoom: e.zoom })
}

pub struct TauriPinHost<R: Runtime> {
    app: AppHandle<R>,
    store: PinStore,
}

impl<R: Runtime> TauriPinHost<R> {
    pub fn new(app: AppHandle<R>, store: PinStore) -> Self {
        Self { app, store }
    }
}

impl<R: Runtime> ink_app::PinHost for TauriPinHost<R> {
    fn open(&mut self, id: u64, rect: PhysRect, w: u32, h: u32, bgra: Arc<Vec<u8>>) -> Result<(), String> {
        self.store.put(id, PinEntry { w, h, bgra, zoom: 100 });
        // 창 만들기는 명령(메인 스레드) 안에서 하면 WebView2가 교착한다 → 다른 스레드에서(완료를 기다리지 않음)
        let app = self.app.clone();
        std::thread::spawn(move || {
            let built = WebviewWindowBuilder::new(&app, label(id), WebviewUrl::App(format!("index.html?view=pin&pin={id}").into()))
                .title("ShookShok pin")
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .focused(true)
                .build();
            match built {
                Ok(win) => {
                    // 맥: 혼합 배율에서 Tauri 물리 좌표는 옆 화면으로 어긋난다 → 메인 스레드에서 포인트로 직접 놓는다(M16)
                    #[cfg(target_os = "macos")]
                    {
                        let bounds = PhysRect::new(rect.x, rect.y, w.max(1) as i32, h.max(1) as i32);
                        let _ = app.run_on_main_thread(move || {
                            let placed = win.ns_window().is_ok_and(|ns| ink_platform_macos::window::place_rect(ns, &ink_platform_macos::space(), bounds));
                            if !placed {
                                let _ = win.set_position(PhysicalPosition::new(rect.x, rect.y));
                                let _ = win.set_size(PhysicalSize::new(w.max(1), h.max(1)));
                            }
                            let _ = win.show();
                            let _ = win.set_focus();
                        });
                    }
                    // 위치 먼저(그 모니터 배율을 받게) → 크기(원래 그림 1:1)
                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = win.set_position(PhysicalPosition::new(rect.x, rect.y));
                        let _ = win.set_size(PhysicalSize::new(w.max(1), h.max(1)));
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
                Err(e) => log::warn!("pin window {id}: {e}"),
            }
        });
        Ok(())
    }

    fn close(&mut self, id: u64) {
        self.store.remove(id);
        if let Some(w) = self.app.get_webview_window(&label(id)) {
            let _ = w.destroy();
        }
    }
}

/// 휠 한 칸 확대·축소: 커서 자리(창 안 비율 `fx`·`fy`)를 그대로 두고 창 크기·자리를 바꾼다. 새 배율(%).
pub fn zoom<R: Runtime>(win: &tauri::WebviewWindow<R>, store: &PinStore, id: u64, dy: i32, fx: f64, fy: f64) -> Option<u32> {
    let e = store.get(id)?;
    let z = next_zoom(e.zoom, dy);
    if z == e.zoom {
        return Some(z);
    }
    // 맥: 지금 자리도 포인트에서 계약 물리 px로 읽고, 새 자리도 포인트로 놓는다(혼합 배율, open과 같은 까닭)
    #[cfg(target_os = "macos")]
    let space = ink_platform_macos::space();
    #[cfg(target_os = "macos")]
    let mac_cur = win.ns_window().ok().and_then(|ns| ink_platform_macos::window::frame_phys(ns, &space));
    #[cfg(not(target_os = "macos"))]
    let mac_cur: Option<PhysRect> = None;
    let cur = match mac_cur {
        Some(r) => r,
        None => {
            let pos = win.outer_position().ok()?;
            let size = win.outer_size().ok()?;
            PhysRect::new(pos.x, pos.y, size.width as i32, size.height as i32)
        }
    };
    let next = zoom_rect(cur, e.w, e.h, z, (fx, fy));
    #[cfg(target_os = "macos")]
    let placed = win.ns_window().is_ok_and(|ns| ink_platform_macos::window::place_rect(ns, &space, next));
    #[cfg(not(target_os = "macos"))]
    let placed = false;
    if !placed {
        let _ = win.set_size(PhysicalSize::new(next.w as u32, next.h as u32));
        let _ = win.set_position(PhysicalPosition::new(next.x, next.y));
    }
    store.set_zoom(id, z);
    Some(z)
}

/// 오른쪽 클릭 메뉴 [복사][저장] | [닫기][모두 닫기] — 고르면 `pin:<동작>:<id>` 메뉴 사건.
pub fn popup_menu<R: Runtime>(win: &tauri::WebviewWindow<R>, id: u64) -> tauri::Result<()> {
    let app = win.app_handle();
    let item = |action: &str, text: &str| MenuItem::with_id(app, format!("{MENU_PREFIX}{action}:{id}"), text, true, None::<&str>);
    let menu = Menu::with_items(
        app,
        &[&item("copy", "복사")?, &item("save", "저장")?, &PredefinedMenuItem::separator(app)?, &item("close", "닫기")?, &item("closeAll", "모두 닫기")?],
    )?;
    win.popup_menu(&menu)
}

/// 메뉴 항목 id → (동작, 핀 id).
pub fn parse_menu_id(id: &str) -> Option<(&str, u64)> {
    let rest = id.strip_prefix(MENU_PREFIX)?;
    let (action, n) = rest.rsplit_once(':')?;
    Some((action, n.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 핀_이름과_메뉴_id() {
        assert_eq!(label(3), "ink-pin-3");
        assert!(pin_url(3).ends_with("/pin/3"));
        assert_eq!(parse_menu_id("pin:closeAll:12"), Some(("closeAll", 12)));
        assert_eq!(parse_menu_id("pin:copy:x"), None);
        assert_eq!(parse_menu_id("tray:quit"), None);
    }

    #[test]
    fn 그림_저장소는_배율을_기억한다() {
        let s = PinStore::default();
        s.put(1, PinEntry { w: 4, h: 2, bgra: Arc::new(vec![0; 32]), zoom: 100 });
        s.set_zoom(1, 150);
        let v = view(&s, 1).unwrap();
        assert_eq!((v.w, v.h, v.zoom), (4, 2, 150));
        s.remove(1);
        assert!(view(&s, 1).is_none());
    }
}
