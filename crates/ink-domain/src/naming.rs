//! 파일 이름 규칙(R-OUT-1) — `{yyyy}{MM}{dd}{HH}{mm}{ss}{앱}{n}` 토큰만, 금지 글자 경고,
//! 같은 이름이면 ` (2)`, 경로가 260자를 넘으면 이름 앞부분을 줄인다(05 §11-5·6).

use crate::i18n::{tr, trf, Lang};
use serde::{Deserialize, Serialize};

pub const TOKENS: [&str; 8] = ["yyyy", "MM", "dd", "HH", "mm", "ss", "앱", "n"];
/// 템플릿 결과(확장자 빼고) 최대 글자 수(05 §1).
pub const MAX_STEM_CHARS: usize = 200;
/// Windows 기본 경로 한도.
pub const MAX_PATH_CHARS: usize = 260;
const FORBIDDEN: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NamingError {
    Empty,
    UnknownToken(String),
    Unclosed,
    ForbiddenChar(char),
    TooLong,
}

impl NamingError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            Self::Empty => tr(lang, "naming.error.empty"),
            Self::UnknownToken(tok) => trf(lang, "naming.error.unknownToken", &[("token", tok)]),
            Self::Unclosed => tr(lang, "naming.error.unclosed"),
            Self::ForbiddenChar(ch) => trf(lang, "naming.error.forbiddenChar", &[("ch", ch)]),
            Self::TooLong => tr(lang, "naming.error.tooLong"),
        }
    }
}

impl std::fmt::Display for NamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message(Lang::Ko))
    }
}

impl std::error::Error for NamingError {}

/// 렌더에 쓰는 값. `app`은 실행 파일 이름 줄기(예: "chrome")만 — 창 제목은 쓰지 않는다.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameContext {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub app: Option<String>,
    /// 오늘의 순번(1부터)
    pub n: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    Lit(String),
    Tok(&'static str),
}

fn parse(template: &str) -> Result<Vec<Part>, NamingError> {
    let mut parts = Vec::new();
    let mut lit = String::new();
    let mut chars = template.chars();
    while let Some(c) = chars.next() {
        if c != '{' {
            if FORBIDDEN.contains(&c) || c.is_control() {
                return Err(NamingError::ForbiddenChar(c));
            }
            lit.push(c);
            continue;
        }
        let mut name = String::new();
        let mut closed = false;
        for c in chars.by_ref() {
            if c == '}' {
                closed = true;
                break;
            }
            name.push(c);
        }
        if !closed {
            return Err(NamingError::Unclosed);
        }
        let tok = TOKENS.iter().find(|t| **t == name || (name.eq_ignore_ascii_case("app") && **t == "앱")).ok_or(NamingError::UnknownToken(name))?;
        if !lit.is_empty() {
            parts.push(Part::Lit(std::mem::take(&mut lit)));
        }
        parts.push(Part::Tok(tok));
    }
    if !lit.is_empty() {
        parts.push(Part::Lit(lit));
    }
    if parts.is_empty() {
        return Err(NamingError::Empty);
    }
    Ok(parts)
}

/// 설정에 넣기 전 검사.
pub fn validate(template: &str) -> Result<(), NamingError> {
    let parts = parse(template.trim())?;
    let sample = render_parts(&parts, &NameContext { year: 2026, month: 12, day: 31, hour: 23, minute: 59, second: 59, app: None, n: 999 });
    if sample.chars().count() > MAX_STEM_CHARS {
        return Err(NamingError::TooLong);
    }
    Ok(())
}

fn render_parts(parts: &[Part], ctx: &NameContext) -> String {
    let mut out = String::new();
    for p in parts {
        match p {
            Part::Lit(s) => out.push_str(s),
            Part::Tok("yyyy") => out.push_str(&format!("{:04}", ctx.year)),
            Part::Tok("MM") => out.push_str(&format!("{:02}", ctx.month)),
            Part::Tok("dd") => out.push_str(&format!("{:02}", ctx.day)),
            Part::Tok("HH") => out.push_str(&format!("{:02}", ctx.hour)),
            Part::Tok("mm") => out.push_str(&format!("{:02}", ctx.minute)),
            Part::Tok("ss") => out.push_str(&format!("{:02}", ctx.second)),
            Part::Tok("앱") => out.push_str(&sanitize(ctx.app.as_deref().unwrap_or("screen"))),
            Part::Tok("n") => out.push_str(&ctx.n.max(1).to_string()),
            Part::Tok(_) => {}
        }
    }
    out
}

/// 템플릿 → 파일 이름 줄기(확장자 없음). 결과도 한 번 더 정리한다(앱 이름 등 바깥 값).
pub fn render(template: &str, ctx: &NameContext) -> Result<String, NamingError> {
    let parts = parse(template.trim())?;
    let s = sanitize(&render_parts(&parts, ctx));
    Ok(if s.is_empty() { "Shookshok".into() } else { s })
}

