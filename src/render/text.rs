//! 표시 폭 계산과 줄바꿈. 한글 등 전각 문자는 2칸으로 센다.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// 터미널에서 차지하는 칸 수. 이스케이프가 섞이지 않은 순수 텍스트에만 쓴다.
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// 공백 기준으로 탐욕적으로 줄을 나눈다. 한 단어가 `width`보다 길면 문자 단위로 쪼갠다.
/// 각 줄의 표시 폭은 `width` 이하다(문자 하나가 `width`보다 넓은 경우만 예외).
/// `width`가 0이면 1로 본다.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut line_w = 0;

    for word in text.split_whitespace() {
        let word_w = display_width(word);
        let needed = if line.is_empty() {
            word_w
        } else {
            line_w + 1 + word_w
        };
        if needed <= width {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
            line_w = needed;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line_w = 0;
        }
        if word_w <= width {
            line.push_str(word);
            line_w = word_w;
            continue;
        }
        // 너무 긴 단어는 문자 단위로 자른다. 마지막 조각은 다음 단어와 이어질 수 있다.
        for ch in word.chars() {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if line_w + ch_w > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line_w = 0;
            }
            line.push(ch);
            line_w += ch_w;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hangul_is_double_width() {
        assert_eq!(display_width("피카츄"), 6);
        assert_eq!(display_width("Pokémon"), 7);
    }

    #[test]
    fn wrap_on_spaces() {
        assert_eq!(wrap("가나 다라 마바", 10), vec!["가나 다라", "마바"]);
    }

    #[test]
    fn wrap_splits_long_word() {
        assert_eq!(wrap("가나다라마", 4), vec!["가나", "다라", "마"]);
    }

    #[test]
    fn wrap_long_word_then_more_words() {
        assert_eq!(wrap("abcdefgh ij", 5), vec!["abcde", "fgh", "ij"]);
    }

    #[test]
    fn wrap_lines_never_exceed_width() {
        let text = "꼬리를 세우고 Pokémon superlongwordwithoutspaces 주변의 상황을 살피다 보면 가끔 꼬리에 번개가 친다.";
        for width in 2..30 {
            for line in wrap(text, width) {
                assert!(display_width(&line) <= width, "폭 {width}: {line:?}");
            }
        }
    }

    #[test]
    fn wrap_keeps_all_words() {
        let text = "a bb ccc 가나 다라마바사";
        let joined: String = wrap(text, 4).concat().replace(' ', "");
        assert_eq!(joined, text.replace(' ', ""));
    }

    #[test]
    fn wrap_empty() {
        assert!(wrap("", 10).is_empty());
        assert!(wrap("   ", 10).is_empty());
    }

    #[test]
    fn wrap_zero_width_is_one() {
        assert_eq!(wrap("ab", 0), vec!["a", "b"]);
    }
}
