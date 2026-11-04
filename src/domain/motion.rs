//! 등장 모션: 그림을 몇 프레임 동안 움직였다가 멈추는 규칙.
//!
//! 모션은 그림 주변에 여백(캔버스)을 두고 그 안에서 그림 위치만 바꾼다.
//! 마지막 프레임은 항상 쉬는 자세(`Pose::REST`)라서 화면에 남는 모습이 고정된다.

use std::time::Duration;

use super::sprite::Sprite;

/// 모션 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// 제자리에서 깡총 두 번 뛴다.
    Hop,
    /// 좌우로 흔든다.
    Shake,
}

/// 한 프레임의 그림 위치. 쉬는 자세 기준으로 오른쪽 `dx`픽셀, 위쪽 `lift`픽셀.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pose {
    pub dx: u32,
    pub lift: u32,
}

impl Pose {
    pub const REST: Pose = Pose { dx: 0, lift: 0 };
}

const fn up(lift: u32) -> Pose {
    Pose { dx: 0, lift }
}

const fn right(dx: u32) -> Pose {
    Pose { dx, lift: 0 }
}

/// 깡총: 크게 한 번, 작게 한 번. 꼭대기에서 한 프레임 머물러 체공감을 준다.
const HOP: [Pose; 13] = [
    up(0),
    up(2),
    up(3),
    up(4),
    up(4),
    up(3),
    up(2),
    up(0),
    up(0),
    up(1),
    up(2),
    up(1),
    up(0),
];

/// 흔들기: 오른쪽으로 두 칸까지 빠르게 왕복한다.
const SHAKE: [Pose; 10] = [
    right(0),
    right(1),
    right(2),
    right(1),
    right(0),
    right(1),
    right(2),
    right(1),
    right(0),
    right(0),
];

impl Motion {
    /// `hop`, `shake`를 받는다. 대소문자는 가리지 않는다.
    /// `none`은 `Some(None)`(모션 끔), 모르는 값은 `None`이다.
    pub fn parse(s: &str) -> Option<Option<Motion>> {
        match s.to_ascii_lowercase().as_str() {
            "hop" => Some(Some(Motion::Hop)),
            "shake" => Some(Some(Motion::Shake)),
            "none" => Some(None),
            _ => None,
        }
    }

    /// 프레임 순서. 처음과 끝은 쉬는 자세다.
    pub fn poses(self) -> &'static [Pose] {
        match self {
            Motion::Hop => &HOP,
            Motion::Shake => &SHAKE,
        }
    }

    /// 프레임 사이 간격.
    pub fn frame_interval(self) -> Duration {
        match self {
            Motion::Hop => Duration::from_millis(50),
            Motion::Shake => Duration::from_millis(45),
        }
    }

    /// 그림 둘레에 필요한 여백 `(가로, 세로)` 픽셀. 모든 자세가 이 안에 들어간다.
    pub fn margin(self) -> (u32, u32) {
        let poses = self.poses();
        let w = poses.iter().map(|p| p.dx).max().unwrap_or(0);
        let h = poses.iter().map(|p| p.lift).max().unwrap_or(0);
        (w, h)
    }

    /// 자세에 맞춰 여백 캔버스 위에 그림을 놓는다.
    /// 캔버스 크기는 자세와 상관없이 같다. 세로 여백은 위쪽에, 가로 여백은 오른쪽에 둔다.
    pub fn place(self, sprite: &Sprite, pose: Pose) -> Sprite {
        let (mw, mh) = self.margin();
        sprite.placed(
            sprite.width + mw,
            sprite.height + mh,
            pose.dx.min(mw),
            mh - pose.lift.min(mh),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: [u8; 4] = [1, 2, 3, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];

    #[test]
    fn parse_values() {
        assert_eq!(Motion::parse("hop"), Some(Some(Motion::Hop)));
        assert_eq!(Motion::parse("SHAKE"), Some(Some(Motion::Shake)));
        assert_eq!(Motion::parse("none"), Some(None));
        assert_eq!(Motion::parse("spin"), None);
    }

    #[test]
    fn every_motion_starts_and_ends_at_rest() {
        for m in [Motion::Hop, Motion::Shake] {
            let poses = m.poses();
            assert_eq!(poses.first(), Some(&Pose::REST));
            assert_eq!(poses.last(), Some(&Pose::REST));
        }
    }

    #[test]
    fn motions_stay_short() {
        // 셸 시작을 너무 늦추지 않도록 1초 안에 끝나야 한다.
        for m in [Motion::Hop, Motion::Shake] {
            let total = m.frame_interval() * (m.poses().len() as u32 - 1);
            assert!(total < Duration::from_secs(1), "{m:?}: {total:?}");
        }
    }

    #[test]
    fn margins() {
        assert_eq!(Motion::Hop.margin(), (0, 4));
        assert_eq!(Motion::Shake.margin(), (2, 0));
    }

    #[test]
    fn hop_rest_sits_at_bottom_and_lift_moves_up() {
        let s = Sprite::new(1, 1, vec![P]);
        let rest = Motion::Hop.place(&s, Pose::REST);
        assert_eq!((rest.width, rest.height), (1, 5));
        assert_eq!(rest.pixel(0, 4), P);
        assert_eq!(rest.pixel(0, 0), CLEAR);
        let top = Motion::Hop.place(&s, up(4));
        assert_eq!((top.width, top.height), (1, 5));
        assert_eq!(top.pixel(0, 0), P);
        assert_eq!(top.pixel(0, 4), CLEAR);
    }

    #[test]
    fn shake_moves_right_within_same_canvas() {
        let s = Sprite::new(1, 1, vec![P]);
        let moved = Motion::Shake.place(&s, right(2));
        assert_eq!((moved.width, moved.height), (3, 1));
        assert_eq!(moved.pixels, vec![CLEAR, CLEAR, P]);
    }
}
