//! 직접 배포 MSIX의 적용은 Windows에 맡긴다. 비패키지에서는 검증한 파일의 위치만 안내한다.
use crate::package;
use ink_domain::package::{InstallChannel, APPINSTALLER_URI};
use serde::Deserialize;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::Builder;
use windows::core::HSTRING;
use windows::ApplicationModel::Package;
use windows::Foundation::Uri;
use windows::Management::Deployment::{AddPackageByAppInstallerOptions, PackageManager};
use windows_sys::Win32::System::SystemInformation::{GetLocalTime, GetSystemDirectoryW};

const FEED: &str = "https://github.com/djqanrudgns12/screen-ink/releases/latest/download/latest-win.json";
const ASSETS: &str = "https://github.com/djqanrudgns12/screen-ink/releases/download/";

#[derive(Clone)]
pub enum Payload {
    AppInstaller(String),
    VerifiedFile(PathBuf),
}

#[derive(Clone)]
pub struct Ready {
    pub version: String,
    pub payload: Payload,
}

fn version(text: &str) -> Result<[u16; 4], String> {
    let mut out = [0; 4];
    let parts: Vec<_> = text.split('.').collect();
    if (3..=4).contains(&parts.len()) {
        for (part, slot) in parts.iter().zip(&mut out) {
            if part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()) {
                return Err("판 번호에 숫자가 아닌 자리가 있어요".into());
            }
            *slot = part.parse().map_err(|_| "판 번호가 범위를 벗어났어요")?;
        }
        return Ok(out);
    }
    Err("판 번호는 세 자리 또는 네 자리여야 해요".into())
}

#[derive(Deserialize)]
struct Feed {
    version: String,
    #[serde(default, rename = "notes")]
    _notes: Vec<String>,
    url: String,
    sha256: String,
}

fn parse_feed(body: &[u8]) -> Result<Feed, String> {
    let feed: Feed = serde_json::from_slice(body).map_err(|e| format!("업데이트 목록 형식 오류: {e}"))?;
    version(&feed.version)?;
    let prefix = format!("{ASSETS}v{}/", feed.version);
    let file = feed.url.strip_prefix(&prefix).unwrap_or_default();
    if !file.ends_with(".msix") || !file.bytes().all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c)) {
        return Err("해당 릴리스의 MSIX 주소가 아니에요".into());
    }
    if feed.sha256.len() != 64 || !feed.sha256.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("SHA-256 형식이 잘못됐어요".into());
    }
    Ok(feed)
}

#[derive(Deserialize)]
struct Installer {
    #[serde(rename = "MainPackage")]
    main: MainPackage,
}

#[derive(Deserialize)]
struct MainPackage {
    #[serde(rename = "@Name")]
    name: String,
    #[serde(rename = "@Publisher")]
    publisher: String,
    #[serde(rename = "@Version")]
    version: String,
}

fn package_offer(body: &[u8], name: &str, publisher: &str, installed: [u16; 4]) -> Result<Option<String>, String> {
    let xml: Installer = quick_xml::de::from_reader(body).map_err(|e| format!("앱 설치 관리자 파일 형식 오류: {e}"))?;
    if xml.main.name != name || xml.main.publisher != publisher {
        return Err("설치된 앱과 패키지 이름 또는 게시자가 달라요".into());
    }
    Ok((version(&xml.main.version)? > installed).then_some(xml.main.version))
}

pub fn local_date() -> String {
    // 현재 사용자의 지역 날짜를 읽는다.
    let mut now = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut now) };
    format!("{:04}-{:02}-{:02}", now.wYear, now.wMonth, now.wDay)
}

fn system_tool(name: &str) -> Result<Command, String> {
    let mut buffer = [0u16; 32768];
    let len = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if len == 0 || len >= buffer.len() {
        return Err("Windows 시스템 폴더를 찾을 수 없어요".into());
    }
    let mut cmd = Command::new(PathBuf::from(String::from_utf16_lossy(&buffer[..len])).join(name));
    cmd.creation_flags(0x08000000); // 콘솔 창을 만들지 않는다.
    Ok(cmd)
}

