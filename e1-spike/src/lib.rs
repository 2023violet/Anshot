//! E1 minimal local pixel redaction experiment. Synthetic fixtures ONLY.
use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;

pub const MAX_INPUT_BYTES: usize = 25 * 1024 * 1024;
pub const MAX_PIXELS: u64 = 24_000_000;
pub const COVER: Rgba<u8> = Rgba([17, 24, 39, 255]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}
impl Region {
    pub const fn new(x0: i64, y0: i64, x1: i64, y1: i64) -> Self {
        Self { x0, y0, x1, y1 }
    }
    /// Normalize signed drag direction and clip to half-open source-image pixels.
    pub fn clipped(self, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let bound = |n: i64, max: u32| n.clamp(0, i64::from(max)) as u32;
        let l = bound(self.x0.min(self.x1), width);
        let r = bound(self.x0.max(self.x1), width);
        let t = bound(self.y0.min(self.y1), height);
        let b = bound(self.y0.max(self.y1), height);
        (l < r && t < b).then_some((l, t, r, b))
    }
}

pub fn paint_rgba(img: &mut RgbaImage, regions: &[Region]) {
    let (w, h) = img.dimensions();
    for region in regions {
        if let Some((l, t, r, b)) = region.clipped(w, h) {
            for y in t..b {
                for x in l..r {
                    img.put_pixel(x, y, COVER);
                }
            }
        }
    }
}

fn is_png(input: &[u8]) -> bool {
    input.starts_with(b"\x89PNG\r\n\x1a\n")
}
fn is_jpeg(input: &[u8]) -> bool {
    input.starts_with(&[0xff, 0xd8, 0xff])
}

/// Extract EXIF orientation from JPEG APP1 TIFF IFD0 without copying private metadata.
pub fn jpeg_orientation(input: &[u8]) -> u16 {
    if !is_jpeg(input) { return 1; }
    let mut i = 2usize;
    while i + 4 <= input.len() {
        if input[i] != 0xff { break; }
        let marker = input[i + 1];
        i += 2;
        if marker == 0xd9 || marker == 0xda { break; }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) { continue; }
        let len = match input.get(i..i + 2) {
            Some(a) => u16::from_be_bytes([a[0], a[1]]) as usize,
            None => break,
        };
        if len < 2 || i.checked_add(len).is_none_or(|end| end > input.len()) { break; }
        if marker == 0xe1 {
            let data = &input[i + 2..i + len];
            if data.starts_with(b"Exif\0\0") {
                if let Some(v) = read_exif_orientation(&data[6..]) { return v; }
            }
        }
        i += len;
    }
    1
}

fn read_exif_orientation(t: &[u8]) -> Option<u16> {
    let little = if t.get(0..2)? == b"II" { true } else if t.get(0..2)? == b"MM" { false } else { return None };
    let u16_at = |offset: usize| -> Option<u16> {
        let b = t.get(offset..offset.checked_add(2)?)?;
        Some(if little { u16::from_le_bytes([b[0], b[1]]) }
             else { u16::from_be_bytes([b[0], b[1]]) })
    };
    let u32_at = |offset: usize| -> Option<u32> {
        let b = t.get(offset..offset.checked_add(4)?)?;
        Some(if little { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) }
             else { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) })
    };
    if u16_at(2)? != 42 { return None; }
    let start = usize::try_from(u32_at(4)?).ok()?;
    let count = usize::from(u16_at(start)?);
    if count > 512 { return None; }
    for n in 0..count {
        let pos = start.checked_add(2)?.checked_add(n.checked_mul(12)?)?;
        if u16_at(pos)? != 0x0112 { continue; }
        if u16_at(pos + 2)? != 3 || u32_at(pos + 4)? != 1 { return None; }
        let value = u16_at(pos + 8)?;
        return (1..=8).contains(&value).then_some(value);
    }
    None
}

