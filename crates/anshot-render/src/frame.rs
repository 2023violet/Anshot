//! Output beautify (ADR 0.5-BEAUTIFY): the finished export placed on a background with
//! padding, rounded corners, a shadow, an output ratio and optional window chrome.
//!
//! Runs after crop, on the rendered export only. Pixels of the export are copied unchanged
//! wherever the rounded shape fully covers them, and the result is always opaque.

use crate::{glyphs, paint, rounded, text_extent};
use anshot_core::MAX_PIXELS;
use image::RgbaImage;
use tiny_skia::{Color, FillRule, GradientStop, LinearGradient, Mask, PathBuilder, Pixmap, Point, Rect, SpreadMode, Stroke, Transform};

/// Two colours of a top-left to bottom-right gradient (equal colours give a solid fill),
/// or colours taken from the export's own outer ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background { Gradient([u8; 4], [u8; 4]), Auto }

/// Background presets (04_DESIGN v1.3).
pub const PRESETS: [Background; 8] = [
    Background::Gradient([0xe8, 0xe6, 0xdf, 255], [0xc9, 0xc6, 0xbb, 255]),
    Background::Gradient([0xff, 0x8a, 0x4c, 255], [0xf5, 0x4e, 0x00, 255]),
    Background::Gradient([0xa1, 0xc4, 0xfd, 255], [0xc2, 0xe9, 0xfb, 255]),
    Background::Gradient([0x84, 0xfa, 0xb0, 255], [0x8f, 0xd3, 0xf4, 255]),
    Background::Gradient([0xa1, 0x8c, 0xd1, 255], [0xfb, 0xc2, 0xeb, 255]),
    Background::Gradient([0x23, 0x25, 0x26, 255], [0x41, 0x43, 0x45, 255]),
    Background::Gradient([0xff, 0xff, 0xff, 255], [0xff, 0xff, 0xff, 255]),
    Background::Gradient([0x1a, 0x19, 0x15, 255], [0x1a, 0x19, 0x15, 255]),
];

/// None / small / medium / large, scaled by the export's long side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Step { #[default] None, Small, Medium, Large }

impl Step {
    pub const ALL: [Step; 4] = [Step::None, Step::Small, Step::Medium, Step::Large];
    fn pick(self, table: [f32; 4]) -> f32 { table[self as usize] }
}

/// Output aspect ratio; the export is centred and never cut.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ratio { #[default] Free, Square, R4x3, R16x9, R3x4, R9x16 }

impl Ratio {
    pub const ALL: [Ratio; 6] = [Ratio::Free, Ratio::Square, Ratio::R4x3, Ratio::R16x9, Ratio::R3x4, Ratio::R9x16];
    pub(crate) fn wh(self) -> Option<(u64, u64)> {
        match self { Ratio::Free => None, Ratio::Square => Some((1, 1)), Ratio::R4x3 => Some((4, 3)), Ratio::R16x9 => Some((16, 9)), Ratio::R3x4 => Some((3, 4)), Ratio::R9x16 => Some((9, 16)) }
    }
}

/// Simplified window chrome drawn above the export.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Chrome { #[default] None, Mac, Windows, Browser }

impl Chrome {
    pub const ALL: [Chrome; 4] = [Chrome::None, Chrome::Mac, Chrome::Windows, Chrome::Browser];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub background: Background,
    pub padding: Step,
    pub radius: Step,
    pub shadow: Step,
    pub ratio: Ratio,
    pub chrome: Chrome,
    /// Title or address text, typed by the user only; empty draws no text.
    pub text: String,
}

impl Default for Frame {
    fn default() -> Self {
        Frame { background: PRESETS[0], padding: Step::Medium, radius: Step::Medium, shadow: Step::Medium, ratio: Ratio::Free, chrome: Chrome::None, text: String::new() }
    }
}

const PADDING: [f32; 4] = [0.0, 0.04, 0.08, 0.12];
const RADIUS: [f32; 4] = [0.0, 0.008, 0.016, 0.024];
const RADIUS_MIN: [f32; 4] = [0.0, 4.0, 8.0, 12.0];
const BLUR: [f32; 4] = [0.0, 0.01, 0.02, 0.03];
const SHADOW_ALPHA: [f32; 4] = [0.0, 0.25, 0.35, 0.45];
const BAR: f32 = 0.035;
const BAR_MIN: u32 = 28;
const BAR_MAX: u32 = 72;

/// Where everything goes, in output pixels: `(x, y, w, h)` rectangles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub canvas: (u32, u32),
    /// The export, copied unchanged except under the rounded corners.
    pub image: (u32, u32, u32, u32),
    /// The chrome bar above the export; zero height without chrome.
    pub bar: (u32, u32, u32, u32),
    pub radius: f32,
    pub blur: f32,
}

