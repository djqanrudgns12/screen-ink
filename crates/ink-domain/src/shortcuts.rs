//! 단축키 — 조합 정규화·판정(analyze)·추천(02 §6).
//!
//! 규칙과 "이미 쓰이는 키" 목록은 시안 선택기(docs/shortcuts/shortcut-picker.html)와 같은
//! `docs/shortcuts/catalog-v1.json`에서 읽는다. 그래서 선택기에서 "막힘 0"이던 확정본은
//! 앱에서도 막힘 0이어야 한다(아래 시험이 지킨다). 기본값은 `defaults-v1.json`.

use crate::i18n::{tr, tr_os, trf, Lang};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

const CATALOG_JSON: &str = include_str!("../../../docs/shortcuts/catalog-v1.json");
const DEFAULTS_JSON: &str = include_str!("../../../docs/shortcuts/defaults-v1.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Win,
    Mac,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Platform::Mac
        } else {
            Platform::Win
        }
    }

    /// 수식 키 표기 순서(선택기 `MOD_ORDER`).
    pub fn mod_order(self) -> &'static [&'static str] {
        match self {
            Platform::Win => &["Ctrl", "Alt", "Shift", "Win"],
            Platform::Mac => &["Ctrl", "Option", "Shift", "Cmd"],
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Win => "win",
            Platform::Mac => "mac",
        }
    }

    fn idx(self) -> usize {
        match self {
            Platform::Win => 0,
            Platform::Mac => 1,
        }
    }
}

/// 판정 단계. 순서가 곧 심각도(Ok < Repl < Warn < Block).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Ok,
    /// 슉쇽이 대신할 앱과 겹침 — 그 앱을 계속 쓰지 않으면 괜찮다.
    Repl,
    Warn,
    Block,
}

const MODIFIERS: [&str; 6] = ["Ctrl", "Alt", "Shift", "Win", "Option", "Cmd"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
    pub mods: Vec<String>,
    pub key: String,
}

/// 표준 표기(`Ctrl+Shift+1`)를 수식 키와 키로 나눈다.
pub fn parse(combo: &str) -> Parsed {
    let parts: Vec<&str> = combo.split('+').map(str::trim).filter(|s| !s.is_empty()).collect();
    let mods = parts.iter().filter(|p| MODIFIERS.contains(p)).map(|s| s.to_string()).collect();
    let key = parts.iter().filter(|p| !MODIFIERS.contains(p)).copied().collect::<Vec<_>>().join("+");
    Parsed { mods, key }
}

/// 수식 키 순서를 플랫폼 표기 순서로 맞춘다. 그 플랫폼에 없는 수식 키는 버린다(선택기 `norm`).
pub fn normalize(combo: &str, p: Platform) -> String {
    let Parsed { mods, key } = parse(combo);
    let mut out: Vec<&str> = p.mod_order().iter().copied().filter(|m| mods.iter().any(|x| x == m)).collect();
    if !key.is_empty() {
        out.push(&key);
    }
    out.join("+")
}

/// 사람이 쓴 값·가져온 설정(`ctrl+shift+a`, `alt+escape`)을 표준 표기로. 모르는 키·키 두 개면 `None`, 빈 값은 빈 값.
pub fn canonicalize(input: &str, p: Platform) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return Some(String::new());
    }
    let mac = p == Platform::Mac;
    let mut mods: Vec<&'static str> = Vec::new();
    let mut key: Option<String> = None;
    for raw in input.split('+').map(str::trim).filter(|s| !s.is_empty()) {
        let m = match raw.to_lowercase().as_str() {
            "ctrl" | "control" | "ctl" | "⌃" => Some("Ctrl"),
            "alt" | "option" | "opt" | "⌥" => Some(if mac { "Option" } else { "Alt" }),
            "shift" | "⇧" => Some("Shift"),
            "win" | "windows" | "super" | "meta" | "cmd" | "command" | "⌘" => Some(if mac { "Cmd" } else { "Win" }),
            _ => None,
        };
        if let Some(m) = m {
            if !mods.contains(&m) {
                mods.push(m);
            }
            continue;
        }
        if key.is_some() {
            return None;
        }
        key = Some(canonical_key(raw)?);
    }
    let key = key?;
    Some(normalize(&format!("{}+{}", mods.join("+"), key), p))
}

