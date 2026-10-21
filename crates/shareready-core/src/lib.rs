//! ShareReady headless Rust image core. ABI/presentation live in the adapters.
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;

pub const MAX_INPUT_BYTES: usize = 25 * 1024 * 1024;
pub const MAX_PIXELS: u64 = 24_000_000;
// Locked RGBA8/Fast/no-metadata writer: filtered bytes <=5*MAX_PIXELS,
// plus stored-deflate/PNG overhead <=120009223B. Recheck on encoder changes.
pub const MAX_GENERATED_PNG_BYTES: usize = 128 * 1024 * 1024;
pub const COVER: Rgba<u8> = Rgba([17, 24, 39, 255]);

mod clipboard;
pub use clipboard::{dib_to_png,MAX_DIB_BYTES};

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
    if fmt == ImageFormat::Jpeg && !jpeg_reaches_eoi(input) {
        return Err("invalid image header: incomplete or malformed JPEG marker structure".into());
    }
    // M4 finding: the pinned zune-jpeg 0.4.21 (via image =0.25.8) panics on some
    // malformed JPEGs, e.g. a slice index past a 16-entry buffer in worker.rs. A
    // hostile file must fail closed, not unwind through the caller. Only the
    // decoder runs inside; it shares no state with the caller beyond `input`.
    std::panic::catch_unwind(|| decode_untrusted(input, fmt))
        .unwrap_or_else(|_| Err("image decode failed: decoder panicked on malformed data".into()))
}

/// Walk JPEG markers strictly from SOI and require an EOI after the last scan.
/// M4 finding: zune-jpeg 0.4.21's progressive decoder (non-strict, as image uses
/// it) loops forever when data runs out before EOI, re-handling the pending
/// marker. Rejecting streams that end before EOI keeps a hostile or truncated
/// file from hanging intake. Bytes after EOI are ignored, as before.
fn jpeg_reaches_eoi(input: &[u8]) -> bool {
    let mut i = 2usize;
    loop {
        if input.get(i) != Some(&0xff) { return false; }
        while input.get(i) == Some(&0xff) { i += 1; }
        let Some(&marker) = input.get(i) else { return false; };
        i += 1;
        match marker {
            0xd9 => return true,
            0x00 | 0xd8 => return false,
            0x01 | 0xd0..=0xd7 => continue,
            _ => {}
        }
        let len = match input.get(i..i + 2) {
            Some(a) => usize::from(u16::from_be_bytes([a[0], a[1]])),
            None => return false,
        };
        if len < 2 || i + len > input.len() { return false; }
        i += len;
        if marker != 0xda { continue; }
        // Entropy-coded data runs to the next marker other than a stuffed
        // 0xff00 or an in-scan restart marker; fill bytes 0xff may precede it.
        loop {
            let Some(at) = input[i..].iter().position(|&b| b == 0xff) else { return false; };
            let mut next = i + at + 1;
            while input.get(next) == Some(&0xff) { next += 1; }
            match input.get(next) {
                None => return false,
                Some(0x00 | 0xd0..=0xd7) => i = next + 1,
                Some(_) => { i += at; break; }
            }
        }
    }
}

