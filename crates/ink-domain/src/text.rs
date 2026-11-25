//! 한국어 문구 도우미 — 받침에 따라 조사를 고른다("‘범위 캡처’와", "‘창 항상 위’와", "‘저장’과").

/// 단어 끝 글자에 받침이 있으면 `with`, 없으면 `without`. 한글이 아니면 소리로 판단하기 어려워 `with(without)` 형태.
pub fn josa<'a>(word: &str, with: &'a str, without: &'a str) -> String {
    let last = word
        .chars()
        .rev()
        .find(|c| !matches!(c, '’' | '\'' | '"' | '”' | ')' | ' '));
    match last {
        Some(c @ '\u{AC00}'..='\u{D7A3}') => {
            let has_batchim = (c as u32 - 0xAC00) % 28 != 0;
            (if has_batchim { with } else { without }).to_string()
        }
        Some(c) if c.is_ascii_digit() => {
            // 0 영 1 일 3 삼 6 육 7 칠 8 팔 = 받침 있음, 2 이 4 사 5 오 9 구 = 없음
            (if matches!(c, '0' | '1' | '3' | '6' | '7' | '8') { with } else { without }).to_string()
        }
        _ => format!("{with}({without})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 받침에_맞는_조사() {
        assert_eq!(josa("‘범위 캡처’", "과", "와"), "와");
        assert_eq!(josa("‘저장’", "과", "와"), "과");
        assert_eq!(josa("F1", "이", "가"), "이");
        assert_eq!(josa("F2", "이", "가"), "가");
        assert_eq!(josa("Snipaste", "이", "가"), "이(가)");
    }
}