/// Lay out `frame` around an export of `size`. Padding shrinks when the result would pass
/// `MAX_PIXELS`; an error means it does not fit even without padding.
pub fn frame_layout(size: (u32, u32), frame: &Frame) -> Result<Layout, String> {
    let (w, h) = size;
    if w == 0 || h == 0 { return Err("empty image".into()); }
    let long = w.max(h) as f32;
    let bar = if frame.chrome == Chrome::None { 0 } else { ((long * BAR).round() as u32).clamp(BAR_MIN, BAR_MAX) };
    let pad0 = (long * frame.padding.pick(PADDING)).round() as u64;
    for pad in [pad0, pad0 / 2, pad0 / 4, 0] {
        let (mut cw, mut ch) = (u64::from(w) + 2 * pad, u64::from(h) + u64::from(bar) + 2 * pad);
        if let Some((rw, rh)) = frame.ratio.wh() {
            if cw * rh < ch * rw { cw = (ch * rw).div_ceil(rh); } else { ch = (cw * rh).div_ceil(rw); }
        }
        if cw * ch > MAX_PIXELS as u64 { continue; }
        let (cw, ch) = (cw as u32, ch as u32);
        let (x, y) = ((cw - w) / 2, (ch - h - bar) / 2);
        let short = w.min(h + bar) as f32;
        let radius = match frame.radius {
            Step::None => 0.0,
            s => (long * s.pick(RADIUS)).round().max(s.pick(RADIUS_MIN)).min(short / 2.0),
        };
        return Ok(Layout { canvas: (cw, ch), image: (x, y + bar, w, h), bar: (x, y, w, bar), radius, blur: (long * frame.shadow.pick(BLUR)).round() });
    }
    Err("too large to beautify".into())
}

/// Lighter and darker versions of the median colour of the export's 2 px outer ring.
pub fn auto_colors(img: &RgbaImage) -> ([u8; 4], [u8; 4]) {
    let (w, h) = img.dimensions();
    let mut ch: [Vec<u8>; 3] = Default::default();
    for (x, y, p) in img.enumerate_pixels() {
        if x >= 2 && y >= 2 && x + 2 < w && y + 2 < h { continue; }
        for k in 0..3 { ch[k].push(p[k]); }
    }
    let mut mid = [0u8; 3];
    for k in 0..3 {
        if ch[k].is_empty() { continue; }
        let n = ch[k].len() / 2;
        mid[k] = *ch[k].select_nth_unstable(n).1;
    }
    let light = mid.map(|c| (f32::from(c) + (255.0 - f32::from(c)) * 0.12).round() as u8);
    let dark = mid.map(|c| (f32::from(c) * 0.88).round() as u8);
    ([light[0], light[1], light[2], 255], [dark[0], dark[1], dark[2], 255])
}

fn color(c: [u8; 4]) -> Color { Color::from_rgba8(c[0], c[1], c[2], 255) }

/// The export framed as `frame` says; every output pixel is opaque.
pub fn render_framed(img: &RgbaImage, frame: &Frame) -> Result<RgbaImage, String> {
    let lay = frame_layout(img.dimensions(), frame)?;
    let (cw, ch) = lay.canvas;
    let mut pm = Pixmap::new(cw, ch).ok_or("canvas too large")?;
    let full = Rect::from_xywh(0.0, 0.0, cw as f32, ch as f32).ok_or("empty canvas")?;

    // 1. Background.
    let (from, to) = match frame.background { Background::Gradient(a, b) => (a, b), Background::Auto => auto_colors(img) };
    let mut bg = paint(from);
    if from != to {
        bg.shader = LinearGradient::new(Point::from_xy(0.0, 0.0), Point::from_xy(cw as f32, ch as f32),
            vec![GradientStop::new(0.0, color(from)), GradientStop::new(1.0, color(to))], SpreadMode::Pad, Transform::identity())
            .ok_or("gradient")?;
    }
    pm.fill_rect(full, &bg, Transform::identity(), None);

    let (wx, wy, ww, wh) = (lay.bar.0 as f32, lay.bar.1 as f32, lay.bar.2 as f32, (lay.bar.3 + lay.image.3) as f32);
    let window = Rect::from_xywh(wx, wy, ww, wh).ok_or("empty window")?;
    let shape = if lay.radius > 0.0 { rounded(window, lay.radius) } else { Some(PathBuilder::from_rect(window)) }.ok_or("window path")?;

    // 2. Shadow below the window.
    if lay.blur > 0.0 { shadow(&mut pm, window, lay.radius, lay.blur, frame.shadow.pick(SHADOW_ALPHA)); }

    // 3. The window: chrome bar, then the export, both clipped to the rounded shape.
    let mut clip = Mask::new(cw, ch).ok_or("mask")?;
    clip.fill_path(&shape, FillRule::Winding, true, Transform::identity());
    if lay.bar.3 > 0 { chrome(&mut pm, &lay, frame, &clip); }
    paste(&mut pm, img, &lay, &clip);

    let mut data = pm.take();
    for p in data.chunks_exact_mut(4) { p[3] = 255; }
    RgbaImage::from_raw(cw, ch, data).ok_or_else(|| "frame buffer".into())
}

