//! 맥 자동 업데이트(R2-UPD) — **OS 도구만** 쓴다(`/usr/bin/curl`·`shasum`·`ditto`·`codesign`): 앱에 HTTP 라이브러리를
//! 들이지 않는다(1차 R-NET 정신 — 통신은 OS 도구가 시스템 인증서·프록시로). 서비스 잠금 밖(뒤 스레드)에서 부른다.
//!
//! 흐름: 목록(`latest-mac.json`, GitHub 릴리스) 받기 → 판이 더 새로우면 .zip 받기 → SHA-256 확인 → 풀기 →
//! **새 앱의 코드 서명이 지금 앱의 지정 요구 사항(DR = 식별자 + 같은 자체 서명 인증서)을 만족하는지** 확인 → 준비 끝.
//! DR이 같으면 macOS(TCC)가 같은 앱으로 보아 화면 기록·손쉬운 사용 권한이 이어진다. 인증서를 가진 사람만 그 DR로
//! 서명할 수 있으므로 이 확인이 곧 진짜 확인이다(따로 EdDSA 키가 필요 없다).
//! 설치: 지금 앱 꾸러미를 옆으로 옮기고 새 꾸러미를 그 자리에 → 잠깐 뒤 다시 여는 작은 셸을 띄우고 앱을 끝낸다.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// GitHub 릴리스의 최신 판 목록(공개 저장소 — 로그인 없이 받는다). `releases/latest`는 초안·미리보기 판을 뺀다.
pub const FEED_URL: &str = "https://github.com/djqanrudgns12/screen-ink/releases/latest/download/latest-mac.json";

/// 목록 한 장.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Feed {
    pub version: String,
    /// 바뀐 점(최대 3줄을 보인다)
    #[serde(default)]
    pub notes: Vec<String>,
    pub url: String,
    pub sha256: String,
}

/// 받아서 확인까지 끝난 새 판.
#[derive(Clone, Debug, PartialEq)]
pub struct Staged {
    pub version: String,
    pub notes: Vec<String>,
    pub app: PathBuf,
}

fn run(cmd: &mut Command) -> Result<String, String> {
    let out = cmd.output().map_err(|e| format!("{e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(err.lines().last().unwrap_or("failed").chars().take(200).collect());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `a`가 `b`보다 새 판인가(`0.10.0` > `0.9.3`, 앞의 `v`·뒤의 `-beta` 같은 꼬리는 숫자 부분만 본다).
pub fn is_newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.trim().trim_start_matches(['v', 'V']).split(['.', '-', '+']).map_while(|p| p.parse::<u64>().ok()).collect()
    };
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        let (x, y) = (pa.get(i).copied().unwrap_or(0), pb.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// 목록 글 → [`Feed`]. 주소는 https만(목록·파일 모두 — 위조 목록이 다른 곳을 가리키지 못하게 GitHub만).
pub fn parse_feed(json: &str) -> Result<Feed, String> {
    let f: Feed = serde_json::from_str(json).map_err(|e| format!("feed: {e}"))?;
    let ok_host = f.url.starts_with("https://github.com/djqanrudgns12/screen-ink/releases/download/") || f.url.starts_with("file://");
    if !ok_host {
        return Err("feed: unexpected download url".into());
    }
    if f.sha256.len() != 64 || !f.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("feed: bad sha256".into());
    }
    Ok(f)
}

/// 목록 받기(curl, 15초).
pub fn fetch_feed(url: &str) -> Result<Feed, String> {
    let body = run(Command::new("/usr/bin/curl").args(["-fsSL", "--max-time", "15", "--proto", "=https,file", url]))?;
    parse_feed(&body)
}

/// 지금 돌고 있는 앱 꾸러미(`…/Shookshok.app`). 개발 실행(꾸러미 밖)이면 None.
pub fn current_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let app = exe.parent()?.parent()?.parent()?.to_path_buf();
    (app.extension().is_some_and(|e| e == "app")).then_some(app)
}

/// 꾸러미의 지정 요구 사항(`codesign -dr -`의 `designated => …`).
pub fn designated_requirement(app: &Path) -> Result<String, String> {
    let out = Command::new("/usr/bin/codesign").args(["-dr", "-"]).arg(app).output().map_err(|e| e.to_string())?;
    // codesign은 이 정보를 stdout 또는 stderr에 쓴다
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    text.lines().find_map(|l| l.strip_prefix("designated => ")).map(|s| s.trim().to_string()).ok_or_else(|| "no designated requirement".into())
}

/// `candidate`가 서명이 온전하고 `requirement`(지금 앱의 DR)를 만족하는지.
pub fn verify_same_signer(candidate: &Path, requirement: &str) -> Result<(), String> {
    run(Command::new("/usr/bin/codesign").args(["--verify", "--deep", "--strict"]).arg(candidate))?;
    run(Command::new("/usr/bin/codesign").arg("--verify").arg(format!("-R={requirement}")).arg(candidate))?;
    Ok(())
}

fn sha256_of(file: &Path) -> Result<String, String> {
    let out = run(Command::new("/usr/bin/shasum").args(["-a", "256"]).arg(file))?;
    out.split_whitespace().next().map(str::to_ascii_lowercase).ok_or_else(|| "shasum: no output".into())
}

/// 새 판이 있으면 받아서 확인까지(`cache` 아래). 새 판이 없으면 Ok(None). 실패는 Err(앱 동작에는 영향 없음).
pub fn check_and_stage(feed_url: &str, current_version: &str, current_app: &Path, cache: &Path) -> Result<Option<Staged>, String> {
    let feed = fetch_feed(feed_url)?;
    if !is_newer(&feed.version, current_version) {
        return Ok(None);
    }
    stage(&feed, current_app, cache).map(Some)
}

/// 목록 하나를 받아 확인까지(시험이 file:// 목록으로 부른다).
pub fn stage(feed: &Feed, current_app: &Path, cache: &Path) -> Result<Staged, String> {
    let requirement = designated_requirement(current_app)?;
    let dir = cache.join(format!("update-{}", feed.version.replace(['/', '\\', '.'], "_")));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let zip = dir.join("update.zip");
    run(Command::new("/usr/bin/curl").args(["-fsSL", "--max-time", "600", "--proto", "=https,file", "-o"]).arg(&zip).arg(&feed.url))?;
    let got = sha256_of(&zip)?;
    if got != feed.sha256.to_ascii_lowercase() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err("sha256 mismatch".into());
    }
    let out = dir.join("unpacked");
    run(Command::new("/usr/bin/ditto").args(["-x", "-k"]).arg(&zip).arg(&out))?;
    let app = std::fs::read_dir(&out)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "app"))
        .ok_or("no .app in update")?;
    if let Err(e) = verify_same_signer(&app, &requirement) {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(format!("signature: {e}"));
    }
    log::info!("update staged {}", feed.version);
    Ok(Staged { version: feed.version.clone(), notes: feed.notes.iter().take(3).cloned().collect(), app })
}

