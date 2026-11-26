//! 설정 v3 — 경로("draw.width")별 스키마·기본값·검증, 단축키 표, 읽기 관용·마이그레이션(05 §1).
//!
//! 설정 창은 한 칸씩 바로 반영하므로(`updateSetting(path, value)`) 값은 점 경로 → JSON 값의 평평한 표로 둔다.
//! 저장 파일 구역 `settings`의 `data` = `{ "values": {...}, "shortcuts": {...} }`.

use crate::i18n::{tr, trf, trf_os, Lang};
use crate::naming;
use crate::shortcuts::{canonicalize, catalog, Platform};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub const SCHEMA_VERSION: u32 = 3;
pub const MAX_EXCLUDED_APPS: usize = 64;

#[derive(Clone, Debug)]
enum Rule {
    Bool,
    Choice(&'static [&'static str]),
    IntRange(i64, i64),
    IntSet(&'static [i64]),
    NumSet(&'static [f64]),
    Folder,
    Template,
    ExcludedApps,
    /// 자유 글(장치 ID 등) — 제어 문자 없이 `max`자까지
    Text(usize),
}

struct Field {
    path: &'static str,
    rule: Rule,
    default: Value,
}

fn fields() -> &'static [Field] {
    static FIELDS: OnceLock<Vec<Field>> = OnceLock::new();
    FIELDS.get_or_init(|| {
        use Rule::*;
        let f = |path, rule, default| Field { path, rule, default };
        vec![
            f("capture.result", Choice(&["copy", "save", "both"]), json!("copy")),
            f("capture.cursor", Bool, json!(false)),
            f("capture.sound", Bool, json!(false)),
            f("capture.element", Bool, json!(true)),
            f("capture.magnet", Bool, json!(true)),
            f("capture.historyMax", IntRange(0, 50), json!(20)),
            f("capture.excluded", ExcludedApps, json!([])),
            // 숨은 설정(설정 화면에 없음): 고르기 중 사슬 마디마다 근거 색·점수(08 §4.5, 시험대 정답 만들기)
            f("capture.evidence", Bool, json!(false)),
            // 빈 값 = 사진 폴더 아래 Shookshok(어댑터가 정함)
            f("output.folder", Folder, json!("")),
            f("output.template", Template, json!("Shookshok_{yyyy}{MM}{dd}_{HH}{mm}{ss}")),
            f("record.fps", IntSet(&[30, 60]), json!(30)),
            f("record.mic", Bool, json!(false)),
            // 빈 값 = 기본 장치. 장치가 사라지면 마이크를 끄고 알린다(05 §1)
            f("record.micDevice", Text(256), json!("")),
            // 녹화에 커서를 넣을지(R-REC-3) · 첫 녹화 안내("판서는 영상에 함께")를 보였는지 · 지연 캡처 초(R-CAP-13)
            f("record.cursor", Bool, json!(true)),
            f("record.firstHintDone", Bool, json!(false)),
            f("capture.delay", IntSet(&[3, 5, 10]), json!(3)),
            f("draw.color", Choice(&["R", "Y", "G", "O", "B", "L", "W", "K"]), json!("R")),
            f("draw.width", IntRange(1, 30), json!(4)),
            // 캡처·그리기 표시 도구의 마지막 색·굵기·최근 색(R2-ANN-4) — 화면이 JSON 글로 쓰고 읽는다(숨은 값)
            f("annotate.style", Text(2000), json!("")),
            // 글자 도구의 마지막 글꼴 이름(R2-TXT-2, 숨은 값 — 이 컴퓨터 글꼴 이름은 밖으로 내보내지 않는다)
            f("annotate.font", Text(200), json!("")),
            f("draw.smoothing", Choice(&["off", "light", "strong"]), json!("light")),
            f("draw.straighten", Bool, json!(true)),
            f("draw.shapeOnHold", Bool, json!(true)),
            // 0.5초 멈춤 도형 보정: suggest = 물어보고 바꾸기(04 S12), auto = 바로 바꾸기(02 §3.3) — 사용자 결정 2026-11-26
            f("draw.shapeApply", Choice(&["suggest", "auto"]), json!("suggest")),
            f("draw.startZoom", NumSet(&[1.25, 1.5, 1.75, 2.0, 3.0, 4.0]), json!(1.75)),
            f("draw.keep", Bool, json!(true)),
            f("draw.esc", Choice(&["step", "exit"]), json!("step")),
            f("draw.rightClickPan", Bool, json!(true)),
            f("zoom.follow", Choice(&["pointer", "smooth", "instant"]), json!("pointer")),
            f("zoom.minimap", Bool, json!(true)),
            f("general.theme", Choice(&["system", "light", "dark"]), json!("system")),
            f("general.lang", Choice(&["ko", "en"]), json!("ko")),
            f("general.autostart", Bool, json!(true)),
            // 자동 업데이트(R2-UPD): 하루 한 번 + 켠 뒤 10분에 확인(끄면 통신 0), "오늘은 그만"을 누른 날(YYYY-MM-DD, 숨은 값)
            f("update.auto", Bool, json!(true)),
            f("update.snooze", Text(10), json!("")),
            f("topmost.border", IntSet(&[0, 2, 4, 6]), json!(4)),
            f("topmost.sound", Bool, json!(false)),
            f("onboarding.done", Bool, json!(false)),
            // 첫 실행 카드 몇 번째 장까지 왔나(R2-ONB-3 — 화면 기록 켠 뒤 다시 시작하면 그 장부터, 숨은 값)
            f("onboarding.step", IntSet(&[0, 1, 2, 3, 4]), json!(0)),
            // 맥 권한이 전에 있었는지(M10 — 업데이트 뒤 권한 풀림 감지, 숨은 값)
            f("mac.hadScreenPerm", Bool, json!(false)),
            f("mac.hadAxPerm", Bool, json!(false)),
        ]
    })
}

