//! Long screenshot: stitch successive captures of one screen region while the user
//! scrolls the content underneath. Pure Rust, no OS calls.
//!
//! Each row is reduced to a small signature (mean brightness of 16 horizontal
//! segments) so that matching tolerates the slight resampling some apps apply while
//! scrolling. Rows that stay put between frames at the top (a sticky header) or bottom
//! (a sticky footer) are recognised and kept once; the scrolled body in between is
//! matched by its vertical offset, and only the newly revealed rows are appended, taken
//! from the captured pixels. Frames that do not overlap the previous one (scrolled too
//! fast) are rejected rather than guessed.
//!
//! When only part of the region's width scrolls (a sidebar or a second pane stays put),
//! the first scroll narrows the stitch to the band of columns that moved; later frames are
//! cut to the same columns. A still column only counts as another pane if it has content
//! (blank margins scroll with the page as far as anyone can tell). A narrowed result is
//! reported so the UI can say so.
#![forbid(unsafe_code)]
use image::RgbaImage;

/// Smallest overlap (rows) accepted as a match.
const MIN_OVERLAP: usize = 12;
/// Signature segments per row.
const SEGMENTS: usize = 16;
/// Mean signature difference (0-255 scale) below which two rows count as the same.
const ROW_TOL: f32 = 1.5;
/// Mean difference over the overlap accepted for a scroll match.
const MATCH_TOL: f32 = 2.0;
/// Narrowest scrolling column band worth stitching on its own (px).
const MIN_BAND: u32 = 48;
/// Narrowest band of still, non-blank columns treated as a separate pane (px).
const MIN_PANE: u32 = 16;
/// Largest per-channel sum difference for a pixel to count as unchanged in place.
const PX_TOL: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Push {
    /// Same picture as before: nothing scrolled.
    Unchanged,
    /// `n` new rows were added.
    Appended(u32),
    /// No overlap with the previous frame (scrolled too fast or content changed).
    Lost,
    /// The pixel limit is reached; further frames are ignored.
    Full,
}

type Sig = [f32; SEGMENTS];

pub struct Stitcher {
    width: u32,
    /// Stitched pixels so far, row-major RGBA, *without* the sticky footer.
    body: Vec<u8>,
    rows: u32,
    /// Sticky footer rows seen in the last accepted frame (appended at the end).
    footer: Vec<u8>,
    /// Footer height, fixed at the first scroll so body rows are never cut twice.
    footer_rows: Option<usize>,
    last: RgbaImage,
    last_sigs: Vec<Sig>,
    max_pixels: u64,
    full: bool,
    /// Columns `[x0, x1)` of each captured frame that are stitched (all, unless narrowed).
    cols: (u32, u32),
    /// Set once the content has scrolled; narrowing is only tried before that.
    scrolled: bool,
    /// Width of the captured frames.
    frame_w: u32,
}

fn signatures(img: &RgbaImage) -> Vec<Sig> {
    let w = img.width() as usize;
    let stride = w * 4;
    img.as_raw().chunks(stride).map(|r| {
        let mut sig = [0f32; SEGMENTS];
        for (k, v) in sig.iter_mut().enumerate() {
            let (a, b) = (k * w / SEGMENTS, ((k + 1) * w / SEGMENTS).max(k * w / SEGMENTS + 1).min(w));
            let sum: u32 = (a..b).map(|x| { let p = &r[x * 4..x * 4 + 3]; (u32::from(p[0]) * 3 + u32::from(p[1]) * 6 + u32::from(p[2])) / 10 }).sum();
            *v = sum as f32 / (b - a).max(1) as f32;
        }
        sig
    }).collect()
}
fn row_dist(a: &Sig, b: &Sig) -> f32 { a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>() / SEGMENTS as f32 }

/// Mean row difference for a shift of `dy` rows within `rows` (indices into the frame),
/// ignoring the worst quarter (small animations must not hide the real shift).
fn trimmed_cost(prev: &[Sig], cur: &[Sig], lo: usize, hi: usize, dy: usize) -> f32 {
    let mut d: Vec<f32> = (lo..hi - dy).map(|i| row_dist(&prev[i + dy], &cur[i])).collect();
    d.sort_by(f32::total_cmp);
    let keep = (d.len() * 3 / 4).max(1);
    d[..keep].iter().sum::<f32>() / keep as f32
}

enum Motion { Still, Scrolled(usize), Lost }

