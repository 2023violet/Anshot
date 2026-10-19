//! Canvas geometry: where the image sits inside the canvas area, and what a
//! pointer position hits. Pure Rust so it is unit-tested without a window.
#![forbid(unsafe_code)]
use shareready_core::Region;
use shareready_editor::{Handle, Point, Viewport};

/// Visual handles are 8 logical px; the hit box is 16 x 16 (half-open).
pub const HANDLE_HIT_HALF: f64 = 8.0;
const FIT_MARGIN: f64 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zoom { Fit, Actual }

/// Image frame in canvas-local logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View { pub img_w: u32, pub img_h: u32, pub x: f64, pub y: f64, pub scale: f64 }

impl View {
    /// Fit inside the area with a margin, never enlarging past 100%.
    pub fn layout(img_w: u32, img_h: u32, area_w: f64, area_h: f64, zoom: Zoom) -> Self {
        let fit = ((area_w - 2.0 * FIT_MARGIN) / f64::from(img_w))
            .min((area_h - 2.0 * FIT_MARGIN) / f64::from(img_h))
            .clamp(0.01, 1.0);
        let scale = match zoom { Zoom::Fit => fit, Zoom::Actual => 1.0 };
        let (w, h) = (f64::from(img_w) * scale, f64::from(img_h) * scale);
        // Center when it fits; otherwise pin to the margin (B1 has no panning yet).
        let x = if w + 2.0 * FIT_MARGIN <= area_w { ((area_w - w) / 2.0).round() } else { FIT_MARGIN };
        let y = if h + 2.0 * FIT_MARGIN <= area_h { ((area_h - h) / 2.0).round() } else { FIT_MARGIN };
        Self { img_w, img_h, x, y, scale }
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
        let v = View::layout(400, 200, 1000.0, 600.0, Zoom::Fit);
        assert_eq!(v.scale, 1.0);
        assert_eq!((v.x, v.y), (300.0, 200.0));
        let big = View::layout(4000, 2000, 1000.0, 600.0, Zoom::Fit);
        assert!((big.scale - (952.0 / 4000.0)).abs() < 1e-12);
        assert!(big.width() <= 952.0 + 1e-9 && big.height() <= 552.0 + 1e-9);
    }

    #[test]
    fn rect_and_delta_round_trip_at_fractional_scale() {
        let v = View::layout(3000, 2000, 1024.0, 700.0, Zoom::Fit);
        let (x, y, w, h) = v.rect(Region::new(300, 200, 600, 400));
        assert!((w / v.scale - 300.0).abs() < 1e-9 && (h / v.scale - 200.0).abs() < 1e-9);
        let d = v.source_delta(Point { x, y }, Point { x: x + 10.0 * v.scale, y: y - 3.0 * v.scale });
        assert_eq!(d, (10, -3));
    }
}
