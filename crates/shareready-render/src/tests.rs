use super::*;
use image::Rgba;

fn source(w: u32, h: u32, seed: u8) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| Rgba([(x as u8).wrapping_mul(3) ^ seed, (y as u8).wrapping_mul(5), seed, 255]))
}
fn png(img: &RgbaImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(img.clone()).write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}
/// Order-sensitive 64-bit digest for frozen render goldens.
pub(crate) fn digest(img: &RgbaImage) -> u64 {
    img.as_raw().chunks(8).fold(0xcbf2_9ce4_8422_2325u64, |h, c| {
        let mut b = [0u8; 8];
        b[..c.len()].copy_from_slice(c);
        mix(h ^ u64::from_le_bytes(b))
    }) ^ (u64::from(img.width()) << 32 | u64::from(img.height()))
}

pub(crate) fn golden_scene() -> Scene {
    Scene {
        redactions: vec![
            Redaction { region: Region::new(20, 30, 140, 60), style: RedactStyle::Solid([0x1a, 0x19, 0x15, 255]) },
            Redaction { region: Region::new(160, 30, 300, 60), style: RedactStyle::SafeMosaic },
        ],
        annotations: vec![
            Annotation { shape: Shape::Rect { region: Region::new(10, 80, 200, 150), width: 4.0 }, color: [0xe5, 0x34, 0x2b, 255] },
            Annotation { shape: Shape::Arrow { from: (300.0, 180.0), to: (210.0, 120.0), width: 4.0 }, color: [0xe5, 0x34, 0x2b, 255] },
            Annotation { shape: Shape::Highlight { region: Region::new(220, 70, 310, 95) }, color: [0xff, 0xc6, 0x0a, 255] },
            Annotation { shape: Shape::Number { center: (200.0, 80.0), n: 1, radius: 13.0 }, color: [0xe5, 0x34, 0x2b, 255] },
            Annotation { shape: Shape::Text { origin: (20.0, 160.0), text: "这里看凭证 OK".into(), size: 18.0, background: false }, color: [0x2f, 0x6f, 0xf0, 255] },
            Annotation { shape: Shape::Text { origin: (180.0, 200.0), text: "备注".into(), size: 14.0, background: true }, color: [0x22, 0xa6, 0x52, 255] },
        ],
        crop: None,
    }
}

#[test]
fn solid_redaction_replaces_exact_half_open_pixels_only() {
    let mut img = source(64, 48, 9);
    let before = img.clone();
    let scene = Scene { redactions: vec![Redaction { region: Region::new(8, 6, 20, 10), style: RedactStyle::Solid([1, 2, 3, 255]) }], ..Scene::default() };
    render_into(&mut img, &scene, 1.0);
    for (x, y, p) in img.enumerate_pixels() {
        if (8..20).contains(&x) && (6..10).contains(&y) { assert_eq!(p.0, [1, 2, 3, 255]); }
        else { assert_eq!(p, before.get_pixel(x, y)); }
    }
}

#[test]
fn safe_mosaic_pixels_are_independent_of_the_source() {
    let scene = Scene { redactions: vec![Redaction { region: Region::new(5, 5, 45, 33), style: RedactStyle::SafeMosaic }], ..Scene::default() };
    let (mut a, mut b) = (source(64, 48, 1), source(64, 48, 200));
    render_into(&mut a, &scene, 1.0);
    render_into(&mut b, &scene, 1.0);
    let mut colours = std::collections::BTreeSet::new();
    for y in 5..33 { for x in 5..45 {
        assert_eq!(a.get_pixel(x, y), b.get_pixel(x, y), "({x},{y}) leaked source");
        assert!(MOSAIC.contains(&a.get_pixel(x, y).0));
        colours.insert(a.get_pixel(x, y).0);
    }}
    assert!(colours.len() >= 2, "mosaic should not be a flat block");
}

#[test]
fn preview_scale_rounds_redactions_outward() {
    let mut img = RgbaImage::from_pixel(50, 50, Rgba([255, 255, 255, 255]));
    let scene = Scene { redactions: vec![Redaction { region: Region::new(3, 3, 7, 7), style: RedactStyle::Solid([0, 0, 0, 255]) }], ..Scene::default() };
    render_into(&mut img, &scene, 0.3); // 0.9..2.1 -> 0..3
    for y in 0..3 { for x in 0..3 { assert_eq!(img.get_pixel(x, y).0, [0, 0, 0, 255]); } }
    assert_eq!(img.get_pixel(3, 3).0, [255, 255, 255, 255]);
}