fn field(path: &str) -> Option<&'static Field> {
    fields().iter().find(|f| f.path == path)
}

/// 옛 이름 → 지금 이름(초안 v1·v2, 사람이 쓴 파일).
const RENAMED: [(&str, &str); 3] = [("detection.excluded", "capture.excluded"), ("output.name", "output.template"), ("general.startup", "general.autostart")];

/// 옛 기본값 → 지금 기본값(앱 이름을 슉쇽으로 바꾸기 전 파일). 설정 파일은 값을 모두 적어 두어서,
/// 이름을 바꾸기 전에 쓴 PC는 옛 기본 파일 이름 틀(`screen-ink_…`)을 그대로 읽었다(2026-11-26 맥에서 발견).
const OLD_DEFAULTS: [(&str, &str); 2] = [
    ("output.template", "screen-ink_{yyyy}{MM}{dd}_{HH}{mm}{ss}"),
    // 실시간 확대 따라가기 기본값이 마우스 위치 비례 지도로 바뀌었다(2026-11-27) — 옛 기본값을 저장해 둔 사람도 새 기본값으로
    ("zoom.follow", "smooth"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingError {
    UnknownKey(String),
    Invalid { path: String, reason: String },
    UnknownSlot(String),
    BadCombo(String),
}

impl SettingError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            Self::UnknownKey(path) => trf(lang, "error.settings.unknown", &[("path", path)]),
            Self::Invalid { reason, .. } => reason.clone(),
            Self::UnknownSlot(id) => trf(lang, "error.settings.unknownSlot", &[("id", id)]),
            Self::BadCombo(combo) => trf(lang, "error.settings.badCombo", &[("combo", combo)]),
        }
    }
}

impl std::fmt::Display for SettingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message(Lang::Ko))
    }
}

impl std::error::Error for SettingError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    NotSettings,
    NewerSchema(u32),
}

impl LoadError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            Self::NotSettings => tr(lang, "error.settings.notSettings"),
            Self::NewerSchema(v) => trf(lang, "error.settings.newerVersion", &[("version", v)]),
        }
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message(Lang::Ko))
    }
}

impl std::error::Error for LoadError {}

