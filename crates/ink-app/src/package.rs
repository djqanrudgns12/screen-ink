//! 패키지 모드(MSIX)·로그인 자동 실행·OS 업데이터 포트 연결(D3 · 11-distribution §5·§7).
//!
//! `Ports` 구조체를 늘리지 않고 `InkService::set_*`로 꽂는다(기본 = `UnpackagedRuntime`·`NoopAutostart`·`NoopUpdater`).
//! 맥·가짜 어댑터는 손대지 않고 그대로 빌드된다.

use crate::ports::{AutostartControl, NoopAutostart, NoopUpdater, PackageRuntime, ReadyUpdate, UnpackagedRuntime, Updater};
use crate::service::InkService;
use ink_domain::package::{AutostartNotice, InstallChannel};

pub(crate) struct PackageState {
    pub(crate) runtime: Box<dyn PackageRuntime>,
    pub(crate) autostart: Box<dyn AutostartControl>,
    pub(crate) updater: Box<dyn Updater>,
    pub(crate) autostart_notice: Option<AutostartNotice>,
}

impl Default for PackageState {
    fn default() -> Self {
        Self {
            runtime: Box::new(UnpackagedRuntime),
            autostart: Box::new(NoopAutostart),
            updater: Box::new(NoopUpdater),
            autostart_notice: None,
        }
    }
}

impl InkService {
    /// 패키지 런타임 어댑터를 꽂는다(기본 = `UnpackagedRuntime`).
    pub fn set_package_runtime(&mut self, runtime: Box<dyn PackageRuntime>) {
        self.pkg.runtime = runtime;
    }

    /// 로그인 자동 실행 어댑터를 꽂는다(기본 = `NoopAutostart`).
    pub fn set_autostart(&mut self, autostart: Box<dyn AutostartControl>) {
        self.pkg.autostart = autostart;
    }

    /// OS 업데이터 어댑터를 꽂는다(기본 = `NoopUpdater`).
    pub fn set_updater(&mut self, updater: Box<dyn Updater>) {
        self.pkg.updater = updater;
    }

    /// 현재 설치 채널(`Unpackaged` | `MsixStore` | `MsixDirect`).
    pub fn install_channel(&self) -> InstallChannel {
        self.pkg.runtime.channel()
    }

    /// MSIX 패키지 모드 여부.
    pub fn is_packaged(&self) -> bool {
        self.install_channel().is_packaged()
    }

    /// 현재 로그인 자동 실행 안내(`DisabledByUser`·정책 차단 시 설정 화면에 한 줄 표시).
    pub fn autostart_notice(&self) -> Option<&AutostartNotice> {
        self.pkg.autostart_notice.as_ref()
    }

    /// 자동 실행 안내 상태를 갱신하고 바뀌면 UI로 알린다.
    pub fn set_autostart_notice(&mut self, notice: Option<AutostartNotice>) {
        let next = notice.filter(|n| n.guidance_ko.is_some());
        if self.pkg.autostart_notice != next {
            self.pkg.autostart_notice = next;
            self.emit_state();
        }
    }

    /// 연결된 `AutostartControl` 포트로 자동 실행을 반영하고 안내 문구를 상태에 기록한다.
    pub fn sync_autostart_via_port(&mut self, enabled: bool) -> Result<AutostartNotice, String> {
        let notice = self.pkg.autostart.set_enabled(enabled)?;
        self.set_autostart_notice(Some(notice.clone()));
        Ok(notice)
    }

    /// 연결된 `Updater` 포트로 준비된 새 버전을 확인한다(1차 비패키지는 `NoopUpdater` → `Ok(None)`).
    pub fn check_update(&mut self) -> Result<Option<ReadyUpdate>, String> {
        self.pkg.updater.check_and_stage()
    }

