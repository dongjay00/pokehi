//! 스프라이트: RGBA 픽셀 격자와 크롭·축소 규칙.

use serde::{Deserialize, Serialize};

/// RGBA 픽셀 하나.
pub type Rgba = [u8; 4];

/// 인사 화면에 그릴 그림의 최대 크기(픽셀). 32열 × 16행이 된다.
pub const MAX_SPRITE_PX: u32 = 32;

/// 행 우선으로 저장한 RGBA 픽셀 격자.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sprite {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Rgba>,
}

impl Sprite {
    /// 픽셀 수가 `width * height`와 같아야 한다.
    pub fn new(width: u32, height: u32, pixels: Vec<Rgba>) -> Sprite {
        assert_eq!(
            pixels.len(),
            (width as usize) * (height as usize),
            "픽셀 수 불일치"
        );
        Sprite {
            width,
            height,
            pixels,
        }
    }

    /// (x, y) 픽셀.
    pub fn pixel(&self, x: u32, y: u32) -> Rgba {
        self.pixels[(y as usize) * (self.width as usize) + x as usize]
    }

    /// 알파 128 이상이면 불투명으로 본다.
    pub fn is_opaque(px: Rgba) -> bool {
        px[3] >= 128
    }

    /// 가로나 세로가 0이면 빈 스프라이트다.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// `canvas_w × canvas_h` 투명 캔버스의 (x, y)에 이 그림을 놓는다. 캔버스를 벗어나는 부분은 버린다.
    pub fn placed(&self, canvas_w: u32, canvas_h: u32, x: u32, y: u32) -> Sprite {
        let mut pixels = vec![[0, 0, 0, 0]; (canvas_w as usize) * (canvas_h as usize)];
        for sy in 0..self.height {
            for sx in 0..self.width {
                let (cx, cy) = (x + sx, y + sy);
                if cx < canvas_w && cy < canvas_h {
                    pixels[(cy as usize) * (canvas_w as usize) + cx as usize] = self.pixel(sx, sy);
                }
            }
        }
        Sprite::new(canvas_w, canvas_h, pixels)
    }

    /// 불투명 픽셀의 경계 상자로 자른다. 전부 투명하면 0×0이 된다.
    pub fn crop_transparent(&self) -> Sprite {
        let mut bounds: Option<(u32, u32, u32, u32)> = None;
        for y in 0..self.height {
            for x in 0..self.width {
                if Self::is_opaque(self.pixel(x, y)) {
                    bounds = Some(match bounds {
                        None => (x, y, x, y),
                        Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                    });
                }
            }
        }
        let Some((x0, y0, x1, y1)) = bounds else {
            return Sprite::new(0, 0, Vec::new());
        };
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut pixels = Vec::with_capacity((w * h) as usize);
        for y in y0..=y1 {
            for x in x0..=x1 {
                pixels.push(self.pixel(x, y));
            }
        }
        Sprite::new(w, h, pixels)
    }

    /// 가로세로 비율을 유지하며 `max_w × max_h` 안에 들어가게 최근접 이웃으로 줄인다.
    /// 원본이 더 작으면 키우지 않는다.
    pub fn fit_within(&self, max_w: u32, max_h: u32) -> Sprite {
        if self.is_empty() {
            return self.clone();
        }
        let (w, h) = (self.width as f64, self.height as f64);
        let scale = (max_w as f64 / w).min(max_h as f64 / h).min(1.0);
        if scale >= 1.0 {
            return self.clone();
        }
        let new_w = ((w * scale).round() as u32).max(1);
        let new_h = ((h * scale).round() as u32).max(1);
        let src = |dst: u32, limit: u32| -> u32 {
            (((dst as f64 + 0.5) / scale).floor() as u32).min(limit - 1)
        };
        let mut pixels = Vec::with_capacity((new_w * new_h) as usize);
        for y in 0..new_h {
            let sy = src(y, self.height);
            for x in 0..new_w {
                pixels.push(self.pixel(src(x, self.width), sy));
            }
        }
        Sprite::new(new_w, new_h, pixels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Rgba = [255, 0, 0, 255];
    const B: Rgba = [0, 0, 255, 255];
    const CLEAR: Rgba = [0, 0, 0, 0];

    fn solid(w: u32, h: u32, px: Rgba) -> Sprite {
        Sprite::new(w, h, vec![px; (w * h) as usize])
    }

    #[test]
    fn placed_puts_sprite_on_transparent_canvas() {
        let s = Sprite::new(2, 1, vec![R, B]);
        let c = s.placed(3, 2, 1, 1);
        assert_eq!((c.width, c.height), (3, 2));
        assert_eq!(c.pixels, vec![CLEAR, CLEAR, CLEAR, CLEAR, R, B]);
    }

    #[test]
    fn placed_clips_outside_canvas() {
        let s = Sprite::new(2, 1, vec![R, B]);
        assert_eq!(s.placed(2, 1, 1, 0).pixels, vec![CLEAR, R]);
    }

    fn with_opaque_at(w: u32, h: u32, points: &[(u32, u32)]) -> Sprite {
        let mut pixels = vec![CLEAR; (w * h) as usize];
        for &(x, y) in points {
            pixels[(y * w + x) as usize] = R;
        }
        Sprite::new(w, h, pixels)
    }

    #[test]
    fn crop_removes_transparent_margin() {
        let s = with_opaque_at(96, 96, &[(10, 20), (40, 70)]).crop_transparent();
        assert_eq!((s.width, s.height), (31, 51));
        assert_eq!(s.pixel(0, 0), R);
        assert_eq!(s.pixel(30, 50), R);
    }

    #[test]
    fn alpha_below_128_is_transparent() {
        assert!(solid(3, 3, [9, 9, 9, 127]).crop_transparent().is_empty());
        let mut pixels = vec![[9, 9, 9, 127]; 9];
        pixels[4] = [9, 9, 9, 128];
        let s = Sprite::new(3, 3, pixels).crop_transparent();
        assert_eq!((s.width, s.height), (1, 1));
    }

    #[test]
    fn crop_all_transparent_is_empty() {
        assert!(solid(96, 96, CLEAR).crop_transparent().is_empty());
    }

    #[test]
    fn fit_shrinks_keeping_ratio() {
        let s = solid(64, 48, [1, 2, 3, 255]).fit_within(32, 32);
        assert_eq!((s.width, s.height), (32, 24));
    }

    #[test]
    fn fit_never_enlarges() {
        let s = solid(20, 10, [1, 2, 3, 255]).fit_within(32, 32);
        assert_eq!((s.width, s.height), (20, 10));
    }

    #[test]
    fn fit_is_nearest_neighbor() {
        let s = Sprite::new(4, 1, vec![R, R, B, B]).fit_within(2, 2);
        assert_eq!((s.width, s.height), (2, 1));
        assert_eq!(s.pixels, vec![R, B]);
    }

    #[test]
    fn fit_empty_stays_empty() {
        assert!(Sprite::new(0, 0, vec![]).fit_within(32, 32).is_empty());
    }
}