/// How the content moved between two frames of the same size.
fn motion(prev: &[Sig], cur: &[Sig]) -> Motion {
    let h = prev.len();
    // Rows that did not change in place at the very top / bottom are candidates for
    // sticky bars; leave them out of the shift search.
    let same = |i: usize| row_dist(&prev[i], &cur[i]) <= ROW_TOL;
    // Too short to tell a scroll from a change: only "nothing moved" is safe to accept.
    if h <= MIN_OVERLAP * 2 { return if (0..h).all(same) { Motion::Still } else { Motion::Lost }; }
    let lo = (0..h).take_while(|&i| same(i)).count();
    let hi = h - (0..h - lo).take_while(|&k| same(h - 1 - k)).count();
    if hi - lo <= MIN_OVERLAP { return Motion::Still; }
    let still = trimmed_cost(prev, cur, lo, hi, 0);
    let mut best: Option<(usize, f32)> = None;
    for dy in 1..=hi - lo - MIN_OVERLAP {
        let c = trimmed_cost(prev, cur, lo, hi, dy);
        if best.is_none_or(|(_, b)| c < b - 0.01) { best = Some((dy, c)); }
        if c == 0.0 { break; }
    }
    match best {
        // A real scroll explains the frame clearly better than "nothing moved".
        Some((dy, c)) if c <= MATCH_TOL && c < still * 0.6 => Motion::Scrolled(dy),
        _ if still <= MATCH_TOL => Motion::Still,
        _ => Motion::Lost,
    }
}

/// Sticky rows for a known shift at the top / bottom of the frame.
fn fixed_bands(prev: &[Sig], cur: &[Sig], dy: usize) -> (usize, usize) {
    let h = prev.len();
    // Candidate bands: runs of rows unchanged in place from each edge. A run is sticky
    // only if at least one of its rows contradicts the scroll (a uniform bar always has one
    // at its inner edge; blank page content moves consistently and never does).
    let in_place = |i: usize| row_dist(&prev[i], &cur[i]) <= ROW_TOL;
    let against_scroll = |i: usize| (i + dy < h && row_dist(&prev[i + dy], &cur[i]) > ROW_TOL)
        || (i >= dy && row_dist(&prev[i], &cur[i - dy]) > ROW_TOL);
    let top_run = (0..h).take_while(|&i| in_place(i)).count();
    let top = if (0..top_run).any(against_scroll) { top_run } else { 0 };
    let bottom_run = (0..h - top).take_while(|&k| in_place(h - 1 - k)).count();
    let bottom = if (0..bottom_run).any(|k| against_scroll(h - 1 - k)) { bottom_run } else { 0 };
    (top, bottom)
}

impl Stitcher {
    /// Start from the first frame. `max_pixels` caps width × stitched height.
    pub fn new(first: RgbaImage, max_pixels: u64) -> Self {
        let width = first.width();
        let rows = first.height();
        let sigs = signatures(&first);
        Self { width, body: first.as_raw().clone(), rows, footer: Vec::new(), footer_rows: None, last: first, last_sigs: sigs, max_pixels, full: false, cols: (0, width), scrolled: false, frame_w: width }
    }
    /// The captured columns kept in the result, when only part of the region scrolled.
    pub fn narrowed(&self) -> Option<(u32, u32)> { (self.cols != (0, self.frame_w)).then_some(self.cols) }
    pub fn height(&self) -> u32 { self.rows + (self.footer.len() / (self.width as usize * 4)) as u32 }

    pub fn push(&mut self, frame: RgbaImage) -> Push {
        if self.full { return Push::Full; }
        if frame.width() != self.frame_w { return Push::Lost; }
        let frame = if self.cols == (0, self.frame_w) { frame } else {
            image::imageops::crop_imm(&frame, self.cols.0, 0, self.cols.1 - self.cols.0, frame.height()).to_image()
        };
        if frame.dimensions() != self.last.dimensions() { return Push::Lost; }
        let mut frame = frame;
        let mut cur = signatures(&frame);
        let full = motion(&self.last_sigs, &cur);
        if matches!(full, Motion::Still) { return Push::Unchanged; }
        // Before the first scroll, a still pane beside the moving one must not be stitched
        // as if it scrolled (a low-contrast sidebar can still "match" a full-width shift).
        let dy = match (self.narrow(&frame), full) {
            (Some((f, c, dy)), _) => { frame = f; cur = c; dy }
            (None, Motion::Scrolled(dy)) => dy,
            _ => return Push::Lost,
        };
        self.scrolled = true;
        let (top, found) = fixed_bands(&self.last_sigs, &cur, dy);
        let first_scroll = self.footer_rows.is_none();
        let bottom = *self.footer_rows.get_or_insert(found);
        // The newly revealed body rows must exist between the sticky bands.
        if top + bottom + dy > cur.len() { return Push::Lost; }
        let stride = self.width as usize * 4;
        let h = frame.height() as usize;
        // On the first move the sticky footer becomes known: take it off the body once.
        if first_scroll && bottom > 0 {
            let cut = bottom.min(self.rows as usize);
            self.body.truncate(self.body.len() - cut * stride);
            self.rows -= cut as u32;
        }
        // New rows are the last `dy` rows of the body area of the current frame.
        let (from, to) = (h - bottom - dy, h - bottom);
        let mut take = dy;
        let room = (self.max_pixels / u64::from(self.width.max(1))).saturating_sub(u64::from(self.rows) + bottom as u64) as usize;
        if take >= room { take = room; self.full = true; }
        self.body.extend_from_slice(&frame.as_raw()[from * stride..(from + take) * stride]);
        self.rows += take as u32;
        self.footer = frame.as_raw()[to * stride..h * stride].to_vec();
        self.last_sigs = cur;
        self.last = frame;
        if self.full { Push::Full } else { Push::Appended(take as u32) }
    }

