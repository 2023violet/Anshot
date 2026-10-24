//! Capture session: frozen desktop pixels, window snapping and the selection,
//! all in physical virtual-desktop pixels. Pure Rust; overlays only render it.
#![forbid(unsafe_code)]
use image::{DynamicImage, ImageFormat, RgbaImage};
use anshot_platform_windows::{Capture, Monitor, ScreenRect};
use std::io::Cursor;

const DRAG_THRESHOLD: i32 = 4;
/// How close (physical px) a press must be to a selection edge to grab it.
const EDGE: i32 = 8;

/// What a press on the overlay grabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grab {
    /// Outside the selection: a new selection (or a click-to-snap).
    New,
    /// Inside the selection: move it.
    Move(ScreenRect),
    /// On an edge or corner: resize it with the opposite edges fixed.
    Edge { rect: ScreenRect, left: bool, right: bool, top: bool, bottom: bool },
}

pub struct Session {
    pub frozen: Capture,
    pub monitors: Vec<Monitor>,
    windows: Vec<ScreenRect>,
    pub selection: Option<ScreenRect>,
    pub hover: Option<ScreenRect>,
    press: Option<((i32, i32), Grab)>,
    current: (i32, i32),
    /// Shift held: a new selection is square, a corner drag keeps the ratio.
    pub lock: bool,
}

pub fn union(monitors: &[Monitor]) -> Option<ScreenRect> {
    let first = monitors.first()?.bounds;
    Some(monitors.iter().fold(first, |a, m| {
        let b = m.bounds;
        let (l, t) = (a.x.min(b.x), a.y.min(b.y));
        let (r, bt) = ((a.x + a.w).max(b.x + b.w), (a.y + a.h).max(b.y + b.h));
        ScreenRect { x: l, y: t, w: r - l, h: bt - t }
    }))
}

fn span(a: (i32, i32), b: (i32, i32)) -> ScreenRect {
    let (l, t) = (a.0.min(b.0), a.1.min(b.1));
    ScreenRect { x: l, y: t, w: (a.0 - b.0).abs().max(1), h: (a.1 - b.1).abs().max(1) }
}

