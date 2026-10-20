//! Canvas geometry: where the image sits inside the canvas area, and what a
//! pointer position hits. Pure Rust so it is unit-tested without a window.
#![forbid(unsafe_code)]
use shareready_core::Region;
use shareready_editor::{Handle, Point, Viewport};

/// Visual handles are 8 logical px; the hit box is 16 x 16 (half-open).
pub const HANDLE_HIT_HALF: f64 = 8.0;
const FIT_MARGIN: f64 = 24.0;

/// `Fit` follows the window; `Scale(k)` is an explicit zoom (k = canvas px per source px).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zoom { Fit, Scale(f64) }
pub const MIN_ZOOM: f64 = 0.05;
pub const MAX_ZOOM: f64 = 8.0;

/// Image frame in canvas-local logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View { pub img_w: u32, pub img_h: u32, pub x: f64, pub y: f64, pub scale: f64 }

/// Center an axis when it fits; otherwise honour the requested origin, clamped so the
/// image edge never leaves a gap wider than the margin.
fn axis(len: f64, area: f64, want: Option<f64>) -> f64 {
    if len + 2.0 * FIT_MARGIN <= area { return ((area - len) / 2.0).round(); }
    want.unwrap_or(FIT_MARGIN).clamp(area - len - FIT_MARGIN, FIT_MARGIN).round()
}

impl View {
    pub fn fit_scale(img_w: u32, img_h: u32, area_w: f64, area_h: f64) -> f64 {
        ((area_w - 2.0 * FIT_MARGIN) / f64::from(img_w)).min((area_h - 2.0 * FIT_MARGIN) / f64::from(img_h)).clamp(0.01, 1.0)
    }
    /// `pan` is the requested image origin when the image overflows the area.
    pub fn layout(img_w: u32, img_h: u32, area_w: f64, area_h: f64, zoom: Zoom, pan: Option<(f64, f64)>) -> Self {
        let scale = match zoom { Zoom::Fit => Self::fit_scale(img_w, img_h, area_w, area_h), Zoom::Scale(k) => k.clamp(MIN_ZOOM, MAX_ZOOM) };
        let (w, h) = (f64::from(img_w) * scale, f64::from(img_h) * scale);
        Self { img_w, img_h, x: axis(w, area_w, pan.map(|p| p.0)), y: axis(h, area_h, pan.map(|p| p.1)), scale }
    }
    pub fn width(self) -> f64 { f64::from(self.img_w) * self.scale }
    pub fn height(self) -> f64 { f64::from(self.img_h) * self.scale }
    pub fn viewport(self) -> Viewport {
        Viewport::new(self.img_w, self.img_h, self.x, self.y, self.width(), self.height())
            .expect("layout always yields a positive finite frame")
    }
    /// Source-pixel region to a canvas rectangle (x, y, w, h).
    pub fn rect(self, r: Region) -> (f64, f64, f64, f64) {
        let (l, t) = (r.x0.min(r.x1) as f64, r.y0.min(r.y1) as f64);
        let (rr, b) = (r.x0.max(r.x1) as f64, r.y0.max(r.y1) as f64);
        (self.x + l * self.scale, self.y + t * self.scale, (rr - l) * self.scale, (b - t) * self.scale)
    }
    /// Pointer delta in canvas px to whole source pixels.
    pub fn source_delta(self, from: Point, to: Point) -> (i64, i64) {
        (((to.x - from.x) / self.scale).round() as i64, ((to.y - from.y) / self.scale).round() as i64)
    }
}

pub const HANDLES: [(Handle, f64, f64); 8] = [
    (Handle::Nw, 0.0, 0.0), (Handle::N, 0.5, 0.0), (Handle::Ne, 1.0, 0.0), (Handle::W, 0.0, 0.5),
    (Handle::E, 1.0, 0.5), (Handle::Sw, 0.0, 1.0), (Handle::S, 0.5, 1.0), (Handle::Se, 1.0, 1.0),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_never_upscales_and_centers() {
        let v = View::layout(400, 200, 1000.0, 600.0, Zoom::Fit, None);
        assert_eq!(v.scale, 1.0);
        assert_eq!((v.x, v.y), (300.0, 200.0));
        let big = View::layout(4000, 2000, 1000.0, 600.0, Zoom::Fit, None);
        assert!((big.scale - (952.0 / 4000.0)).abs() < 1e-12);
        assert!(big.width() <= 952.0 + 1e-9 && big.height() <= 552.0 + 1e-9);
    }

    #[test]
    fn overflowing_zoom_pans_within_clamped_bounds() {
        let v = View::layout(1000, 800, 600.0, 400.0, Zoom::Scale(1.0), Some((-300.0, -2000.0)));
        assert_eq!((v.x, v.y), (-300.0, 400.0 - 800.0 - 24.0));
        let v = View::layout(1000, 800, 600.0, 400.0, Zoom::Scale(1.0), Some((500.0, 0.0)));
        assert_eq!(v.x, 24.0, "never more than the margin of gap");
        let small = View::layout(100, 80, 600.0, 400.0, Zoom::Scale(2.0), Some((-999.0, -999.0)));
        assert_eq!((small.x, small.y), (200.0, 120.0), "fitting axes stay centred");
    }

    #[test]
    fn rect_and_delta_round_trip_at_fractional_scale() {
        let v = View::layout(3000, 2000, 1024.0, 700.0, Zoom::Fit, None);
        let (x, y, w, h) = v.rect(Region::new(300, 200, 600, 400));
        assert!((w / v.scale - 300.0).abs() < 1e-9 && (h / v.scale - 200.0).abs() < 1e-9);
        let d = v.source_delta(Point { x, y }, Point { x: x + 10.0 * v.scale, y: y - 3.0 * v.scale });
        assert_eq!(d, (10, -3));
    }
}