fn invalid(path: &str, reason: impl Into<String>) -> SettingError {
    SettingError::Invalid { path: path.to_string(), reason: reason.into() }
}

/// 한 칸 검증 — 통과하면 저장할 정규화 값.
pub fn validate(path: &str, value: &Value) -> Result<Value, SettingError> {
    validate_for(path, value, Platform::Win, Lang::Ko)
}

/// 플랫폼·언어별 한 칸 검증(예외 앱은 Windows `.exe` vs 맥 앱 이름, R2-FIX-5).
pub fn validate_for(path: &str, value: &Value, p: Platform, lang: Lang) -> Result<Value, SettingError> {
    let f = field(path).ok_or_else(|| SettingError::UnknownKey(path.to_string()))?;
    match &f.rule {
        Rule::Bool => value.as_bool().map(Value::Bool).ok_or_else(|| invalid(path, tr(lang, "error.settings.bool"))),
        Rule::Choice(list) => match value.as_str() {
            Some(s) if list.contains(&s) => Ok(json!(s)),
            _ => Err(invalid(path, trf(lang, "error.settings.choice", &[("options", &list.join("·"))]))),
        },
        Rule::IntRange(lo, hi) => match as_int(value) {
            Some(n) if (*lo..=*hi).contains(&n) => Ok(json!(n)),
            _ => Err(invalid(path, trf(lang, "error.settings.range", &[("lo", lo), ("hi", hi)]))),
        },
        Rule::IntSet(set) => match as_int(value) {
            Some(n) if set.contains(&n) => Ok(json!(n)),
            _ => {
                let options = set.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("·");
                Err(invalid(path, trf(lang, "error.settings.choice", &[("options", &options)])))
            }
        },
        Rule::NumSet(set) => match value.as_f64() {
            Some(x) if set.iter().any(|s| (s - x).abs() < 1e-9) => Ok(json!(x)),
            _ => {
                let options = set.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("·");
                Err(invalid(path, trf(lang, "error.settings.choice", &[("options", &options)])))
            }
        },
        Rule::Folder => match value.as_str() {
            Some(s) if s.chars().count() <= naming::MAX_PATH_CHARS && !s.chars().any(char::is_control) => Ok(json!(s.trim())),
            _ => Err(invalid(path, tr(lang, "error.settings.folder"))),
        },
        Rule::Template => {
            let s = value.as_str().ok_or_else(|| invalid(path, tr(lang, "error.settings.string")))?;
            naming::validate(s).map_err(|e| invalid(path, e.message(lang)))?;
            Ok(json!(s.trim()))
        }
        Rule::ExcludedApps => validate_excluded(path, value, p, lang),
        Rule::Text(max) => match value.as_str() {
            Some(s) if s.chars().count() <= *max && !s.chars().any(char::is_control) => Ok(json!(s)),
            _ => Err(invalid(path, trf(lang, "error.settings.maxChars", &[("max", max)]))),
        },
    }
}

/// 내보내기 파일(05 §1 R): 설정만, 기록·로그 제외.
pub fn export_file(s: &Settings) -> Value {
    json!({ "schemaVersion": SCHEMA_VERSION, "app": "shookshok", "settings": s.to_data() })
}

fn as_int(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_f64().filter(|x| x.fract() == 0.0).map(|x| x as i64))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcludedApp {
    /// 실행 파일 이름만(경로 X)
    pub exe: String,
    /// exclude = 찾지 않음 · windowOnly = 창 단위로만
    pub mode: String,
    /// 느려서 자동으로 넣은 항목
    #[serde(default)]
    pub auto: bool,
}

fn valid_excluded_name(name: &str, p: Platform) -> bool {
    let lower = name.to_ascii_lowercase();
    if name.is_empty() || name.chars().any(char::is_control) || name.contains(['\\', '/', ':']) {
        return false;
    }
    match p {
        Platform::Win => lower.ends_with(".exe") && lower != ".exe",
        Platform::Mac => name.chars().count() <= 128 && !lower.ends_with(".exe") && lower != ".app",
    }
}

