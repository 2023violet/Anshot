//! Scene rendering for Anshot exports and the on-screen preview.
//!
//! Order is fixed (02 U06): source pixels -> redactions (opaque replacement,
//! no source information) -> annotations (anti-aliased, on top) -> crop.
//! Every coordinate is in source pixels; `scale` maps them onto a preview.
#![forbid(unsafe_code)]

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{DynamicImage, ImageFormat, RgbaImage};
use anshot_core::{decode_generated_png, decode_source, Region};
use std::io::Cursor;
use std::sync::OnceLock;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

static FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/NotoSansCJKsc-Regular.otf");

/// The bundled Noto Sans CJK SC bytes, shared with the UI so the exe embeds one copy.
pub fn font_bytes() -> &'static [u8] { FONT_BYTES }

fn font() -> &'static FontRef<'static> {
    static FONT: OnceLock<FontRef<'static>> = OnceLock::new();
    FONT.get_or_init(|| FontRef::try_from_slice(FONT_BYTES).expect("bundled Noto Sans CJK SC parses"))
}

/// Safe-mosaic palette (04_DESIGN `redact.mosaic`): colours never derived from the source.
pub const MOSAIC: [[u8; 4]; 4] = [[0x3a, 0x39, 0x34, 255], [0x4b, 0x4a, 0x44, 255], [0x5d, 0x5b, 0x54, 255], [0x6f, 0x6d, 0x65, 255]];
/// Mosaic cell edge in source pixels.
pub const MOSAIC_CELL: i64 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedactStyle { Solid([u8; 4]), SafeMosaic }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Redaction { pub region: Region, pub style: RedactStyle }

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// Outline drawn inside `region`.
    Rect { region: Region, width: f32 },
    Arrow { from: (f32, f32), to: (f32, f32), width: f32 },
    /// Translucent marker fill over `region`.
    Highlight { region: Region },
    /// Filled circle with a number, centred.
    Number { center: (f32, f32), n: u32, radius: f32 },
    /// Text whose top-left is `origin`; `size` is the font px size.
    Text { origin: (f32, f32), text: String, size: f32, background: bool },
    /// Oval outline drawn inside `region`.
    Ellipse { region: Region, width: f32 },
    /// Straight line with round caps.
    Line { from: (f32, f32), to: (f32, f32), width: f32 },
    /// Freehand stroke through `points` (source px), round joins and caps.
    Pen { points: Vec<(f32, f32)>, width: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Annotation { pub shape: Shape, pub color: [u8; 4] }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene { pub redactions: Vec<Redaction>, pub annotations: Vec<Annotation>, pub crop: Option<Region> }

fn mix(a: u64) -> u64 {
    let mut z = a.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
/// Colour of the mosaic cell containing source pixel (x, y): a pure function of position.
pub fn mosaic_color(x: i64, y: i64) -> [u8; 4] {
    let (cx, cy) = (x.div_euclid(MOSAIC_CELL), y.div_euclid(MOSAIC_CELL));
    MOSAIC[(mix((cx as u64) ^ (cy as u64).rotate_left(32)) % 4) as usize]
}

/// Bounding box of an annotation in source pixels (for hit-testing and selection).
pub fn bounds(a: &Annotation) -> (f32, f32, f32, f32) {
    let r = |g: &Region| (g.x0.min(g.x1) as f32, g.y0.min(g.y1) as f32, g.x0.max(g.x1) as f32, g.y0.max(g.y1) as f32);
    match &a.shape {
        Shape::Rect { region, .. } | Shape::Highlight { region } | Shape::Ellipse { region, .. } => r(region),
        Shape::Line { from, to, width } => {
            let pad = width / 2.0 + 2.0;
            (from.0.min(to.0) - pad, from.1.min(to.1) - pad, from.0.max(to.0) + pad, from.1.max(to.1) + pad)
        }
        Shape::Pen { points, width } => {
            let pad = width / 2.0 + 2.0;
            let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for p in points { x0 = x0.min(p.0); y0 = y0.min(p.1); x1 = x1.max(p.0); y1 = y1.max(p.1); }
            if points.is_empty() { return (0.0, 0.0, 0.0, 0.0); }
            (x0 - pad, y0 - pad, x1 + pad, y1 + pad)
        }
        Shape::Arrow { from, to, width } => {
            let pad = width * 3.0;
            (from.0.min(to.0) - pad, from.1.min(to.1) - pad, from.0.max(to.0) + pad, from.1.max(to.1) + pad)
        }
        Shape::Number { center, radius, .. } => (center.0 - radius, center.1 - radius, center.0 + radius, center.1 + radius),
        Shape::Text { origin, text, size, background } => {
            let (w, h) = text_extent(text, *size);
            let pad = if *background { size * 0.35 } else { 0.0 };
            (origin.0 - pad, origin.1 - pad, origin.0 + w + pad, origin.1 + h + pad)
        }
    }
}

/// Width and height of (possibly multi-line) text at `size` px.
pub fn text_extent(text: &str, size: f32) -> (f32, f32) {
    let f = font().as_scaled(PxScale::from(size));
    let line_h = f.height() + f.line_gap();
    let mut w: f32 = 0.0;
    let lines = text.split('\n').collect::<Vec<_>>();
    for line in &lines {
        let mut x = 0.0;
        let mut prev = None;
        for c in line.chars() {
            let id = f.glyph_id(c);
            if let Some(p) = prev { x += f.kern(p, id); }
            x += f.h_advance(id);
            prev = Some(id);
        }
        w = w.max(x);
    }
    (w.ceil(), (line_h * lines.len().max(1) as f32).ceil())
}

fn premultiply(img: &RgbaImage) -> Vec<u8> {
    let mut v = img.as_raw().clone();
    for p in v.chunks_exact_mut(4) {
        let a = u16::from(p[3]);
        if a < 255 { for c in &mut p[..3] { *c = ((u16::from(*c) * a + 127) / 255) as u8; } }
    }
    v
}
fn demultiply(data: &mut [u8]) {
    for p in data.chunks_exact_mut(4) {
        let a = u16::from(p[3]);
        if a > 0 && a < 255 { for c in &mut p[..3] { *c = ((u16::from(*c) * 255 + a / 2) / a).min(255) as u8; } }
    }
}

fn paint(color: [u8; 4]) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(color[0], color[1], color[2], color[3]);
    p.anti_alias = true;
    p
}

/// Draw `scene` onto `img`, where one source pixel spans `scale` pixels of `img`.
/// Export uses scale 1; the preview uses its downscale factor.
pub fn render_into(img: &mut RgbaImage, scene: &Scene, scale: f32) {
    let (w, h) = img.dimensions();
    // 1. Redactions replace pixels outright (no blending, no source reads).
    for red in &scene.redactions {
        let r = red.region;
        let (l, t) = ((r.x0.min(r.x1) as f32 * scale).floor(), (r.y0.min(r.y1) as f32 * scale).floor());
        let (rr, b) = ((r.x0.max(r.x1) as f32 * scale).ceil(), (r.y0.max(r.y1) as f32 * scale).ceil());
        let (l, t) = (l.clamp(0.0, w as f32) as u32, t.clamp(0.0, h as f32) as u32);
        let (rr, b) = (rr.clamp(0.0, w as f32) as u32, b.clamp(0.0, h as f32) as u32);
        for y in t..b {
            for x in l..rr {
                let c = match red.style {
                    RedactStyle::Solid(c) => c,
                    RedactStyle::SafeMosaic => mosaic_color((x as f32 / scale) as i64, (y as f32 / scale) as i64),
                };
                img.put_pixel(x, y, image::Rgba(c));
            }
        }
    }
    if scene.annotations.is_empty() { return; }
    // 2. Annotations, anti-aliased, on a premultiplied copy.
    let Some(size) = tiny_skia::IntSize::from_wh(w, h) else { return };
    let Some(mut pm) = Pixmap::from_vec(premultiply(img), size) else { return };
    let tf = Transform::from_scale(scale, scale);
    for a in &scene.annotations { draw(&mut pm, a, tf, scale); }
    let mut data = pm.take();
    demultiply(&mut data);
    if let Some(out) = RgbaImage::from_raw(w, h, data) { *img = out; }
}

fn rect_of(r: &Region) -> Option<Rect> {
    Rect::from_ltrb(r.x0.min(r.x1) as f32, r.y0.min(r.y1) as f32, r.x0.max(r.x1) as f32, r.y0.max(r.y1) as f32)
}

fn draw(pm: &mut Pixmap, a: &Annotation, tf: Transform, scale: f32) {
    let color = a.color;
    match &a.shape {
        Shape::Rect { region, width } => {
            let Some(r) = rect_of(region) else { return };
            let half = width / 2.0;
            let Some(inner) = Rect::from_ltrb(r.left() + half, r.top() + half, r.right() - half, r.bottom() - half) else { return };
            let path = PathBuilder::from_rect(inner);
            let stroke = Stroke { width: *width, line_join: LineJoin::Miter, ..Stroke::default() };
            pm.stroke_path(&path, &paint(color), &stroke, tf, None);
        }
        Shape::Highlight { region } => {
            let Some(r) = rect_of(region) else { return };
            pm.fill_rect(r, &paint([color[0], color[1], color[2], 96]), tf, None);
        }
        Shape::Arrow { from, to, width } => {
            let (dx, dy) = (to.0 - from.0, to.1 - from.1);
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1.0 { return; }
            let (ux, uy) = (dx / len, dy / len);
            let head = (width * 4.5).max(12.0).min(len);
            let base = (to.0 - ux * head, to.1 - uy * head);
            let mut pb = PathBuilder::new();
            pb.move_to(from.0, from.1);
            pb.line_to(base.0 + ux * 1.0, base.1 + uy * 1.0);
            if let Some(line) = pb.finish() {
                let stroke = Stroke { width: *width, line_cap: LineCap::Round, ..Stroke::default() };
                pm.stroke_path(&line, &paint(color), &stroke, tf, None);
            }
            let half = head * 0.55;
            let mut hb = PathBuilder::new();
            hb.move_to(to.0, to.1);
            hb.line_to(base.0 - uy * half, base.1 + ux * half);
            hb.line_to(base.0 + uy * half, base.1 - ux * half);
            hb.close();
            if let Some(tri) = hb.finish() { pm.fill_path(&tri, &paint(color), FillRule::Winding, tf, None); }
        }
        Shape::Number { center, n, radius } => {
            if let Some(c) = PathBuilder::from_circle(center.0, center.1, *radius) {
                pm.fill_path(&c, &paint(color), FillRule::Winding, tf, None);
            }
            let label = n.to_string();
            let size = radius * 1.15;
            let (tw, th) = text_extent(&label, size);
            glyphs(pm, &label, (center.0 - tw / 2.0, center.1 - th / 2.0), size, contrast(color), scale);
        }
        Shape::Text { origin, text, size, background } => {
            let fg = if *background {
                let (tw, th) = text_extent(text, *size);
                let pad = size * 0.35;
                if let Some(r) = Rect::from_xywh(origin.0 - pad, origin.1 - pad, tw + pad * 2.0, th + pad * 2.0) {
                    let path = rounded(r, size * 0.25);
                    if let Some(p) = path { pm.fill_path(&p, &paint(color), FillRule::Winding, tf, None); }
                }
                contrast(color)
            } else { color };
            glyphs(pm, text, *origin, *size, fg, scale);
        }
        Shape::Ellipse { region, width } => {
            let Some(r) = rect_of(region) else { return };
            let half = width / 2.0;
            let Some(inner) = Rect::from_ltrb(r.left() + half, r.top() + half, r.right() - half, r.bottom() - half) else { return };
            let Some(path) = PathBuilder::from_oval(inner) else { return };
            pm.stroke_path(&path, &paint(color), &Stroke { width: *width, ..Stroke::default() }, tf, None);
        }
        Shape::Line { from, to, width } => {
            let mut pb = PathBuilder::new();
            pb.move_to(from.0, from.1);
            pb.line_to(to.0, to.1);
            if let Some(line) = pb.finish() {
                pm.stroke_path(&line, &paint(color), &Stroke { width: *width, line_cap: LineCap::Round, ..Stroke::default() }, tf, None);
            }
        }
        Shape::Pen { points, width } => {
            if let Some(path) = pen_path(points) {
                let stroke = Stroke { width: *width, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
                pm.stroke_path(&path, &paint(color), &stroke, tf, None);
            }
        }
    }
}

/// Smooth a freehand polyline: straight segments between the midpoints of consecutive
/// samples, with each sample as the quadratic control point (no overshoot).
fn pen_path(points: &[(f32, f32)]) -> Option<tiny_skia::Path> {
    let (&first, rest) = points.split_first()?;
    let mut pb = PathBuilder::new();
    pb.move_to(first.0, first.1);
    match rest {
        [] => pb.line_to(first.0 + 0.01, first.1), // a dot
        [only] => pb.line_to(only.0, only.1),
        _ => {
            for w in points.windows(2).skip(1) {
                let (c, n) = (w[0], w[1]);
                pb.quad_to(c.0, c.1, (c.0 + n.0) / 2.0, (c.1 + n.1) / 2.0);
            }
            let last = points[points.len() - 1];
            pb.line_to(last.0, last.1);
        }
    }
    pb.finish()
}

fn rounded(r: Rect, rad: f32) -> Option<tiny_skia::Path> {
    let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
    let k = rad.min(r.width() / 2.0).min(r.height() / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(l + k, t);
    pb.line_to(rr - k, t); pb.quad_to(rr, t, rr, t + k);
    pb.line_to(rr, b - k); pb.quad_to(rr, b, rr - k, b);
    pb.line_to(l + k, b); pb.quad_to(l, b, l, b - k);
    pb.line_to(l, t + k); pb.quad_to(l, t, l + k, t);
    pb.close();
    pb.finish()
}

/// White or near-black text, whichever reads better on `bg`.
pub fn contrast(bg: [u8; 4]) -> [u8; 4] {
    let lum = 0.2126 * f32::from(bg[0]) + 0.7152 * f32::from(bg[1]) + 0.0722 * f32::from(bg[2]);
    if lum > 150.0 { [0x1a, 0x19, 0x15, 255] } else { [255, 255, 255, 255] }
}

/// Rasterise text with coverage blending; `origin` is the top-left in source px.
fn glyphs(pm: &mut Pixmap, text: &str, origin: (f32, f32), size: f32, color: [u8; 4], scale: f32) {
    let px = size * scale;
    let f = font().as_scaled(PxScale::from(px));
    let line_h = f.height() + f.line_gap();
    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let stride = pm.width() as usize * 4;
    let data = pm.data_mut();
    for (li, line) in text.split('\n').enumerate() {
        let mut x = origin.0 * scale;
        let y = origin.1 * scale + f.ascent() + line_h * li as f32;
        let mut prev = None;
        for c in line.chars() {
            let id = f.glyph_id(c);
            if let Some(p) = prev { x += f.kern(p, id); }
            let glyph = id.with_scale_and_position(px, ab_glyph::point(x, y));
            x += f.h_advance(id);
            prev = Some(id);
            let Some(outline) = f.outline_glyph(glyph) else { continue };
            let bb = outline.px_bounds();
            outline.draw(|gx, gy, cov| {
                let (ix, iy) = (bb.min.x as i32 + gx as i32, bb.min.y as i32 + gy as i32);
                if ix < 0 || iy < 0 || ix >= w || iy >= h { return; }
                let a = cov.clamp(0.0, 1.0) * f32::from(color[3]) / 255.0;
                let i = iy as usize * stride + ix as usize * 4;
                // Premultiplied source-over.
                for k in 0..3 {
                    let s = f32::from(color[k]) * a;
                    data[i + k] = (s + f32::from(data[i + k]) * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
                }
                data[i + 3] = (a * 255.0 + f32::from(data[i + 3]) * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
            });
        }
    }
}

fn crop(img: RgbaImage, r: Region) -> Option<RgbaImage> {
    let (l, t, rr, b) = r.clipped(img.width(), img.height())?;
    Some(image::imageops::crop_imm(&img, l, t, rr - l, b - t).to_image())
}

/// Export: decode the source through Core, render at scale 1, crop, encode a new PNG,
/// and fail closed unless the bytes reopen at the expected dimensions.
pub fn render_png(source: &[u8], scene: &Scene) -> Result<Vec<u8>, String> {
    let mut img = decode_source(source)?;
    render_into(&mut img, scene, 1.0);
    let img = match scene.crop { Some(r) => crop(img, r).ok_or("crop is empty")?, None => img };
    let dims = img.dimensions();
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(img).write_to(&mut out, ImageFormat::Png).map_err(|e| format!("new PNG encode failed: {e}"))?;
    let bytes = out.into_inner();
    decode_generated_png(&bytes, dims)?;
    Ok(bytes)
}

/// Output dimensions after crop.
pub fn output_size(source: (u32, u32), scene: &Scene) -> Option<(u32, u32)> {
    match scene.crop {
        None => Some(source),
        Some(r) => r.clipped(source.0, source.1).map(|(l, t, rr, b)| (rr - l, b - t)),
    }
}

#[cfg(test)]
mod tests;
