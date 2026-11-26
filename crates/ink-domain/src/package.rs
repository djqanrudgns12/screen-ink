//! 패키지 모드(MSIX)·자동 시작·저장/로그 위치·업데이트 경로 순수 규칙(D3 · 11-distribution §5·§7).
//!
//! 운영체제 호출은 어댑터(`ink-platform-windows::package`)가 사실만 읽어 넘기고,
//! 채널 분류·포터블 표시 파일 무시·자동 시작 안내·업데이트 연기 판정은 모두 여기서 한다.

use serde::{Deserialize, Serialize};

/// MSIX 매니페스트 `<desktop:StartupTask TaskId="...">` 식별자.
pub const STARTUP_TASK_ID: &str = "ShookshokStartup";

/// 직접 배포판 `.appinstaller` 고정 주소(D12 확정, 11-distribution §7.1·§10.2).
pub const APPINSTALLER_URI: &str = "https://ink.ssamuniverse.com/shookshok.appinstaller";

/// 설치 경로(채널) — 패키지 여부와 서명·AppInstaller 존재 여부로 나눈다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallChannel {
    /// NSIS 설치본 또는 포터블 ZIP(비패키지).
    Unpackaged,
    /// Microsoft Store MSIX 패키지(스토어 서명·스토어 자동 업데이트).
    MsixStore,
    /// 서명 MSIX + `.appinstaller` 직접 배포(또는 사이드로드).
    MsixDirect,
}

impl InstallChannel {
    /// MSIX 패키지(앱 모델) 안에서 실행 중인지.
    pub fn is_packaged(self) -> bool {
        matches!(self, Self::MsixStore | Self::MsixDirect)
    }

    /// UI·로그용 고정 식별자.
    pub fn id(self) -> &'static str {
        match self {
            Self::Unpackaged => "unpackaged",
            Self::MsixStore => "msix-store",
            Self::MsixDirect => "msix-direct",
        }
    }

    /// `ReadyUpdate.channel` 표기(`"store"` | `"direct"`). 비패키지면 `None`.
    pub fn update_channel(self) -> Option<&'static str> {
        match self {
            Self::Unpackaged => None,
            Self::MsixStore => Some("store"),
            Self::MsixDirect => Some("direct"),
        }
    }
}

/// WinRT `Windows.ApplicationModel.PackageSignatureKind` 대응 값.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageSignature {
    Unknown,
    None,
    Developer,
    Enterprise,
    Store,
    System,
}

impl PackageSignature {
    /// WinRT `PackageSignatureKind` 정수 값(0..=4)을 변환한다.
    pub fn from_winrt(raw: i32) -> Self {
        match raw {
            0 => Self::None,
            1 => Self::Developer,
            2 => Self::Enterprise,
            3 => Self::Store,
            4 => Self::System,
            _ => Self::Unknown,
        }
    }
}

/// 패키지 여부·AppInstaller 정보·서명 종류로 설치 채널을 판정한다.
///
/// 우선순위:
/// 1. 패키지가 아니면 항상 `Unpackaged`.
/// 2. `GetAppInstallerInfo()`가 있으면 `.appinstaller`로 설치된 `MsixDirect`.
/// 3. 서명 종류가 `Store`면 `MsixStore`.
/// 4. 서명 종류가 `Developer`·`Enterprise`(자체 서명·SignPath·학교 정책 배포)면 `MsixDirect`.
/// 5. 그 밖(WinRT 조회 불가 등)은 `MsixStore`로 본다.
pub fn classify_channel(is_packaged: bool, has_app_installer: bool, signature: PackageSignature) -> InstallChannel {
    if !is_packaged {
        return InstallChannel::Unpackaged;
    }
    if has_app_installer {
        return InstallChannel::MsixDirect;
    }
    match signature {
        PackageSignature::Store => InstallChannel::MsixStore,
        PackageSignature::Developer | PackageSignature::Enterprise => InstallChannel::MsixDirect,
        PackageSignature::None | PackageSignature::System | PackageSignature::Unknown => InstallChannel::MsixStore,
    }
}

/// 설정 저장 방식(S05 일반 > 설정 위치 표기와 포터블 판정).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageMode {
    /// 비패키지 + 실행 파일 옆 `Shookshok.portable`(또는 옛 `screen-ink.portable`) 있음 → `exe_dir/data`.
    Portable,
    /// 비패키지 + 표시 파일 없음 → `%APPDATA%\Shookshok`.
    Installed,
    /// MSIX 패키지 → 설치 폴더 읽기 전용, 포터블 표시 파일 무시, 패키지 가상화 `%APPDATA%\Shookshok`.
    MsixPackaged,
}