fn validate_excluded(path: &str, value: &Value, p: Platform, lang: Lang) -> Result<Value, SettingError> {
    let list: Vec<ExcludedApp> = serde_json::from_value(value.clone()).map_err(|_| invalid(path, tr(lang, "error.settings.listFormat")))?;
    if list.len() > MAX_EXCLUDED_APPS {
        return Err(invalid(path, trf(lang, "error.settings.maxItems", &[("max", &MAX_EXCLUDED_APPS)])));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(list.len());
    for mut app in list {
        app.exe = app.exe.trim().to_string();
        if !valid_excluded_name(&app.exe, p) {
            return Err(invalid(path, trf_os(lang, p, "error.settings.invalidApp", &[("name", &app.exe)])));
        }
        if !matches!(app.mode.as_str(), "exclude" | "windowOnly") {
            return Err(invalid(path, tr(lang, "error.settings.invalidExcludedMode")));
        }
        let lower = app.exe.to_lowercase();
        if !seen.insert(lower) {
            return Err(invalid(path, trf(lang, "error.settings.duplicateApp", &[("name", &app.exe)])));
        }
        out.push(app);
    }
    Ok(serde_json::to_value(out).expect("직렬화"))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub values: BTreeMap<String, Value>,
    /// 칸 ID → 조합(빈 값 = 비움). 그 플랫폼 칸만 있다.
    pub shortcuts: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub path: String,
    pub before: Value,
    pub after: Value,
}

impl Settings {
    pub fn defaults(p: Platform) -> Self {
        Settings {
            values: fields().iter().map(|f| (f.path.to_string(), f.default.clone())).collect(),
            shortcuts: catalog().defaults(p),
        }
    }

    pub fn default_value(path: &str) -> Option<Value> {
        field(path).map(|f| f.default.clone())
    }

    pub fn paths() -> impl Iterator<Item = &'static str> {
        fields().iter().map(|f| f.path)
    }

    pub fn get(&self, path: &str) -> Option<&Value> {
        self.values.get(path)
    }

    pub fn bool(&self, path: &str) -> bool {
        self.get(path).and_then(Value::as_bool).unwrap_or(false)
    }

    pub fn str(&self, path: &str) -> &str {
        self.get(path).and_then(Value::as_str).unwrap_or("")
    }

    /// 검증 뒤 바꾸고 이전 값을 돌려준다(되돌리기 기록용).
    pub fn set(&mut self, path: &str, value: &Value) -> Result<Value, SettingError> {
        self.set_for(path, value, Platform::Win, Lang::Ko)
    }

    pub fn set_for(&mut self, path: &str, value: &Value, p: Platform, lang: Lang) -> Result<Value, SettingError> {
        let v = validate_for(path, value, p, lang)?;
        Ok(self.values.insert(path.to_string(), v).unwrap_or(Value::Null))
    }

    pub fn shortcut(&self, id: &str) -> &str {
        self.shortcuts.get(id).map(String::as_str).unwrap_or("")
    }

    /// 구조 검사만(칸이 있는지·읽을 수 있는 키인지). 겹침 판정은 `Catalog::analyze`에서.
    pub fn set_shortcut(&mut self, id: &str, combo: &str, p: Platform) -> Result<String, SettingError> {
        if !self.shortcuts.contains_key(id) {
            return Err(SettingError::UnknownSlot(id.to_string()));
        }
        let c = canonicalize(combo, p).ok_or_else(|| SettingError::BadCombo(combo.to_string()))?;
        Ok(self.shortcuts.insert(id.to_string(), c).unwrap_or_default())
    }

    pub fn to_data(&self) -> Value {
        json!({ "values": self.values, "shortcuts": self.shortcuts })
    }

    /// 저장 구역 `data`(또는 가져온 파일)를 읽는다. 모르는 키는 버리고, 틀린 값은 기본값으로 두고 경고를 모은다.
    /// 평평한 `values` 표와 중첩된 객체(`{"draw": {"width": 4}}`) 둘 다 받는다.
    pub fn from_data(data: &Value, p: Platform) -> (Settings, Vec<String>) {
        Self::from_data_for(data, p, Lang::Ko)
    }

    pub fn from_data_for(data: &Value, p: Platform, lang: Lang) -> (Settings, Vec<String>) {
        let mut s = Settings::defaults(p);
        let mut warnings = Vec::new();
        let Some(obj) = data.as_object() else {
            warnings.push(tr(lang, "error.settings.invalidFormat"));
            return (s, warnings);
        };
        let mut flat = Map::new();
        let values = obj.get("values").unwrap_or(data);
        flatten("", values, &mut flat);
        for (old, new) in RENAMED {
            if let Some(v) = flat.remove(old) {
                flat.entry(new.to_string()).or_insert(v);
            }
        }
        for (path, old) in OLD_DEFAULTS {
            if flat.get(path).and_then(Value::as_str) == Some(old) {
                flat.remove(path);
            }
        }
        for (path, v) in &flat {
            if path.starts_with("shortcuts.") || field(path).is_none() {
                continue;
            }
            if let Err(e) = s.set_for(path, v, p, lang) {
                let err = e.message(lang);
                warnings.push(trf(lang, "error.settings.fieldFallback", &[("path", path), ("err", &err)]));
            }
        }
        let shortcuts = obj.get("shortcuts").and_then(Value::as_object);
        if let Some(map) = shortcuts {
            for (id, v) in map {
                let Some(c) = v.as_str() else { continue };
                if !s.shortcuts.contains_key(id) {
                    continue;
                }
                if s.set_shortcut(id, c, p).is_err() {
                    warnings.push(trf(lang, "error.settings.shortcutFallback", &[("id", id)]));
                }
            }
        }
        (s, warnings)
    }

    /// 가져오기 확인 시트용 — 바뀌는 항목 목록("12개 항목이 바뀌어요").
    pub fn diff(&self, other: &Settings) -> Vec<Change> {
        let mut out = Vec::new();
        for (path, before) in &self.values {
            let after = other.values.get(path).cloned().unwrap_or(Value::Null);
            if *before != after {
                out.push(Change { path: path.clone(), before: before.clone(), after });
            }
        }
        for (id, before) in &self.shortcuts {
            let after = other.shortcut(id);
            if before != after {
                out.push(Change { path: format!("shortcuts.{id}"), before: json!(before), after: json!(after) });
            }
        }
        out
    }
}

