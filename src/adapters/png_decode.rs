//! PNG 디코딩. 어떤 색 형식이든 RGBA8 `Sprite`로 바꾼다.

use crate::domain::sprite::{Rgba, Sprite};

/// PNG 바이트를 RGBA8 스프라이트로 디코딩한다. 실패하면 `None`.
///
/// 팔레트·그레이·RGB는 RGBA로 확장하고, 16비트는 8비트로 줄인다.
pub fn decode_png(bytes: &[u8]) -> Option<Sprite> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(
        png::Transformations::EXPAND | png::Transformations::ALPHA | png::Transformations::STRIP_16,
    );
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let data = &buf[..info.buffer_size()];
    let pixels: Vec<Rgba> = match info.color_type {
        png::ColorType::Rgba => data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => data.iter().map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    if pixels.len() != (info.width as usize) * (info.height as usize) {
        return None;
    }
    Some(Sprite::new(info.width, info.height, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(
        w: u32,
        h: u32,
        color: png::ColorType,
        data: &[u8],
        palette: Option<(&[u8], &[u8])>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w, h);
            enc.set_color(color);
            enc.set_depth(png::BitDepth::Eight);
            if let Some((plte, trns)) = palette {
                enc.set_palette(plte.to_vec());
                enc.set_trns(trns.to_vec());
            }
            let mut writer = enc.write_header().unwrap();
            writer.write_image_data(data).unwrap();
        }
        out
    }

    #[test]
    fn decode_rgba_png() {
        let data = [255, 0, 0, 255, 0, 0, 255, 10];
        let s = decode_png(&encode(2, 1, png::ColorType::Rgba, &data, None)).unwrap();
        assert_eq!((s.width, s.height), (2, 1));
        assert_eq!(s.pixels, vec![[255, 0, 0, 255], [0, 0, 255, 10]]);
    }

    #[test]
    fn decode_rgb_png_is_opaque() {
        let s = decode_png(&encode(1, 1, png::ColorType::Rgb, &[1, 2, 3], None)).unwrap();
        assert_eq!(s.pixels, vec![[1, 2, 3, 255]]);
    }

    #[test]
    fn decode_palette_png_expands() {
        let plte = [255, 0, 0, 0, 255, 0];
        let trns = [255, 0];
        let png = encode(2, 1, png::ColorType::Indexed, &[0, 1], Some((&plte, &trns)));
        let s = decode_png(&png).unwrap();
        assert_eq!(s.pixels, vec![[255, 0, 0, 255], [0, 255, 0, 0]]);
    }

    #[test]
    fn decode_grayscale_alpha_png() {
        let s = decode_png(&encode(
            1,
            1,
            png::ColorType::GrayscaleAlpha,
            &[100, 200],
            None,
        ))
        .unwrap();
        assert_eq!(s.pixels, vec![[100, 100, 100, 200]]);
    }

    #[test]
    fn decode_garbage_is_none() {
        assert!(decode_png(b"not a png").is_none());
        assert!(decode_png(&[]).is_none());
    }
}