fn run(cmd: &mut Command, action: &str) -> Result<Output, String> {
    let out = cmd.output().map_err(|e| format!("{action} 실패: {e}"))?;
    if !out.status.success() {
        return Err(format!("{action} 실패 ({})", out.status));
    }
    Ok(out)
}

fn download(url: &str, file: Option<&Path>) -> Result<Vec<u8>, String> {
    let mut curl = system_tool("curl.exe")?;
    curl.args(["-q", "--fail", "--silent", "--location", "--proto", "=https", "--proto-redir", "=https"]);
    match file {
        Some(path) => curl.args(["--max-time", "600", "--max-filesize", "536870912", "--output"]).arg(path),
        None => curl.args(["--max-time", "20", "--max-filesize", "131072"]),
    };
    Ok(run(curl.arg("--").arg(url), "업데이트 받기")?.stdout)
}

fn verify_msix(path: &Path, sha256: &str) -> Result<(), String> {
    // 경로·해시는 셸 코드에 삽입하지 않는다. Windows의 MSIX 서명 공급자가 신뢰 체인까지 검증한다.
    let mut ps = system_tool(r"WindowsPowerShell\v1.0\powershell.exe")?;
    ps.env("INK_UPDATE_FILE", path).env("INK_UPDATE_HASH", sha256).args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        r#"
            $ErrorActionPreference = 'Stop'
            $env:PSModulePath = [IO.Path]::Combine($PSHOME, 'Modules')
            if ((Get-FileHash -LiteralPath $env:INK_UPDATE_FILE -Algorithm SHA256).Hash -ne $env:INK_UPDATE_HASH) { exit 1 }
            if ((Get-AuthenticodeSignature -LiteralPath $env:INK_UPDATE_FILE).Status -ne 'Valid') { exit 2 }
        "#,
    ]);
    run(&mut ps, "업데이트 파일의 해시·서명 확인").map(|_| ())
}

pub fn check(current: &str, cache: &Path, enabled: impl Fn() -> bool) -> Result<Option<Ready>, String> {
    check_with(current, cache, enabled, download)
}

fn check_with(
    current: &str,
    cache: &Path,
    enabled: impl Fn() -> bool,
    fetch: impl Fn(&str, Option<&Path>) -> Result<Vec<u8>, String>,
) -> Result<Option<Ready>, String> {
    if !enabled() {
        return Ok(None);
    }
    match package::detect_channel() {
        InstallChannel::MsixStore => Err("Microsoft Store에서 업데이트를 관리해요".into()),
        InstallChannel::MsixDirect => package::with_winrt(|| {
            let uri = package::app_installer_uri().unwrap_or_else(|| APPINSTALLER_URI.into());
            if !uri.starts_with("https://") {
                return Err("앱 설치 관리자 주소는 HTTPS여야 해요".into());
            }
            let id = Package::Current().and_then(|p| p.Id()).map_err(|e| e.to_string())?;
            let installed = id.Version().map_err(|e| e.to_string())?;
            let body = fetch(&uri, None)?;
            let offered = package_offer(
                &body,
                &id.Name().map_err(|e| e.to_string())?.to_string(),
                &id.Publisher().map_err(|e| e.to_string())?.to_string(),
                [installed.Major, installed.Minor, installed.Build, installed.Revision],
            )?;
            Ok(offered.map(|version| Ready {
                version,
                payload: Payload::AppInstaller(uri),
            }))
        }),
        InstallChannel::Unpackaged => {
            let feed = parse_feed(&fetch(FEED, None)?)?;
            if version(&feed.version)? <= version(current)? || !enabled() {
                return Ok(None);
            }
            fs::create_dir_all(cache).map_err(|e| e.to_string())?;
            let tmp = Builder::new()
                .prefix("update-")
                .suffix(".msix")
                .tempfile_in(cache)
                .map_err(|e| e.to_string())?
                .into_temp_path();
            fetch(&feed.url, Some(&tmp))?;
            if !enabled() {
                return Ok(None);
            }
            verify_msix(&tmp, &feed.sha256)?;
            let path = cache.join(format!("Shookshok-{}.msix", feed.version));
            tmp.persist(&path).map_err(|e| e.to_string())?;
            Ok(Some(Ready {
                version: feed.version,
                payload: Payload::VerifiedFile(path),
            }))
        }
    }
}

