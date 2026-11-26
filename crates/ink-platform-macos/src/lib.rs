//! macOS 어댑터. 판정은 하지 않고 사실만 읽는다(창 제목은 읽지 않는다, 05 개인정보).
//! 좌표는 모두 [`coords`]를 거쳐 계약 좌표(가상 화면 물리 px, 왼쪽 위 원점)로 바꾼다.

pub mod coords;

#[cfg(target_os = "macos")]
pub mod ax;
#[cfg(target_os = "macos")]
pub mod capture;
#[cfg(target_os = "macos")]
pub mod clipboard;
#[cfg(target_os = "macos")]
pub mod cursor;
#[cfg(target_os = "macos")]
pub mod display;
#[cfg(target_os = "macos")]
pub mod events;
#[cfg(target_os = "macos")]
pub mod freeze;
#[cfg(target_os = "macos")]
pub mod permissions;
#[cfg(target_os = "macos")]
mod pixels;
#[cfg(target_os = "macos")]
pub mod recorder;
#[cfg(target_os = "macos")]
pub mod window;
#[cfg(target_os = "macos")]
pub mod window_shot;
#[cfg(target_os = "macos")]
pub mod windows;

#[cfg(target_os = "macos")]
pub use mac::*;

#[cfg(target_os = "macos")]
mod mac {
    use crate::coords::Space;
    use crate::{capture, cursor, display, windows};
    use ink_app::{Frame, MonitorProbe, ScreenSource, SystemProbe, WindowCatalog};
    use ink_domain::geometry::{MonitorLayout, PhysPoint};
    use ink_domain::shortcuts::LiveBinding;
    use ink_domain::snapshot::RawWindow;
    use std::sync::Mutex;

    /// 마지막으로 읽은 배치. 세 포트(캡처·창 목록·모니터)와 오버레이 창 놓기가 모두 같은 변환을 쓴다.
    static SPACE: Mutex<Option<Space>> = Mutex::new(None);

    /// 지금 배치(아직 안 읽었으면 새로 읽음).
    pub fn space() -> Space {
        let mut g = SPACE.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(display::space).clone()
    }

    /// 배치를 다시 읽는다(진입마다·디스플레이 변경). 바뀌었으면 캡처 내용 목록도 버린다.
    pub fn refresh_space() -> Space {
        let s = display::space();
        let mut g = SPACE.lock().unwrap_or_else(|e| e.into_inner());
        if g.as_ref() != Some(&s) {
            capture::invalidate();
            *g = Some(s.clone());
        }
        s
    }

    /// `ScreenSource` + `WindowCatalog` + `MonitorProbe` 한 몸.
    #[derive(Default)]
    pub struct MacScreen;

    impl ScreenSource for MacScreen {
        fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String> {
            capture::capture(layout, &space())
        }

        fn cursor_sprite(&mut self) -> Option<ink_domain::image::CursorSprite> {
            cursor::sprite(&space())
        }

        fn prepare_window(&mut self, window: Option<u64>) {
            crate::window_shot::prepare(window);
        }

        fn capture_window(&mut self, window: u64, bounds: ink_domain::geometry::PhysRect) -> Result<ink_app::WindowShot, String> {
            crate::window_shot::shoot(window, bounds)
        }
    }

    impl WindowCatalog for MacScreen {
        fn windows(&self) -> Vec<RawWindow> {
            windows::top_windows(&space())
        }
    }

    impl MonitorProbe for MacScreen {
        fn layout(&self) -> MonitorLayout {
            let layout = refresh_space().layout();
            // 첫 진입이 내용 목록 받기(수십 ms)를 기다리지 않게
            capture::warm();
            layout
        }

        fn cursor(&self) -> PhysPoint {
            display::cursor(&space())
        }

        fn set_cursor(&self, p: PhysPoint) {
            display::set_cursor(&space(), p)
        }

        fn alt_down(&self) -> bool {
            display::alt_down()
        }
    }

    /// `SystemProbe`: 실행 중인 앱(실행 파일 이름, 소문자). PowerToys 같은 다른 앱 설정 읽기는 맥에 없다.
    #[derive(Default)]
    pub struct MacProbe;

    impl SystemProbe for MacProbe {
        fn running_apps(&self) -> Vec<String> {
            use objc2_app_kit::NSWorkspace;
            let ws = NSWorkspace::sharedWorkspace();
            let mut out: Vec<String> = ws
                .runningApplications()
                .iter()
                .filter_map(|a| a.executableURL().and_then(|u| u.lastPathComponent()).map(|s| s.to_string().to_lowercase()))
                .collect();
            out.sort();
            out.dedup();
            out
        }

        fn live_bindings(&self) -> Option<Vec<LiveBinding>> {
            Some(Vec::new())
        }

        fn secure_desktop(&self) -> bool {
            crate::events::screen_locked()
        }

        fn permissions(&self) -> Option<(bool, bool)> {
            Some((crate::permissions::screen(), crate::permissions::accessibility()))
        }
    }

    /// 지역 시각(이름 규칙 {yyyy}…{ss}).
    pub fn local_time() -> ink_domain::naming::NameContext {
        // SAFETY: localtime_r은 출력 구조체에만 쓴다.
        let t = unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut tm);
            tm
        };
        ink_domain::naming::NameContext {
            year: (t.tm_year + 1900) as u16,
            month: (t.tm_mon + 1) as u8,
            day: t.tm_mday as u8,
            hour: t.tm_hour as u8,
            minute: t.tm_min as u8,
            second: t.tm_sec as u8,
            app: None,
            n: 0,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn 이_맥의_모니터와_창() {
            let s = MacScreen;
            let l = s.layout();
            assert!(!l.monitors.is_empty());
            assert_eq!(l.monitors.iter().filter(|m| m.primary).count(), 1);
            for (i, a) in l.monitors.iter().enumerate() {
                for b in &l.monitors[i + 1..] {
                    assert!(a.bounds.intersect(&b.bounds).is_none(), "겹침 {a:?} {b:?}");
                }
                assert!(a.work.w > 0 && a.work.h > 0);
            }
            eprintln!("layout packed={} {:?}", space().packed, l.monitors.iter().map(|m| (m.id, m.bounds, m.scale)).collect::<Vec<_>>());
            let ws = s.windows();
            assert!(ws.iter().all(|w| !w.exe.contains('/')), "경로 없이 이름만");
            let c = s.cursor();
            assert!(l.nearest(c).is_some());
            let t = local_time();
            assert!(t.year >= 2025);
        }
    }
}