fn canonical_key(raw: &str) -> Option<String> {
    let named = match raw.to_lowercase().as_str() {
        "esc" | "escape" => "Esc",
        "enter" | "return" => "Enter",
        "tab" => "Tab",
        "space" | "spacebar" => "Space",
        "backspace" => "Backspace",
        "delete" | "del" => "Delete",
        "insert" | "ins" => "Insert",
        "home" => "Home",
        "end" => "End",
        "pgup" | "pageup" => "PgUp",
        "pgdn" | "pagedown" => "PgDn",
        "up" | "arrowup" => "Up",
        "down" | "arrowdown" => "Down",
        "left" | "arrowleft" => "Left",
        "right" | "arrowright" => "Right",
        "prtscn" | "prtsc" | "printscreen" => "PrtScn",
        "pause" => "Pause",
        "scrlk" | "scrolllock" => "ScrLk",
        "caps lock" | "capslock" => "Caps Lock",
        "한/영" | "hangul" | "hangulmode" => "한/영",
        "한자" | "hanja" | "hanjamode" => "한자",
        "plus" => "Plus",
        "minus" => "Minus",
        "menu" | "contextmenu" => "Menu",
        "`" | "backquote" | "grave" => "`",
        "-" => "-",
        "=" => "=",
        "[" => "[",
        "]" => "]",
        "\\" => "\\",
        ";" => ";",
        "'" => "'",
        "," => ",",
        "." => ".",
        "/" => "/",
        _ => "",
    };
    if !named.is_empty() {
        return Some(named.to_string());
    }
    let up = raw.to_uppercase();
    let mut chars = up.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return c.is_ascii_alphanumeric().then(|| c.to_string());
    }
    if let Some(n) = up.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()) {
        return (1..=24).contains(&n).then(|| format!("F{n}"));
    }
    if let Some(n) = up.strip_prefix("NUM") {
        if n.len() == 1 && n.chars().all(|c| c.is_ascii_digit()) {
            return Some(format!("Num{n}"));
        }
    }
    None
}

/// `Win+1~9`, `F1~12` 같은 범위를 펼친다(선택기 `expand`).
pub fn expand(combo: &str) -> Vec<String> {
    let Some((left, right)) = combo.rsplit_once('~') else {
        return vec![combo.to_string()];
    };
    let Ok(end) = right.parse::<u32>() else {
        return vec![combo.to_string()];
    };
    let digits = left.chars().rev().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return vec![combo.to_string()];
    }
    let (prefix, start) = left.split_at(left.len() - digits);
    let Ok(start) = start.parse::<u32>() else {
        return vec![combo.to_string()];
    };
    (start..=end).map(|i| format!("{prefix}{i}")).collect()
}

pub fn is_fkey(key: &str) -> bool {
    key.strip_prefix('F').is_some_and(|n| !n.is_empty() && n.len() <= 2 && n.chars().all(|c| c.is_ascii_digit()))
}

/// 섬 문구용 읽기 표기: Windows `Shift F2`, 맥 `⇧F2`. 영어에서는 `한/영`·`한자`를 `Hangul`·`Hanja`로.
pub fn spoken_for(combo: &str, p: Platform, lang: Lang) -> String {
    let Parsed { mods, key } = parse(combo);
    let disp_key = match (lang, key.as_str()) {
        (Lang::En, "한/영") => "Hangul",
        (Lang::En, "한자") => "Hanja",
        _ => key.as_str(),
    };
    match p {
        Platform::Win => mods.iter().map(String::as_str).chain(std::iter::once(disp_key)).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" "),
        Platform::Mac => {
            let sym = |m: &str| match m {
                "Ctrl" => "⌃",
                "Option" => "⌥",
                "Shift" => "⇧",
                "Cmd" => "⌘",
                _ => "",
            };
            format!("{}{}", mods.iter().map(|m| sym(m)).collect::<String>(), disp_key)
        }
    }
}

pub fn spoken(combo: &str, p: Platform) -> String {
    spoken_for(combo, p, Lang::Ko)
}