/// 채널과 포터블 표시 파일 유무에 따른 설정 저장 정책.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoragePolicy {
    pub mode: StorageMode,
    /// 패키지 모드에서는 설치 폴더가 읽기 전용이므로 `screen-ink.portable`을 무시한다.
    pub ignore_portable_marker: bool,
    /// `exe_dir/data`를 쓸지 여부.
    pub use_portable_dir: bool,
    /// Windows 패키지 가상화(`%LOCALAPPDATA%\Packages\<PFN>\LocalCache\Roaming\...`) 적용 여부.
    pub virtualized_by_package: bool,
    /// 앱 제거 시 설정도 함께 지워지는지(11-distribution §5).
    pub removed_on_uninstall: bool,
    pub label_ko: &'static str,
    pub label_en: &'static str,
}

/// 설정 저장 정책을 판정한다.
pub fn resolve_storage_policy(channel: InstallChannel, has_portable_marker: bool) -> StoragePolicy {
    if channel.is_packaged() {
        StoragePolicy {
            mode: StorageMode::MsixPackaged,
            ignore_portable_marker: true,
            use_portable_dir: false,
            virtualized_by_package: true,
            removed_on_uninstall: true,
            label_ko: "MSIX 패키지",
            label_en: "MSIX package",
        }
    } else if has_portable_marker {
        StoragePolicy {
            mode: StorageMode::Portable,
            ignore_portable_marker: false,
            use_portable_dir: true,
            virtualized_by_package: false,
            removed_on_uninstall: false,
            label_ko: "포터블",
            label_en: "Portable",
        }
    } else {
        StoragePolicy {
            mode: StorageMode::Installed,
            ignore_portable_marker: false,
            use_portable_dir: false,
            virtualized_by_package: false,
            removed_on_uninstall: false,
            label_ko: "설치",
            label_en: "Installed",
        }
    }
}

/// 로그 저장 위치 종류.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogLocation {
    /// 일반 설치본의 OS 로그 폴더(`%LOCALAPPDATA%\<id>\logs`).
    OsLogDir,
    /// 포터블 실행 폴더 아래 `data\logs`.
    PortableDataLogs,
    /// MSIX 패키지 가상화 로그 폴더(제거 시 함께 정리됨).
    PackageVirtualizedLogDir,
}

/// 로그 저장 정책.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogPolicy {
    pub location: LogLocation,
    pub virtualized_by_package: bool,
    pub removed_on_uninstall: bool,
}

/// 채널과 포터블 표시 파일 유무에 따른 로그 위치 정책을 판정한다.
pub fn resolve_log_policy(channel: InstallChannel, has_portable_marker: bool) -> LogPolicy {
    if channel.is_packaged() {
        LogPolicy {
            location: LogLocation::PackageVirtualizedLogDir,
            virtualized_by_package: true,
            removed_on_uninstall: true,
        }
    } else if has_portable_marker {
        LogPolicy {
            location: LogLocation::PortableDataLogs,
            virtualized_by_package: false,
            removed_on_uninstall: false,
        }
    } else {
        LogPolicy {
            location: LogLocation::OsLogDir,
            virtualized_by_package: false,
            removed_on_uninstall: false,
        }
    }
}

/// 로그인 자동 실행 구현 경로(11-distribution §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutostartBackend {
    /// 비패키지: Run 레지스트리(`tauri-plugin-autostart`).
    RegistryRun,
    /// MSIX 패키지: `Windows.ApplicationModel.StartupTask` (`ShookshokStartup`).
    StartupTask,
}

/// 채널별 자동 시작 구현 경로를 고른다.
pub fn autostart_backend(channel: InstallChannel) -> AutostartBackend {
    if channel.is_packaged() {
        AutostartBackend::StartupTask
    } else {
        AutostartBackend::RegistryRun
    }
}

/// WinRT `Windows.ApplicationModel.StartupTaskState` 대응 값.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupTaskState {
    Disabled,
    DisabledByUser,
    Enabled,
    DisabledByPolicy,
    EnabledByPolicy,
}

