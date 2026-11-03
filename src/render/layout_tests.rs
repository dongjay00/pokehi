//! `render` 배치 테스트.

use super::*;
use crate::domain::greeting::{Badge, Greeting};
use crate::domain::sprite::Sprite;
use crate::render::text::display_width;

/// ANSI 이스케이프(`ESC [ ... 문자`)를 지운다.
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn opaque(w: u32, h: u32) -> Sprite {
    Sprite::new(w, h, vec![[200, 100, 50, 255]; (w * h) as usize])
}

// 합성 데이터.
fn g() -> Greeting {
    Greeting {
        number_label: "No.025".into(),
        name: "테스트몬".into(),
        secondary_name: Some("Testmon".into()),
        badges: vec![
            Badge {
                label: "전기".into(),
                rgb: (0xF7, 0xD0, 0x2C),
            },
            Badge {
                label: "비행".into(),
                rgb: (0xA9, 0x8F, 0xF3),
            },
        ],
        genus: Some("시험포켓몬".into()),
        flavor: Some("꼬리를 세우고 주변을 살피는 합성 설명 문장".into()),
        sprite: None,
    }
}

fn g_with_sprite() -> Greeting {
    Greeting {
        sprite: Some(opaque(10, 8)),
        ..g()
    }
}

#[test]
fn layout_info_column_aligned_with_korean() {
    let out = strip_ansi(&render(&g_with_sprite(), 80, ColorMode::TrueColor));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 5); // 그림 4행 + 빈 줄
    let prefix = format!("{}{}", "▀".repeat(10), " ".repeat(GAP));
    let expected = [
        "No.025 테스트몬 Testmon",
        " 전기   비행 ",
        "시험포켓몬",
        "꼬리를 세우고 주변을 살피는 합성 설명 문장",
    ];
    for (line, want) in lines[..4].iter().zip(expected) {
        assert_eq!(*line, format!("{prefix}{want}"));
    }
}

#[test]
fn info_vertically_centered() {
    let gr = Greeting {
        sprite: Some(opaque(10, 32)),
        ..g()
    };
    let out = strip_ansi(&render(&gr, 80, ColorMode::TrueColor));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 17);
    assert!(!lines[5].contains("No.025"));
    assert!(lines[6].contains("No.025"));
}

#[test]
fn info_longer_than_sprite_starts_at_top() {
    let gr = Greeting {
        sprite: Some(opaque(4, 2)),
        ..g()
    };
    let out = strip_ansi(&render(&gr, 80, ColorMode::TrueColor));
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[0].contains("No.025"));
    // 그림이 끝난 행도 정보 열은 같은 위치에서 시작한다.
    assert!(lines[1].starts_with(&" ".repeat(4 + GAP)));
}

#[test]
fn no_sprite_prints_info_only() {
    let out = strip_ansi(&render(&g(), 80, ColorMode::TrueColor));
    assert!(out.starts_with("No.025"));
}

#[test]
fn color_none_has_no_escapes_and_bracket_badges() {
    let out = render(&g_with_sprite(), 80, ColorMode::None);
    assert!(!out.contains('\x1b'));
    assert!(out.contains("[전기] [비행]"));
    assert!(!out.contains('▀'));
    assert!(out.starts_with("No.025 테스트몬 Testmon\n"));
}

#[test]
fn badge_text_color_follows_luminance() {
    let out = render(&g(), 80, ColorMode::TrueColor);
    // 밝은 전기 뱃지는 검정 글자, 어두운 색이면 흰 글자.
    assert!(out.contains("\x1b[48;2;247;208;44m\x1b[38;2;0;0;0m 전기 "));
    let dark = Greeting {
        badges: vec![Badge {
            label: "악".into(),
            rgb: (0x70, 0x57, 0x46),
        }],
        ..g()
    };
    assert!(render(&dark, 80, ColorMode::TrueColor).contains("\x1b[38;2;255;255;255m 악 "));
}

#[test]
fn flavor_wraps_to_info_width() {
    let gr = Greeting {
        sprite: Some(opaque(32, 32)),
        flavor: Some("가나다 라마바사 아자차카 타파하 가나다 라마바사 아자차카".into()),
        ..g()
    };
    let out = strip_ansi(&render(&gr, 50, ColorMode::TrueColor));
    let flavor_lines: Vec<String> = out
        .lines()
        .filter(|l| l.contains("가나다") || l.contains("아자차카") || l.contains("라마바사"))
        .map(|l| l.chars().skip(32 + GAP).collect())
        .collect();
    assert!(flavor_lines.len() >= 2);
    for l in flavor_lines {
        assert!(display_width(&l) <= 20, "{l:?}");
    }
}

#[test]
fn narrow_terminal_uses_min_info_width() {
    let gr = Greeting {
        sprite: Some(opaque(32, 32)),
        ..g()
    };
    let out = strip_ansi(&render(&gr, 40, ColorMode::TrueColor));
    assert!(out.contains("No.025"));
    let tiny = strip_ansi(&render(&gr, 0, ColorMode::TrueColor));
    assert!(tiny.contains("No.025"));
}

#[test]
fn missing_lines_are_omitted() {
    let gr = Greeting {
        genus: None,
        flavor: None,
        secondary_name: None,
        ..g()
    };
    let out = render(&gr, 80, ColorMode::None);
    assert_eq!(out, "No.025 테스트몬\n[전기] [비행]\n\n");
}

#[test]
fn ends_with_blank_line() {
    assert!(render(&g(), 80, ColorMode::TrueColor).ends_with("\n\n"));
    assert!(render(&g_with_sprite(), 80, ColorMode::Ansi256).ends_with("\n\n"));
}