/// 금지 글자·제어 문자를 `_`로, 끝의 점·공백 제거, 예약 이름(CON·NUL·COM1…)은 앞에 `_`.
pub fn sanitize(s: &str) -> String {
    let mut out: String = s.chars().map(|c| if FORBIDDEN.contains(&c) || c.is_control() { '_' } else { c }).collect();
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    let upper = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((upper.starts_with("COM") || upper.starts_with("LPT")) && upper.len() == 4 && upper[3..].chars().all(|c| ('1'..='9').contains(&c)));
    if reserved {
        out.insert(0, '_');
    }
    out
}

/// 폴더 경로 길이까지 합쳐 260자를 넘으면 줄기 **앞부분**을 줄인다(뒤쪽 날짜·순번을 살림).
pub fn fit_path(dir: &str, stem: &str, ext: &str) -> String {
    // dir + 구분자 + stem + " (99)" 여유 + "." + ext
    let budget = MAX_PATH_CHARS.saturating_sub(dir.chars().count() + 1 + 5 + 1 + ext.chars().count());
    let n = stem.chars().count();
    if n <= budget {
        return stem.to_string();
    }
    let keep = budget.max(8);
    stem.chars().skip(n.saturating_sub(keep)).collect()
}

/// 같은 이름이 있으면 ` (2)`, ` (3)` … 를 붙인다. `exists`는 "확장자 포함 파일 이름"을 받는다.
pub fn unique_file_name(stem: &str, ext: &str, exists: impl Fn(&str) -> bool) -> String {
    let first = format!("{stem}.{ext}");
    if !exists(&first) {
        return first;
    }
    (2..10_000).map(|i| format!("{stem} ({i}).{ext}")).find(|n| !exists(n)).unwrap_or_else(|| format!("{stem} (x).{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> NameContext {
        NameContext { year: 2026, month: 3, day: 4, hour: 9, minute: 5, second: 7, app: Some("chrome".into()), n: 3 }
    }

    #[test]
    fn 토큰_치환() {
        assert_eq!(render("수업_{yyyy}{MM}{dd}_{n}", &ctx()).unwrap(), "수업_20260304_3");
        assert_eq!(render("{HH}{mm}{ss}-{앱}", &ctx()).unwrap(), "090507-chrome");
        assert_eq!(render("{app}", &ctx()).unwrap(), "chrome");
    }

    #[test]
    fn 금지_글자와_모르는_칸() {
        assert_eq!(validate("a/b"), Err(NamingError::ForbiddenChar('/')));
        assert_eq!(validate("{yyyy"), Err(NamingError::Unclosed));
        assert_eq!(validate("{year}"), Err(NamingError::UnknownToken("year".into())));
        assert_eq!(validate("   "), Err(NamingError::Empty));
        assert_eq!(validate(&"가".repeat(201)), Err(NamingError::TooLong));
        assert!(validate("Shookshok_{yyyy}{MM}{dd}_{HH}{mm}{ss}").is_ok());
    }

    #[test]
    fn 바깥_값도_정리하고_예약_이름을_피한다() {
        let mut c = ctx();
        c.app = Some("a:b?".into());
        assert_eq!(render("{앱}", &c).unwrap(), "a_b_");
        assert_eq!(sanitize("CON"), "_CON");
        assert_eq!(sanitize("com1.txt"), "_com1.txt");
        assert_eq!(sanitize("이름. "), "이름");
    }

    #[test]
    fn 중복이면_2를_붙인다() {
        let taken = ["a.png", "a (2).png"];
        assert_eq!(unique_file_name("a", "png", |n| taken.contains(&n)), "a (3).png");
        assert_eq!(unique_file_name("b", "png", |n| taken.contains(&n)), "b.png");
    }

    #[test]
    fn 긴_경로는_앞부분을_줄인다() {
        let dir = format!("C:\\{}", "d".repeat(200));
        let stem = format!("{}_20260304_3", "x".repeat(100));
        let fitted = fit_path(&dir, &stem, "png");
        assert!(fitted.ends_with("_20260304_3"));
        assert!(dir.chars().count() + 1 + fitted.chars().count() + 5 + 4 <= MAX_PATH_CHARS);
    }

    #[test]
    fn 다국어_오류_문구() {
        assert_eq!(NamingError::Empty.message(Lang::Ko), "이름 규칙이 비어 있어요");
        assert_eq!(NamingError::Empty.message(Lang::En), "File name pattern is empty");
        assert_eq!(NamingError::UnknownToken("year".into()).message(Lang::Ko), "지원하지 않는 항목이에요: {year}");
        assert_eq!(NamingError::UnknownToken("year".into()).message(Lang::En), "Unsupported token: {year}");
        assert_eq!(NamingError::Unclosed.message(Lang::En), "Missing closing '}'");
        assert_eq!(NamingError::ForbiddenChar('/').message(Lang::En), "Character not allowed in file name: /");
        assert_eq!(NamingError::TooLong.message(Lang::En), "File name is too long (max 200 chars)");
    }
}