/// `global-hotkey`(tauri-plugin-global-shortcut)가 받는 가속기 문자열. 등록할 수 없는 키(한/영 등)는 `None`.
pub fn to_accelerator(combo: &str, p: Platform) -> Option<String> {
    let Parsed { mods, key } = parse(&normalize(combo, p));
    if key.is_empty() {
        return None;
    }
    let code = match key.as_str() {
        k if k.len() == 1 && k.chars().all(|c| c.is_ascii_uppercase()) => format!("Key{k}"),
        k if k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()) => format!("Digit{k}"),
        k if is_fkey(k) => k.to_string(),
        k if k.starts_with("Num") && k.len() == 4 => format!("Numpad{}", &k[3..]),
        "Esc" => "Escape".into(),
        "Enter" | "Tab" | "Space" | "Backspace" | "Delete" | "Insert" | "Home" | "End" | "Pause" => key.clone(),
        "PgUp" => "PageUp".into(),
        "PgDn" => "PageDown".into(),
        "Up" => "ArrowUp".into(),
        "Down" => "ArrowDown".into(),
        "Left" => "ArrowLeft".into(),
        "Right" => "ArrowRight".into(),
        "PrtScn" => "PrintScreen".into(),
        "ScrLk" => "ScrollLock".into(),
        "`" => "Backquote".into(),
        "-" => "Minus".into(),
        "=" => "Equal".into(),
        "[" => "BracketLeft".into(),
        "]" => "BracketRight".into(),
        "\\" => "Backslash".into(),
        ";" => "Semicolon".into(),
        "'" => "Quote".into(),
        "," => "Comma".into(),
        "." => "Period".into(),
        "/" => "Slash".into(),
        "Plus" => "NumpadAdd".into(),
        "Minus" => "NumpadSubtract".into(),
        _ => return None,
    };
    let mut out: Vec<String> = mods
        .iter()
        .map(|m| match m.as_str() {
            "Ctrl" => "Control",
            "Alt" | "Option" => "Alt",
            "Shift" => "Shift",
            _ => "Super",
        })
        .map(String::from)
        .collect();
    out.push(code);
    Some(out.join("+"))
}

/// Windows 가상 키 코드 + 수식 키 → 표준 표기. PowerToys 설정 파일(`{win,ctrl,alt,shift,code}`)을 읽을 때 쓴다.
pub fn combo_from_vk(ctrl: bool, alt: bool, shift: bool, win: bool, vk: u32) -> Option<String> {
    let key: String = match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(vk)?.to_string(),
        0x60..=0x69 => format!("Num{}", vk - 0x60),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        0x08 => "Backspace".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x13 => "Pause".into(),
        0x14 => "Caps Lock".into(),
        0x15 => "한/영".into(),
        0x19 => "한자".into(),
        0x1B => "Esc".into(),
        0x20 => "Space".into(),
        0x21 => "PgUp".into(),
        0x22 => "PgDn".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x25 => "Left".into(),
        0x26 => "Up".into(),
        0x27 => "Right".into(),
        0x28 => "Down".into(),
        0x2C => "PrtScn".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x5D => "Menu".into(),
        0x6B => "Plus".into(),
        0x6D => "Minus".into(),
        0x91 => "ScrLk".into(),
        0xBA => ";".into(),
        0xBB => "=".into(),
        0xBC => ",".into(),
        0xBD => "-".into(),
        0xBE => ".".into(),
        0xBF => "/".into(),
        0xC0 => "`".into(),
        0xDB => "[".into(),
        0xDC => "\\".into(),
        0xDD => "]".into(),
        0xDE => "'".into(),
        _ => return None,
    };
    let mods = [(ctrl, "Ctrl"), (alt, "Alt"), (shift, "Shift"), (win, "Win")];
    let parts: Vec<&str> = mods.iter().filter(|(on, _)| *on).map(|(_, m)| *m).chain(std::iter::once(key.as_str())).collect();
    Some(parts.join("+"))
}

// ───────────────────────── 카탈로그 ─────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotDef {
    pub id: String,
    /// global | capture | draw | zoom
    pub scope: String,
    /// 설정 화면 묶음(capture·tools·quick·capture-mode·draw-mode·zoom-mode)
    pub group: String,
    pub label: String,
    pub required: bool,
    pub later: bool,
    pub windows_only: bool,
}