impl StartupTaskState {
    /// WinRT `StartupTaskState` 정수 값(0..=4)을 변환한다.
    pub fn from_winrt(raw: i32) -> Option<Self> {
        match raw {
            0 => Some(Self::Disabled),
            1 => Some(Self::DisabledByUser),
            2 => Some(Self::Enabled),
            3 => Some(Self::DisabledByPolicy),
            4 => Some(Self::EnabledByPolicy),
            _ => None,
        }
    }

    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled | Self::EnabledByPolicy)
    }
}

/// `StartupTask` 반영 뒤 설정 화면에 보여 줄 상태·안내.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutostartNotice {
    pub effective_enabled: bool,
    pub blocked_by_user: bool,
    pub blocked_by_policy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance_ko: Option<String>,
}

/// 사용자가 바라는 자동 시작 상태와 OS가 돌려준 `StartupTaskState`를 대조해 안내 문구를 만든다.
pub fn evaluate_startup_task(desired_enabled: bool, state: StartupTaskState) -> AutostartNotice {
    match (desired_enabled, state) {
        (true, StartupTaskState::DisabledByUser) => AutostartNotice {
            effective_enabled: false,
            blocked_by_user: true,
            blocked_by_policy: false,
            guidance_ko: Some("Windows 시작 앱 설정에서 켜 주세요".into()),
        },
        (true, StartupTaskState::DisabledByPolicy) => AutostartNotice {
            effective_enabled: false,
            blocked_by_user: false,
            blocked_by_policy: true,
            guidance_ko: Some("학교·조직 정책으로 자동 실행이 막혀 있어요".into()),
        },
        (false, StartupTaskState::EnabledByPolicy) => AutostartNotice {
            effective_enabled: true,
            blocked_by_user: false,
            blocked_by_policy: true,
            guidance_ko: Some("학교·조직 정책으로 자동 실행이 고정되어 있어요".into()),
        },
        _ => AutostartNotice {
            effective_enabled: state.is_enabled(),
            blocked_by_user: false,
            blocked_by_policy: false,
            guidance_ko: None,
        },
    }
}

/// 채널별 업데이트 경로(11-distribution §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateBackend {
    /// 비패키지 1차 빌드: 앱 안 업데이터 없음(네트워크 0).
    None,
    /// Microsoft Store: `Windows.Services.Store.StoreContext`.
    StoreContext,
    /// 직접 배포 MSIX: `Package::CheckUpdateAvailabilityAsync` + `.appinstaller` + `PackageManager`.
    AppInstaller,
}

/// 채널별 업데이트 경로를 고른다.
pub fn update_backend(channel: InstallChannel) -> UpdateBackend {
    match channel {
        InstallChannel::Unpackaged => UpdateBackend::None,
        InstallChannel::MsixStore => UpdateBackend::StoreContext,
        InstallChannel::MsixDirect => UpdateBackend::AppInstaller,
    }
}

/// 업데이트 팝업 연기 판정 입력(11-distribution §7.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateDeferInput<'a> {
    /// 현재 모드가 `Idle`인지(`mode == Mode::Idle`).
    pub mode_idle: bool,
    /// 녹화 활성 중인지.
    pub recording_active: bool,
    /// 전체 화면 앱(슬라이드 쇼·영상 등)이 앞에 있는지(`SHQueryUserNotificationState`).
    pub fullscreen_app: bool,
    /// UAC·잠금 화면 등 보안 데스크톱인지.
    pub secure_desktop: bool,
    /// 사용자가 "오늘은 그만"을 누른 지역 날짜(`YYYY-MM-DD`).
    pub snooze_date: Option<&'a str>,
    /// 오늘 지역 날짜(`YYYY-MM-DD`).
    pub today_local: &'a str,
}

/// 업데이트 팝업을 미루는 이유.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateDeferReason {
    ActiveMode,
    Recording,
    FullscreenApp,
    SecureDesktop,
    SnoozedToday,
}

/// 지금 업데이트 팝업을 미뤄야 하는 이유를 돌려준다. `None`이면 즉시 띄울 수 있다.
pub fn update_defer_reason(input: &UpdateDeferInput<'_>) -> Option<UpdateDeferReason> {
    if !input.mode_idle {
        return Some(UpdateDeferReason::ActiveMode);
    }
    if input.recording_active {
        return Some(UpdateDeferReason::Recording);
    }
    if input.fullscreen_app {
        return Some(UpdateDeferReason::FullscreenApp);
    }
    if input.secure_desktop {
        return Some(UpdateDeferReason::SecureDesktop);
    }
    if let Some(snoozed) = input.snooze_date {
        if !snoozed.is_empty() && snoozed == input.today_local {
            return Some(UpdateDeferReason::SnoozedToday);
        }
    }
    None
}

