//! Display preview: decoded source pixels downscaled for the screen. The
//! preview is never exported; delivery always re-runs Core on the source bytes.
#![forbid(unsafe_code)]
use image::{imageops::FilterType, RgbaImage};

/// Upper bound for the on-screen texture (Gate 03: full 24MP uploads ~0.1 s).
pub const MAX_PREVIEW: (u32, u32) = (2560, 1600);

pub fn preview(source: RgbaImage) -> RgbaImage {
    let (w, h) = source.dimensions();
    let k = (f64::from(MAX_PREVIEW.0) / f64::from(w)).min(f64::from(MAX_PREVIEW.1) / f64::from(h));
    if k >= 1.0 { return source; }
    let (pw, ph) = (((f64::from(w) * k).round() as u32).max(1), ((f64::from(h) * k).round() as u32).max(1));
    image::imageops::resize(&source, pw, ph, FilterType::Triangle)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn small_images_pass_through_and_large_ones_fit_the_cap() {
        let small = RgbaImage::new(800, 600);
        assert_eq!(preview(small).dimensions(), (800, 600));
        let big = preview(RgbaImage::new(6000, 4000));
        assert!(big.width() <= MAX_PREVIEW.0 && big.height() <= MAX_PREVIEW.1);
        assert_eq!(big.dimensions(), (2400, 1600));
    }
}