/// Copy the export into its rectangle: exact where the clip fully covers a pixel, blended
/// by coverage under the corners. Transparent export pixels sit on whatever is below.
fn paste(pm: &mut Pixmap, img: &RgbaImage, lay: &Layout, clip: &Mask) {
    let (x0, y0, w, h) = lay.image;
    let cw = pm.width() as usize;
    let cov = clip.data();
    let data = pm.data_mut();
    for y in 0..h {
        for x in 0..w {
            let (ox, oy) = ((x0 + x) as usize, (y0 + y) as usize);
            let i = oy * cw + ox;
            let c = u32::from(cov[i]);
            if c == 0 { continue; }
            let s = img.get_pixel(x, y).0;
            let a = u32::from(s[3]) * c / 255;
            let d = &mut data[i * 4..i * 4 + 4];
            if a == 255 { d.copy_from_slice(&[s[0], s[1], s[2], 255]); continue; }
            for k in 0..3 { d[k] = ((u32::from(s[k]) * a + u32::from(d[k]) * (255 - a) + 127) / 255) as u8; }
        }
    }
}

/// A soft black shadow of `window`, offset down by half the blur. The mask is blurred at
/// reduced resolution (three box passes) and sampled back bilinearly.
fn shadow(pm: &mut Pixmap, window: Rect, radius: f32, blur: f32, alpha: f32) {
    let k: f32 = if blur >= 16.0 { 4.0 } else { 1.0 };
    let (cw, ch) = (pm.width(), pm.height());
    let (sw, sh) = ((cw as f32 / k).ceil() as u32 + 1, (ch as f32 / k).ceil() as u32 + 1);
    let Some(mut small) = Mask::new(sw, sh) else { return };
    let Some(r) = Rect::from_xywh(window.x() / k, (window.y() + blur / 2.0) / k, window.width() / k, window.height() / k) else { return };
    let Some(path) = rounded(r, radius / k) else { return };
    small.fill_path(&path, FillRule::Winding, true, Transform::identity());
    let mut m: Vec<f32> = small.data().iter().map(|&v| f32::from(v) / 255.0).collect();
    let rad = ((blur / k) / 2.0).round().max(1.0) as usize;
    for _ in 0..3 { box_blur(&mut m, sw as usize, sh as usize, rad); }
    let data = pm.data_mut();
    for y in 0..ch {
        for x in 0..cw {
            let (fx, fy) = ((x as f32 + 0.5) / k - 0.5, (y as f32 + 0.5) / k - 0.5);
            let a = bilinear(&m, sw as usize, sh as usize, fx, fy) * alpha;
            if a <= 0.0 { continue; }
            let i = (y * cw + x) as usize * 4;
            for c in &mut data[i..i + 3] { *c = (f32::from(*c) * (1.0 - a)).round() as u8; }
        }
    }
}

fn box_blur(m: &mut [f32], w: usize, h: usize, r: usize) {
    let mut tmp = vec![0.0f32; m.len()];
    let n = (2 * r + 1) as f32;
    for y in 0..h {
        let row = &m[y * w..(y + 1) * w];
        let mut sum: f32 = (0..=r).map(|i| row[i.min(w - 1)]).sum::<f32>() + row[0] * r as f32;
        for x in 0..w {
            tmp[y * w + x] = sum / n;
            sum += row[(x + r + 1).min(w - 1)] - row[x.saturating_sub(r)];
        }
    }
    for x in 0..w {
        let col = |y: usize| tmp[y.min(h - 1) * w + x];
        let mut sum: f32 = (0..=r).map(col).sum::<f32>() + col(0) * r as f32;
        for y in 0..h {
            m[y * w + x] = sum / n;
            sum += col(y + r + 1) - col(y.saturating_sub(r));
        }
    }
}

fn bilinear(m: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
    let (x, y) = (x.clamp(0.0, (w - 1) as f32), y.clamp(0.0, (h - 1) as f32));
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let top = m[y0 * w + x0] * (1.0 - fx) + m[y0 * w + x1] * fx;
    let bot = m[y1 * w + x0] * (1.0 - fx) + m[y1 * w + x1] * fx;
    top * (1.0 - fy) + bot * fy
}

