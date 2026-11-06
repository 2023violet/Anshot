//! Step image collection (ADR 0.6-STEPS). Each step is a finished picture (its
//! redactions and annotations already in the pixels, or an unedited capture marked
//! `raw`), kept in memory only as PNG bytes plus two small stand-ins for the panel and
//! the preview. Nothing here is ever written to disk; captions are not remembered.
//! The layout itself is drawn by `anshot_render::render_steps`.
#![forbid(unsafe_code)]
use anshot_render::{Arrangement, Backdrop, Gap, StepPiece, StepsStyle, MAX_STEPS};
use image::{imageops, RgbaImage};
use std::sync::Arc;

/// Longest side of the panel thumbnail.
const THUMB: u32 = 128;
/// Longest side of the stand-in used for the on-screen preview.
const STANDIN: u32 = 800;
/// Longest caption kept (characters).
pub const CAPTION_MAX: usize = 120;

/// A step ready to be added: built off the UI thread by `prepare`.
#[derive(Clone)]
pub struct StepData { pub png: Arc<[u8]>, pub size: (u32, u32), pub raw: bool, pub thumb: Arc<RgbaImage>, pub standin: Arc<RgbaImage> }

#[derive(Clone)]
pub struct Step { pub id: u64, pub data: StepData, pub caption: String }

impl std::fmt::Debug for Step {
    // Never print the caption: it may hold what the user typed about private things.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Step").field("id", &self.id).field("size", &self.data.size).field("raw", &self.data.raw).finish_non_exhaustive()
    }
}

fn shrink(img: &RgbaImage, longest: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let k = (longest as f32 / w.max(h) as f32).min(1.0);
    if k >= 1.0 { return img.clone(); }
    imageops::thumbnail(img, ((w as f32 * k).round() as u32).max(1), ((h as f32 * k).round() as u32).max(1))
}

/// Encode a finished picture as a step (worker thread).
pub fn prepare(img: RgbaImage, raw: bool) -> Result<StepData, String> {
    let size = img.dimensions();
    if size.0 == 0 || size.1 == 0 { return Err("empty".into()); }
    let thumb = Arc::new(shrink(&img, THUMB));
    let standin = Arc::new(shrink(&img, STANDIN));
    let png: Arc<[u8]> = crate::capture::encode_png(img)?.into();
    Ok(StepData { png, size, raw, thumb, standin })
}

#[derive(Debug, PartialEq, Eq)]
pub struct Full;

#[derive(Default)]
pub struct Steps {
    items: Vec<Step>,
    next_id: u64,
    /// The last removed step and where it was, for one undo.
    removed: Option<(usize, Step)>,
    pub style: StepsStyle,
}

