//! Feasibility spike: offline OCR through Windows.Media.Ocr from an unpackaged exe.
//!
//! Pixels arrive as raw BGRA (`.bgra`: u32 LE width, u32 LE height, then w*h*4 bytes), the
//! same shape the product would hand over after decoding in Rust; no image decoder here.
//!
//! Usage:
//!   ocr-spike langs
//!   ocr-spike recog <lang|user> <file.bgra>...       one line per file: path<TAB>text
//!   ocr-spike bench <lang|user> <file.bgra> <warm-runs>
//!
//! OCR_UPSCALE=<factor> (e.g. 2) bilinearly enlarges the pixels in Rust before upload.

use std::time::Instant;

use windows::core::{Result, HSTRING};
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

fn engine(lang: &str) -> Result<OcrEngine> {
    if lang == "user" {
        OcrEngine::TryCreateFromUserProfileLanguages()
    } else {
        OcrEngine::TryCreateFromLanguage(&Language::CreateLanguage(&HSTRING::from(lang))?)
    }
}

fn load_bitmap(path: &str) -> Result<SoftwareBitmap> {
    let raw = std::fs::read(path).expect("read fixture");
    let w = u32::from_le_bytes(raw[0..4].try_into().unwrap());
    let h = u32::from_le_bytes(raw[4..8].try_into().unwrap());
    let mut px = raw[8..].to_vec();
    assert_eq!(px.len(), (w * h * 4) as usize, "bad fixture size: {path}");
    let (mut w, mut h) = (w, h);
    if let Some(f) = std::env::var("OCR_UPSCALE").ok().and_then(|v| v.parse::<f32>().ok()) {
        (px, w, h) = upscale(&px, w, h, f);
    }
    let writer = DataWriter::new()?;
    writer.WriteBytes(&px)?;
    let buffer = writer.DetachBuffer()?;
    SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, w as i32, h as i32)
}

/// Bilinear resize of BGRA pixels by `f` (pixel centres aligned).
fn upscale(px: &[u8], w: u32, h: u32, f: f32) -> (Vec<u8>, u32, u32) {
    let (nw, nh) = ((w as f32 * f).round() as u32, (h as f32 * f).round() as u32);
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    let at = |x: u32, y: u32, c: usize| px[((y * w + x) * 4) as usize + c] as f32;
    for y in 0..nh {
        let sy = ((y as f32 + 0.5) / f - 0.5).clamp(0.0, (h - 1) as f32);
        let (y0, fy) = (sy.floor() as u32, sy.fract());
        let y1 = (y0 + 1).min(h - 1);
        for x in 0..nw {
            let sx = ((x as f32 + 0.5) / f - 0.5).clamp(0.0, (w - 1) as f32);
            let (x0, fx) = (sx.floor() as u32, sx.fract());
            let x1 = (x0 + 1).min(w - 1);
            for c in 0..4 {
                let top = at(x0, y0, c) * (1.0 - fx) + at(x1, y0, c) * fx;
                let bot = at(x0, y1, c) * (1.0 - fx) + at(x1, y1, c) * fx;
                out[((y * nw + x) * 4) as usize + c] = (top * (1.0 - fy) + bot * fy).round() as u8;
            }
        }
    }
    (out, nw, nh)
}

fn recognize(engine: &OcrEngine, bitmap: &SoftwareBitmap) -> Result<String> {
    let result = engine.RecognizeAsync(bitmap)?.join()?;
    let mut lines = Vec::new();
    for line in result.Lines()? {
        lines.push(line.Text()?.to_string());
    }
    Ok(lines.join(" \u{2424} "))
}

fn langs() -> Result<()> {
    println!("MaxImageDimension\t{}", OcrEngine::MaxImageDimension()?);
    for l in OcrEngine::AvailableRecognizerLanguages()? {
        println!("available\t{}\t{}", l.LanguageTag()?, l.DisplayName()?);
    }
    for tag in ["user", "zh-Hans", "zh-Hans-CN", "en-US"] {
        if tag != "user" {
            let lang = Language::CreateLanguage(&HSTRING::from(tag))?;
            println!("IsLanguageSupported\t{tag}\t{}", OcrEngine::IsLanguageSupported(&lang)?);
        }
        match engine(tag) {
            Ok(e) => println!("create\t{tag}\tOK\t{}", e.RecognizerLanguage()?.LanguageTag()?),
            Err(err) => println!("create\t{tag}\tFAILED\t{err:?}"),
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let t0 = Instant::now();
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("langs") => langs(),
        Some("recog") if args.len() >= 3 => {
            let e = engine(&args[1])?;
            for path in &args[2..] {
                println!("{path}\t{}", recognize(&e, &load_bitmap(path)?)?);
            }
            Ok(())
        }
        Some("bench") if args.len() == 4 => {
            let runs: usize = args[3].parse().expect("warm-runs");
            // Cold: engine creation + bitmap upload + first recognition in a fresh process.
            let t = Instant::now();
            let e = engine(&args[1])?;
            let bmp = load_bitmap(&args[2])?;
            let first = recognize(&e, &bmp)?;
            let cold = t.elapsed();
            println!("cold_ms\t{:.2}\tsince_process_start_ms\t{:.2}", ms(cold), ms(t0.elapsed()));
            println!("chars\t{}", first.chars().filter(|c| !c.is_whitespace()).count());
            // Warm: engine reused; each run includes the bitmap upload and recognition.
            for _ in 0..runs {
                let t = Instant::now();
                let bmp = load_bitmap(&args[2])?;
                recognize(&e, &bmp)?;
                println!("warm_ms\t{:.2}", ms(t.elapsed()));
            }
            Ok(())
        }
        _ => {
            eprintln!("usage: ocr-spike langs | recog <lang|user> <file.bgra>... | bench <lang|user> <file.bgra> <runs>");
            std::process::exit(2);
        }
    }
}

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}