#[test]
fn annotations_draw_on_top_and_text_renders_cjk_glyphs() {
    let mut img = RgbaImage::from_pixel(320, 240, Rgba([255, 255, 255, 255]));
    render_into(&mut img, &golden_scene(), 1.0);
    // Rect stroke centre lies 2px inside the region edge.
    assert_eq!(img.get_pixel(100, 82).0, [0xe5, 0x34, 0x2b, 255]);
    // Highlight is translucent: blended, not opaque.
    let hl = img.get_pixel(300, 72).0;
    assert!(hl[2] < 255 && hl[0] == 255, "{hl:?}");
    // Blue text produced some strongly blue pixels in its box.
    let (x0, y0, x1, y1) = bounds(&golden_scene().annotations[4]);
    let blue = (x0 as u32..x1 as u32).flat_map(|x| (y0 as u32..y1 as u32).map(move |y| (x, y)))
        .filter(|&(x, y)| { let p = img.get_pixel(x, y).0; p[2] > 200 && p[0] < 120 }).count();
    assert!(blue > 60, "text pixels {blue}");
    // Redactions stay exact even with annotations present.
    assert_eq!(img.get_pixel(20, 30).0, [0x1a, 0x19, 0x15, 255]);
}

#[test]
fn render_png_crops_and_reopens_without_requiring_redactions() {
    let src = png(&source(120, 90, 4));
    let scene = Scene { crop: Some(Region::new(10, 20, 70, 60)), ..Scene::default() };
    let out = render_png(&src, &scene).unwrap();
    let img = decode_generated_png(&out, (60, 40)).unwrap();
    assert_eq!(img.get_pixel(0, 0), source(120, 90, 4).get_pixel(10, 20));
    assert_eq!(output_size((120, 90), &scene), Some((60, 40)));
    assert!(render_png(&src, &Scene { crop: Some(Region::new(500, 500, 600, 600)), ..Scene::default() }).is_err());
}

#[test]
fn transparent_source_alpha_survives_annotation_compositing() {
    let mut img = RgbaImage::from_pixel(40, 40, Rgba([10, 200, 30, 128]));
    let scene = Scene { annotations: vec![Annotation { shape: Shape::Highlight { region: Region::new(30, 30, 40, 40) }, color: [255, 0, 0, 255] }], ..Scene::default() };
    render_into(&mut img, &scene, 1.0);
    let p = img.get_pixel(2, 2).0;
    assert!((i16::from(p[1]) - 200).abs() <= 2 && p[3] == 128, "{p:?}");
}

/// Frozen render golden (reviewed visually as tests/golden/render/scene-v1.png).
#[test]
fn golden_scene_digest_is_frozen() {
    let mut img = source(320, 240, 77);
    render_into(&mut img, &golden_scene(), 1.0);
    if std::env::var_os("SHAREREADY_WRITE_RENDER_GOLDEN").is_some() {
        img.save(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden/render/scene-v1.png")).unwrap();
        println!("DIGEST={:#018x}", digest(&img));
    }
    assert_eq!(digest(&img), GOLDEN_SCENE_V1, "render output changed; review and re-freeze deliberately");
}

/// Frozen 2026-10-19 after visual review; tied to tiny-skia 0.11.4, ab_glyph 0.2.32, x86_64.
const GOLDEN_SCENE_V1: u64 = 0x4140_1843_656f_0559;

/// The golden depends on the exact bundled font file.
#[test]
fn bundled_font_is_the_pinned_file() {
    let mut h = 0u64;
    for c in FONT_BYTES.chunks(8) { let mut b = [0u8; 8]; b[..c.len()].copy_from_slice(c); h = mix(h ^ u64::from_le_bytes(b)); }
    assert_eq!((FONT_BYTES.len(), h), (FONT_LEN, FONT_DIGEST), "font changed: re-review render goldens");
}
const FONT_LEN: usize = 16_437_364;
const FONT_DIGEST: u64 = 8_555_162_396_145_626_508;
