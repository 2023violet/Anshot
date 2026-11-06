//! Step image (ADR 0.6-STEPS): several finished exports laid out as one new image, each
//! under a heading row with a number badge and a one-line caption.
//!
//! The pieces are already rendered (redactions and annotations are in their pixels), so
//! nothing here reads an original capture. Badges and captions sit above each piece and
//! never cover it: an unscaled piece is copied pixel for pixel. The result is opaque and
//! stays within `MAX_PIXELS`; when it would not, everything is scaled down together, and
//! below `MIN_SCALE` the layout is refused instead.

use crate::{fit_text, glyphs, paint, text_extent};
use anshot_core::MAX_PIXELS;
use image::{imageops, RgbaImage};
use tiny_skia::{FillRule, PathBuilder, Pixmap};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Arrangement { #[default] Vertical, Horizontal, Grid }

impl Arrangement {
    pub const ALL: [Arrangement; 3] = [Arrangement::Vertical, Arrangement::Horizontal, Arrangement::Grid];
}

/// Margin and gap between steps (04_DESIGN v1.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gap { Small, #[default] Medium, Large }

impl Gap {
    pub const ALL: [Gap; 3] = [Gap::Small, Gap::Medium, Gap::Large];
    fn px(self, reference: f32) -> f32 {
        let (share, least) = match self { Gap::Small => (0.02, 8.0), Gap::Medium => (0.04, 16.0), Gap::Large => (0.06, 24.0) };
        (reference * share).max(least).round()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backdrop { #[default] White, Light, Dark }

impl Backdrop {
    pub const ALL: [Backdrop; 3] = [Backdrop::White, Backdrop::Light, Backdrop::Dark];
    pub fn color(self) -> [u8; 4] {
        match self { Backdrop::White => [0xff, 0xff, 0xff, 255], Backdrop::Light => [0xf2, 0xf1, 0xed, 255], Backdrop::Dark => [0x1a, 0x19, 0x15, 255] }
    }
    fn ink(self) -> [u8; 4] {
        match self { Backdrop::Dark => [0xf7, 0xf7, 0xf4, 255], _ => [0x1a, 0x19, 0x15, 255] }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepsStyle {
    pub arrangement: Arrangement,
    pub gap: Gap,
    pub badges: bool,
    pub captions: bool,
    pub backdrop: Backdrop,
    /// Bring every piece to the median width (height when horizontal), at most 2x up.
    pub uniform: bool,
}

impl Default for StepsStyle {
    fn default() -> Self { Self { arrangement: Arrangement::Vertical, gap: Gap::Medium, badges: true, captions: true, backdrop: Backdrop::White, uniform: false } }
}

/// One step: its pixels, its real size (the image may be a smaller stand-in for a
/// preview) and its caption.
#[derive(Clone, Copy, Debug)]
pub struct StepPiece<'a> { pub image: &'a RgbaImage, pub size: (u32, u32), pub caption: &'a str }

/// Where one step goes, in output pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    /// `(x, y, w, h)` of the step's pixels.
    pub image: (u32, u32, u32, u32),
    /// Badge centre and diameter.
    pub badge: Option<(f32, f32, f32)>,
    /// Caption text (already cut to fit), its top-left and its size.
    pub caption: Option<(String, (f32, f32), f32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepsLayout {
    pub canvas: (u32, u32),
    pub steps: Vec<Placed>,
    /// Overall scale applied to fit `MAX_PIXELS` (1 when it fits as is).
    pub scale: f32,
}

/// Most steps in one image.
pub const MAX_STEPS: usize = 20;
/// Smallest overall scale before a layout is refused.
const MIN_SCALE: f32 = 0.25;
const MAX_UP: f32 = 2.0;
const BRAND: [u8; 4] = [0xf5, 0x4e, 0x00, 255];

/// Grid columns for `n` steps (04_DESIGN v1.5).
pub fn grid_columns(n: usize) -> usize {
    match n { 0 | 1 => 1, 2..=4 => 2, 5..=9 => 3, _ => 4 }
}

fn median(mut v: Vec<u32>) -> f32 {
    v.sort_unstable();
    v[v.len() / 2] as f32
}

/// Lay out steps of the given real sizes and captions.
pub fn steps_layout(sizes: &[(u32, u32)], captions: &[&str], style: &StepsStyle) -> Result<StepsLayout, String> {
    if sizes.is_empty() { return Err("no steps".into()); }
    if sizes.len() > MAX_STEPS { return Err(format!("at most {MAX_STEPS} steps")); }
    if sizes.iter().any(|&(w, h)| w == 0 || h == 0) { return Err("empty step".into()); }
    let reference = median(sizes.iter().map(|&(w, h)| w.max(h)).collect());
    let own: Vec<f32> = if style.uniform {
        let horizontal = style.arrangement == Arrangement::Horizontal;
        let target = median(sizes.iter().map(|&(w, h)| if horizontal { h } else { w }).collect());
        sizes.iter().map(|&(w, h)| (target / if horizontal { h } else { w } as f32).min(MAX_UP)).collect()
    } else { vec![1.0; sizes.len()] };
    let mut scale = 1.0f32;
    loop {
        let l = layout_at(sizes, captions, style, &own, reference, scale);
        let pixels = u64::from(l.canvas.0) * u64::from(l.canvas.1);
        if pixels <= MAX_PIXELS { return Ok(l); }
        let next = scale * ((MAX_PIXELS as f64 / pixels as f64).sqrt() as f32) * 0.995;
        if next < MIN_SCALE {
            // One last try at the floor before giving up.
            if scale > MIN_SCALE { scale = MIN_SCALE; continue; }
            return Err("step image too large".into());
        }
        scale = next;
    }
}

fn layout_at(sizes: &[(u32, u32)], captions: &[&str], style: &StepsStyle, own: &[f32], reference: f32, scale: f32) -> StepsLayout {
    let n = sizes.len();
    let r = reference * scale;
    let gap = style.gap.px(r);
    let d = (r * 0.05).clamp(32.0, 72.0).round();
    let heading = d + (d * 0.25).round();
    let dims: Vec<(u32, u32)> = sizes.iter().zip(own).map(|(&(w, h), &k)| {
        let k = k * scale;
        if (k - 1.0).abs() < f32::EPSILON { (w, h) } else { (((w as f32) * k).round().max(1.0) as u32, ((h as f32) * k).round().max(1.0) as u32) }
    }).collect();
    let text: Vec<&str> = (0..n).map(|i| captions.get(i).copied().unwrap_or("").trim()).collect();
    let has_caption = |i: usize| style.captions && !text[i].is_empty();
    let has_heading = |i: usize| style.badges || has_caption(i);
    let cols = match style.arrangement { Arrangement::Vertical => 1, Arrangement::Horizontal => n, Arrangement::Grid => grid_columns(n) };
    let rows = n.div_ceil(cols);
    let mut col_w = vec![0u32; cols];
    let mut row_img = vec![0u32; rows];
    let mut row_head = vec![0f32; rows];
    for (i, &(w, h)) in dims.iter().enumerate() {
        let (c, rr) = (i % cols, i / cols);
        col_w[c] = col_w[c].max(w);
        row_img[rr] = row_img[rr].max(h);
        if has_heading(i) { row_head[rr] = heading; }
    }
    let gap_u = gap as u32;
    let mut col_x = vec![gap_u; cols];
    for c in 1..cols { col_x[c] = col_x[c - 1] + col_w[c - 1] + gap_u; }
    let mut row_y = vec![gap_u; rows];
    for rr in 1..rows { row_y[rr] = row_y[rr - 1] + row_head[rr - 1] as u32 + row_img[rr - 1] + gap_u; }
    let width = col_x[cols - 1] + col_w[cols - 1] + gap_u;
    let height = row_y[rows - 1] + row_head[rows - 1] as u32 + row_img[rows - 1] + gap_u;
    let size = d * 0.5;
    let steps = dims.iter().enumerate().map(|(i, &(w, h))| {
        let (c, rr) = (i % cols, i / cols);
        let x = col_x[c] + (col_w[c] - w) / 2;
        let y = row_y[rr] + row_head[rr] as u32;
        let top = row_y[rr] as f32;
        let badge = style.badges.then(|| (x as f32 + d / 2.0, top + d / 2.0, d));
        let caption = has_caption(i).then(|| {
            let left = x as f32 + if style.badges { d * 1.3 } else { 0.0 };
            let room = (col_x[c] + col_w[c]) as f32 - left;
            let s = fit_text(text[i], size, room.max(0.0));
            let th = text_extent(if s.is_empty() { " " } else { &s }, size).1;
            (s, (left, top + (d - th) / 2.0), size)
        }).filter(|(s, _, _)| !s.is_empty());
        Placed { image: (x, y, w, h), badge, caption }
    }).collect();
    StepsLayout { canvas: (width, height), steps, scale }
}

/// The finished step image.
pub fn render_steps(pieces: &[StepPiece], style: &StepsStyle) -> Result<RgbaImage, String> {
    if pieces.iter().any(|p| p.image.dimensions() != p.size) { return Err("step size does not match its pixels".into()); }
    render_steps_preview(pieces, style, 1.0)
}

/// The step image drawn at `k` (0 < k <= 1) of its export size, for the on-screen
/// preview; pieces may be smaller stand-ins, the layout follows their real sizes.
pub fn render_steps_preview(pieces: &[StepPiece], style: &StepsStyle, k: f32) -> Result<RgbaImage, String> {
    let sizes: Vec<(u32, u32)> = pieces.iter().map(|p| p.size).collect();
    let captions: Vec<&str> = pieces.iter().map(|p| p.caption).collect();
    let layout = steps_layout(&sizes, &captions, style)?;
    let k = k.clamp(0.01, 1.0);
    let at = |v: u32| ((v as f32) * k).round() as u32;
    let (cw, ch) = (at(layout.canvas.0).max(1), at(layout.canvas.1).max(1));
    let bg = style.backdrop.color();
    let mut out = RgbaImage::from_pixel(cw, ch, image::Rgba(bg));
    for (piece, place) in pieces.iter().zip(&layout.steps) {
        let (x, y, w, h) = place.image;
        let (w, h) = (at(w).max(1), at(h).max(1));
        let resized;
        let img = if piece.image.dimensions() == (w, h) { piece.image } else {
            // Area averaging is far quicker than a filtered resize for shrinking.
            resized = if w <= piece.image.width() && h <= piece.image.height() { imageops::thumbnail(piece.image, w, h) }
                else { imageops::resize(piece.image, w, h, imageops::FilterType::Triangle) };
            &resized
        };
        paste_over(&mut out, img, at(x), at(y), bg);
    }
    let Some(mut pm) = Pixmap::from_vec(out.into_raw(), tiny_skia::IntSize::from_wh(cw, ch).ok_or("empty canvas")?) else { return Err("canvas".into()); };
    let ink = style.backdrop.ink();
    for (i, place) in layout.steps.iter().enumerate() {
        if let Some((cx, cy, d)) = place.badge {
            if let Some(c) = PathBuilder::from_circle(cx * k, cy * k, d * k / 2.0) {
                pm.fill_path(&c, &paint(BRAND), FillRule::Winding, tiny_skia::Transform::identity(), None);
            }
            let label = (i + 1).to_string();
            let size = d * 0.55;
            let (tw, th) = text_extent(&label, size);
            glyphs(&mut pm, &label, (cx - tw / 2.0, cy - th / 2.0), size, [255, 255, 255, 255], k);
        }
        if let Some((text, origin, size)) = &place.caption {
            glyphs(&mut pm, text, *origin, *size, ink, k);
        }
    }
    let mut data = pm.take();
    for p in data.chunks_exact_mut(4) { p[3] = 255; }
    RgbaImage::from_raw(cw, ch, data).ok_or_else(|| "canvas".into())
}

/// Copy `img` into `out` at `(x, y)`; partly transparent pixels are laid over `bg`.
fn paste_over(out: &mut RgbaImage, img: &RgbaImage, x: u32, y: u32, bg: [u8; 4]) {
    let (ow, oh) = out.dimensions();
    if x >= ow || y >= oh { return; }
    let w = img.width().min(ow - x) as usize;
    let rows = img.height().min(oh - y) as usize;
    let (src_stride, dst_stride) = (img.width() as usize * 4, ow as usize * 4);
    let (src, dst) = (img.as_raw(), &mut **out);
    for row in 0..rows {
        let s = &src[row * src_stride..row * src_stride + w * 4];
        let start = (y as usize + row) * dst_stride + x as usize * 4;
        let d = &mut dst[start..start + w * 4];
        d.copy_from_slice(s);
        for p in d.chunks_exact_mut(4) {
            let a = u32::from(p[3]);
            if a == 255 { continue; }
            for k in 0..3 { p[k] = ((u32::from(p[k]) * a + u32::from(bg[k]) * (255 - a) + 127) / 255) as u8; }
            p[3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn img(w: u32, h: u32, seed: u8) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| Rgba([(x as u8).wrapping_mul(7) ^ seed, (y as u8).wrapping_mul(3), seed, 255]))
    }
    fn plain() -> StepsStyle { StepsStyle { badges: false, captions: false, ..StepsStyle::default() } }

    #[test]
    fn vertical_stacks_with_gaps_and_no_heading_when_off() {
        let l = steps_layout(&[(400, 300), (200, 100)], &["", ""], &plain()).unwrap();
        // Reference = median long side = 400 (index 1 of [200, 400]); medium gap = max(16, 16).
        assert_eq!(l.canvas, (16 + 400 + 16, 16 + 300 + 16 + 100 + 16));
        assert_eq!(l.steps[0].image, (16, 16, 400, 300));
        assert_eq!(l.steps[1].image, (16 + 100, 16 + 300 + 16, 200, 100)); // centred
        assert!(l.steps.iter().all(|s| s.badge.is_none() && s.caption.is_none()));
    }

    #[test]
    fn heading_row_is_reserved_only_when_needed() {
        let s = StepsStyle { badges: false, ..StepsStyle::default() };
        let l = steps_layout(&[(400, 300), (400, 300)], &["打开设置", "  "], &s).unwrap();
        let d = 32.0f32; // 5% of 400 = 20, clamped up to 32
        let head = (d + (d * 0.25).round()) as u32;
        assert_eq!(l.steps[0].image.1, 16 + head);
        assert!(l.steps[0].caption.is_some());
        // The second caption is blank: no heading row.
        assert_eq!(l.steps[1].image.1, 16 + head + 300 + 16);
        assert!(l.steps[1].caption.is_none());
        let with_badges = steps_layout(&[(400, 300), (400, 300)], &["", ""], &StepsStyle::default()).unwrap();
        assert!(with_badges.steps.iter().all(|s| s.badge.is_some() && s.caption.is_none()));
    }

    #[test]
    fn horizontal_and_grid_columns() {
        let sizes = [(100, 80); 5];
        let h = steps_layout(&sizes, &[""; 5], &StepsStyle { arrangement: Arrangement::Horizontal, ..plain() }).unwrap();
        assert_eq!(h.canvas, (16 * 6 + 500, 16 * 2 + 80));
        let g = steps_layout(&sizes, &[""; 5], &StepsStyle { arrangement: Arrangement::Grid, ..plain() }).unwrap();
        assert_eq!(g.canvas, (16 * 4 + 300, 16 * 3 + 160));
        assert_eq!(g.steps[3].image, (16, 16 + 80 + 16, 100, 80));
        assert_eq!([1, 2, 4, 5, 9, 10, 20].map(grid_columns), [1, 2, 2, 3, 3, 4, 4]);
    }

    #[test]
    fn gap_steps_scale_with_the_reference() {
        let one = |g| steps_layout(&[(2000, 1000)], &[""], &StepsStyle { gap: g, ..plain() }).unwrap().steps[0].image.0;
        assert_eq!([Gap::Small, Gap::Medium, Gap::Large].map(one), [40, 80, 120]);
        let small = |g| steps_layout(&[(100, 50)], &[""], &StepsStyle { gap: g, ..plain() }).unwrap().steps[0].image.0;
        assert_eq!([Gap::Small, Gap::Medium, Gap::Large].map(small), [8, 16, 24]);
    }

    #[test]
    fn uniform_brings_pieces_to_the_median_width_at_most_twice() {
        let s = StepsStyle { uniform: true, ..plain() };
        let l = steps_layout(&[(400, 200), (100, 100), (800, 400)], &[""; 3], &s).unwrap();
        assert_eq!(l.steps[0].image.2, 400);
        assert_eq!((l.steps[1].image.2, l.steps[1].image.3), (200, 200)); // capped at 2x
        assert_eq!((l.steps[2].image.2, l.steps[2].image.3), (400, 200));
    }

    #[test]
    fn too_large_scales_down_then_refuses() {
        let l = steps_layout(&[(6000, 4000), (6000, 4000)], &["", ""], &plain()).unwrap();
        assert!(l.scale < 1.0 && l.scale >= MIN_SCALE);
        assert!(u64::from(l.canvas.0) * u64::from(l.canvas.1) <= MAX_PIXELS);
        let huge = vec![(20000, 20000); 20];
        assert!(steps_layout(&huge, &[""; 20], &plain()).is_err());
        assert!(steps_layout(&[], &[], &plain()).is_err());
        assert!(steps_layout(&vec![(10, 10); MAX_STEPS + 1], &vec![""; MAX_STEPS + 1], &plain()).is_err());
    }

    #[test]
    fn unscaled_pieces_are_copied_exactly_and_the_result_is_opaque() {
        let (a, b) = (img(120, 90, 3), img(80, 60, 200));
        let pieces = [StepPiece { image: &a, size: (120, 90), caption: "第一步：打开" }, StepPiece { image: &b, size: (80, 60), caption: "" }];
        for arrangement in Arrangement::ALL {
            let style = StepsStyle { arrangement, ..StepsStyle::default() };
            let out = render_steps(&pieces, &style).unwrap();
            let l = steps_layout(&[(120, 90), (80, 60)], &["第一步：打开", ""], &style).unwrap();
            assert_eq!(out.dimensions(), l.canvas);
            for (p, place) in [&a, &b].iter().zip(&l.steps) {
                let (x, y, w, h) = place.image;
                assert_eq!((w, h), p.dimensions());
                for (px, py, v) in p.enumerate_pixels() { assert_eq!(out.get_pixel(x + px, y + py), v); }
            }
            assert!(out.pixels().all(|p| p[3] == 255));
        }
    }

    #[test]
    fn transparent_pieces_land_on_the_backdrop() {
        let clear = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]));
        let out = render_steps(&[StepPiece { image: &clear, size: (10, 10), caption: "" }], &StepsStyle { backdrop: Backdrop::Dark, ..plain() }).unwrap();
        assert!(out.pixels().all(|p| *p == Rgba(Backdrop::Dark.color())));
    }

    #[test]
    fn preview_follows_the_export_layout() {
        let a = img(400, 300, 9);
        let small = imageops::resize(&a, 100, 75, imageops::FilterType::Triangle);
        let full = render_steps(&[StepPiece { image: &a, size: (400, 300), caption: "说明" }], &StepsStyle::default()).unwrap();
        let prev = render_steps_preview(&[StepPiece { image: &small, size: (400, 300), caption: "说明" }], &StepsStyle::default(), 0.25).unwrap();
        assert_eq!(prev.dimensions(), (((full.width() as f32) * 0.25).round() as u32, ((full.height() as f32) * 0.25).round() as u32));
        assert!(render_steps(&[StepPiece { image: &small, size: (400, 300), caption: "" }], &StepsStyle::default()).is_err());
    }

    #[test]
    fn long_captions_are_cut_to_the_column() {
        let l = steps_layout(&[(200, 100)], &["这是一段非常非常长的说明文字，用来检查截断是否生效以及省略号"], &StepsStyle::default()).unwrap();
        let (text, origin, size) = l.steps[0].caption.clone().unwrap();
        assert!(text.ends_with('…'));
        assert!(origin.0 + text_extent(&text, size).0 <= (16 + 200) as f32);
    }
}