/// 준비 단계와 구분하여 사용자가 누른 뒤에만 OS에 적용을 요청한다.
pub fn apply(uri: &str) -> Result<(), String> {
    package::with_winrt(|| {
        let manager = PackageManager::new().map_err(|e| e.to_string())?;
        let uri = Uri::CreateUri(&HSTRING::from(uri)).map_err(|e| e.to_string())?;
        package::register_application_restart("")?;
        let result = manager
            .AddPackageByAppInstallerFileAsync(&uri, AddPackageByAppInstallerOptions::ForceTargetAppShutdown, None)
            .and_then(|op| op.join())
            .and_then(|res| res.ExtendedErrorCode()?.ok());
        if result.is_err() {
            let _ = package::unregister_application_restart();
        }
        result.map_err(|e| format!("Windows 업데이트 적용 실패: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn feed() -> serde_json::Value {
        serde_json::json!({"version":"0.10.0", "notes":["새 기능"],
            "url":format!("{ASSETS}v0.10.0/shookshok_0.10.0.0_x64_direct.msix"), "sha256":"a".repeat(64)})
    }

    #[test]
    fn 판_번호는_숫자로_비교한다() {
        assert!(version("0.10.0").unwrap() > version("0.9.99").unwrap());
        assert_eq!(version("0.10.0"), version("0.10.0.0"));
        assert!(version("0.10.0.1").unwrap() > version("0.10.0").unwrap());
        for bad in ["1", "0..0", "0.1.beta", "1.2.65536", "1.2.3.4.5"] {
            assert!(version(bad).is_err());
        }
    }

    #[test]
    fn 릴리스_목록의_주소와_해시를_검사한다() {
        assert!(parse_feed(feed().to_string().as_bytes()).is_ok());
        for (field, bad) in [
            ("url", "file:///other.msix"),
            ("url", &format!("{ASSETS}v0.10.0/../x.msix")),
            ("sha256", "abcd"),
            ("version", "0.9.0"),
        ] {
            let mut value = feed();
            value[field] = bad.into();
            assert!(parse_feed(value.to_string().as_bytes()).is_err());
        }
    }

    #[test]
    fn 설치된_패키지와_이름과_게시자가_같은_새_판만_안내한다() {
        let xml = br#"<AppInstaller xmlns="http://schemas.microsoft.com/appx/appinstaller/2021" Version="9.0.0.0"><MainPackage Name="Test" Publisher="CN=Test" Version="1.2.0.0"/></AppInstaller>"#;
        let check = |name, publisher, installed| package_offer(xml, name, publisher, installed);
        assert_eq!(check("Test", "CN=Test", [1, 1, 9, 0]).unwrap().as_deref(), Some("1.2.0.0"));
        assert!(check("Test", "CN=Test", [1, 2, 0, 0]).unwrap().is_none());
        assert!(check("Test", "CN=Test", [2, 0, 0, 0]).unwrap().is_none());
        assert!(check("Other", "CN=Test", [0; 4]).is_err());
        assert!(check("Test", "CN=Other", [0; 4]).is_err());
    }

    #[test]
    fn 설정을_끄면_목록과_설치_파일_요청이_모두_영이다() {
        let calls = Cell::new(0);
        let enabled = Cell::new(false);
        let cache = tempfile::tempdir().unwrap();
        let check = || {
            check_with(
                "0.1.0",
                cache.path(),
                || enabled.get(),
                |_, file| {
                    calls.set(calls.get() + 1);
                    assert!(file.is_none());
                    enabled.set(false);
                    Ok(feed().to_string().into_bytes())
                },
            )
        };
        assert!(check().unwrap().is_none());
        assert_eq!(calls.get(), 0);
        enabled.set(true);
        assert!(check().unwrap().is_none());
        assert_eq!(calls.get(), 1);
        assert_eq!(fs::read_dir(cache.path()).unwrap().count(), 0);
    }

    #[test]
    fn 해시가_맞아도_서명이_없으면_거절한다() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unsigned.msix");
        fs::write(&path, b"abc").unwrap();
        let error = verify_msix(&path, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad").unwrap_err();
        assert!(error.contains("2"), "{error}");
        assert!(verify_msix(&path, &"0".repeat(64)).unwrap_err().contains("1"));
    }
}