const DOTS: [[u8; 4]; 3] = [[0xff, 0x5f, 0x57, 255], [0xfe, 0xbc, 0x2e, 255], [0x28, 0xc8, 0x40, 255]];
const INK: [u8; 4] = [0x45, 0x44, 0x3e, 255];

/// `text` cut to fit `room` px at `size`, with an ellipsis when cut.
pub fn fit_text(text: &str, size: f32, room: f32) -> String {
    let one_line: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    if text_extent(&one_line, size).0 <= room { return one_line; }
    let mut chars: Vec<char> = one_line.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let s: String = chars.iter().collect::<String>() + "…";
        if text_extent(&s, size).0 <= room { return s; }
    }
    String::new()
}

fn chrome(pm: &mut Pixmap, lay: &Layout, frame: &Frame, clip: &Mask) {
    let (x, y, w, h) = (lay.bar.0 as f32, lay.bar.1 as f32, lay.bar.2 as f32, lay.bar.3 as f32);
    let id = Transform::identity();
    let fill = match frame.chrome { Chrome::Windows => [0xf3, 0xf3, 0xf3, 255], Chrome::Browser => [0xec, 0xeb, 0xe6, 255], _ => [0xf2, 0xf1, 0xed, 255] };
    if let Some(r) = Rect::from_xywh(x, y, w, h) { pm.fill_rect(r, &paint(fill), id, Some(clip)); }
    let size = h * 0.42;
    let dot = h * 0.43 / 2.0;
    let gap = dot * 3.2;
    let dots = |pm: &mut Pixmap| {
        for (i, c) in DOTS.iter().enumerate() {
            let cx = x + h * 0.5 + gap * i as f32;
            if let Some(p) = PathBuilder::from_circle(cx, y + h / 2.0, dot) { pm.fill_path(&p, &paint(*c), FillRule::Winding, id, Some(clip)); }
        }
        x + h * 0.5 + gap * 2.0 + dot + h * 0.4
    };
    match frame.chrome {
        Chrome::None => {}
        Chrome::Mac => {
            let left = dots(pm);
            let room = w - 2.0 * (left - x);
            let t = fit_text(&frame.text, size, room.max(0.0));
            if !t.is_empty() {
                let (tw, th) = text_extent(&t, size);
                glyphs(pm, &t, (x + (w - tw) / 2.0, y + (h - th) / 2.0), size, INK, 1.0);
            }
        }
        Chrome::Windows => {
            let icon = h * 0.28;
            let stroke = Stroke { width: (h / 28.0).max(1.0), ..Stroke::default() };
            let slot = h * 1.3;
            for i in 0..3 {
                let cx = x + w - slot * (i as f32 + 0.5);
                let (cy, s) = (y + h / 2.0, icon / 2.0);
                let mut pb = PathBuilder::new();
                match i {
                    0 => { pb.move_to(cx - s, cy - s); pb.line_to(cx + s, cy + s); pb.move_to(cx + s, cy - s); pb.line_to(cx - s, cy + s); }
                    1 => { if let Some(r) = Rect::from_ltrb(cx - s, cy - s, cx + s, cy + s) { pb.push_rect(r); } }
                    _ => { pb.move_to(cx - s, cy); pb.line_to(cx + s, cy); }
                }
                if let Some(p) = pb.finish() { pm.stroke_path(&p, &paint(INK), &stroke, id, Some(clip)); }
            }
            let left = x + h * 0.5;
            let t = fit_text(&frame.text, size, (w - slot * 3.0 - h * 0.5 - h * 0.5).max(0.0));
            if !t.is_empty() {
                let th = text_extent(&t, size).1;
                glyphs(pm, &t, (left, y + (h - th) / 2.0), size, INK, 1.0);
            }
        }
        Chrome::Browser => {
            let left = dots(pm);
            let bh = h * 0.62;
            let right = x + w - h * 0.5;
            if right - left > bh {
                if let Some(r) = Rect::from_ltrb(left, y + (h - bh) / 2.0, right, y + (h + bh) / 2.0) {
                    if let Some(p) = rounded(r, bh / 2.0) { pm.fill_path(&p, &paint([255, 255, 255, 255]), FillRule::Winding, id, Some(clip)); }
                }
                let t = fit_text(&frame.text, size, (right - left - bh).max(0.0));
                if !t.is_empty() {
                    let th = text_extent(&t, size).1;
                    glyphs(pm, &t, (left + bh / 2.0, y + (h - th) / 2.0), size, INK, 1.0);
                }
            }
        }
    }
}