fn decode_untrusted(input: &[u8], fmt: ImageFormat) -> Result<RgbaImage, String> {
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

// Validate only the current writer's IHDR/IDAT/IEND stream, not general PNG
// input. The image decoder stops at image data and need not consume its tail.
fn generated_chunk_stream(input:&[u8])->bool{
    const CRC_TABLE:[u32;256]={
        let mut table=[0;256];let mut n=0;
        while n<256{let mut value=n as u32;let mut bit=0;
            while bit<8{value=if value&1!=0{(value>>1)^0xedb8_8320}else{value>>1};bit+=1;}
            table[n]=value;n+=1;
        }
        table
    };
    let mut at=8usize;let mut image_data=false;
    while at.checked_add(12).is_some_and(|end|end<=input.len()){
        let length=u32::from_be_bytes(input[at..at+4].try_into().unwrap()) as usize;
        let Some(end)=at.checked_add(length).and_then(|n|n.checked_add(12)).filter(|&n|n<=input.len()) else{return false;};
        let kind=&input[at+4..at+8];
        let mut crc=u32::MAX;
        for &byte in &input[at+4..end-4]{crc=CRC_TABLE[((crc^u32::from(byte))&255) as usize]^(crc>>8);}
        if !crc!=u32::from_be_bytes(input[end-4..end].try_into().unwrap()){return false;}
        match kind{
            b"IHDR" if at==8 && length==13=>{},
            b"IDAT" if at>8=>image_data=true,
            b"IEND" if length==0 && image_data=>return end==input.len(),
            _=>return false,
        }
        at=end;
    }
    false
}

/// Read back PNG bytes owned by a current Rust-generated artifact, never source intake.
/// The caller must establish ownership/version; this decoder cannot prove provenance
/// or redaction coverage. User input must continue through `decode_source`.
pub fn decode_generated_png(input:&[u8],expected:(u32,u32))->Result<RgbaImage,String>{
    let invalid=||"invalid generated PNG or artifact dimensions".to_string();
    if input.len()<45 || input.len()>MAX_GENERATED_PNG_BYTES || !is_png(input) ||
        expected.0==0 || expected.1==0 || u64::from(expected.0)*u64::from(expected.1)>MAX_PIXELS{
        return Err(invalid());
    }
    // Current writer emits one noninterlaced RGBA8 image. Require its fixed
    // header shape before a decoder allocates image storage, and a complete end.
    if input[8..16]!=[0,0,0,13,b'I',b'H',b'D',b'R'] || input[24..29]!=[8,6,0,0,0] ||
        input[input.len()-12..]!=[0,0,0,0,b'I',b'E',b'N',b'D',174,66,96,130]{return Err(invalid());}
    let dimensions=(u32::from_be_bytes(input[16..20].try_into().unwrap()),
        u32::from_be_bytes(input[20..24].try_into().unwrap()));
    if dimensions!=expected || !generated_chunk_stream(input){return Err(invalid());}
    let mut limits=image::Limits::default();
    limits.max_image_width=Some(expected.0);limits.max_image_height=Some(expected.1);
    // Decoder allocation accounting is best-effort, not an OS memory ceiling.
    limits.max_alloc=Some(512*1024*1024);
    let decoder=image::codecs::png::PngDecoder::with_limits(Cursor::new(input),limits)
        .map_err(|e|format!("generated PNG header failed: {e}"))?;
    if decoder.dimensions()!=expected || decoder.color_type()!=image::ColorType::Rgba8 ||
        decoder.is_apng().map_err(|e|format!("generated PNG animation check failed: {e}"))?{return Err(invalid());}
    let decoded=DynamicImage::from_decoder(decoder)
        .map_err(|e|format!("generated PNG readback failed: {e}"))?.into_rgba8();
    if decoded.dimensions()!=expected{return Err(invalid());}
    Ok(decoded)
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
    let _ = decode_generated_png(&bytes,dimensions)?;
    Ok(bytes)
}

/// Decode and orient a user-selected JPEG/PNG *before* constructing the edit
/// canvas. The preview is a fresh PNG containing normalized source pixels,
/// rather than a browser EXIF-dependent view of the original file. The same
/// decoded dimensions and orientation are used again at the export boundary.
pub fn preview_png(input: &[u8]) -> Result<Vec<u8>, String> {
    let image = decode_source(input)?;
    let size = image.dimensions();
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image).write_to(&mut out, ImageFormat::Png)
        .map_err(|e| format!("preview PNG encode failed: {e}"))?;
    let png = out.into_inner();
    let decoded = image::load_from_memory_with_format(&png, ImageFormat::Png)
        .map_err(|e| format!("preview PNG verification failed: {e}"))?;
    if decoded.dimensions() != size {
        return Err("preview dimensions changed".into());
    }
    Ok(png)
}