/// Normalize EXIF transformations to pixel coordinates before any redaction.
pub fn orient_rgba(input: RgbaImage, orientation: u16) -> RgbaImage {
    let (w, h) = input.dimensions();
    if orientation <= 1 || orientation > 8 { return input; }
    let swapped = orientation >= 5;
    let mut out = RgbaImage::new(if swapped { h } else { w }, if swapped { w } else { h });
    for (x, y, px) in input.enumerate_pixels() {
        let (dx, dy) = match orientation {
            2 => (w - 1 - x, y),
            3 => (w - 1 - x, h - 1 - y),
            4 => (x, h - 1 - y),
            5 => (y, x),
            6 => (h - 1 - y, x),
            7 => (h - 1 - y, w - 1 - x),
            8 => (y, w - 1 - x),
            _ => (x, y),
        };
        out.put_pixel(dx, dy, *px);
    }
    out
}

pub fn decode_source(input: &[u8]) -> Result<RgbaImage, String> {
    if input.is_empty() || input.len() > MAX_INPUT_BYTES {
        return Err("invalid or oversized compressed input".into());
    }
    let fmt = if is_png(input) { ImageFormat::Png }
              else if is_jpeg(input) { ImageFormat::Jpeg }
              else { return Err("only PNG and JPEG magic accepted".into()) };
    let probe = image::ImageReader::with_format(Cursor::new(input), fmt);
    let (w, h) = probe.into_dimensions().map_err(|e| format!("invalid image header: {e}"))?;
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err("decoded pixel dimensions out of bounds".into());
    }
    let decoded = image::load_from_memory_with_format(input, fmt)
        .map_err(|e| format!("image decode failed: {e}"))?.to_rgba8();
    let oriented = orient_rgba(decoded, if fmt == ImageFormat::Jpeg {
        jpeg_orientation(input)
    } else { 1 });
    Ok(oriented)
}

pub fn redact(input: &[u8], regions: &[Region]) -> Result<Vec<u8>, String> {
    // The export boundary must never produce a valid-looking PNG of an unmasked
    // source. Reject zero effective coverage even if a caller supplied rectangles
    // that normalize to zero area or lie completely outside the decoded image.
    if regions.is_empty() {
        return Err("no effective redaction region; refusing unmasked PNG export".into());
    }
    let mut img = decode_source(input)?;
    let (width, height) = img.dimensions();
    if !regions.iter().any(|region| region.clipped(width, height).is_some()) {
        return Err("no effective redaction region; refusing unmasked PNG export".into());
    }
    paint_rgba(&mut img, regions);
    let dimensions = img.dimensions();
    let mut out = Cursor::new(Vec::<u8>::new());
    DynamicImage::ImageRgba8(img).write_to(&mut out, ImageFormat::Png)
        .map_err(|e| format!("new PNG encode failed: {e}"))?;
    let bytes = out.into_inner();
    // Fail closed if the newly encoded output cannot be reopened at its source-pixel dimensions.
    let readback = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
        .map_err(|e| format!("new PNG readback failed: {e}"))?;
    if readback.dimensions() != dimensions { return Err("output dimensions changed".into()); }
    Ok(bytes)
}