impl SlotDef {
    pub fn is_global(&self) -> bool {
        self.scope == "global"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SourceLevel {
    Block,
    Warn,
    Keep,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceDef {
    label: String,
    level: SourceLevel,
    keep_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rules {
    ime_keys: Vec<String>,
    special_alone: Vec<String>,
    mode_check_sources: Vec<String>,
    default_keep: BTreeMap<String, bool>,
}

#[derive(Debug, Deserialize)]
struct RefDef {
    src: String,
    platform: Platform,
    combo: String,
    what: String,
    level: Option<Level>,
}

#[derive(Debug, Deserialize)]
struct CatalogFile {
    rules: Rules,
    slots: Vec<SlotDef>,
    sources: BTreeMap<String, SourceDef>,
    refs: Vec<RefDef>,
}

#[derive(Debug, Deserialize)]
struct DefaultSlot {
    win: Option<String>,
    mac: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DefaultsFile {
    slots: BTreeMap<String, DefaultSlot>,
}

#[derive(Clone, Debug)]
struct RefHit {
    src: String,
    what: String,
    level: Option<Level>,
}

pub struct Catalog {
    rules: Rules,
    slots: Vec<SlotDef>,
    sources: BTreeMap<String, SourceDef>,
    index: [HashMap<String, Vec<RefHit>>; 2],
    defaults: BTreeMap<String, DefaultSlot>,
}

/// 빌드에 박힌 확정본 v1 카탈로그(한 번만 읽는다).
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| Catalog::from_json(CATALOG_JSON, DEFAULTS_JSON).expect("catalog-v1.json / defaults-v1.json 형식"))
}

/// 판정에 필요한 이 PC의 사실 — 어댑터가 채운다.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    /// 대체 앱을 계속 쓰는지(keepKey → bool). 실행 중이면 true.
    pub keep: BTreeMap<String, bool>,
    /// 설정 파일 등에서 직접 읽은, 지금 실제로 잡혀 있는 다른 앱의 전역 키.
    pub live: Vec<LiveBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveBinding {
    /// 표시 이름(예: "PowerToys 항상 위")
    pub owner: String,
    pub combo: String,
    pub what: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub level: Level,
    /// 출처 표시 이름: 규칙 · 이 표 · Windows · Snipaste …
    pub source: String,
    pub what: String,
    /// 키를 가진 다른 앱 이름(그 앱 때문일 때만)
    pub owner: Option<String>,
    /// 카탈로그 기본값으로 추정한 겹침 — 실제로 등록해 보면 확인된다(그 앱이 키를 바꿨을 수 있음).
    /// 규칙·이 표·운영체제 예약·설정 파일에서 읽은 키는 false(등록이 성공해도 막힘).
    #[serde(default)]
    pub verifiable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Analysis {
    pub level: Level,
    pub items: Vec<Issue>,
    pub empty: bool,
}

impl Analysis {
    /// 가장 심각한 첫 문제(Ok·Repl이면 없음).
    pub fn top(&self) -> Option<&Issue> {
        if self.level <= Level::Repl {
            return None;
        }
        self.items.iter().find(|i| i.level == self.level)
    }

    /// 등록해 보기 전에 이미 확정인 막힘(규칙·이 표·운영체제·설정 파일에서 읽은 키).
    pub fn hard_block(&self) -> Option<&Issue> {
        self.items.iter().find(|i| i.level == Level::Block && !i.verifiable)
    }

    /// 실제 등록이 성공해, 카탈로그로 추정한 다른 앱 겹침이 없다고 확인됐을 때의 판정.
    pub fn confirmed_free(&self) -> Analysis {
        let items: Vec<Issue> = self.items.iter().filter(|i| !i.verifiable).cloned().collect();
        let level = items.iter().map(|i| i.level).max().unwrap_or(Level::Ok);
        Analysis { level, items, empty: self.empty }
    }

    /// 줄 아래에 쓸 한 문장.
    pub fn reason(&self) -> Option<String> {
        self.reason_for(Lang::Ko)
    }

    pub fn reason_for(&self, lang: Lang) -> Option<String> {
        let top = self.top()?;
        Some(match (&top.owner, top.level) {
            (Some(owner), Level::Block) => trf(lang, "shortcut.reason.owner", &[("owner", owner)]),
            _ => top.what.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggestion {
    pub combo: String,
    pub level: Level,
}

pub fn scope_title(scope: &str, lang: Lang) -> String {
    let key = match scope {
        "global" => "shortcut.scope.global",
        "capture" => "shortcut.scope.capture",
        "draw" => "shortcut.scope.draw",
        "zoom" => "shortcut.scope.zoom",
        _ => "shortcut.scope.other",
    };
    tr(lang, key)
}

fn issue(level: Level, source: &str, what: impl Into<String>) -> Issue {
    Issue { level, source: source.to_string(), what: what.into(), owner: None, verifiable: false }
}

impl Catalog {
    fn from_json(catalog: &str, defaults: &str) -> Result<Self, serde_json::Error> {
        let file: CatalogFile = serde_json::from_str(catalog)?;
        let defaults: DefaultsFile = serde_json::from_str(defaults)?;
        let mut index: [HashMap<String, Vec<RefHit>>; 2] = [HashMap::new(), HashMap::new()];
        for r in &file.refs {
            for c in expand(&r.combo) {
                index[r.platform.idx()]
                    .entry(normalize(&c, r.platform))
                    .or_default()
                    .push(RefHit { src: r.src.clone(), what: r.what.clone(), level: r.level });
            }
        }
        Ok(Catalog { rules: file.rules, slots: file.slots, sources: file.sources, index, defaults: defaults.slots })
    }

    pub fn slots(&self) -> &[SlotDef] {
        &self.slots
    }

    /// 그 플랫폼에 있는 칸만(맥은 창 항상 위 칸 없음).
    pub fn slots_for(&self, p: Platform) -> impl Iterator<Item = &SlotDef> {
        self.slots.iter().filter(move |s| !(p == Platform::Mac && s.windows_only))
    }

    pub fn slot(&self, id: &str) -> Option<&SlotDef> {
        self.slots.iter().find(|s| s.id == id)
    }

    /// 대체 앱 기본 정책(선택기 `DEFAULT_KEEP`).
    pub fn default_environment(&self) -> Environment {
        Environment { keep: self.rules.default_keep.clone(), live: Vec::new() }
    }

    /// 확정본 v1 기본 키(없으면 빈 값).
    pub fn default_combo(&self, id: &str, p: Platform) -> String {
        self.defaults
            .get(id)
            .and_then(|d| match p {
                Platform::Win => d.win.clone(),
                Platform::Mac => d.mac.clone(),
            })
            .unwrap_or_default()
    }

    pub fn defaults(&self, p: Platform) -> BTreeMap<String, String> {
        self.slots_for(p).map(|s| (s.id.clone(), self.default_combo(&s.id, p))).collect()
    }

    fn source_level(&self, hit: &RefHit, env: &Environment) -> (Level, Option<String>) {
        let Some(src) = self.sources.get(&hit.src) else {
            return (hit.level.unwrap_or(Level::Warn), None);
        };
        match src.level {
            SourceLevel::Keep => {
                let kept = src.keep_key.as_ref().and_then(|k| env.keep.get(k)).copied().unwrap_or(false);
                (if kept { Level::Block } else { Level::Repl }, Some(src.label.clone()))
            }
            SourceLevel::Block => (hit.level.unwrap_or(Level::Block), None),
            SourceLevel::Warn => (hit.level.unwrap_or(Level::Warn), None),
        }
    }

    /// 한 칸에 `combo`를 넣었을 때의 문제를 모두 모은다(선택기 `analyze`와 같은 규칙).
    /// `assigned`는 같은 플랫폼의 다른 칸 값(칸 ID → 조합).
    pub fn analyze(&self, slot: &SlotDef, p: Platform, combo: &str, assigned: &BTreeMap<String, String>, env: &Environment) -> Analysis {
        self.analyze_for(slot, p, combo, assigned, env, Lang::Ko)
    }

    pub fn analyze_for(&self, slot: &SlotDef, p: Platform, combo: &str, assigned: &BTreeMap<String, String>, env: &Environment, lang: Lang) -> Analysis {
        let src_required = tr(lang, "shortcut.source.required");
        let src_rule = tr(lang, "shortcut.source.rule");
        let src_table = tr(lang, "shortcut.source.table");
        if combo.trim().is_empty() {
            let items = if slot.required { vec![issue(Level::Warn, &src_required, tr(lang, "shortcut.rule.required"))] } else { Vec::new() };
            return Analysis { level: if slot.required { Level::Warn } else { Level::Ok }, items, empty: true };
        }
        let combo = normalize(combo, p);
        let Parsed { mods, key } = parse(&combo);
        let global = slot.is_global();
        let mut items = Vec::new();

        if self.rules.ime_keys.contains(&key) {
            items.push(issue(Level::Block, &src_rule, tr(lang, "shortcut.rule.imeKey")));
        }
        if global {
            if mods.is_empty() && !is_fkey(&key) && !self.rules.special_alone.contains(&key) {
                items.push(issue(Level::Block, &src_rule, tr_os(lang, p, "shortcut.rule.plainKeyGlobal")));
            } else if mods.len() == 1 && mods[0] == "Shift" && !is_fkey(&key) {
                items.push(issue(Level::Block, &src_rule, tr(lang, "shortcut.rule.shiftPlainGlobal")));
            }
            if p == Platform::Mac && is_fkey(&key) {
                items.push(issue(Level::Warn, &src_rule, tr(lang, "shortcut.rule.macFkey")));
            }
        }

        if let Some(hits) = self.index[p.idx()].get(&combo) {
            for h in hits {
                let (lv, owner) = self.source_level(h, env);
                // 모드 안 키는 그 화면이 떠 있을 때만 쓰므로 다른 앱 기능과는 겹쳐도 된다 — 운영체제 예약만 본다.
                if !global && (!self.rules.mode_check_sources.contains(&h.src) || lv != Level::Block) {
                    continue;
                }
                let label = self.sources.get(&h.src).map(|s| s.label.as_str()).unwrap_or(h.src.as_str());
                let verifiable = owner.is_some();
                items.push(Issue { level: lv, source: label.to_string(), what: h.what.clone(), owner, verifiable });
            }
        }

        // 지금 실제로 잡혀 있는 다른 앱의 전역 키(저수준 훅 포함)는 모드 안에서도 먼저 가져간다.
        for b in &env.live {
            if normalize(&b.combo, p) == combo {
                items.push(Issue { level: Level::Block, source: b.owner.clone(), what: b.what.clone(), owner: Some(b.owner.clone()), verifiable: false });
            }
        }

        // 맥에서 ⌥(·⇧) + 글자·숫자·기호는 특수 문자 입력 — 목록에 없는 조합도 알려 준다.
        let mac_label = self.sources.get("mac").map(|s| s.label.as_str()).unwrap_or("macOS");
        if global
            && p == Platform::Mac
            && mods.iter().any(|m| m == "Option")
            && !mods.iter().any(|m| m == "Cmd" || m == "Ctrl")
            && key.chars().count() == 1
            && !items.iter().any(|i| i.source == mac_label)
        {
            items.push(issue(Level::Warn, &src_rule, tr(lang, "shortcut.rule.macOptionChar")));
        }

        for other in &self.slots {
            if other.id == slot.id {
                continue;
            }
            let Some(v) = assigned.get(&other.id) else { continue };
            if v.is_empty() || normalize(v, p) != combo {
                continue;
            }
            if other.scope == slot.scope {
                items.push(issue(Level::Block, &src_table, trf(lang, "shortcut.rule.sameScope", &[("name", &other.label)])));
            } else if global {
                items.push(issue(Level::Warn, &src_table, trf(lang, "shortcut.rule.globalShadowsMode", &[("name", &other.label)])));
            } else if other.is_global() {
                items.push(issue(Level::Block, &src_table, trf(lang, "shortcut.rule.shadowedByGlobal", &[("name", &other.label)])));
            }
        }

        let level = items.iter().map(|i| i.level).max().unwrap_or(Level::Ok);
        Analysis { level, items, empty: false }
    }

    /// 겹치지 않는 추천(선택기 `suggestions` 순서). `avoid`에 든 조합은 건너뛴다 —
    /// 여러 줄이 함께 실패했을 때 줄마다 첫 추천이 달라지게(02 §6.2-4). Ok가 Repl보다 앞.
    pub fn suggestions(
        &self,
        slot: &SlotDef,
        p: Platform,
        assigned: &BTreeMap<String, String>,
        env: &Environment,
        avoid: &BTreeSet<String>,
        limit: usize,
    ) -> Vec<Suggestion> {
        let global = slot.is_global();
        let modsets: Vec<Vec<&str>> = if global {
            match p {
                Platform::Win => vec![vec!["Alt"], vec!["Ctrl", "Shift"], vec!["Ctrl", "Alt"], vec!["Alt", "Shift"], vec![], vec!["Shift"], vec!["Ctrl"]],
                Platform::Mac => vec![vec!["Option"], vec!["Ctrl", "Option"], vec!["Shift", "Cmd"], vec!["Option", "Cmd"], vec!["Ctrl", "Cmd"], vec![]],
            }
        } else {
            vec![vec![], vec!["Shift"], vec![if p == Platform::Win { "Ctrl" } else { "Cmd" }]]
        };
        let keys: Vec<String> = if global {
            ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "F1", "F2", "F3", "F4", "F6", "F7", "F8", "F9", "Q", "A", "Z", "X", "S", "D"]
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            "QWERTYUIOPASDFGHJKLZXCVBNM1234567890".chars().map(|c| c.to_string()).collect()
        };
        let mut ok = Vec::new();
        let mut repl = Vec::new();
        let mut tried = BTreeSet::new();
        'outer: for ms in &modsets {
            for k in &keys {
                if global && ms.is_empty() && !is_fkey(k) {
                    continue;
                }
                let c = normalize(&format!("{}+{}", ms.join("+"), k), p);
                if !tried.insert(c.clone()) || avoid.contains(&c) {
                    continue;
                }
                match self.analyze(slot, p, &c, assigned, env).level {
                    Level::Ok => ok.push(Suggestion { combo: c, level: Level::Ok }),
                    Level::Repl => repl.push(Suggestion { combo: c, level: Level::Repl }),
                    _ => {}
                }
                if ok.len() >= limit {
                    break 'outer;
                }
            }
        }
        ok.extend(repl);
        ok.truncate(limit);
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_default() -> Environment {
        catalog().default_environment()
    }

    #[test]
    fn 카탈로그와_기본값이_57칸으로_맞는다() {
        let c = catalog();
        assert_eq!(c.slots().len(), 57);
        let scopes = |s: &str| c.slots().iter().filter(|x| x.scope == s).count();
        assert_eq!((scopes("global"), scopes("capture"), scopes("draw"), scopes("zoom")), (16, 11, 29, 1));
        assert_eq!(c.defaults(Platform::Win).len(), 57);
        assert_eq!(c.defaults(Platform::Mac).len(), 56, "맥은 창 항상 위 칸이 없다");
        assert_eq!(c.default_combo("region", Platform::Win), "F1");
        assert_eq!(c.default_combo("pause", Platform::Win), "Alt+`");
    }

    #[test]
    fn 확정본은_두_플랫폼_모두_막힘이_없다() {
        let c = catalog();
        for p in [Platform::Win, Platform::Mac] {
            let assigned = c.defaults(p);
            for s in c.slots_for(p) {
                let v = &assigned[&s.id];
                let a = c.analyze(s, p, v, &assigned, &env_default());
                assert!(a.level < Level::Block, "{p:?} {} = {v}: {:?}", s.id, a.items);
            }
        }
    }

    #[test]
    fn 정규화와_관대한_입력() {
        assert_eq!(normalize("Shift+Ctrl+1", Platform::Win), "Ctrl+Shift+1");
        assert_eq!(normalize("Cmd+Shift+S", Platform::Mac), "Shift+Cmd+S");
        assert_eq!(canonicalize("ctrl+shift+a", Platform::Win).as_deref(), Some("Ctrl+Shift+A"));
        assert_eq!(canonicalize(" alt + escape ", Platform::Win).as_deref(), Some("Alt+Esc"));
        assert_eq!(canonicalize("alt+1", Platform::Mac).as_deref(), Some("Option+1"));
        assert_eq!(canonicalize("f13", Platform::Win).as_deref(), Some("F13"));
        assert_eq!(canonicalize("Ctrl+Q+W", Platform::Win), None);
        assert_eq!(canonicalize("Ctrl+", Platform::Win), None);
        assert_eq!(canonicalize("", Platform::Win).as_deref(), Some(""));
    }

    #[test]
    fn 범위_펼치기() {
        assert_eq!(expand("Win+1~9").len(), 9);
        assert_eq!(expand("Win+1~9")[4], "Win+5");
        assert_eq!(expand("F1~12").last().map(String::as_str), Some("F12"));
        assert_eq!(expand("Alt+F4"), vec!["Alt+F4".to_string()]);
    }

    #[test]
    fn 글자_하나와_shift_글자는_전역으로_못_쓴다() {
        let c = catalog();
        let s = c.slot("active").unwrap();
        let assigned = c.defaults(Platform::Win);
        assert_eq!(c.analyze(s, Platform::Win, "Q", &assigned, &env_default()).level, Level::Block);
        assert_eq!(c.analyze(s, Platform::Win, "Shift+Q", &assigned, &env_default()).level, Level::Block);
        assert!(c.analyze(s, Platform::Win, "Shift+F9", &assigned, &env_default()).level < Level::Block);
    }

    #[test]
    fn 한영_키는_모드_안에서도_막힌다() {
        let c = catalog();
        let s = c.slots().iter().find(|s| s.scope == "draw").unwrap();
        let a = c.analyze(s, Platform::Win, "한/영", &c.defaults(Platform::Win), &env_default());
        assert_eq!(a.level, Level::Block);
    }

    #[test]
    fn 내부_겹침은_이름을_말해_준다() {
        let c = catalog();
        let s = c.slot("active").unwrap();
        let a = c.analyze(s, Platform::Win, "Alt+1", &c.defaults(Platform::Win), &env_default());
        assert_eq!(a.level, Level::Block);
        let top = a.top().unwrap();
        assert_eq!(top.source, "이 표");
        assert!(top.what.contains("판서"), "{}", top.what);
    }

    #[test]
    fn 모드_안_키가_전역_키와_같으면_막힌다() {
        let c = catalog();
        let s = c.slots().iter().find(|s| s.scope == "capture").unwrap();
        let a = c.analyze(s, Platform::Win, "Alt+1", &c.defaults(Platform::Win), &env_default());
        assert_eq!(a.level, Level::Block);
        assert!(a.top().unwrap().what.starts_with("어디서나"));
    }

    #[test]
    fn 다국어와_os별_단축키_판정_문구() {
        let c = catalog();
        let s = c.slot("active").unwrap();
        let win_plain = c.analyze_for(s, Platform::Win, "Q", &c.defaults(Platform::Win), &env_default(), Lang::En);
        assert_eq!(win_plain.top().unwrap().what, "Combine with Ctrl or Alt");
        let mac_plain = c.analyze_for(s, Platform::Mac, "Q", &c.defaults(Platform::Mac), &env_default(), Lang::En);
        assert_eq!(mac_plain.top().unwrap().what, "Combine with ⌘ or ⌥");
        assert_eq!(spoken_for("한/영", Platform::Win, Lang::En), "Hangul");
        assert_eq!(spoken_for("한자", Platform::Mac, Lang::En), "Hanja");
        assert_eq!(scope_title("draw", Lang::En), "While drawing");
    }

    #[test]
    fn snipaste를_계속_쓰면_f1_계열이_막히고_이유에_앱_이름() {
        let c = catalog();
        let mut env = env_default();
        env.keep.insert("snipaste".into(), true);
        let assigned = c.defaults(Platform::Win);
        let region = c.analyze(c.slot("region").unwrap(), Platform::Win, "F1", &assigned, &env);
        assert_eq!(region.level, Level::Block);
        assert_eq!(region.reason().as_deref(), Some("Snipaste에서 먼저 쓰고 있어요"));
        let window = c.analyze(c.slot("window").unwrap(), Platform::Win, "F2", &assigned, &env);
        assert!(window.level < Level::Block);
        // 끄면(대체) 막힘이 아니다
        let off = c.analyze(c.slot("region").unwrap(), Platform::Win, "F1", &assigned, &env_default());
        assert!(off.level < Level::Block);
    }

    #[test]
    fn 설정_파일에서_읽은_다른_앱_키는_막힘() {
        let c = catalog();
        let mut env = env_default();
        env.live.push(LiveBinding { owner: "PowerToys 항상 위".into(), combo: "Alt+Shift+D".into(), what: "창 항상 위 고정".into() });
        let a = c.analyze(c.slot("topmost").unwrap(), Platform::Win, "Alt+Shift+D", &c.defaults(Platform::Win), &env);
        assert_eq!(a.level, Level::Block);
        assert_eq!(a.reason().as_deref(), Some("PowerToys 항상 위에서 먼저 쓰고 있어요"));
    }

    #[test]
    fn 추천은_겹치지_않고_피할_목록을_존중한다() {
        let c = catalog();
        let s = c.slot("active").unwrap();
        let assigned = c.defaults(Platform::Win);
        let env = env_default();
        let first = c.suggestions(s, Platform::Win, &assigned, &env, &BTreeSet::new(), 3);
        assert_eq!(first.len(), 3);
        for x in &first {
            assert!(c.analyze(s, Platform::Win, &x.combo, &assigned, &env).level <= Level::Repl, "{}", x.combo);
            assert!(!assigned.values().any(|v| v == &x.combo));
        }
        let avoid: BTreeSet<String> = [first[0].combo.clone()].into();
        let second = c.suggestions(s, Platform::Win, &assigned, &env, &avoid, 3);
        assert_ne!(second[0].combo, first[0].combo);
    }

    #[test]
    fn 가속기_문자열과_가상_키() {
        assert_eq!(to_accelerator("Alt+`", Platform::Win).as_deref(), Some("Alt+Backquote"));
        assert_eq!(to_accelerator("Ctrl+Shift+F1", Platform::Win).as_deref(), Some("Control+Shift+F1"));
        assert_eq!(to_accelerator("Option+Shift+1", Platform::Mac).as_deref(), Some("Alt+Shift+Digit1"));
        assert_eq!(to_accelerator("Shift+Cmd+S", Platform::Mac).as_deref(), Some("Shift+Super+KeyS"));
        assert_eq!(to_accelerator("한/영", Platform::Win), None);
        assert_eq!(combo_from_vk(false, true, true, false, 68).as_deref(), Some("Alt+Shift+D"));
        assert_eq!(combo_from_vk(true, false, false, false, 0x70).as_deref(), Some("Ctrl+F1"));
        assert_eq!(spoken("Shift+F2", Platform::Win), "Shift F2");
        assert_eq!(spoken("Option+Shift+1", Platform::Mac), "⌥⇧1");
    }
}