impl Session {
    pub fn new(frozen: Capture, monitors: Vec<Monitor>, windows: Vec<ScreenRect>) -> Self {
        Self { frozen, monitors, windows, selection: None, hover: None, press: None, current: (0, 0), lock: false }
    }
    pub fn monitor_at(&self, x: i32, y: i32) -> Option<Monitor> {
        self.monitors.iter().copied().find(|m| m.bounds.contains(x, y))
    }
    /// Selections never cross a monitor edge (each overlay is one monitor).
    fn clamp(&self, r: ScreenRect, anchor: (i32, i32)) -> Option<ScreenRect> {
        let m = self.monitor_at(anchor.0, anchor.1)?;
        r.intersect(m.bounds)?.intersect(self.frozen.area)
    }
    fn window_at(&self, x: i32, y: i32) -> Option<ScreenRect> {
        let w = self.windows.iter().copied().find(|w| w.contains(x, y))?;
        self.clamp(w, (x, y))
    }
    /// A button is held on the overlay (selecting, moving or resizing).
    pub fn pressed(&self) -> bool { self.press.is_some() }
    pub fn dragging(&self) -> bool {
        self.press.is_some_and(|(p, _)| (p.0 - self.current.0).abs() > DRAG_THRESHOLD || (p.1 - self.current.1).abs() > DRAG_THRESHOLD)
    }
    /// What a press at (x, y) would grab; also drives the overlay's pointer shape.
    pub fn grab_at(&self, x: i32, y: i32) -> Grab {
        let Some(r) = self.selection else { return Grab::New };
        let near = |v: i32, edge: i32| (v - edge).abs() <= EDGE;
        let within = |v: i32, lo: i32, hi: i32| v >= lo - EDGE && v <= hi + EDGE;
        let (l, t, rr, b) = (r.x, r.y, r.x + r.w, r.y + r.h);
        if within(x, l, rr) && within(y, t, b) {
            let (left, right, top, bottom) = (near(x, l), near(x, rr) && !near(x, l), near(y, t), near(y, b) && !near(y, t));
            if left || right || top || bottom { return Grab::Edge { rect: r, left, right, top, bottom }; }
            if x > l && x < rr && y > t && y < b { return Grab::Move(r); }
        }
        Grab::New
    }
    pub fn pointer_move(&mut self, x: i32, y: i32) {
        self.current = (x, y);
        if let Some((p, grab)) = self.press {
            if self.dragging() {
                let (dx, dy) = (x - p.0, y - p.1);
                self.selection = match grab {
                    Grab::New if self.lock => {
                        let side = dx.abs().max(dy.abs());
                        self.clamp(span(p, (p.0 + side * dx.signum(), p.1 + side * dy.signum())), p)
                    }
                    Grab::New => self.clamp(span(p, (x, y)), p),
                    Grab::Edge { rect: r, left, right, top, bottom } if self.lock && (left || right) && (top || bottom) => {
                        // Corner with Shift: follow the horizontal drag, keep the ratio.
                        let w = (r.w + if right { dx } else { -dx }).max(1);
                        let h = (i64::from(w) * i64::from(r.h) / i64::from(r.w.max(1))).max(1) as i32;
                        let (l, t) = (if left { r.x + r.w - w } else { r.x }, if top { r.y + r.h - h } else { r.y });
                        self.clamp(ScreenRect { x: l, y: t, w, h }, (r.x + r.w / 2, r.y + r.h / 2))
                    }
                    Grab::Move(r) => self.shifted(r, dx, dy),
                    Grab::Edge { rect: r, left, right, top, bottom } => {
                        let (mut l, mut t, mut rr, mut b) = (r.x, r.y, r.x + r.w, r.y + r.h);
                        if left { l += dx; } if right { rr += dx; } if top { t += dy; } if bottom { b += dy; }
                        self.clamp(span((l, t), (rr, b)), (r.x + r.w / 2, r.y + r.h / 2))
                    }
                };
            }
        } else if self.selection.is_none() {
            self.hover = self.window_at(x, y).or_else(|| self.monitor_at(x, y).map(|m| m.bounds));
        }
    }
    /// Move a selection, kept whole inside its monitor.
    fn shifted(&self, r: ScreenRect, dx: i32, dy: i32) -> Option<ScreenRect> {
        let m = self.monitor_at(r.x + r.w / 2, r.y + r.h / 2)?.bounds;
        let x = (r.x + dx).clamp(m.x, (m.x + m.w - r.w).max(m.x));
        let y = (r.y + dy).clamp(m.y, (m.y + m.h - r.h).max(m.y));
        Some(ScreenRect { x, y, w: r.w, h: r.h })
    }
    pub fn pointer_down(&mut self, x: i32, y: i32) {
        self.press = Some(((x, y), self.grab_at(x, y)));
        self.current = (x, y);
    }
    /// A click without dragging outside the selection snaps to the window (or monitor)
    /// under the cursor; a click inside keeps the selection.
    pub fn pointer_up(&mut self, x: i32, y: i32) {
        self.current = (x, y);
        let dragged = self.dragging();
        let grab = self.press.take().map(|(_, g)| g);
        if !dragged && grab == Some(Grab::New) { self.selection = self.window_at(x, y).or_else(|| self.monitor_at(x, y).map(|m| m.bounds)); }
        self.hover = None;
    }
    /// Enter: the selection, else the monitor under the cursor.
    pub fn confirm_area(&self) -> Option<ScreenRect> {
        self.selection.or_else(|| self.monitor_at(self.current.0, self.current.1).map(|m| m.bounds))
    }
    /// Colour of the frozen pixel at (x, y), for the magnifier's readout.
    pub fn pixel(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        let a = self.frozen.area;
        if !a.contains(x, y) { return None; }
        let i = ((y - a.y) as usize * a.w as usize + (x - a.x) as usize) * 4;
        self.frozen.rgba.get(i..i + 4).map(|p| [p[0], p[1], p[2], p[3]])
    }
    /// Put back an earlier region (the last capture's), kept on the monitor under its centre.
    /// (Monitors may have changed since: fall back to the one under its corner, then the primary.)
    pub fn restore(&mut self, r: ScreenRect) -> bool {
        let m = self.monitor_at(r.x + r.w / 2, r.y + r.h / 2).or_else(|| self.monitor_at(r.x, r.y))
            .or_else(|| self.monitors.iter().copied().find(|m| m.primary)).map(|m| m.bounds);
        let Some(m) = m else { return false };
        let (w, h) = (r.w.clamp(1, m.w), r.h.clamp(1, m.h));
        let (x, y) = (r.x.clamp(m.x, m.x + m.w - w), r.y.clamp(m.y, m.y + m.h - h));
        self.selection = Some(ScreenRect { x, y, w, h });
        true
    }
    /// Exact size in physical px, keeping the top-left corner where it fits.
    pub fn resize_to(&mut self, w: i32, h: i32) -> bool {
        let Some(r) = self.selection else { return false };
        if w < 1 || h < 1 { return false; }
        let Some(m) = self.monitor_at(r.x + r.w / 2, r.y + r.h / 2) else { return false };
        let (w, h) = (w.min(m.bounds.w), h.min(m.bounds.h));
        self.selection = self.shifted(ScreenRect { x: r.x, y: r.y, w, h }, 0, 0);
        self.selection.is_some()
    }
    /// Change to the ratio `rw:rh` about the centre, keeping the width unless that is too tall.
    pub fn set_ratio(&mut self, rw: i32, rh: i32) -> bool {
        let Some(r) = self.selection else { return false };
        if rw < 1 || rh < 1 { return false; }
        let Some(m) = self.monitor_at(r.x + r.w / 2, r.y + r.h / 2) else { return false };
        let (mut w, mut h) = (r.w, (i64::from(r.w) * i64::from(rh) / i64::from(rw)) as i32);
        if h > m.bounds.h { h = m.bounds.h; w = (i64::from(h) * i64::from(rw) / i64::from(rh)) as i32; }
        let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
        self.selection = self.shifted(ScreenRect { x: cx - w / 2, y: cy - h / 2, w: w.max(1), h: h.max(1) }, 0, 0);
        self.selection.is_some()
    }
    pub fn crop(&self, r: ScreenRect) -> Option<RgbaImage> {
        let r = r.intersect(self.frozen.area)?;
        let (ox, oy) = ((r.x - self.frozen.area.x) as usize, (r.y - self.frozen.area.y) as usize);
        let stride = self.frozen.area.w as usize * 4;
        let mut out = Vec::with_capacity(r.w as usize * r.h as usize * 4);
        for row in 0..r.h as usize {
            let start = (oy + row) * stride + ox * 4;
            out.extend_from_slice(self.frozen.rgba.get(start..start + r.w as usize * 4)?);
        }
        RgbaImage::from_raw(r.w as u32, r.h as u32, out)
    }
}