impl Steps {
    pub fn items(&self) -> &[Step] { &self.items }
    pub fn len(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn position(&self, id: u64) -> Option<usize> { self.items.iter().position(|s| s.id == id) }
    pub fn get(&self, id: u64) -> Option<&Step> { self.items.iter().find(|s| s.id == id) }
    pub fn any_raw(&self) -> bool { self.items.iter().any(|s| s.data.raw) }
    pub fn can_undo(&self) -> bool { self.removed.is_some() }

    /// Append a step; returns its 1-based number.
    pub fn add(&mut self, data: StepData) -> Result<usize, Full> {
        if self.items.len() >= MAX_STEPS { return Err(Full); }
        self.next_id += 1;
        self.items.push(Step { id: self.next_id, data, caption: String::new() });
        Ok(self.items.len())
    }
    /// New pixels for an existing step (its caption stays); returns its number.
    pub fn replace(&mut self, id: u64, data: StepData) -> Option<usize> {
        let i = self.position(id)?;
        self.items[i].data = data;
        Some(i + 1)
    }
    /// Remove a step; returns the number it had. Only the latest removal can be undone.
    pub fn remove(&mut self, id: u64) -> Option<usize> {
        let i = self.position(id)?;
        self.removed = Some((i, self.items.remove(i)));
        Some(i + 1)
    }
    /// Put the last removed step back where it was; returns its number.
    pub fn undo_remove(&mut self) -> Option<usize> {
        if self.items.len() >= MAX_STEPS { return None; }
        let (i, s) = self.removed.take()?;
        let i = i.min(self.items.len());
        self.items.insert(i, s);
        Some(i + 1)
    }
    pub fn move_by(&mut self, id: u64, delta: isize) -> bool {
        let Some(i) = self.position(id) else { return false };
        let j = i as isize + delta;
        if j < 0 || j as usize >= self.items.len() { return false; }
        self.items.swap(i, j as usize);
        true
    }
    pub fn set_caption(&mut self, id: u64, text: &str) -> bool {
        let Some(i) = self.position(id) else { return false };
        let clean: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).take(CAPTION_MAX).collect();
        if self.items[i].caption == clean { return false; }
        self.items[i].caption = clean;
        true
    }
    pub fn clear(&mut self) {
        self.items.clear();
        self.removed = None;
    }
    /// The real sizes and captions, for `anshot_render::steps_layout`.
    pub fn layout_input(&self) -> (Vec<(u32, u32)>, Vec<String>) {
        (self.items.iter().map(|s| s.data.size).collect(), self.items.iter().map(|s| s.caption.clone()).collect())
    }
    /// Everything a worker needs to draw the preview: stand-ins, real sizes, captions.
    pub fn preview_input(&self) -> Vec<(Arc<RgbaImage>, (u32, u32), String)> {
        self.items.iter().map(|s| (s.data.standin.clone(), s.data.size, s.caption.clone())).collect()
    }
    /// Everything a worker needs for the export: PNG bytes, real sizes, captions.
    pub fn compose_input(&self) -> Vec<(Arc<[u8]>, (u32, u32), String)> {
        self.items.iter().map(|s| (s.data.png.clone(), s.data.size, s.caption.clone())).collect()
    }
}

/// Decode every step and draw the finished step image (worker thread).
pub fn compose(input: &[(Arc<[u8]>, (u32, u32), String)], style: &StepsStyle) -> Result<RgbaImage, String> {
    let images = input.iter().map(|(png, size, _)| {
        let img = anshot_core::decode_source(png)?;
        if img.dimensions() != *size { return Err("step size changed".to_string()); }
        Ok(img)
    }).collect::<Result<Vec<_>, String>>()?;
    let pieces: Vec<StepPiece> = images.iter().zip(input).map(|(image, (_, size, caption))| StepPiece { image, size: *size, caption }).collect();
    anshot_render::render_steps(&pieces, style)
}

/// The preview at `k` of the export size, from the stand-ins (worker thread).
pub fn preview(input: &[(Arc<RgbaImage>, (u32, u32), String)], style: &StepsStyle, k: f32) -> Result<RgbaImage, String> {
    let pieces: Vec<StepPiece> = input.iter().map(|(image, size, caption)| StepPiece { image, size: *size, caption }).collect();
    anshot_render::render_steps_preview(&pieces, style, k)
}

/// `arrangement,gap,badges,captions,backdrop,uniform` as indices, for the settings file.
pub fn render_style(s: &StepsStyle) -> String {
    let at = |v: bool| usize::from(v);
    format!("{},{},{},{},{},{}",
        Arrangement::ALL.iter().position(|a| *a == s.arrangement).unwrap_or(0), Gap::ALL.iter().position(|g| *g == s.gap).unwrap_or(1),
        at(s.badges), at(s.captions), Backdrop::ALL.iter().position(|b| *b == s.backdrop).unwrap_or(0), at(s.uniform))
}