    /// Before anything has scrolled: if a band of columns moved while the rest stayed in
    /// place, restart the stitch on that band and return the cut frame, its signatures and
    /// the shift.
    fn narrow(&mut self, frame: &RgbaImage) -> Option<(RgbaImage, Vec<Sig>, usize)> {
        if self.scrolled || self.cols != (0, self.width) { return None; }
        let (w, h) = frame.dimensions();
        let near = |a: [u8; 4], b: [u8; 4]| (0..3).map(|i| u32::from(a[i].abs_diff(b[i]))).sum::<u32>() <= PX_TOL;
        // Rows that stayed in place across the whole width at the top / bottom are sticky
        // bars; judge columns only between them, so a margin under a full-width header
        // still counts as blank.
        let (ps, cs) = (&self.last_sigs, signatures(frame));
        let same = |i: usize| row_dist(&ps[i], &cs[i]) <= ROW_TOL;
        let lo = (0..h as usize).take_while(|&i| same(i)).count() as u32;
        let hi = h - (0..(h - lo) as usize).take_while(|&k| same(h as usize - 1 - k)).count() as u32;
        if hi <= lo { return None; }
        // A column is another pane if it stayed in place and is not blank.
        let pane = |x: u32| {
            let first = self.last.get_pixel(x, lo).0;
            (lo..hi).all(|y| near(self.last.get_pixel(x, y).0, frame.get_pixel(x, y).0))
                && (lo..hi).any(|y| !near(self.last.get_pixel(x, y).0, first))
        };
        let panes: Vec<bool> = (0..w).map(pane).collect();
        // Only bands of pane columns wide enough to be real panes count.
        let mut keep = vec![true; w as usize];
        let mut x = 0;
        while x < w as usize {
            if !panes[x] { x += 1; continue; }
            let s0 = x;
            while x < w as usize && panes[x] { x += 1; }
            if (x - s0) as u32 >= MIN_PANE { keep[s0..x].iter_mut().for_each(|k| *k = false); }
        }
        // Longest run of the remaining columns.
        let (mut best, mut start) = ((0u32, 0u32), None);
        for x in 0..=w {
            let k = x < w && keep[x as usize];
            match (k, start) {
                (true, None) => start = Some(x),
                (false, Some(s0)) => { if x - s0 > best.1 - best.0 { best = (s0, x); } start = None; }
                _ => {}
            }
        }
        let (x0, x1) = best;
        if x1 - x0 < MIN_BAND || x1 - x0 == w { return None; }
        let cut = |img: &RgbaImage| image::imageops::crop_imm(img, x0, 0, x1 - x0, h).to_image();
        let (first, now) = (cut(&self.last), cut(frame));
        let (ps, cs) = (signatures(&first), signatures(&now));
        let Motion::Scrolled(dy) = motion(&ps, &cs) else { return None };
        *self = Self { cols: (x0, x1), frame_w: w, ..Self::new(first, self.max_pixels) };
        Some((now, cs, dy))
    }