/// New PNG of the selected pixels for Core intake (no metadata).
pub fn encode_png(img: RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(img).write_to(&mut out, ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mon(x: i32, y: i32, w: i32, h: i32, primary: bool) -> Monitor {
        let r = ScreenRect { x, y, w, h };
        Monitor { bounds: r, work: r, primary }
    }
    fn session() -> Session {
        let monitors = vec![mon(0, 0, 100, 60, true), mon(-80, 10, 80, 50, false)];
        let area = union(&monitors).unwrap();
        let rgba = (0..area.w * area.h).flat_map(|i| [(i % 251) as u8, (i / 251 % 251) as u8, 7, 255]).collect();
        let windows = vec![ScreenRect { x: 10, y: 10, w: 30, h: 20 }, ScreenRect { x: 0, y: 0, w: 100, h: 60 }];
        Session::new(Capture { area, rgba }, monitors, windows)
    }

    #[test]
    fn union_spans_negative_monitor_coordinates() {
        let s = session();
        assert_eq!(s.frozen.area, ScreenRect { x: -80, y: 0, w: 180, h: 60 });
    }

    #[test]
    fn click_snaps_to_topmost_window_and_drag_selects_clamped_to_one_monitor() {
        let mut s = session();
        s.pointer_move(15, 15);
        assert_eq!(s.hover, Some(ScreenRect { x: 10, y: 10, w: 30, h: 20 }));
        s.pointer_down(15, 15);
        s.pointer_up(16, 15);
        assert_eq!(s.selection, Some(ScreenRect { x: 10, y: 10, w: 30, h: 20 }));
        // Drag from the primary monitor past its left edge: clamped at x = 0.
        s.pointer_down(50, 30);
        s.pointer_move(-30, 50);
        s.pointer_up(-30, 50);
        assert_eq!(s.selection, Some(ScreenRect { x: 0, y: 30, w: 50, h: 20 }));
    }

    #[test]
    fn crop_matches_frozen_pixels_including_negative_origin() {
        let s = session();
        let r = ScreenRect { x: -5, y: 12, w: 7, h: 3 };
        let img = s.crop(r).unwrap();
        assert_eq!(img.dimensions(), (7, 3));
        let (ox, oy) = ((r.x - s.frozen.area.x) as usize, (r.y - s.frozen.area.y) as usize);
        let i = oy * s.frozen.area.w as usize + ox;
        assert_eq!(img.get_pixel(0, 0).0, [(i % 251) as u8, (i / 251 % 251) as u8, 7, 255]);
        let png = encode_png(img).unwrap();
        let back = anshot_core::decode_source(&png).unwrap();
        assert_eq!(back.dimensions(), (7, 3));
    }

    #[test]
    fn selection_edges_resize_inside_moves_and_outside_starts_over() {
        let mut s = session();
        s.pointer_down(20, 20); s.pointer_move(60, 40); s.pointer_up(60, 40);
        assert_eq!(s.selection, Some(ScreenRect { x: 20, y: 20, w: 40, h: 20 }));
        // Bottom-right corner: grow.
        assert!(matches!(s.grab_at(60, 40), Grab::Edge { right: true, bottom: true, left: false, top: false, .. }));
        s.pointer_down(60, 40); s.pointer_move(70, 50); s.pointer_up(70, 50);
        assert_eq!(s.selection, Some(ScreenRect { x: 20, y: 20, w: 50, h: 30 }));
        // Inside: move, kept whole on the monitor (right edge at 100).
        s.pointer_down(45, 35); s.pointer_move(85, 35); s.pointer_up(85, 35);
        assert_eq!(s.selection, Some(ScreenRect { x: 50, y: 20, w: 50, h: 30 }));
        // Right edge dragged past the left edge flips instead of vanishing.
        s.pointer_down(100, 30); s.pointer_move(30, 30); s.pointer_up(30, 30);
        assert_eq!(s.selection, Some(ScreenRect { x: 30, y: 20, w: 20, h: 30 }));
        // A click inside keeps it; a drag outside starts a new one.
        s.pointer_down(40, 35); s.pointer_up(40, 35);
        assert_eq!(s.selection, Some(ScreenRect { x: 30, y: 20, w: 20, h: 30 }));
        s.pointer_down(5, 5); s.pointer_move(25, 15); s.pointer_up(25, 15);
        assert_eq!(s.selection, Some(ScreenRect { x: 5, y: 5, w: 20, h: 10 }));
    }

    #[test]
    fn shift_makes_a_square_and_keeps_the_ratio_on_a_corner() {
        let mut s = session();
        s.lock = true;
        s.pointer_down(10, 10); s.pointer_move(40, 22); s.pointer_up(40, 22);
        assert_eq!(s.selection, Some(ScreenRect { x: 10, y: 10, w: 30, h: 30 }));
        s.lock = false;
        s.selection = None;
        s.pointer_down(10, 10); s.pointer_move(50, 30); s.pointer_up(50, 30);
        assert_eq!(s.selection, Some(ScreenRect { x: 10, y: 10, w: 40, h: 20 }));
        s.lock = true;
        s.pointer_down(50, 30); s.pointer_move(70, 31); s.pointer_up(70, 31);
        assert_eq!(s.selection, Some(ScreenRect { x: 10, y: 10, w: 60, h: 30 }), "2:1 kept");
    }

    #[test]
    fn last_region_exact_size_and_ratio() {
        let mut s = session();
        assert!(s.restore(ScreenRect { x: 90, y: 50, w: 30, h: 20 }), "shifted back on screen");
        assert_eq!(s.selection, Some(ScreenRect { x: 70, y: 40, w: 30, h: 20 }));
        assert!(s.resize_to(40, 25));
        assert_eq!(s.selection, Some(ScreenRect { x: 60, y: 35, w: 40, h: 25 }), "kept whole on the monitor");
        assert!(s.set_ratio(16, 9));
        let r = s.selection.unwrap();
        assert_eq!((r.w, r.h), (40, 22));
        assert!(!s.resize_to(0, 5));
        assert_eq!(s.pixel(-80, 0), Some([0, 0, 7, 255]));
        assert_eq!(s.pixel(200, 0), None);
    }

    #[test]
    fn enter_without_selection_takes_the_monitor_under_the_cursor() {
        let mut s = session();
        s.pointer_move(-40, 30);
        assert_eq!(s.confirm_area(), Some(ScreenRect { x: -80, y: 10, w: 80, h: 50 }));
    }
}