/// 객체를 점 경로로 편다. 배열·값은 그대로 잎.
fn flatten(prefix: &str, v: &Value, out: &mut Map<String, Value>) {
    match v.as_object() {
        Some(obj) if prefix.is_empty() || field(prefix).is_none() => {
            for (k, child) in obj {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                flatten(&key, child, out);
            }
        }
        _ => {
            out.insert(prefix.to_string(), v.clone());
        }
    }
}

/// 파일 전체 `{schemaVersion, sections:{settings:{revision,data}}}` 또는 내보낸 파일 `{schemaVersion, settings:{...}}`에서
/// 설정 `data`를 꺼내 v3로 맞춘다. v1·v2(초안)는 평평하게 펴서 이름 바꾸기만 하면 된다.
pub fn migrate(file: &Value) -> Result<Value, LoadError> {
    let obj = file.as_object().ok_or(LoadError::NotSettings)?;
    let version = obj.get("schemaVersion").and_then(Value::as_u64).ok_or(LoadError::NotSettings)? as u32;
    if version > SCHEMA_VERSION {
        return Err(LoadError::NewerSchema(version));
    }
    let data = obj
        .get("sections")
        .and_then(|s| s.get("settings"))
        .and_then(|s| s.get("data"))
        .or_else(|| obj.get("settings"))
        .ok_or(LoadError::NotSettings)?;
    Ok(data.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 기본값은_모두_자기_검증을_통과한다() {
        for f in fields() {
            assert_eq!(validate(f.path, &f.default).as_ref(), Ok(&f.default), "{}", f.path);
        }
        let s = Settings::defaults(Platform::Win);
        assert_eq!(s.shortcuts.len(), 57);
        assert_eq!(Settings::defaults(Platform::Mac).shortcuts.len(), 56);
    }

    #[test]
    fn 도형_보정은_기본_물어보기_바로_바꾸기만_더() {
        let mut s = Settings::defaults(Platform::Win);
        assert_eq!(s.str("draw.shapeApply"), "suggest");
        assert!(s.set("draw.shapeApply", &json!("maybe")).is_err());
        assert_eq!(s.set("draw.shapeApply", &json!("auto")), Ok(json!("suggest")));
        assert_eq!(s.str("draw.shapeApply"), "auto");
    }

    #[test]
    fn 범위_밖은_거부하고_이전_값을_돌려준다() {
        let mut s = Settings::defaults(Platform::Win);
        assert!(s.set("draw.width", &json!(31)).is_err());
        assert!(s.set("draw.width", &json!(0)).is_err());
        assert_eq!(s.set("draw.width", &json!(12)), Ok(json!(4)));
        assert_eq!(s.set("draw.startZoom", &json!(2)), Ok(json!(1.75)));
        assert!(s.set("record.fps", &json!(45)).is_err());
        assert!(s.set("general.theme", &json!("pink")).is_err());
        assert_eq!(s.set("nope.key", &json!(1)), Err(SettingError::UnknownKey("nope.key".into())));
        assert!(s.set("output.template", &json!("a/{yyyy}")).is_err());
    }

    #[test]
    fn 찾지_않을_앱_목록_검사() {
        let ok = json!([{ "exe": " hwp.exe ", "mode": "windowOnly", "auto": true }]);
        assert_eq!(validate("capture.excluded", &ok).unwrap()[0]["exe"], "hwp.exe");
        assert!(validate("capture.excluded", &json!([{ "exe": "C:\\a\\hwp.exe", "mode": "exclude" }])).is_err());
        assert!(validate("capture.excluded", &json!([{ "exe": "a.exe", "mode": "exclude" }, { "exe": "A.EXE", "mode": "exclude" }])).is_err());
        let many: Vec<Value> = (0..65).map(|i| json!({ "exe": format!("a{i}.exe"), "mode": "exclude" })).collect();
        assert!(validate("capture.excluded", &Value::Array(many)).is_err());
    }

    #[test]
    fn 읽기는_관대하게_틀린_값은_기본값과_경고() {
        let data = json!({
            "values": { "draw.width": 99, "draw.color": "G", "mystery": true },
            "shortcuts": { "active": "ctrl+alt+f2", "region": "Ctrl+Q+W", "ghost": "F9" }
        });
        let (s, warnings) = Settings::from_data(&data, Platform::Win);
        assert_eq!(s.get("draw.width"), Some(&json!(4)));
        assert_eq!(s.get("draw.color"), Some(&json!("G")));
        assert_eq!(s.shortcut("active"), "Ctrl+Alt+F2");
        assert_eq!(s.shortcut("region"), "F1");
        assert_eq!(warnings.len(), 2, "{warnings:?}");
    }

    #[test]
    fn 중첩된_옛_파일도_읽는다() {
        let file = json!({ "schemaVersion": 2, "settings": { "draw": { "width": 6 }, "detection": { "excluded": [{ "exe": "hwp.exe", "mode": "exclude" }] } } });
        let data = migrate(&file).unwrap();
        let (s, w) = Settings::from_data(&data, Platform::Win);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(s.get("draw.width"), Some(&json!(6)));
        assert_eq!(s.get("capture.excluded").unwrap()[0]["exe"], "hwp.exe");
    }

    #[test]
    fn 옛_기본_파일_이름_틀은_새_기본값으로() {
        let (s, w) = Settings::from_data(&json!({ "values": { "output.template": "screen-ink_{yyyy}{MM}{dd}_{HH}{mm}{ss}" } }), Platform::Mac);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(s.str("output.template"), "Shookshok_{yyyy}{MM}{dd}_{HH}{mm}{ss}");
        // 사용자가 고친 틀은 그대로
        let (s, _) = Settings::from_data(&json!({ "values": { "output.template": "screen-ink_{HH}{mm}" } }), Platform::Mac);
        assert_eq!(s.str("output.template"), "screen-ink_{HH}{mm}");
    }

    #[test]
    fn 저장_구역_왕복과_새_스키마_거부() {
        let mut s = Settings::defaults(Platform::Win);
        s.set("zoom.minimap", &json!(false)).unwrap();
        s.set_shortcut("active", "Ctrl+Alt+F2", Platform::Win).unwrap();
        let file = json!({ "schemaVersion": 3, "sections": { "settings": { "revision": 7, "data": s.to_data() } } });
        let (back, w) = Settings::from_data(&migrate(&file).unwrap(), Platform::Win);
        assert!(w.is_empty());
        assert_eq!(back, s);
        assert_eq!(migrate(&json!({ "schemaVersion": 4, "sections": {} })), Err(LoadError::NewerSchema(4)));
        assert_eq!(migrate(&json!({ "hello": 1 })), Err(LoadError::NotSettings));
    }

    #[test]
    fn 내보낸_파일은_그대로_다시_읽힌다() {
        let mut s = Settings::defaults(Platform::Win);
        s.set("record.micDevice", &json!("{0.0.1.00000000}.{abc}")).unwrap();
        s.set_shortcut("pin", "", Platform::Win).unwrap();
        let (back, w) = Settings::from_data(&migrate(&export_file(&s)).unwrap(), Platform::Win);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(back, s);
        assert!(validate("record.micDevice", &json!("a\u{0007}")).is_err());
    }

    #[test]
    fn 바뀌는_항목_세기() {
        let a = Settings::defaults(Platform::Win);
        let mut b = a.clone();
        b.set("draw.width", &json!(8)).unwrap();
        b.set_shortcut("pin", "", Platform::Win).unwrap();
        let d = a.diff(&b);
        assert_eq!(d.len(), 2);
        assert_eq!(d[1].path, "shortcuts.pin");
    }

    #[test]
    fn 예외_앱_검증은_os별_규칙을_따르고_영어로도_말한다() {
        // R2-FIX-5: 맥은 앱/번들 이름 허용(.exe 거부), Windows는 .exe 필수
        let mac_ok = json!([{ "exe": " Safari ", "mode": "windowOnly" }, { "exe": "com.apple.Keynote", "mode": "exclude" }]);
        assert_eq!(validate_for("capture.excluded", &mac_ok, Platform::Mac, Lang::Ko).unwrap()[0]["exe"], "Safari");
        let mac_bad = validate_for("capture.excluded", &json!([{ "exe": "hwp.exe", "mode": "exclude" }]), Platform::Mac, Lang::Ko).unwrap_err();
        assert!(mac_bad.message(Lang::Ko).contains("Safari"), "{mac_bad}");
        let mac_bad_en = validate_for("capture.excluded", &json!([{ "exe": "hwp.exe", "mode": "exclude" }]), Platform::Mac, Lang::En).unwrap_err();
        assert!(mac_bad_en.message(Lang::En).contains("e.g. Safari"), "{mac_bad_en}");

        let win_bad = validate_for("capture.excluded", &json!([{ "exe": "Safari", "mode": "exclude" }]), Platform::Win, Lang::En).unwrap_err();
        assert!(win_bad.message(Lang::En).contains("e.g. hwp.exe"), "{win_bad}");
        assert!(validate_for("capture.excluded", &json!([{ "exe": ".exe", "mode": "exclude" }]), Platform::Win, Lang::Ko).is_err());

        // 영어 오류·경고
        assert_eq!(validate_for("draw.width", &json!(99), Platform::Win, Lang::En).unwrap_err().message(Lang::En), "Choose a value between 1 and 30");
        let (_, w_en) = Settings::from_data_for(&json!("bad"), Platform::Win, Lang::En);
        assert_eq!(w_en, vec!["Invalid settings · started with defaults"]);
        assert_eq!(LoadError::NotSettings.message(Lang::En), "Not a ShookShok settings file");
        assert_eq!(LoadError::NewerSchema(4).message(Lang::En), "Read-only: from newer v4");
    }
}
