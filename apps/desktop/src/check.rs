//! The privacy check: reads text from pixels Rust rendered, several times, and applies the
//! format rules. Runs on a worker thread. Recognised text stays in memory and is never logged.
use crate::i18n::t;
use anshot_core::sensitive::{merge_findings, same_text, scan_line, Certainty, Finding, SensitiveKind, TextWord};
use anshot_core::Region;
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
/// Word boxes run tight: 4 px around them covered the glyphs in 97.7 % of the measured cases.
const BOX_PAD: f32 = 4.0;

/// One suggestion, in source-image pixels. `text` may contain misreads.
#[derive(Clone, Debug, PartialEq)]
pub struct Item { pub kind: SensitiveKind, pub certainty: Certainty, pub text: String, pub region: Region, pub on: bool }
#[derive(Clone, Debug, PartialEq)]
pub struct Review {
    pub document: u64,
    pub items: Vec<Item>,
    /// Same-text suggestions copy this redaction's look.
    pub like: Option<u64>,
}

/// Findings on the exported image, as suggestions on the source image.
pub fn to_items(findings: Vec<Finding>, crop: Option<Region>) -> Vec<Item> {
    let (cx, cy) = crop.map_or((0, 0), |c| (c.x0.min(c.x1), c.y0.min(c.y1)));
    findings.into_iter().map(|f| {
        let (x0, y0, x1, y1) = f.bounds;
        let region = Region::new(
            (x0 - BOX_PAD).floor() as i64 + cx, (y0 - BOX_PAD).floor() as i64 + cy,
            (x1 + BOX_PAD).ceil() as i64 + cx, (y1 + BOX_PAD).ceil() as i64 + cy);
        Item { kind: f.kind, certainty: f.certainty, text: f.text, region, on: true }
    }).collect()
}
/// Drop suggestions the user has already seen and left alone (same place on the image).
pub fn without_dismissed(items: Vec<Item>, dismissed: &[Region]) -> Vec<Item> {
    let hits = |a: &Region, b: &Region| a.x0 < b.x1 && a.x1 > b.x0 && a.y0 < b.y1 && a.y1 > b.y0;
    items.into_iter().filter(|i| !dismissed.iter().any(|d| hits(&i.region, d))).collect()
}
/// Enough of the text to find it on the image, not enough to read it over a shoulder.
pub fn hint(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    if c.len() <= 6 { return format!("{}•••", c.iter().take(1).collect::<String>()); }
    format!("{}•••{}", c[..3].iter().collect::<String>(), c[c.len() - 2..].iter().collect::<String>())
}
pub fn label(item: &Item) -> &'static str {
    let possible = item.certainty == Certainty::Possible;
    match (item.kind, possible) {
        (SensitiveKind::Phone, false) => t("手机号"),
        (SensitiveKind::Phone, true) => t("疑似手机号"),
        (SensitiveKind::IdCard, false) => t("身份证号"),
        (SensitiveKind::IdCard, true) => t("疑似身份证号"),
        (SensitiveKind::BankCard, false) => t("银行卡号"),
        (SensitiveKind::BankCard, true) => t("疑似银行卡号"),
        (SensitiveKind::Email, false) => t("邮箱"),
        (SensitiveKind::Email, true) => t("疑似邮箱"),
        (SensitiveKind::Ipv4, false) => t("IP 地址"),
        (SensitiveKind::Ipv4, true) => t("疑似 IP 地址"),
        (SensitiveKind::ApiKey, false) => t("密钥"),
        (SensitiveKind::ApiKey, true) => t("疑似密钥"),
        (SensitiveKind::SameText, _) => t("相同内容"),
    }
}

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

/// The scales worth reading an image of this size at.
fn scales(w: u32, h: u32) -> impl Iterator<Item = f32> { SCALES.into_iter().filter(move |&f| f == 1.0 || can_upscale(w, h, f)) }

/// Other places in `img` with the text inside `region`; `None` when none could be read there.
/// `img` holds the uncovered pixels, so this stays on a worker and in memory.
pub fn same_as(img: &RgbaImage, region: Region) -> Result<Option<Vec<Finding>>, OcrError> {
    let (w, h) = img.dimensions();
    let area = (region.x0.min(region.x1) as f32, region.y0.min(region.y1) as f32, region.x0.max(region.x1) as f32, region.y0.max(region.y1) as f32);
    let (mut found, mut any) = (Vec::new(), false);
    for f in scales(w, h) {
        if let Some(more) = same_text(&read_at(img, f)?, area) { any = true; found.extend(more); }
    }
    Ok(any.then(|| merge_findings(found)))
}

/// Everything that looks sensitive in `img`. One failed pass does not lose the others, but if
/// every pass fails, so does the check: "nothing found" must mean the image was read.
pub fn scan(img: &RgbaImage) -> Result<Vec<Finding>, OcrError> {
    let (w, h) = img.dimensions();
    let (mut found, mut failure, mut read) = (Vec::new(), None, false);
    for f in scales(w, h) {
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
    fn items_are_padded_and_return_to_source_pixels() {
        let f = Finding { kind: SensitiveKind::Phone, certainty: Certainty::Confirmed, text: "13800138000".into(), bounds: (10.2, 20.5, 100.1, 40.0) };
        let items = to_items(vec![f], Some(Region::new(300, 50, 200, 500)));
        assert_eq!(items[0].region, Region::new(206, 66, 305, 94));
        assert!(items[0].on, "suggestions start ticked");
    }
    #[test]
    fn dismissed_places_are_not_suggested_again() {
        let item = |x0| Item { kind: SensitiveKind::Ipv4, certainty: Certainty::Possible, text: String::new(), region: Region::new(x0, 0, x0 + 10, 10), on: true };
        let left = without_dismissed(vec![item(0), item(100)], &[Region::new(2, 2, 8, 8)]);
        assert_eq!(left, [item(100)]);
    }
    #[test]
    fn hints_never_show_the_middle() {
        assert_eq!(hint("13800138000"), "138•••00");
        assert_eq!(hint("短文字"), "短•••");
        assert_eq!(hint(""), "•••");
    }
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