#[cfg(feature = "web")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "web")]
#[wasm_bindgen]
pub fn redact_web(input: &[u8], rectangles: &[i32]) -> Result<Vec<u8>, JsValue> {
    if rectangles.len() % 4 != 0 { return Err(JsValue::from_str("invalid region shape")); }
    let regions: Vec<Region> = rectangles.chunks_exact(4).map(|s| {
        Region::new(s[0] as i64,s[1] as i64,s[2] as i64,s[3] as i64)
    }).collect();
    redact(input, &regions).map_err(|s| JsValue::from_str(&s))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_direction_one_pixel_and_clamp() {
        let mut im=RgbaImage::from_pixel(4,3,Rgba([1,2,3,255]));
        paint_rgba(&mut im,&[Region::new(3,2,2,1),Region::new(-9,-9,1,1)]);
        assert_eq!(*im.get_pixel(2,1), COVER);
        assert_eq!(*im.get_pixel(0,0), COVER);
        assert_eq!(*im.get_pixel(3,1), Rgba([1,2,3,255]));
        assert_eq!(Region::new(5,5,8,8).clipped(4,3),None);
    }
    #[test]
    fn transposed_orientation_6() {
        let mut im=RgbaImage::new(2,3);
        for y in 0..3 { for x in 0..2 { im.put_pixel(x,y,Rgba([x as u8,y as u8,0,255])); }}
        let out=orient_rgba(im,6);
        assert_eq!(out.dimensions(),(3,2));
        assert_eq!(*out.get_pixel(2,0),Rgba([0,0,0,255]));
        assert_eq!(*out.get_pixel(0,1),Rgba([1,2,0,255]));
    }
    #[test]
    fn malformed_and_wrong_magic_fail_closed() {
        // Exercise the decoder with an effective rectangle; using [] would
        // short-circuit at the fail-closed mask guard, never decoding input.
        let valid_mask = [Region::new(0, 0, 1, 1)];
        for b in [&b"not a real image"[..], &b"\x89PNG\r\n\x1a\n"[..], &b"\xff\xd8\xff"[..]] {
            let err = redact(b, &valid_mask).expect_err("invalid image must fail");
            assert!(!err.contains("effective redaction"), "{err}");
        }
        let err = redact(&vec![0; MAX_INPUT_BYTES + 1], &valid_mask)
            .expect_err("oversized compressed input must fail");
        assert!(err.contains("oversized compressed input"), "{err}");
    }
    #[test]
    fn all_eight_exif_orientations_have_independent_pixel_truth() {
        // Asymmetric 3x2 source exposes every mirror, rotate and transpose.
        let mut src = RgbaImage::new(3, 2);
        for (index, (_, _, pixel)) in src.enumerate_pixels_mut().enumerate() {
            *pixel = Rgba([b'A' + index as u8, 0, 0, 255]);
        }
        let cases: &[(u16, &[&str])] = &[
            (1, &["ABC", "DEF"]),
            (2, &["CBA", "FED"]),
            (3, &["FED", "CBA"]),
            (4, &["DEF", "ABC"]),
            (5, &["AD", "BE", "CF"]),
            (6, &["DA", "EB", "FC"]),
            (7, &["FC", "EB", "DA"]),
            (8, &["CF", "BE", "AD"]),
        ];
        for &(orientation, expected) in cases {
            let out = orient_rgba(src.clone(), orientation);
            assert_eq!(out.height() as usize, expected.len(), "orientation {orientation}");
            assert_eq!(out.width() as usize, expected[0].len(), "orientation {orientation}");
            for (y, line) in expected.iter().enumerate() {
                for (x, pixel) in line.as_bytes().iter().enumerate() {
                    assert_eq!(
                        out.get_pixel(x as u32, y as u32).0[0], *pixel,
                        "orientation={orientation} x={x} y={y}"
                    );
                }
            }
        }
    }
    #[test]
    fn malformed_exif_offsets_and_lengths_are_bounded() {
        // Minimal JPEG APP1 Exif segment with a deliberately invalid TIFF IFD0
        // offset (0xffffffff). The parser must never index or wrap that offset.
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe1, 0, 0];
        jpeg.extend_from_slice(b"Exif\0\0II");
        jpeg.extend_from_slice(&42u16.to_le_bytes());
        jpeg.extend_from_slice(&u32::MAX.to_le_bytes());
        let segment_len = (jpeg.len() - 4) as u16;
        jpeg[4..6].copy_from_slice(&segment_len.to_be_bytes());
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        assert_eq!(jpeg_orientation(&jpeg), 1);
        // Truncation and mutated lengths are intentionally non-crashing;
        // optional EXIF orientation is either valid (1..=8) or defaults to 1.
        for end in 0..=jpeg.len() {
            let result = jpeg_orientation(&jpeg[..end]);
            assert!((1..=8).contains(&result));
        }
        for offset in 4..jpeg.len() - 2 {
            let mut mutated = jpeg.clone();
            mutated[offset] = 0xff;
            let result = jpeg_orientation(&mutated);
            assert!((1..=8).contains(&result));
        }
    }

}