    /// The stitched image: body followed by the sticky footer.
    pub fn finish(self) -> Option<RgbaImage> {
        let mut px = self.body;
        px.extend_from_slice(&self.footer);
        let h = (px.len() / (self.width as usize * 4)) as u32;
        RgbaImage::from_raw(self.width, h, px)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// A tall page whose rows are all different, with a 20-row header and 10-row footer
    /// that stay fixed in the viewport.
    fn page(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| Rgba([(y.wrapping_mul(37) % 256) as u8, (y.wrapping_mul(91) / 7 % 256) as u8, ((x * 7) ^ (y * 13)) as u8, 255]))
    }
    fn viewport(page: &RgbaImage, scroll: u32, vh: u32, header: u32, footer: u32) -> RgbaImage {
        let w = page.width();
        RgbaImage::from_fn(w, vh, |x, y| {
            if y < header { Rgba([200, 10, 10, 255]) }
            else if y >= vh - footer { Rgba([10, 10, 200, 255]) }
            else { *page.get_pixel(x, scroll + y - header) }
        })
    }

    #[test]
    fn stitches_scrolled_frames_keeping_header_and_footer_once() {
        let p = page(40, 1000);
        let (vh, hd, ft) = (200, 20, 10);
        let mut s = Stitcher::new(viewport(&p, 0, vh, hd, ft), u64::MAX);
        assert_eq!(s.push(viewport(&p, 0, vh, hd, ft)), Push::Unchanged);
        let mut scroll = 0;
        for step in [37, 80, 1, 120, 45] {
            scroll += step;
            assert_eq!(s.push(viewport(&p, scroll, vh, hd, ft)), Push::Appended(step), "step {step}");
        }
        let out = s.finish().unwrap();
        let body = vh - hd - ft;
        assert_eq!(out.height(), hd + scroll + body + ft);
        let expected = RgbaImage::from_fn(40, out.height(), |x, y| {
            if y < hd { Rgba([200, 10, 10, 255]) } else if y >= out.height() - ft { Rgba([10, 10, 200, 255]) } else { *p.get_pixel(x, y - hd) }
        });
        assert!(out == expected, "stitched pixels differ from the page");
    }

    #[test]
    fn tolerates_slight_resampling_noise() {
        let p = page(40, 800);
        let noisy = |scroll: u32, seed: u32| RgbaImage::from_fn(40, 150, |x, y| {
            let c = p.get_pixel(x, scroll + y).0;
            let n = ((x * 7 + y * 13 + seed) % 3) as u8; // 0..2 per channel
            Rgba([c[0].saturating_add(n), c[1].saturating_sub(n), c[2].saturating_add(n), 255])
        });
        let mut s = Stitcher::new(noisy(0, 1), u64::MAX);
        assert_eq!(s.push(noisy(0, 2)), Push::Unchanged, "noise alone is not a scroll");
        assert_eq!(s.push(noisy(55, 3)), Push::Appended(55));
        assert_eq!(s.push(noisy(140, 4)), Push::Appended(85));
        assert_eq!(s.finish().unwrap().height(), 290);
    }

    #[test]
    fn blank_rows_at_the_edge_are_not_a_sticky_footer() {
        // Content with blank (white) stretches, scrolled so blank rows sit at the bottom.
        let p = RgbaImage::from_fn(60, 900, |x, y| if (y / 40) % 3 == 2 { Rgba([255, 255, 255, 255]) } else { Rgba([(y * 37 % 256) as u8, (x * 3) as u8, (y * 11 % 256) as u8, 255]) });
        let mut s = Stitcher::new(viewport(&p, 0, 200, 0, 0), u64::MAX);
        let mut scroll = 0;
        for step in [120, 120, 120] { scroll += step; assert_eq!(s.push(viewport(&p, scroll, 200, 0, 0)), Push::Appended(step)); }
        let out = s.finish().unwrap();
        assert_eq!(out.height(), 200 + scroll);
        assert!(out == image::imageops::crop_imm(&p, 0, 0, 60, 200 + scroll).to_image());
    }

    #[test]
    fn rejects_a_jump_without_overlap_and_recovers() {
        let p = page(30, 2000);
        let mut s = Stitcher::new(viewport(&p, 0, 120, 0, 0), u64::MAX);
        assert_eq!(s.push(viewport(&p, 500, 120, 0, 0)), Push::Lost);
        assert_eq!(s.push(viewport(&p, 60, 120, 0, 0)), Push::Appended(60));
        assert_eq!(s.finish().unwrap().height(), 180);
    }

    #[test]
    fn stops_at_the_pixel_limit() {
        let p = page(10, 3000);
        let mut s = Stitcher::new(viewport(&p, 0, 100, 0, 0), 10 * 250);
        assert_eq!(s.push(viewport(&p, 60, 100, 0, 0)), Push::Appended(60));
        assert_eq!(s.push(viewport(&p, 120, 100, 0, 0)), Push::Appended(60));
        assert_eq!(s.push(viewport(&p, 180, 100, 0, 0)), Push::Full);
        assert_eq!(s.push(viewport(&p, 240, 100, 0, 0)), Push::Full);
        assert_eq!(s.finish().unwrap().height(), 250);
    }

