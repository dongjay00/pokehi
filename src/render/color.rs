//! 색 모드 결정과 ANSI 색 이스케이프.

use super::ColorMode;

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

/// xterm 256색 6×6×6 큐브의 단계 값.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

fn dist2(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
    let d = |x: u8, y: u8| (i32::from(x) - i32::from(y)).pow(2) as u32;
    d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2)
}

fn nearest_level(v: u8) -> usize {
    (0..CUBE_LEVELS.len())
        .min_by_key(|&i| (i32::from(CUBE_LEVELS[i]) - i32::from(v)).abs())
        .unwrap()
}

/// RGB를 xterm 256색 번호로 바꾼다. 색상 큐브(16~231)와 회색조(232~255) 중
/// 제곱 거리가 가장 가까운 것을 고르며, 동률이면 큐브를 쓴다. 0~15번은 쓰지 않는다.
pub fn to_ansi256(rgb: (u8, u8, u8)) -> u8 {
    let (ri, gi, bi) = (
        nearest_level(rgb.0),
        nearest_level(rgb.1),
        nearest_level(rgb.2),
    );
    let cube_rgb = (CUBE_LEVELS[ri], CUBE_LEVELS[gi], CUBE_LEVELS[bi]);
    let cube_code = 16 + 36 * ri + 6 * gi + bi;
    let cube_dist = dist2(rgb, cube_rgb);

    let (gray_i, gray_dist) = (0..24u8)
        .map(|i| {
            let v = 8 + 10 * i;
            (i, dist2(rgb, (v, v, v)))
        })
        .min_by_key(|&(_, d)| d)
        .unwrap();

    if gray_dist < cube_dist {
        232 + gray_i
    } else {
        cube_code as u8
    }
}

/// 환경 변수만 보고 색 모드를 정한다. 터미널에 질의하지 않는다.
///
/// `windows_vt_ok`는 Windows에서 VT 모드를 켠 결과이고, 다른 OS에서는 `None`이다.
/// `--color` 강제 지정은 호출하는 쪽(CLI)이 먼저 처리한다.
pub fn detect_color_mode(
    env: &dyn Fn(&str) -> Option<String>,
    windows_vt_ok: Option<bool>,
) -> ColorMode {
    // VT가 꺼진 콘솔에는 어떤 이스케이프도 안전하지 않다.
    if windows_vt_ok == Some(false) {
        return ColorMode::None;
    }
    if env("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return ColorMode::None;
    }
    if env("COLORTERM").is_some_and(|v| v == "truecolor" || v == "24bit") {
        return ColorMode::TrueColor;
    }
    if env("WT_SESSION").is_some() {
        return ColorMode::TrueColor;
    }
    match env("TERM_PROGRAM").as_deref() {
        Some("iTerm.app" | "WezTerm" | "ghostty") => return ColorMode::TrueColor,
        Some("Apple_Terminal") => return ColorMode::Ansi256,
        _ => {}
    }
    if windows_vt_ok == Some(true) {
        return ColorMode::TrueColor;
    }
    // 알 수 없을 때는 더 안전한 256색.
    ColorMode::Ansi256
}

fn sgr(mode: ColorMode, layer: u8, rgb: (u8, u8, u8)) -> String {
    match mode {
        ColorMode::TrueColor => format!("\x1b[{layer};2;{};{};{}m", rgb.0, rgb.1, rgb.2),
        ColorMode::Ansi256 => format!("\x1b[{layer};5;{}m", to_ansi256(rgb)),
        ColorMode::None => String::new(),
    }
}

/// 전경색 이스케이프. 색 없음 모드에서는 빈 문자열.
pub fn fg(mode: ColorMode, rgb: (u8, u8, u8)) -> String {
    sgr(mode, 38, rgb)
}

/// 배경색 이스케이프. 색 없음 모드에서는 빈 문자열.
pub fn bg(mode: ColorMode, rgb: (u8, u8, u8)) -> String {
    sgr(mode, 48, rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: std::collections::HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn ansi256_pins() {
        assert_eq!(to_ansi256((255, 0, 0)), 196);
        assert_eq!(to_ansi256((0, 0, 0)), 16);
        assert_eq!(to_ansi256((255, 255, 255)), 231);
        assert_eq!(to_ansi256((128, 128, 128)), 244);
    }

    #[test]
    fn ansi256_never_uses_basic_16() {
        for r in (0..=255).step_by(17) {
            for g in (0..=255).step_by(51) {
                assert!(to_ansi256((r as u8, g as u8, 7)) >= 16);
            }
        }
    }

    #[test]
    fn colorterm_truecolor() {
        assert_eq!(
            detect_color_mode(&env_of(&[("COLORTERM", "truecolor")]), None),
            ColorMode::TrueColor
        );
        assert_eq!(
            detect_color_mode(&env_of(&[("COLORTERM", "24bit")]), None),
            ColorMode::TrueColor
        );
    }

    #[test]
    fn windows_terminal_session() {
        assert_eq!(
            detect_color_mode(&env_of(&[("WT_SESSION", "abc")]), None),
            ColorMode::TrueColor
        );
    }

    #[test]
    fn truecolor_term_programs() {
        for p in ["iTerm.app", "WezTerm", "ghostty"] {
            assert_eq!(
                detect_color_mode(&env_of(&[("TERM_PROGRAM", p)]), None),
                ColorMode::TrueColor
            );
        }
    }

    #[test]
    fn apple_terminal_is_256() {
        assert_eq!(
            detect_color_mode(&env_of(&[("TERM_PROGRAM", "Apple_Terminal")]), None),
            ColorMode::Ansi256
        );
    }

    #[test]
    fn no_color_wins_over_colorterm() {
        let env = env_of(&[("NO_COLOR", "1"), ("COLORTERM", "truecolor")]);
        assert_eq!(detect_color_mode(&env, None), ColorMode::None);
    }

    #[test]
    fn empty_no_color_ignored() {
        let env = env_of(&[("NO_COLOR", ""), ("COLORTERM", "truecolor")]);
        assert_eq!(detect_color_mode(&env, None), ColorMode::TrueColor);
    }

    #[test]
    fn windows_vt_failed_is_none() {
        assert_eq!(
            detect_color_mode(&env_of(&[("WT_SESSION", "abc")]), Some(false)),
            ColorMode::None
        );
    }

    #[test]
    fn windows_vt_ok_is_truecolor() {
        assert_eq!(
            detect_color_mode(&env_of(&[]), Some(true)),
            ColorMode::TrueColor
        );
    }

    #[test]
    fn unknown_is_256() {
        assert_eq!(detect_color_mode(&env_of(&[]), None), ColorMode::Ansi256);
    }

    #[test]
    fn escape_sequences() {
        assert_eq!(fg(ColorMode::TrueColor, (1, 2, 3)), "\x1b[38;2;1;2;3m");
        assert_eq!(bg(ColorMode::TrueColor, (1, 2, 3)), "\x1b[48;2;1;2;3m");
        assert_eq!(fg(ColorMode::Ansi256, (255, 0, 0)), "\x1b[38;5;196m");
        assert_eq!(bg(ColorMode::Ansi256, (255, 0, 0)), "\x1b[48;5;196m");
        assert_eq!(fg(ColorMode::None, (1, 2, 3)), "");
        assert_eq!(bg(ColorMode::None, (1, 2, 3)), "");
    }
}