/// 준비된 새 판으로 바꾸고 다시 연다. 성공하면 부른 쪽이 곧바로 앱을 끝내야 한다(셸이 1초 뒤 새 앱을 연다).
/// 쓸 수 없는 자리(다른 사용자 소유의 /Applications 등)면 Err — 아무것도 바꾸지 않는다.
pub fn install(staged: &Staged, current_app: &Path) -> Result<(), String> {
    let parent = current_app.parent().ok_or("no parent")?;
    let old = parent.join(format!(".{}.old-{}", current_app.file_name().and_then(|n| n.to_str()).unwrap_or("app"), std::process::id()));
    std::fs::rename(current_app, &old).map_err(|e| format!("move old: {e}"))?;
    if let Err(e) = run(Command::new("/usr/bin/ditto").arg(&staged.app).arg(current_app)) {
        // 되돌림
        let _ = std::fs::remove_dir_all(current_app);
        let _ = std::fs::rename(&old, current_app);
        return Err(format!("copy new: {e}"));
    }
    // 새 앱을 1초 뒤 열고, 옛 꾸러미는 휴지통이 아니라 지운다(같은 이름 두 개가 Launchpad에 남지 않게)
    let script = format!("sleep 1; /usr/bin/open {app}; /bin/rm -rf {old}", app = shell_quote(current_app), old = shell_quote(&old));
    Command::new("/bin/sh").args(["-c", &script]).spawn().map_err(|e| e.to_string())?;
    log::info!("update installed {} — relaunching", staged.version);
    Ok(())
}

fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 판_비교() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.3"));
        assert!(is_newer("v1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("0.2.1-beta", "0.2.0"));
        assert!(!is_newer("garbage", "0.1.0"));
    }

    #[test]
    fn 목록은_github_주소와_올바른_해시만() {
        let ok = r#"{"version":"0.2.0","notes":["a","b"],"url":"https://github.com/djqanrudgns12/screen-ink/releases/download/v0.2.0/Shookshok_0.2.0_aarch64.app.zip","sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}"#;
        let f = parse_feed(ok).expect("feed");
        assert_eq!((f.version.as_str(), f.notes.len()), ("0.2.0", 2));
        assert!(parse_feed(&ok.replace("github.com/djqanrudgns12", "evil.example.com/x")).is_err(), "다른 곳 주소는 거절");
        assert!(parse_feed(&ok.replace("0123456789abcdef0123", "xyz")).is_err(), "해시 형식");
        assert!(parse_feed("{}").is_err());
    }

    #[test]
    fn 따옴표가_든_경로도_셸에서_안전() {
        assert_eq!(shell_quote(Path::new("/A/it's.app")), r"'/A/it'\''s.app'");
    }

    /// 실제 확인(손으로): 설치된 앱을 zip으로 묶어 "새 판"으로 받게 하고 서명 확인까지 지나는지.
    /// `cargo test -p ink-platform-macos updater::tests::실제_설치판으로_받기_확인 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn 실제_설치판으로_받기_확인() {
        let app = Path::new("/Applications/Shookshok.app");
        let tmp = std::env::temp_dir().join("shookshok-update-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let zip = tmp.join("new.zip");
        run(Command::new("/usr/bin/ditto").args(["-c", "-k", "--keepParent"]).arg(app).arg(&zip)).unwrap();
        let sha = sha256_of(&zip).unwrap();
        let feed = Feed { version: "9.9.9".into(), notes: vec!["시험".into()], url: format!("file://{}", zip.display()), sha256: sha };
        let staged = stage(&feed, app, &tmp.join("cache")).expect("받기·확인");
        println!("staged {:?}", staged.app);
        assert!(staged.app.join("Contents/Info.plist").exists());
        // 해시가 틀리면 거절
        let bad = Feed { sha256: "0".repeat(64), ..feed };
        assert!(stage(&bad, app, &tmp.join("cache2")).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