    /// A still sidebar on the left of a scrolling pane (with its own sticky header).
    fn split(page: &RgbaImage, scroll: u32, vh: u32, side: u32) -> RgbaImage {
        let pane = viewport(page, scroll, vh, 20, 0);
        RgbaImage::from_fn(side + page.width(), vh, |x, y| if x < side { Rgba([(y * 5 % 256) as u8, 120, (x * 9 % 256) as u8, 255]) } else { *pane.get_pixel(x - side, y) })
    }

    #[test]
    fn narrows_to_the_pane_that_scrolls() {
        let p = page(120, 1200);
        let (vh, side) = (220, 70);
        let mut s = Stitcher::new(split(&p, 0, vh, side), u64::MAX);
        let mut scroll = 0;
        for step in [60, 90, 75] {
            scroll += step;
            assert_eq!(s.push(split(&p, scroll, vh, side)), Push::Appended(step), "step {step}");
        }
        assert_eq!(s.narrowed(), Some((side, side + 120)));
        let out = s.finish().unwrap();
        assert_eq!(out.dimensions(), (120, vh + scroll));
        let expected = RgbaImage::from_fn(120, vh + scroll, |x, y| if y < 20 { Rgba([200, 10, 10, 255]) } else { *p.get_pixel(x, y - 20) });
        assert!(out == expected, "stitched pane differs from the page");
    }

    #[test]
    fn a_faint_sidebar_is_not_stitched_as_if_it_scrolled() {
        let p = page(120, 1200);
        let faint = |scroll: u32| {
            let pane = viewport(&p, scroll, 200, 0, 0);
            RgbaImage::from_fn(60 + 120, 200, |x, y| if x < 60 { Rgba([240, 240, (230 + y / 20) as u8, 255]) } else { *pane.get_pixel(x - 60, y) })
        };
        let mut s = Stitcher::new(faint(0), u64::MAX);
        assert_eq!(s.push(faint(50)), Push::Appended(50));
        assert_eq!(s.narrowed(), Some((60, 180)));
        assert!(s.finish().unwrap() == image::imageops::crop_imm(&p, 0, 0, 120, 250).to_image());
    }

    #[test]
    fn margins_under_a_full_width_header_stay_in_the_picture() {
        let p = page(100, 900);
        let framed = |scroll: u32| {
            let v = viewport(&p, scroll, 180, 0, 0);
            RgbaImage::from_fn(160, 200, |x, y| if y < 20 { Rgba([200, 10, 10, 255]) } else if !(30..130).contains(&x) { Rgba([255, 255, 255, 255]) } else { *v.get_pixel(x - 30, y - 20) })
        };
        let mut s = Stitcher::new(framed(0), u64::MAX);
        assert_eq!(s.push(framed(40)), Push::Appended(40));
        assert_eq!(s.narrowed(), None);
        assert_eq!(s.finish().unwrap().dimensions(), (160, 240));
    }

    #[test]
    fn blank_margins_stay_in_the_picture() {
        let p = page(100, 900);
        let framed = |scroll: u32| { let v = viewport(&p, scroll, 160, 0, 0); RgbaImage::from_fn(140, 160, |x, y| if !(20..120).contains(&x) { Rgba([255, 255, 255, 255]) } else { *v.get_pixel(x - 20, y) }) };
        let mut s = Stitcher::new(framed(0), u64::MAX);
        assert_eq!(s.push(framed(30)), Push::Appended(30));
        assert_eq!(s.narrowed(), None);
        assert_eq!(s.finish().unwrap().width(), 140);
    }

    #[test]
    fn a_full_width_scroll_is_not_narrowed() {
        let p = page(80, 900);
        let mut s = Stitcher::new(viewport(&p, 0, 150, 0, 0), u64::MAX);
        assert_eq!(s.push(viewport(&p, 40, 150, 0, 0)), Push::Appended(40));
        assert_eq!(s.narrowed(), None);
    }

    #[test]
    fn size_changes_and_tiny_regions_are_refused() {
        let p = page(10, 300);
        let mut s = Stitcher::new(viewport(&p, 0, 10, 0, 0), u64::MAX);
        assert_eq!(s.push(viewport(&p, 3, 10, 0, 0)), Push::Lost, "too few rows to match safely");
        assert_eq!(s.push(RgbaImage::new(5, 10)), Push::Lost);
    }
}

