//! Custom colour picker maths (HSV <-> RGB, hex). The chosen colour only styles
//! annotations; it never touches redaction pixels.
#![forbid(unsafe_code)]

/// h, s, v in 0..=1 -> opaque RGBA.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 4] {
    let (h, s, v) = (h.rem_euclid(1.0) * 6.0, s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x), 3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    let m = v - c;
    let q = |f: f32| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    [q(r), q(g), q(b), 255]
}

pub fn rgb_to_hsv(c: [u8; 4]) -> (f32, f32, f32) {
    let (r, g, b) = (f32::from(c[0]) / 255.0, f32::from(c[1]) / 255.0, f32::from(c[2]) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    let h = if d == 0.0 { 0.0 }
        else if max == r { ((g - b) / d).rem_euclid(6.0) / 6.0 }
        else if max == g { ((b - r) / d + 2.0) / 6.0 }
        else { ((r - g) / d + 4.0) / 6.0 };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

/// "#RRGGBB" or "RRGGBB" (case-insensitive); anything else is rejected.
pub fn parse_hex(text: &str) -> Option<[u8; 4]> {
    let t = text.trim().trim_start_matches('#');
    if t.len() != 6 || !t.chars().all(|c| c.is_ascii_hexdigit()) { return None; }
    let n = |i: usize| u8::from_str_radix(&t[i..i + 2], 16).ok();
    Some([n(0)?, n(2)?, n(4)?, 255])
}

pub fn to_hex(c: [u8; 4]) -> String { format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conversions_round_trip() {
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), [255, 0, 0, 255]);
        assert_eq!(hsv_to_rgb(1.0 / 3.0, 1.0, 1.0), [0, 255, 0, 255]);
        assert_eq!(hsv_to_rgb(0.5, 0.0, 0.5), [128, 128, 128, 255]);
        for c in [[0x9b, 0x30, 0xd9, 255], [0x12, 0x34, 0x56, 255], [255, 255, 255, 255], [0, 0, 0, 255]] {
            let (h, s, v) = rgb_to_hsv(c);
            assert_eq!(hsv_to_rgb(h, s, v), c);
        }
        assert_eq!(parse_hex("#9b30D9"), Some([0x9b, 0x30, 0xd9, 255]));
        assert_eq!(parse_hex("9B30D9"), Some([0x9b, 0x30, 0xd9, 255]));
        for bad in ["", "#12345", "#1234567", "zzzzzz", "#12 456"] { assert_eq!(parse_hex(bad), None, "{bad}"); }
        assert_eq!(to_hex([0x9b, 0x30, 0xd9, 255]), "#9B30D9");
    }
}