#[cfg(test)]
mod tests {
    use super::*;
    fn encoded(image:DynamicImage,format:ImageFormat)->Vec<u8>{
        let mut out=Cursor::new(Vec::new());image.write_to(&mut out,format).unwrap();out.into_inner()
    }
    #[test] fn generated_readback_rejects_a_real_rgba8_animation(){
        let apng=include_bytes!("../../../tests/performance/fixtures/synthetic-two-frame.png");
        assert!(decode_source(apng).is_ok(),"source decoder can read the default frame");
        assert!(decode_generated_png(apng,(2,3)).is_err(),"a generated artifact cannot contain animation");
    }
    #[test] fn generated_stream_accepts_split_image_data_but_rejects_metadata(){
        let split=include_bytes!("../../../tests/performance/fixtures/synthetic-split-idat.png");
        assert_eq!(decode_generated_png(split,(2,3)).unwrap(),RgbaImage::from_pixel(2,3,Rgba([1,2,3,255])));
        let metadata=include_bytes!("../../../tests/performance/fixtures/synthetic-metadata.png");
        assert!(decode_source(metadata).is_ok());
        assert!(decode_generated_png(metadata,(2,3)).is_err());
    }
    #[test] fn generated_readback_rejects_payload_after_first_end(){
        let png=encoded(DynamicImage::ImageRgba8(RgbaImage::from_pixel(2,3,Rgba([1,2,3,255]))),ImageFormat::Png);
        let mut accepted=Vec::new();
        for junk in [&b""[..],&b"unexpected trailing payload"[..]]{
            let mut extra=png.clone();extra.extend_from_slice(junk);extra.extend_from_slice(&png[png.len()-12..]);
            assert!(decode_source(&extra).is_ok(),"default image decoder ignores trailing bytes");
            if decode_generated_png(&extra,(2,3)).is_ok(){accepted.push(junk.len());}
        }
        assert!(accepted.is_empty(),"generated decoder accepted tail lengths {accepted:?}");
    }
    #[test] fn generated_readback_requires_expected_rgba8_png_and_preserves_pixels(){
        let original=RgbaImage::from_pixel(2,3,Rgba([123,7,91,0]));
        let png=encoded(DynamicImage::ImageRgba8(original.clone()),ImageFormat::Png);
        assert_eq!(decode_generated_png(&png,(2,3)).unwrap(),original);
        assert!(decode_generated_png(&png,(3,2)).is_err(),"must bind dimensions to current Editor");
        assert!(decode_generated_png(&png,(0,3)).is_err());
        assert!(decode_generated_png(&png,(24_000_001,1)).is_err());
        let rgb=image::RgbImage::from_pixel(2,3,image::Rgb([12,13,14]));
        assert!(decode_generated_png(&encoded(DynamicImage::ImageRgb8(rgb.clone()),ImageFormat::Png),(2,3)).is_err(),"generated encoder contract is RGBA8");
        assert!(decode_generated_png(&encoded(DynamicImage::ImageRgb8(rgb),ImageFormat::Jpeg),(2,3)).is_err(),"no JPEG fallback for artifacts");
        let wide=image::ImageBuffer::from_pixel(2,3,Rgba([1u16,2,3,u16::MAX]));
        assert!(decode_generated_png(&encoded(DynamicImage::ImageRgba16(wide),ImageFormat::Png),(2,3)).is_err());
    }
    #[test] fn generated_readback_rejects_truncated_corrupt_and_oversized_data(){
        let png=encoded(DynamicImage::ImageRgba8(RgbaImage::from_pixel(2,3,Rgba([1,2,3,255]))),ImageFormat::Png);
        for bytes in [&b""[..],&b"\x89PNG\r\n\x1a\n"[..],&png[..png.len()/2],&png[..png.len()-1]]{
            assert!(decode_generated_png(bytes,(2,3)).is_err());
        }
        let mut corrupt=png.clone();corrupt[44]^=1;
        assert!(decode_generated_png(&corrupt,(2,3)).is_err());
        let mut huge_header=png.clone();huge_header[16..20].copy_from_slice(&u32::MAX.to_be_bytes());
        huge_header[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(decode_generated_png(&huge_header,(2,3)).is_err());
        let mut oversized=vec![0;MAX_GENERATED_PNG_BYTES+1];oversized[..png.len()].copy_from_slice(&png);
        assert!(decode_generated_png(&oversized,(2,3)).is_err());
        assert!(decode_source(&oversized[..MAX_INPUT_BYTES+1]).is_err(),"untrusted source ceiling stays 25MiB");
    }
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
