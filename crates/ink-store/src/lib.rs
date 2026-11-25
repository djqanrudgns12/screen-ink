//! 설정 파일 하나에 구역 여러 개: `{ "schemaVersion": 3, "sections": { "<이름>": { "revision": n, "data": {...} } } }`.
//!
//! - 쓰기: 임시 파일 → fsync → 이름 바꾸기(원자적). 반쯤 쓴 파일이 남지 않는다(05 §11-3).
//! - 백업: 열 때와 5분마다 `settings.json.bak` 교대. 새 파일이 이전 백업의 절반 이하로 줄면 백업을 덮지 않는다.
//! - 손상: 원본을 `settings.json.broken-<시각>`으로 보관 → 백업으로 복구 → 그것도 안 되면 첫 실행처럼.
//! - 위치: 실행 파일 옆 `screen-ink.portable`이 있으면 그 옆 `data\`, 아니면 사용자 설정 폴더(R-SET-1).

use ink_app::{Loaded, SettingsStore, StoreError, StoreLocation};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const FILE_NAME: &str = "settings.json";
pub const PORTABLE_MARKER: &str = "screen-ink.portable";
const BACKUP_EVERY: Duration = Duration::from_secs(5 * 60);

/// 설정 폴더 정하기. `exe_dir` = 실행 파일 폴더, `user_config` = `%APPDATA%\screen-ink` 같은 사용자 폴더.
pub fn resolve_dir(exe_dir: &Path, user_config: &Path) -> (PathBuf, bool) {
    if exe_dir.join(PORTABLE_MARKER).is_file() {
        (exe_dir.join("data"), true)
    } else {
        (user_config.to_path_buf(), false)
    }
}

pub struct JsonFileStore {
    dir: PathBuf,
    portable: bool,
    read_only: bool,
    file: Value,
    last_backup: Option<Instant>,
}

