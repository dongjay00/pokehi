//! 반 블록(`▀`/`▄`) 문자로 스프라이트를 그린다. 한 칸에 세로 2픽셀이 들어간다.

use super::color::{bg, fg, RESET};
use super::ColorMode;
use crate::domain::sprite::{Rgba, Sprite};

fn rgb(px: Rgba) -> (u8, u8, u8) {
    (px[0], px[1], px[2])
}

/// 스프라이트를 `ceil(h/2)`줄로 그린다. 각 줄의 폭은 `sprite.width`칸이다.
///
/// 색을 쓴 셀 뒤에는 바로 `RESET`을 넣어 투명 셀에 배경색이 새지 않게 하고,
/// 줄 끝에도 `RESET`을 붙인다.
pub fn sprite_rows(sprite: &Sprite, mode: ColorMode) -> Vec<String> {
    let mut rows = Vec::with_capacity(sprite.height.div_ceil(2) as usize);
    for y in (0..sprite.height).step_by(2) {
        let mut row = String::new();
        for x in 0..sprite.width {
            let top = Some(sprite.pixel(x, y)).filter(|&p| Sprite::is_opaque(p));
            let bottom = (y + 1 < sprite.height)
                .then(|| sprite.pixel(x, y + 1))
                .filter(|&p| Sprite::is_opaque(p));
            match (top, bottom) {
                (None, None) => row.push(' '),
                (Some(t), None) => {
                    row.push_str(&fg(mode, rgb(t)));
                    row.push('▀');
                    row.push_str(RESET);
                }
                (None, Some(b)) => {
                    row.push_str(&fg(mode, rgb(b)));
                    row.push('▄');
                    row.push_str(RESET);
                }
                (Some(t), Some(b)) => {
                    row.push_str(&fg(mode, rgb(t)));
                    row.push_str(&bg(mode, rgb(b)));
                    row.push('▀');
                    row.push_str(RESET);
                }
            }
        }
        row.push_str(RESET);
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::color::RESET;

    const R: Rgba = [255, 0, 0, 255];
    const B: Rgba = [0, 0, 255, 255];
    const CLEAR: Rgba = [0, 0, 0, 0];

    fn two_by_two() -> Sprite {
        Sprite::new(2, 2, vec![R, CLEAR, CLEAR, B])
    }

    #[test]
    fn halfblock_four_cases() {
        let rows = sprite_rows(&two_by_two(), ColorMode::TrueColor);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert!(row.starts_with("\x1b[38;2;255;0;0m▀"));
        assert!(row.contains("\x1b[38;2;0;0;255m▄"));
        assert!(row.ends_with(RESET));
    }

    #[test]
    fn both_transparent_is_space() {
        let rows = sprite_rows(&Sprite::new(1, 2, vec![CLEAR, CLEAR]), ColorMode::TrueColor);
        assert_eq!(rows[0], format!(" {RESET}"));
    }

    #[test]
    fn halfblock_both_opaque_uses_bg() {
        let rows = sprite_rows(&Sprite::new(1, 2, vec![R, B]), ColorMode::TrueColor);
        assert!(rows[0].contains("\x1b[38;2;255;0;0m\x1b[48;2;0;0;255m▀"));
    }

    #[test]
    fn colored_cell_resets_before_next_cell() {
        // 배경색이 다음 투명 셀로 새지 않아야 한다.
        let s = Sprite::new(2, 2, vec![R, CLEAR, B, CLEAR]);
        let row = &sprite_rows(&s, ColorMode::TrueColor)[0];
        assert!(row.contains(&format!("▀{RESET} ")));
    }

    #[test]
    fn odd_height_last_row_bottom_transparent() {
        let rows = sprite_rows(&Sprite::new(1, 3, vec![R, R, B]), ColorMode::TrueColor);
        assert_eq!(rows.len(), 2);
        assert!(rows[1].starts_with("\x1b[38;2;0;0;255m▀"));
    }

    #[test]
    fn ansi256_mode_uses_palette_codes() {
        let rows = sprite_rows(&Sprite::new(1, 1, vec![R]), ColorMode::Ansi256);
        assert!(rows[0].contains("\x1b[38;5;196m"));
    }
}
