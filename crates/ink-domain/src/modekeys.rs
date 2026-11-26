//! 화면 안 키(R-KEY-6): 설정된 단축키(표준 표기, `Ctrl+S`·`Cmd+S`·`Esc`·`` ` ``)를 KeyboardEvent(자리 `code`·수식 키)와 견준다.
//!
//! 자리(`code`)로 판정하므로 한글 입력 중에도, 자판 배열이 달라도 같은 자리면 같은 키다.

use crate::selection::Key;
use crate::shortcuts::parse;

/// 고르기(F1) 모드 키 — 확정본 칸 `c-*`의 지금 값.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureKeys {
    pub detect: String,
    pub copy: String,
    pub save: String,
    pub quick: String,
    pub cancel: String,
    pub last: String,
    pub prev: String,
    pub next: String,
    pub full: String,
    pub cursor: String,
}

impl Default for CaptureKeys {
    /// Windows 기본값(확정본 v1). 맥은 설정에서 읽은 값이 들어온다.
    fn default() -> Self {
        Self {
            detect: "Tab".into(),
            copy: "Enter".into(),
            save: "Ctrl+S".into(),
            quick: "Ctrl+Shift+S".into(),
            cancel: "Esc".into(),
            last: "R".into(),
            prev: ",".into(),
            next: ".".into(),
            full: "Ctrl+A".into(),
            cursor: "`".into(),
        }
    }
}

impl CaptureKeys {
    /// 칸 id → 지금 값으로 만든다(빈 값 = 그 기능 키 없음).
    pub fn from_lookup(get: impl Fn(&str) -> String) -> Self {
        Self {
            detect: get("c-detect"),
            copy: get("c-copy"),
            save: get("c-save"),
            quick: get("c-quick"),
            cancel: get("c-cancel"),
            last: get("c-last"),
            prev: get("c-prev"),
            next: get("c-next"),
            full: get("c-full"),
            cursor: get("c-cursor"),
        }
    }
}

/// 표준 표기의 키 이름 → KeyboardEvent.code 후보.
fn codes(key: &str) -> Vec<String> {
    let fixed: &[&str] = match key {
        "Enter" => &["Enter", "NumpadEnter"],
        "Esc" => &["Escape"],
        "Tab" => &["Tab"],
        "Space" => &["Space"],
        "Backspace" => &["Backspace"],
        "Delete" => &["Delete"],
        "Insert" => &["Insert"],
        "Home" => &["Home"],
        "End" => &["End"],
        "PgUp" => &["PageUp"],
        "PgDn" => &["PageDown"],
        "Up" => &["ArrowUp"],
        "Down" => &["ArrowDown"],
        "Left" => &["ArrowLeft"],
        "Right" => &["ArrowRight"],
        "`" => &["Backquote"],
        "-" | "Minus" => &["Minus", "NumpadSubtract"],
        "=" | "Plus" => &["Equal", "NumpadAdd"],
        "[" => &["BracketLeft"],
        "]" => &["BracketRight"],
        "\\" => &["Backslash"],
        ";" => &["Semicolon"],
        "'" => &["Quote"],
        "," => &["Comma"],
        "." => &["Period"],
        "/" => &["Slash"],
        _ => &[],
    };
    if !fixed.is_empty() {
        return fixed.iter().map(|s| s.to_string()).collect();
    }
    let mut chars = key.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_alphabetic() {
            return vec![format!("Key{}", c.to_ascii_uppercase())];
        }
        if c.is_ascii_digit() {
            return vec![format!("Digit{c}"), format!("Numpad{c}")];
        }
    }
    if let Some(n) = key.strip_prefix("Num").filter(|n| n.len() == 1 && n.chars().all(|c| c.is_ascii_digit())) {
        return vec![format!("Numpad{n}")];
    }
    if crate::shortcuts::is_fkey(key) {
        return vec![key.to_string()];
    }
    Vec::new()
}

/// 설정된 조합 하나가 이 키 입력과 같은지. 수식 키는 정확히 같아야 한다(Ctrl+S ≠ Ctrl+Shift+S).
pub fn matches(combo: &str, k: &Key) -> bool {
    let combo = combo.trim();
    if combo.is_empty() {
        return false;
    }
    let p = parse(combo);
    let has = |m: &str| p.mods.iter().any(|x| x == m);
    let want_ctrl = has("Ctrl");
    let want_alt = has("Alt") || has("Option");
    let want_shift = has("Shift");
    let want_meta = has("Cmd") || has("Win");
    if (want_ctrl, want_alt, want_shift, want_meta) != (k.ctrl, k.alt, k.shift, k.meta) {
        return false;
    }
    let cands = codes(&p.key);
    if cands.iter().any(|c| c == &k.code) {
        return true;
    }
    // 자리 정보가 없는 입력(가상 키보드 등)은 글자로
    k.code.is_empty() && !k.key.is_empty() && (k.key.eq_ignore_ascii_case(&p.key) || (p.key == "Esc" && k.key == "Escape"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: &str, ctrl: bool, shift: bool, meta: bool) -> Key {
        Key { key: String::new(), code: code.into(), ctrl, shift, alt: false, meta }
    }

    #[test]
    fn 자리와_수식_키가_정확히_같아야_맞다() {
        assert!(matches("Ctrl+S", &key("KeyS", true, false, false)));
        assert!(!matches("Ctrl+S", &key("KeyS", true, true, false)), "Ctrl+Shift+S는 다른 키");
        assert!(matches("Ctrl+Shift+S", &key("KeyS", true, true, false)));
        assert!(matches("Cmd+S", &key("KeyS", false, false, true)), "맥 Cmd = meta");
        assert!(!matches("Cmd+S", &key("KeyS", true, false, false)));
        assert!(matches("Enter", &key("NumpadEnter", false, false, false)));
        assert!(matches("Esc", &key("Escape", false, false, false)));
        assert!(matches("`", &key("Backquote", false, false, false)));
        assert!(matches(",", &key("Comma", false, false, false)));
        assert!(matches("R", &key("KeyR", false, false, false)));
        assert!(!matches("R", &key("KeyR", true, false, false)));
        assert!(matches("Space", &key("Space", false, false, false)));
        assert!(matches("F5", &key("F5", false, false, false)));
        assert!(matches("1", &key("Numpad1", false, false, false)));
        assert!(!matches("", &key("Enter", false, false, false)), "빈 값 = 키 없음");
    }

    #[test]
    fn 자리_정보가_없으면_글자로() {
        let k = Key { key: "Escape".into(), code: String::new(), ..Default::default() };
        assert!(matches("Esc", &k));
    }

    #[test]
    fn 설정값으로_만든다() {
        let k = CaptureKeys::from_lookup(|id| if id == "c-copy" { "Space".into() } else { String::new() });
        assert_eq!(k.copy, "Space");
        assert_eq!(k.cancel, "");
        assert_eq!(CaptureKeys::default().save, "Ctrl+S");
    }
}
