//! 다국어(R2-I18N) — Rust가 만드는 글(섬·오류·트레이 메뉴·OS 대화상자)용. 순수 함수.
//!
//! 카탈로그는 화면과 **같은 파일**(`packages/ink-ui/src/lib/i18n/ko.json`·`en.json`)을 읽는다 — 키 하나에 두 쪽이 같은 글.
//! 지금 언어는 전역에 두지 않고 부르는 쪽이 넘긴다(서비스는 설정 `general.lang`에서 — 시험이 서로 섞이지 않게).
//! 키는 점 표기, 값 끼우기는 `{이름}`. 없는 키는 한국어 → 키 그대로 순서로 대신한다.
//! OS별 글(R2-OSFIT-1): 같은 키에 `~mac`·`~win`을 붙인 변형이 있으면 그 OS에서 먼저 쓴다([`tr_os`]).
//! ko.json·en.json은 `areas/<영역>.<언어>.json`을 합친 결과(`node scripts/i18n-build.mjs`).

use crate::shortcuts::Platform;

use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const KO_JSON: &str = include_str!("../../../packages/ink-ui/src/lib/i18n/ko.json");
const EN_JSON: &str = include_str!("../../../packages/ink-ui/src/lib/i18n/en.json");

/// 지원 언어.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Lang {
    #[default]
    Ko,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Ko, Lang::En];

    /// 설정 값·OS 언어 문자열 → 지원 언어(모르면 한국어). `en-US`·`en_GB` → `En`.
    pub fn from_setting(v: &str) -> Self {
        if v.trim().to_ascii_lowercase().starts_with("en") {
            Lang::En
        } else {
            Lang::Ko
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::Ko => "ko",
            Lang::En => "en",
        }
    }
}

type Catalog = BTreeMap<String, String>;

fn parse(json: &str) -> Catalog {
    let v: Map<String, Value> = serde_json::from_str(json).expect("i18n 카탈로그 형식(평평한 키 → 글)");
    v.into_iter().filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string()))).collect()
}

fn catalog(lang: Lang) -> &'static Catalog {
    static KO: OnceLock<Catalog> = OnceLock::new();
    static EN: OnceLock<Catalog> = OnceLock::new();
    match lang {
        Lang::Ko => KO.get_or_init(|| parse(KO_JSON)),
        Lang::En => EN.get_or_init(|| parse(EN_JSON)),
    }
}

/// 키의 글(값 끼우기 없음).
pub fn tr(lang: Lang, key: &str) -> String {
    catalog(lang).get(key).or_else(|| catalog(Lang::Ko).get(key)).cloned().unwrap_or_else(|| key.to_string())
}

/// 그 OS 변형(`key~mac`·`key~win`)이 있으면 그것, 없으면 [`tr`].
pub fn tr_os(lang: Lang, os: Platform, key: &str) -> String {
    let v = format!("{key}~{}", if os == Platform::Mac { "mac" } else { "win" });
    catalog(lang).get(&v).or_else(|| catalog(Lang::Ko).get(&v)).cloned().unwrap_or_else(|| tr(lang, key))
}

/// [`tr_os`] + `{이름}` 끼우기.
pub fn trf_os(lang: Lang, os: Platform, key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    fill(tr_os(lang, os, key), args)
}

fn fill(mut s: String, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    for (name, v) in args {
        s = s.replace(&format!("{{{name}}}"), &v.to_string());
    }
    s
}

/// 키의 글 + `{이름}` 끼우기. 없는 이름은 자리 표시 그대로 둔다.
pub fn trf(lang: Lang, key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    fill(tr(lang, key), args)
}

/// 글 안의 `{이름}` 목록(시험·검사용, 정렬).
pub fn placeholders(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        let after = &rest[i + 1..];
        match after.find('}') {
            Some(j) if after[..j].chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && j > 0 => {
                out.push(after[..j].to_string());
                rest = &after[j + 1..];
            }
            _ => rest = after,
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 두_언어의_키와_값_끼우기_이름이_같다() {
        let ko = catalog(Lang::Ko);
        for lang in Lang::ALL {
            let c = catalog(lang);
            assert_eq!(c.keys().collect::<Vec<_>>(), ko.keys().collect::<Vec<_>>(), "{lang:?} 키 집합");
            for (k, v) in c {
                assert_eq!(placeholders(v), placeholders(&ko[k]), "{lang:?}:{k}");
                assert!(!v.trim().is_empty(), "{lang:?}:{k} 비어 있음");
                if let Some((base, _)) = k.split_once('~') {
                    assert!(c.contains_key(base), "{lang:?}:{k} — OS 변형은 기본 키도 있어야 해요");
                }
            }
        }
    }

    #[test]
    fn 번역_값_끼우기_대신하기() {
        assert_eq!(tr(Lang::En, "island.hotkeys.on"), "Shortcuts are back on");
        assert_eq!(trf(Lang::Ko, "island.hotkeys.onSome", &[("n", &2)]), "단축키를 켰어요 · 2개는 다른 앱이 쓰고 있어요");
        assert_eq!(tr(Lang::En, "no.such.key"), "no.such.key");
        assert_eq!(Lang::from_setting("en-US"), Lang::En);
        assert_eq!(Lang::from_setting("ko"), Lang::Ko);
        assert_eq!(Lang::from_setting(""), Lang::Ko);
        assert_eq!(placeholders("{a} 와 {b} {} {x y}"), ["a", "b"]);
        // 변형이 없으면 기본 키
        assert_eq!(tr_os(Lang::Ko, Platform::Mac, "island.hotkeys.off"), tr(Lang::Ko, "island.hotkeys.off"));
    }
}
