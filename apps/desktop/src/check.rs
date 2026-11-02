//! The privacy check: reads text from pixels Rust rendered, several times, and applies the
//! format rules. Runs on a worker thread. Recognised text stays in memory and is never logged.
use crate::i18n::t;
use anshot_core::sensitive::{merge_findings, scan_line, Finding, TextWord};
use anshot_platform_windows::{ocr_recognize, OcrError, OcrLine};
use image::{imageops, RgbaImage};

/// OCR fails on different values at different sizes, so the union finds more than any one pass
/// (`spike/ocr/RESULT.md`: 90 % at these three, 78 % at 1.5 alone).
const SCALES: [f32; 3] = [1.0, 1.5, 2.0];
/// Above this many source pixels, enlarging costs more than it gives.
const UPSCALE_MAX_PIXELS: u64 = 4_000_000;
const UPSCALE_MAX_SIDE: f32 = 9_000.0;
/// Tall images are read in overlapping pieces: long screenshots pass the engine's size limit,
/// and the overlap keeps a line on a seam whole in one of the two pieces.
const TILE_HEIGHT: u32 = 4_000;
const TILE_OVERLAP: u32 = 200;

pub fn can_upscale(w: u32, h: u32, f: f32) -> bool {
    u64::from(w) * u64::from(h) <= UPSCALE_MAX_PIXELS && w.max(h) as f32 * f <= UPSCALE_MAX_SIDE
}
pub fn upscaled(img: &RgbaImage, f: f32) -> RgbaImage {
    if f == 1.0 { return img.clone(); }
    let (w, h) = img.dimensions();
    imageops::resize(img, (w as f32 * f) as u32, (h as f32 * f) as u32, imageops::FilterType::Triangle)
}
pub fn ocr_message(e: &OcrError) -> &'static str {
    match e {
        OcrError::LanguageMissing => t("这台电脑没有可用的文字识别语言。请在 Windows 设置 → 时间和语言 → 语言 中添加“中文(简体)”，安装完成后再试。"),
        OcrError::TooLarge => t("区域太大，无法识别；请框选小一些的区域。"),
        OcrError::Failed(_) => t("识别失败，请重试。"),
    }
}

/// Top and height of each piece of an image `h` pixels tall.
fn tiles(h: u32) -> Vec<(u32, u32)> {
    if h <= TILE_HEIGHT { return vec![(0, h)]; }
    let mut out = Vec::new();
    let mut y = 0;
    loop {
        let piece = TILE_HEIGHT.min(h - y);
        out.push((y, piece));
        if y + piece >= h { return out; }
        y += TILE_HEIGHT - TILE_OVERLAP;
    }
}

fn to_words(line: OcrLine, f: f32, top: f32) -> Vec<TextWord> {
    line.words.into_iter().map(|w| TextWord { text: w.text, x: w.x / f, y: w.y / f + top, w: w.w / f, h: w.h / f }).collect()
}

/// Recognised lines of `img` at scale `f`, boxes in `img`'s pixels.
fn read_at(img: &RgbaImage, f: f32) -> Result<Vec<Vec<TextWord>>, OcrError> {
    let (w, h) = img.dimensions();
    let mut lines = Vec::new();
    for (top, piece) in tiles(h) {
        let tile = imageops::crop_imm(img, 0, top, w, piece).to_image();
        let tile = upscaled(&tile, f);
        let (tw, th) = tile.dimensions();
        lines.extend(ocr_recognize(tile.as_raw(), tw, th)?.into_iter().map(|l| to_words(l, f, top as f32)));
    }
    Ok(lines)
}

/// Everything that looks sensitive in `img`. One failed pass does not lose the others, but if
/// every pass fails, so does the check: "nothing found" must mean the image was read.
pub fn scan(img: &RgbaImage) -> Result<Vec<Finding>, OcrError> {
    let (w, h) = img.dimensions();
    let (mut found, mut failure, mut read) = (Vec::new(), None, false);
    for f in SCALES.into_iter().filter(|&f| f == 1.0 || can_upscale(w, h, f)) {
        match read_at(img, f) {
            Ok(lines) => { read = true; found.extend(lines.iter().flat_map(|l| scan_line(l))); }
            Err(e) => failure = Some(e),
        }
    }
    match (read, failure) {
        (false, Some(e)) => Err(e),
        _ => Ok(merge_findings(found)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_images_are_one_piece() {
        assert_eq!(tiles(1080), [(0, 1080)]);
        assert_eq!(tiles(TILE_HEIGHT), [(0, TILE_HEIGHT)]);
    }
    #[test]
    fn tall_images_overlap_and_reach_the_bottom() {
        for h in [TILE_HEIGHT + 1, 7_800, 7_801, 20_000] {
            let t = tiles(h);
            let last = t.last().unwrap();
            assert_eq!(last.0 + last.1, h, "{h}");
            for pair in t.windows(2) { assert!(pair[1].0 + TILE_OVERLAP <= pair[0].0 + pair[0].1, "{h}"); }
        }
    }
    #[test]
    fn boxes_return_to_image_pixels() {
        use anshot_platform_windows::OcrWord;
        let line = OcrLine { text: "a".into(), x: 30.0, y: 60.0, h: 20.0, words: vec![OcrWord { text: "a".into(), x: 30.0, y: 60.0, w: 15.0, h: 20.0 }] };
        assert_eq!(to_words(line, 2.0, 3800.0), [TextWord { text: "a".into(), x: 15.0, y: 3830.0, w: 7.5, h: 10.0 }]);
    }
    #[test]
    fn large_images_are_not_enlarged() {
        assert!(can_upscale(1920, 1080, 2.0));
        assert!(!can_upscale(6000, 4000, 1.5));
        assert!(!can_upscale(1000, 3000, 4.0));
    }
}