/// `None` unless every field is a valid index.
pub fn parse_style(v: &str) -> Option<StepsStyle> {
    let n: Vec<usize> = v.split(',').map(|p| p.trim().parse().ok()).collect::<Option<_>>()?;
    let [a, g, badges, captions, b, uniform] = n[..] else { return None };
    if badges > 1 || captions > 1 || uniform > 1 { return None; }
    Some(StepsStyle {
        arrangement: *Arrangement::ALL.get(a)?, gap: *Gap::ALL.get(g)?, badges: badges == 1, captions: captions == 1,
        backdrop: *Backdrop::ALL.get(b)?, uniform: uniform == 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn data(w: u32, h: u32) -> StepData { prepare(RgbaImage::from_pixel(w, h, Rgba([10, 20, 30, 255])), false).unwrap() }

    #[test]
    fn add_move_remove_and_undo_keep_the_order() {
        let mut s = Steps::default();
        let ids: Vec<u64> = (0..3).map(|i| { s.add(data(10 + i, 10)).unwrap(); s.items().last().unwrap().id }).collect();
        assert!(s.move_by(ids[2], -1));
        assert!(!s.move_by(ids[0], -1));
        assert_eq!(s.items().iter().map(|x| x.id).collect::<Vec<_>>(), [ids[0], ids[2], ids[1]]);
        assert_eq!(s.remove(ids[2]), Some(2));
        assert_eq!(s.undo_remove(), Some(2));
        assert_eq!(s.undo_remove(), None);
        assert_eq!(s.position(ids[2]), Some(1));
        assert_eq!(s.replace(ids[1], data(50, 40)), Some(3));
        assert_eq!(s.items()[2].data.size, (50, 40));
        s.clear();
        assert!(s.is_empty() && s.undo_remove().is_none());
    }

    #[test]
    fn at_most_twenty_steps() {
        let mut s = Steps::default();
        for n in 1..=MAX_STEPS { assert_eq!(s.add(data(4, 4)), Ok(n)); }
        assert_eq!(s.add(data(4, 4)), Err(Full));
        assert_eq!(s.len(), MAX_STEPS);
    }

    #[test]
    fn captions_are_one_line_and_bounded_and_never_debug_printed() {
        let mut s = Steps::default();
        s.add(data(4, 4)).unwrap();
        let id = s.items()[0].id;
        assert!(s.set_caption(id, "第一行\n第二行"));
        assert_eq!(s.items()[0].caption, "第一行 第二行");
        s.set_caption(id, &"字".repeat(500));
        assert_eq!(s.items()[0].caption.chars().count(), CAPTION_MAX);
        s.set_caption(id, "账号 secret-42");
        assert!(!format!("{:?}", s.items()[0]).contains("secret-42"));
    }

    #[test]
    fn prepare_keeps_the_pixels_and_makes_small_stand_ins() {
        let img = RgbaImage::from_fn(1000, 500, |x, y| Rgba([x as u8, y as u8, 7, 255]));
        let d = prepare(img.clone(), true).unwrap();
        assert_eq!(anshot_core::decode_source(&d.png).unwrap(), img);
        assert_eq!(d.thumb.dimensions(), (128, 64));
        assert_eq!(d.standin.dimensions(), (800, 400));
        assert!(d.raw);
    }

    #[test]
    fn compose_uses_the_stored_pixels() {
        let mut s = Steps::default();
        s.add(data(30, 20)).unwrap();
        s.add(data(40, 10)).unwrap();
        let style = StepsStyle { badges: false, captions: false, ..StepsStyle::default() };
        let out = compose(&s.compose_input(), &style).unwrap();
        let (sizes, captions) = s.layout_input();
        let caps: Vec<&str> = captions.iter().map(String::as_str).collect();
        let l = anshot_render::steps_layout(&sizes, &caps, &style).unwrap();
        assert_eq!(out.dimensions(), l.canvas);
        let (x, y, _, _) = l.steps[1].image;
        assert_eq!(*out.get_pixel(x, y), Rgba([10, 20, 30, 255]));
    }

    #[test]
    fn style_round_trips_and_rejects_bad_values() {
        let odd = StepsStyle { arrangement: Arrangement::Grid, gap: Gap::Large, badges: false, captions: true, backdrop: Backdrop::Dark, uniform: true };
        for s in [StepsStyle::default(), odd] { assert_eq!(parse_style(&render_style(&s)), Some(s)); }
        assert_eq!(render_style(&StepsStyle::default()), "0,1,1,1,0,0");
        for bad in ["", "0,1,1,1,0", "3,1,1,1,0,0", "0,3,1,1,0,0", "0,1,2,1,0,0", "0,1,1,1,3,0", "0,1,1,1,0,2", "x,1,1,1,0,0"] {
            assert_eq!(parse_style(bad), None, "{bad}");
        }
    }
}