    /// 연결된 `Updater` 포트로 업데이트 설치 및 자동 재시작을 실행한다.
    pub fn install_update(&mut self) -> Result<(), String> {
        self.pkg.updater.install_and_restart()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::*;
    use crate::state::{InkState, IslandEvent, OverlaySession};
    use ink_domain::geometry::{MonitorLayout, PhysPoint};
    use ink_domain::naming::NameContext;
    use ink_domain::package::{evaluate_startup_task, AutostartBackend, StartupTaskState};
    use ink_domain::shortcuts::{LiveBinding, Platform};
    use ink_domain::snapshot::RawWindow;
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    struct DummyHotkeys;
    impl HotkeyRegistry for DummyHotkeys {
        fn register(&mut self, _s: &str, _a: &str) -> Result<(), HotkeyError> {
            Ok(())
        }
        fn unregister(&mut self, _a: &str) {}
    }
    struct DummyProbe;
    impl SystemProbe for DummyProbe {
        fn running_apps(&self) -> Vec<String> {
            vec![]
        }
        fn live_bindings(&self) -> Option<Vec<LiveBinding>> {
            Some(vec![])
        }
    }
    struct DummyStore {
        portable: bool,
    }
    impl SettingsStore for DummyStore {
        fn load(&mut self) -> Loaded {
            Loaded::default()
        }
        fn save_section(&mut self, _name: &str, _data: &Value) -> Result<(), StoreError> {
            Ok(())
        }
        fn location(&self) -> StoreLocation {
            StoreLocation {
                path: "C:\\Users\\t\\AppData\\Roaming\\Shookshok\\settings.json".into(),
                portable: self.portable,
                read_only: false,
            }
        }
    }
    #[derive(Clone, Default)]
    struct CountingNotifier(Arc<Mutex<usize>>);
    impl Notifier for CountingNotifier {
        fn state(&self, _state: &InkState) {
            *self.0.lock().unwrap() += 1;
        }
        fn island(&self, _event: &IslandEvent) {}
    }
    struct DummyScreen;
    impl ScreenSource for DummyScreen {
        fn capture(&mut self, _layout: &MonitorLayout) -> Result<Vec<Frame>, String> {
            Ok(vec![])
        }
    }
    impl WindowCatalog for DummyScreen {
        fn windows(&self) -> Vec<RawWindow> {
            vec![]
        }
    }
    impl MonitorProbe for DummyScreen {
        fn layout(&self) -> MonitorLayout {
            MonitorLayout::default()
        }
        fn cursor(&self) -> PhysPoint {
            PhysPoint::default()
        }
    }
    struct DummyOverlay;
    impl OverlayHost for DummyOverlay {
        fn prepare(&mut self, _layout: &MonitorLayout) {}
        fn show(&mut self, _s: &OverlaySession, _f: Vec<Frame>) -> Result<Option<std::time::Instant>, String> {
            Ok(None)
        }
        fn highlight(&mut self, _s: u64, _r: Option<ink_domain::geometry::PhysRect>) {}
        fn hide(&mut self, _s: u64) {}
    }
    struct DummyElements;
    impl ElementSource for DummyElements {
        fn request(&self, _w: u64, _e: &str) {}
        fn elements_at(&self, _w: u64, _p: PhysPoint) -> Vec<ink_domain::recog::ElementCand> {
            vec![]
        }
        fn clear(&self) {}
    }
    struct DummyDesk;
    impl Clipboard for DummyDesk {
        fn set_image(&mut self, _w: u32, _h: u32, _bgra: &[u8]) -> Result<(), String> {
            Ok(())
        }
        fn set_text(&mut self, _text: &str) -> Result<(), String> {
            Ok(())
        }
    }
    impl FileSink for DummyDesk {
        fn save_png(&mut self, _f: &str, _s: &str, _w: u32, _h: u32, _b: &[u8]) -> Result<SavedFile, String> {
            Ok(SavedFile { name: "a.png".into(), path: "a.png".into(), fell_back: false })
        }
    }
    struct DummyClock;
    impl Clock for DummyClock {
        fn local(&self) -> NameContext {
            NameContext { year: 2026, month: 11, day: 26, hour: 12, minute: 0, second: 0, app: None, n: 0 }
        }
        fn unix(&self) -> u64 {
            0
        }
    }

    fn make_service(portable: bool) -> (InkService, CountingNotifier) {
        let notifier = CountingNotifier::default();
        let ports = Ports {
            hotkeys: Box::new(DummyHotkeys),
            probe: Box::new(DummyProbe),
            store: Box::new(DummyStore { portable }),
            notifier: Box::new(notifier.clone()),
            screen: Box::new(DummyScreen),
            windows: Box::new(DummyScreen),
            monitors: Box::new(DummyScreen),
            overlay: Box::new(DummyOverlay),
            elements: Box::new(DummyElements),
            vision: Box::new(NoVision),
            clipboard: Box::new(DummyDesk),
            files: Box::new(DummyDesk),
            pins: Box::new(NoPins),
            clock: Box::new(DummyClock),
        };
        (InkService::new(ports, Platform::Win, "0.1.0"), notifier)
    }

    struct StubMsixRuntime;
    impl PackageRuntime for StubMsixRuntime {
        fn channel(&self) -> InstallChannel {
            InstallChannel::MsixStore
        }
        fn package_full_name(&self) -> Option<String> {
            Some("com.ssamuniverse.shookshok_0.1.0.0_x64__abc".into())
        }
    }

    struct StubBlockedAutostart;
    impl AutostartControl for StubBlockedAutostart {
        fn backend(&self) -> AutostartBackend {
            AutostartBackend::StartupTask
        }
        fn set_enabled(&mut self, enabled: bool) -> Result<AutostartNotice, String> {
            Ok(evaluate_startup_task(enabled, StartupTaskState::DisabledByUser))
        }
    }

    struct StubUpdater;
    impl Updater for StubUpdater {
        fn check_and_stage(&mut self) -> Result<Option<ReadyUpdate>, String> {
            Ok(Some(ReadyUpdate { version: "0.2.0.0".into(), channel: "store" }))
        }
    }

    #[test]
    fn 기본값은_비패키지_noop이고_어댑터를_꽂으면_상태에_반영된다() {
        let (mut svc, notifier) = make_service(true);
        assert_eq!(svc.install_channel(), InstallChannel::Unpackaged);
        assert!(!svc.is_packaged());
        assert!(svc.state().location.portable);
        assert_eq!(svc.state().autostart_notice, None);
        assert_eq!(svc.check_update(), Ok(None));

        // MSIX 런타임 연결 시 channel이 msix-store로 바뀌고 포터블 표시는 꺼진다.
        svc.set_package_runtime(Box::new(StubMsixRuntime));
        assert!(svc.is_packaged());
        let st = svc.state();
        assert_eq!(st.channel, InstallChannel::MsixStore);
        assert!(!st.location.portable);

        // DisabledByUser 시 설정 화면 안내 한 줄이 상태에 실리고 알림이 나간다.
        let before = *notifier.0.lock().unwrap();
        svc.set_autostart(Box::new(StubBlockedAutostart));
        let notice = svc.sync_autostart_via_port(true).unwrap();
        assert!(notice.blocked_by_user);
        assert_eq!(svc.state().autostart_notice.as_deref(), Some("Windows 시작 앱 설정에서 켜 주세요"));
        assert!(*notifier.0.lock().unwrap() > before);

        // 끄기로 전환하면 안내가 걷힌다.
        svc.sync_autostart_via_port(false).unwrap();
        assert_eq!(svc.state().autostart_notice, None);

        // 업데이터 포트 연결 확인
        svc.set_updater(Box::new(StubUpdater));
        assert_eq!(svc.check_update(), Ok(Some(ReadyUpdate { version: "0.2.0.0".into(), channel: "store" })));
    }
}