/// 업데이트 팝업을 미뤄야 하는지.
pub fn should_defer_update(input: &UpdateDeferInput<'_>) -> bool {
    update_defer_reason(input).is_some()
}

/// Win32 `SHQueryUserNotificationState` 결과(`QUERY_USER_NOTIFICATION_STATE`)가
/// 발표·전체 화면 앱 상태인지 판정한다.
/// - `2` = `QUNS_BUSY`
/// - `3` = `QUNS_RUNNING_D3D_FULL_SCREEN`
/// - `4` = `QUNS_PRESENTATION_MODE`
pub fn is_fullscreen_notification_state(quns: i32) -> bool {
    matches!(quns, 2..=4)
}

/// 3자리 SemVer(`0.1.0`) 또는 4자리 MSIX 버전(`0.1.0.0`)을 스토어 규칙에 맞는 4자리 MSIX 버전으로 정규화한다.
/// 스토어 규칙: 4번째 자리는 반드시 `0`, 각 자리는 `0..=65535`.
pub fn to_msix_version(version: &str) -> Result<String, &'static str> {
    let trimmed = version.trim().strip_prefix('v').unwrap_or(version.trim());
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() != 3 && parts.len() != 4 {
        return Err("버전은 X.Y.Z 또는 X.Y.Z.0 형식이어야 해요");
    }
    let mut nums = [0u16; 4];
    for (i, part) in parts.iter().enumerate() {
        nums[i] = part.parse::<u16>().map_err(|_| "버전 각 자리는 0~65535 정수여야 해요")?;
    }
    if nums[3] != 0 {
        return Err("Microsoft Store MSIX 버전의 네 번째 자리는 0이어야 해요");
    }
    Ok(format!("{}.{}.{}.0", nums[0], nums[1], nums[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 채널_분류_비패키지_스토어_직접배포() {
        assert_eq!(classify_channel(false, false, PackageSignature::Unknown), InstallChannel::Unpackaged);
        assert_eq!(classify_channel(false, true, PackageSignature::Store), InstallChannel::Unpackaged);
        assert_eq!(classify_channel(true, false, PackageSignature::Store), InstallChannel::MsixStore);
        assert_eq!(classify_channel(true, true, PackageSignature::Developer), InstallChannel::MsixDirect);
        assert_eq!(classify_channel(true, true, PackageSignature::Store), InstallChannel::MsixDirect);
        assert_eq!(classify_channel(true, false, PackageSignature::Developer), InstallChannel::MsixDirect);
        assert_eq!(classify_channel(true, false, PackageSignature::Enterprise), InstallChannel::MsixDirect);
        assert_eq!(classify_channel(true, false, PackageSignature::Unknown), InstallChannel::MsixStore);
    }

    #[test]
    fn 패키지모드_포터블무시_및_startup_task분기() {
        // 비패키지 + 포터블 표시 파일 있음 → 포터블
        let portable = resolve_storage_policy(InstallChannel::Unpackaged, true);
        assert_eq!(portable.mode, StorageMode::Portable);
        assert!(!portable.ignore_portable_marker);
        assert!(portable.use_portable_dir);
        assert_eq!(portable.label_ko, "포터블");
        assert_eq!(autostart_backend(InstallChannel::Unpackaged), AutostartBackend::RegistryRun);
        assert_eq!(update_backend(InstallChannel::Unpackaged), UpdateBackend::None);
        assert_eq!(resolve_log_policy(InstallChannel::Unpackaged, true).location, LogLocation::PortableDataLogs);

        // 비패키지 + 표시 파일 없음 → 일반 설치
        let installed = resolve_storage_policy(InstallChannel::Unpackaged, false);
        assert_eq!(installed.mode, StorageMode::Installed);
        assert!(!installed.use_portable_dir);
        assert_eq!(installed.label_ko, "설치");
        assert_eq!(resolve_log_policy(InstallChannel::Unpackaged, false).location, LogLocation::OsLogDir);

        // MSIX 스토어/직접 배포: 포터블 표시 파일이 있어도 무시하고 가상화 경로 + StartupTask 사용
        for ch in [InstallChannel::MsixStore, InstallChannel::MsixDirect] {
            let p = resolve_storage_policy(ch, true);
            assert_eq!(p.mode, StorageMode::MsixPackaged);
            assert!(p.ignore_portable_marker);
            assert!(!p.use_portable_dir);
            assert!(p.virtualized_by_package);
            assert!(p.removed_on_uninstall);
            assert_eq!(p.label_ko, "MSIX 패키지");
            assert_eq!(autostart_backend(ch), AutostartBackend::StartupTask);

            let log_p = resolve_log_policy(ch, true);
            assert_eq!(log_p.location, LogLocation::PackageVirtualizedLogDir);
            assert!(log_p.virtualized_by_package);
            assert!(log_p.removed_on_uninstall);
        }
        assert_eq!(update_backend(InstallChannel::MsixStore), UpdateBackend::StoreContext);
        assert_eq!(update_backend(InstallChannel::MsixDirect), UpdateBackend::AppInstaller);
    }

    #[test]
    fn 자동시작_사용자거부_및_정책차단_안내() {
        let user_blocked = evaluate_startup_task(true, StartupTaskState::DisabledByUser);
        assert!(!user_blocked.effective_enabled);
        assert!(user_blocked.blocked_by_user);
        assert_eq!(user_blocked.guidance_ko.as_deref(), Some("Windows 시작 앱 설정에서 켜 주세요"));

        let policy_blocked = evaluate_startup_task(true, StartupTaskState::DisabledByPolicy);
        assert!(!policy_blocked.effective_enabled);
        assert!(policy_blocked.blocked_by_policy);
        assert!(policy_blocked.guidance_ko.is_some());

        let policy_locked = evaluate_startup_task(false, StartupTaskState::EnabledByPolicy);
        assert!(policy_locked.effective_enabled);
        assert!(policy_locked.blocked_by_policy);
        assert!(policy_locked.guidance_ko.is_some());

        let ok = evaluate_startup_task(true, StartupTaskState::Enabled);
        assert!(ok.effective_enabled);
        assert!(!ok.blocked_by_user && !ok.blocked_by_policy);
        assert!(ok.guidance_ko.is_none());
    }

    #[test]
    fn 업데이트_모드중_전체화면_오늘그만_연기판정() {
        let base = UpdateDeferInput {
            mode_idle: true,
            recording_active: false,
            fullscreen_app: false,
            secure_desktop: false,
            snooze_date: None,
            today_local: "2026-11-26",
        };
        assert!(!should_defer_update(&base));
        assert_eq!(update_defer_reason(&base), None);

        assert_eq!(update_defer_reason(&UpdateDeferInput { mode_idle: false, ..base }), Some(UpdateDeferReason::ActiveMode));
        assert_eq!(update_defer_reason(&UpdateDeferInput { recording_active: true, ..base }), Some(UpdateDeferReason::Recording));
        assert_eq!(update_defer_reason(&UpdateDeferInput { fullscreen_app: true, ..base }), Some(UpdateDeferReason::FullscreenApp));
        assert_eq!(update_defer_reason(&UpdateDeferInput { secure_desktop: true, ..base }), Some(UpdateDeferReason::SecureDesktop));
        assert_eq!(update_defer_reason(&UpdateDeferInput { snooze_date: Some("2026-11-26"), ..base }), Some(UpdateDeferReason::SnoozedToday));
        // 어제 "오늘은 그만"을 눌렀으면 오늘은 다시 띄운다
        assert_eq!(update_defer_reason(&UpdateDeferInput { snooze_date: Some("2026-11-25"), ..base }), None);
        assert!(!should_defer_update(&UpdateDeferInput { snooze_date: Some("2026-11-25"), ..base }));

        // QUNS 판정: 2(BUSY), 3(D3D_FULL_SCREEN), 4(PRESENTATION_MODE)만 true
        assert!(!is_fullscreen_notification_state(1));
        assert!(is_fullscreen_notification_state(2));
        assert!(is_fullscreen_notification_state(3));
        assert!(is_fullscreen_notification_state(4));
        assert!(!is_fullscreen_notification_state(5));
        assert!(!is_fullscreen_notification_state(6));
    }

    #[test]
    fn msix_버전_정규화_검증() {
        assert_eq!(to_msix_version("0.1.0"), Ok("0.1.0.0".into()));
        assert_eq!(to_msix_version("v0.2.3"), Ok("0.2.3.0".into()));
        assert_eq!(to_msix_version("1.2.3.0"), Ok("1.2.3.0".into()));
        assert!(to_msix_version("1.2.3.4").is_err());
        assert!(to_msix_version("1.2").is_err());
        assert!(to_msix_version("70000.0.0").is_err());
    }
}
