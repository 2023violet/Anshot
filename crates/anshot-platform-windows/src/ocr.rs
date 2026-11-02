//! Text recognition with the OCR engine that ships with Windows (Windows.Media.Ocr).
//! Nothing is downloaded or sent anywhere; the input is pixels Anshot already rendered.
//! Feasibility and accuracy were measured in `spike/ocr/RESULT.md`.
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OcrError {
    /// No OCR language is installed for the user's languages (or for Chinese).
    LanguageMissing,
    /// The image is too large or empty for the engine.
    TooLarge,
    /// The engine failed; the detail is for logs, never shown raw.
    Failed(String),
}

/// One recognised word and its box in the input image's pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct OcrWord { pub text: String, pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

/// One recognised line and its box in the input image's pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct OcrLine { pub text: String, pub x: f32, pub y: f32, pub h: f32, pub words: Vec<OcrWord> }

fn engine() -> Result<OcrEngine, OcrError> {
    // Prefer Chinese (it also reads Latin text well), then whatever the profile offers.
    let zh = Language::CreateLanguage(&HSTRING::from("zh-Hans")).ok();
    if let Some(lang) = zh.filter(|l| OcrEngine::IsLanguageSupported(l).unwrap_or(false)) {
        if let Ok(e) = OcrEngine::TryCreateFromLanguage(&lang) { return Ok(e); }
    }
    // A missing language comes back as Err (null with S_OK), not as a usable engine.
    OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| OcrError::LanguageMissing)
}

/// Whether any OCR language is available to this user.
pub fn available() -> bool {
    // SAFETY: initialises WinRT for this thread; repeated calls are harmless.
    let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    engine().is_ok()
}

/// Recognise straight RGBA pixels. Call from a worker thread (it blocks).
/// Lines come back sorted top to bottom, then left to right.
pub fn recognize(rgba: &[u8], w: u32, h: u32) -> Result<Vec<OcrLine>, OcrError> {
    if w == 0 || h == 0 || rgba.len() != w as usize * h as usize * 4 { return Err(OcrError::TooLarge); }
    // SAFETY: initialises WinRT for this worker thread; repeated calls are harmless.
    let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    let max = OcrEngine::MaxImageDimension().unwrap_or(10_000);
    if w > max || h > max { return Err(OcrError::TooLarge); }
    let engine = engine()?;
    let fail = |e: windows::core::Error| OcrError::Failed(e.message());
    // The engine takes BGRA.
    let mut bgra = rgba.to_vec();
    for px in bgra.chunks_exact_mut(4) { px.swap(0, 2); }
    let writer = DataWriter::new().map_err(fail)?;
    writer.WriteBytes(&bgra).map_err(fail)?;
    let buffer = writer.DetachBuffer().map_err(fail)?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, w as i32, h as i32).map_err(fail)?;
    let result = engine.RecognizeAsync(&bitmap).map_err(fail)?.join().map_err(fail)?;
    let mut lines = Vec::new();
    for line in result.Lines().map_err(fail)? {
        // A line's box is the union of its words' boxes; its text is rebuilt from the words
        // (the engine treats every CJK character as a word and puts spaces between them).
        let (mut x0, mut y0, mut y1) = (f32::MAX, f32::MAX, f32::MIN);
        let mut words = Vec::new();
        for word in line.Words().map_err(fail)? {
            let r = word.BoundingRect().map_err(fail)?;
            x0 = x0.min(r.X); y0 = y0.min(r.Y); y1 = y1.max(r.Y + r.Height);
            words.push(OcrWord { text: word.Text().map_err(fail)?.to_string(), x: r.X, y: r.Y, w: r.Width, h: r.Height });
        }
        let joined: Vec<String> = words.iter().map(|w| w.text.clone()).collect();
        let text = if joined.is_empty() { line.Text().map_err(fail)?.to_string() } else { join_words(&joined) };
        if y0 == f32::MAX { (x0, y0, y1) = (0.0, 0.0, 0.0); }
        lines.push(OcrLine { text, x: x0, y: y0, h: y1 - y0, words });
    }
    Ok(sort_lines(lines))
}

/// Join recognised words: a space only between two words that both touch it with a Latin
/// letter or digit ("Ctrl + C", "138 0013"), none around CJK characters or punctuation.
pub fn join_words(words: &[String]) -> String {
    let latin = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    let mut out = String::new();
    for w in words {
        if latin(out.chars().last()) && latin(w.chars().next()) { out.push(' '); }
        out.push_str(w);
    }
    out
}

/// Reading order: rows top to bottom (lines whose vertical centres are within half a line
/// height share a row), left to right inside a row. The engine's own order can jump.
pub fn sort_lines(mut lines: Vec<OcrLine>) -> Vec<OcrLine> {
    lines.sort_by(|a, b| (a.y + a.h / 2.0).total_cmp(&(b.y + b.h / 2.0)));
    let mut rows: Vec<Vec<OcrLine>> = Vec::new();
    for l in lines {
        let c = l.y + l.h / 2.0;
        match rows.last_mut() {
            Some(row) if row.iter().any(|r| (r.y + r.h / 2.0 - c).abs() <= r.h.max(l.h) / 2.0) => row.push(l),
            _ => rows.push(vec![l]),
        }
    }
    rows.into_iter().flat_map(|mut r| { r.sort_by(|a, b| a.x.total_cmp(&b.x)); r }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(t: &str, x: f32, y: f32) -> OcrLine { OcrLine { text: t.into(), x, y, h: 20.0, words: Vec::new() } }
    #[test]
    fn cjk_words_join_without_spaces_latin_words_with_one() {
        let w = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(join_words(&w(&["您", "好", "，", "请", "提", "供"])), "您好，请提供");
        assert_eq!(join_words(&w(&["手", "机", "138", "0013", "8000"])), "手机138 0013 8000");
        assert_eq!(join_words(&w(&["Ctrl", "+", "C", "复", "制"])), "Ctrl+C复制");
    }

    #[test]
    fn lines_read_top_down_then_left_right() {
        let got = sort_lines(vec![l("right bubble", 300.0, 132.0), l("header", 54.0, 13.0), l("left", 16.0, 140.0), l("bottom", 16.0, 300.0)]);
        let order: Vec<_> = got.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(order, ["header", "left", "right bubble", "bottom"]);
    }
}
