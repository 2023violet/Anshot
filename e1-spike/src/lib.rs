//! E1 minimal local pixel redaction experiment. Synthetic fixtures ONLY.
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
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
    let little = match t.get(0..2)? { b"II" => true, b"MM" => false, _ => return None };
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
    let mut img = decode_source(input)?;
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
        for b in [&b"not a real image"[..], &b"\x89PNG\r\n\x1a\n"[..],&b"\xff\xd8\xff"[..]] {
            assert!(redact(b, &[]).is_err());
        }
        assert!(redact(&vec![0;MAX_INPUT_BYTES+1], &[]).is_err());
    }
}
