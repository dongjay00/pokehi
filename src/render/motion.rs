//! 모션 프레임 그리기. 프레임마다 인사 화면 전체 문자열을 만든다.

use std::time::Duration;

use super::text::visible_width;
use super::{render_layout, ColorMode, GAP};
use crate::domain::greeting::Greeting;
use crate::domain::motion::Motion;

/// 미리 그려 둔 프레임 묶음.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
    pub frames: Vec<String>,
    pub interval: Duration,
}

impl Animation {
    /// 모든 프레임이 줄바꿈 없이 화면 안에 들어가는지 본다.
    /// 다시 그릴 때 커서를 프레임 줄 수만큼 올리므로, 넘치면 화면이 깨진다.
    pub fn fits(&self, term_width: usize, term_height: usize) -> bool {
        self.frames.iter().all(|f| {
            f.lines().count() < term_height && f.lines().all(|l| visible_width(l) < term_width)
        })
    }
}

/// 모션 프레임을 만든다. 그림이 없거나 색을 쓰지 않으면 `None`이다.
///
/// 가로 여백은 그림과 정보 사이 간격에서 빌려 와서, 정보 열 위치는 정지 화면과 같다.
pub fn animate(
    g: &Greeting,
    motion: Motion,
    term_width: usize,
    mode: ColorMode,
) -> Option<Animation> {
    if mode == ColorMode::None {
        return None;
    }
    let sprite = g.sprite.as_ref().filter(|s| !s.is_empty())?;
    let (mw, _) = motion.margin();
    let gap = GAP.saturating_sub(mw as usize).max(1);
    let frames = motion
        .poses()
        .iter()
        .map(|&pose| {
            let placed = motion.place(sprite, pose);
            render_layout(g, Some(&placed), gap, term_width, mode)
        })
        .collect();
    Some(Animation {
        frames,
        interval: motion.frame_interval(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::sprite::Sprite;
    use crate::render::render;
    use crate::render::text::strip_ansi;

    fn g() -> Greeting {
        Greeting {
            number_label: "No.025".into(),
            name: "테스트몬".into(),
            secondary_name: None,
            badges: vec![],
            genus: Some("시험포켓몬".into()),
            flavor: None,
            sprite: Some(Sprite::new(4, 4, vec![[200, 100, 50, 255]; 16])),
        }
    }

    #[test]
    fn no_animation_without_color_or_sprite() {
        assert!(animate(&g(), Motion::Hop, 80, ColorMode::None).is_none());
        let no_sprite = Greeting {
            sprite: None,
            ..g()
        };
        assert!(animate(&no_sprite, Motion::Hop, 80, ColorMode::TrueColor).is_none());
    }

    #[test]
    fn all_frames_have_same_line_count() {
        for m in [Motion::Hop, Motion::Shake] {
            let a = animate(&g(), m, 80, ColorMode::TrueColor).unwrap();
            assert_eq!(a.frames.len(), m.poses().len());
            let n = a.frames[0].lines().count();
            assert!(a.frames.iter().all(|f| f.lines().count() == n), "{m:?}");
        }
    }

    #[test]
    fn hop_frames_differ_and_end_at_rest() {
        let a = animate(&g(), Motion::Hop, 80, ColorMode::TrueColor).unwrap();
        assert_ne!(a.frames[0], a.frames[3]);
        assert_eq!(a.frames.first(), a.frames.last());
        // 쉬는 자세에서는 위쪽 여백 두 줄(4픽셀)이 비어 있다.
        let rest = strip_ansi(&a.frames[0]);
        let lines: Vec<&str> = rest.lines().collect();
        assert!(lines[0].starts_with("    "));
        assert!(lines[2].starts_with("▀▀▀▀"));
    }

    #[test]
    fn shake_last_frame_looks_like_static_render() {
        let a = animate(&g(), Motion::Shake, 80, ColorMode::TrueColor).unwrap();
        let last = strip_ansi(a.frames.last().unwrap());
        assert_eq!(last, strip_ansi(&render(&g(), 80, ColorMode::TrueColor)));
        // 흔드는 중에도 정보 열 위치는 그대로다.
        let mid = strip_ansi(&a.frames[2]);
        let col = |s: &str| s.lines().map(|l| l.find("No.025")).find_map(|c| c);
        assert_eq!(col(&mid), col(&last));
    }

    #[test]
    fn fits_checks_width_and_height() {
        let a = animate(&g(), Motion::Hop, 80, ColorMode::TrueColor).unwrap();
        let lines = a.frames[0].lines().count();
        assert!(a.fits(80, lines + 1));
        assert!(!a.fits(80, lines));
        assert!(!a.fits(10, 50));
    }
}
