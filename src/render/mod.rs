//! 렌더링 계층. `Greeting`을 ANSI 문자열로 바꾸는 순수 함수만 둔다.

pub mod color;
pub mod halfblock;
pub mod motion;
pub mod text;

/// 색 출력 방식.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
    /// 색 없음. 이스케이프 시퀀스를 하나도 쓰지 않는다.
    None,
}

use crate::domain::greeting::Greeting;
use crate::domain::sprite::Sprite;
use color::{bg, fg, BOLD, DIM, RESET};

/// 그림과 정보 사이 간격(칸).
pub const GAP: usize = 3;
/// 터미널이 좁아도 정보 열은 최소 이 폭을 쓴다.
pub const MIN_INFO_WIDTH: usize = 20;

/// 색 모드가 `None`이면 스타일 이스케이프를 쓰지 않는다.
fn style(mode: ColorMode, code: &'static str) -> &'static str {
    if mode == ColorMode::None {
        ""
    } else {
        code
    }
}

/// 뱃지 배경색 위 글자색. 밝은 배경이면 검정, 아니면 흰색.
fn badge_text_rgb(rgb: (u8, u8, u8)) -> (u8, u8, u8) {
    let luma = 0.299 * f64::from(rgb.0) + 0.587 * f64::from(rgb.1) + 0.114 * f64::from(rgb.2);
    if luma > 150.0 {
        (0, 0, 0)
    } else {
        (255, 255, 255)
    }
}

fn info_lines(g: &Greeting, info_width: usize, mode: ColorMode) -> Vec<String> {
    let (bold, dim, reset) = (style(mode, BOLD), style(mode, DIM), style(mode, RESET));
    let mut lines = Vec::new();

    let mut title = format!("{bold}{} {}{reset}", g.number_label, g.name);
    if let Some(secondary) = &g.secondary_name {
        title.push_str(&format!(" {dim}{secondary}{reset}"));
    }
    lines.push(title);

    if !g.badges.is_empty() {
        let badges: Vec<String> = g
            .badges
            .iter()
            .map(|b| match mode {
                ColorMode::None => format!("[{}]", b.label),
                _ => format!(
                    "{}{} {} {RESET}",
                    bg(mode, b.rgb),
                    fg(mode, badge_text_rgb(b.rgb)),
                    b.label
                ),
            })
            .collect();
        lines.push(badges.join(" "));
    }

    if let Some(genus) = &g.genus {
        lines.push(genus.clone());
    }
    if let Some(flavor) = &g.flavor {
        lines.extend(text::wrap(flavor, info_width));
    }
    lines
}

/// 인사 화면 전체를 그린다. 왼쪽에 그림, 오른쪽에 정보(A 배치).
///
/// 색 모드가 `None`이면 그림을 생략하고 정보만 이스케이프 없이 출력한다.
/// 출력은 빈 줄 하나로 끝난다.
pub fn render(g: &Greeting, term_width: usize, mode: ColorMode) -> String {
    render_layout(g, g.sprite.as_ref(), GAP, term_width, mode)
}

/// `render`와 같지만 그림과 간격을 바꿔 그린다. 모션 프레임을 만들 때 쓴다.
pub(crate) fn render_layout(
    g: &Greeting,
    sprite: Option<&Sprite>,
    gap: usize,
    term_width: usize,
    mode: ColorMode,
) -> String {
    let sprite = sprite.filter(|_| mode != ColorMode::None);
    let sprite_rows = sprite
        .map(|s| halfblock::sprite_rows(s, mode))
        .unwrap_or_default();
    let sprite_width = sprite.map_or(0, |s| s.width as usize);
    let left_width = if sprite.is_some() {
        sprite_width + gap
    } else {
        0
    };

    let info_width = term_width.saturating_sub(left_width).max(MIN_INFO_WIDTH);
    let info = info_lines(g, info_width, mode);

    let top = sprite_rows.len().saturating_sub(info.len()) / 2;
    let total = sprite_rows.len().max(top + info.len());

    let mut out = String::new();
    for row in 0..total {
        let info_line = row.checked_sub(top).and_then(|i| info.get(i));
        if sprite.is_some() {
            match sprite_rows.get(row) {
                Some(cells) => out.push_str(cells),
                None => out.push_str(&" ".repeat(sprite_width)),
            }
            if let Some(line) = info_line {
                out.push_str(&" ".repeat(gap));
                out.push_str(line);
            }
        } else if let Some(line) = info_line {
            out.push_str(line);
        }
        out.push('\n');
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod layout_tests;