impl JsonFileStore {
    pub fn new(dir: impl Into<PathBuf>, portable: bool) -> Self {
        let dir = dir.into();
        let read_only = fs::create_dir_all(&dir).is_err() || !probe_writable(&dir);
        JsonFileStore { dir, portable, read_only, file: json!({ "schemaVersion": 3, "sections": {} }), last_backup: None }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    fn backup_path(&self) -> PathBuf {
        self.dir.join(format!("{FILE_NAME}.bak"))
    }

    fn read(path: &Path) -> Result<Option<Value>, String> {
        match fs::read(path) {
            Ok(bytes) => {
                let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes); // 메모장 BOM
                let v: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
                if v.get("schemaVersion").is_none() {
                    return Err("schemaVersion 없음".into());
                }
                Ok(Some(v))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// 백업 교대 — 새 파일이 이전 백업의 절반 이하로 줄었으면(뭔가 날아감) 덮지 않는다.
    fn rotate_backup(&mut self) {
        let main = self.path();
        let bak = self.backup_path();
        let (Ok(m), b) = (fs::metadata(&main), fs::metadata(&bak)) else { return };
        if let Ok(b) = b {
            if m.len() * 2 <= b.len() {
                log::warn!("settings backup kept: new {} bytes <= half of backup {} bytes", m.len(), b.len());
                return;
            }
        }
        if let Err(e) = fs::copy(&main, &bak) {
            log::warn!("settings backup failed: {e}");
        }
        self.last_backup = Some(Instant::now());
    }

    fn write_atomic(&self, value: &Value) -> std::io::Result<()> {
        let tmp = self.dir.join(format!("{FILE_NAME}.tmp"));
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(serde_json::to_string_pretty(value).expect("직렬화").as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, self.path())
    }
}

fn probe_writable(dir: &Path) -> bool {
    let p = dir.join(".write-test");
    let ok = fs::write(&p, b"ok").is_ok();
    let _ = fs::remove_file(&p);
    ok
}

fn now_stamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl SettingsStore for JsonFileStore {
    fn load(&mut self) -> Loaded {
        let main = self.path();
        match Self::read(&main) {
            Ok(None) => Loaded { file: None, notice: None },
            Ok(Some(v)) => {
                self.file = v.clone();
                if !self.read_only {
                    self.rotate_backup();
                }
                Loaded { file: Some(v), notice: None }
            }
            Err(err) => {
                log::warn!("settings unreadable: {err}");
                let broken = self.dir.join(format!("{FILE_NAME}.broken-{}", now_stamp()));
                let kept = fs::rename(&main, &broken).is_ok();
                match Self::read(&self.backup_path()) {
                    Ok(Some(v)) => {
                        self.file = v.clone();
                        let _ = self.write_atomic(&v);
                        Loaded { file: Some(v), notice: Some("설정 파일이 손상돼 백업으로 되살렸어요(원본은 보관했어요)".into()) }
                    }
                    _ => Loaded {
                        file: None,
                        notice: Some(if kept { "설정 파일이 손상돼 처음 값으로 시작했어요(원본은 보관했어요)".into() } else { "설정 파일이 손상돼 처음 값으로 시작했어요".into() }),
                    },
                }
            }
        }
    }

    fn save_section(&mut self, name: &str, data: &Value) -> Result<(), StoreError> {
        if self.read_only {
            return Err(StoreError::Io("read-only".into()));
        }
        let mut next = self.file.clone();
        next["schemaVersion"] = json!(3);
        if !next["sections"].is_object() {
            next["sections"] = json!({});
        }
        let revision = next["sections"][name]["revision"].as_u64().unwrap_or(0) + 1;
        next["sections"][name] = json!({ "revision": revision, "data": data });
        self.write_atomic(&next).map_err(|e| StoreError::Io(e.to_string()))?;
        self.file = next;
        if self.last_backup.map_or(true, |t| t.elapsed() >= BACKUP_EVERY) {
            self.rotate_backup();
        }
        Ok(())
    }

    fn location(&self) -> StoreLocation {
        StoreLocation { path: self.path().to_string_lossy().into_owned(), portable: self.portable, read_only: self.read_only }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("screen-ink-store-{name}-{}-{}", std::process::id(), now_stamp()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn 첫_실행은_파일_없음_저장하면_구역과_revision() {
        let d = temp_dir("first");
        let mut s = JsonFileStore::new(&d, false);
        assert!(s.load().file.is_none());
        s.save_section("settings", &json!({ "values": { "a": 1 } })).unwrap();
        s.save_section("settings", &json!({ "values": { "a": 2 } })).unwrap();
        let v: Value = serde_json::from_slice(&fs::read(s.path()).unwrap()).unwrap();
        assert_eq!(v["schemaVersion"], 3);
        assert_eq!(v["sections"]["settings"]["revision"], 2);
        assert_eq!(v["sections"]["settings"]["data"]["values"]["a"], 2);
        assert!(!d.join("settings.json.tmp").exists(), "임시 파일이 남지 않는다");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn 손상되면_보관하고_백업으로_되살린다() {
        let d = temp_dir("broken");
        let mut s = JsonFileStore::new(&d, false);
        s.save_section("settings", &json!({ "values": { "a": 1 } })).unwrap(); // 첫 저장 때 백업도 생김
        fs::write(s.path(), b"{ not json").unwrap();
        let mut s2 = JsonFileStore::new(&d, false);
        let loaded = s2.load();
        assert_eq!(loaded.file.unwrap()["sections"]["settings"]["data"]["values"]["a"], 1);
        assert!(loaded.notice.unwrap().contains("백업"));
        assert!(fs::read_dir(&d).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().starts_with("settings.json.broken-")));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn 백업도_없으면_처음_값으로() {
        let d = temp_dir("nobak");
        fs::write(d.join(FILE_NAME), b"garbage").unwrap();
        let mut s = JsonFileStore::new(&d, false);
        let loaded = s.load();
        assert!(loaded.file.is_none());
        assert!(loaded.notice.unwrap().contains("처음 값"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn 크게_줄어든_파일은_백업을_덮지_않는다() {
        let d = temp_dir("shrink");
        let mut s = JsonFileStore::new(&d, false);
        let big: Vec<i32> = (0..500).collect();
        s.save_section("settings", &json!({ "big": big })).unwrap();
        let bak_len = fs::metadata(s.backup_path()).unwrap().len();
        s.last_backup = None; // 5분이 지났다고 치고
        s.save_section("settings", &json!({})).unwrap();
        assert_eq!(fs::metadata(s.backup_path()).unwrap().len(), bak_len);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn 포터블_표시_파일이_있으면_실행_파일_옆_data() {
        let d = temp_dir("portable");
        let user = d.join("user");
        assert_eq!(resolve_dir(&d, &user), (user.clone(), false));
        fs::write(d.join(PORTABLE_MARKER), b"").unwrap();
        assert_eq!(resolve_dir(&d, &user), (d.join("data"), true));
        let _ = fs::remove_dir_all(&d);
    }
}
