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
        labels: Vec::new(),
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
        crop: None, frame: None,
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
    if std::env::var_os("ANSHOT_WRITE_RENDER_GOLDEN").is_some() {
        img.save(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden/render/scene-v1.png")).unwrap();
        println!("DIGEST={:#018x}", digest(&img));
    }
    assert_eq!(digest(&img), GOLDEN_SCENE_V1, "render output changed; review and re-freeze deliberately");
}

/// Frozen 2026-10-19 after visual review; tied to tiny-skia 0.11.4, ab_glyph 0.2.32, x86_64.
const GOLDEN_SCENE_V1: u64 = 0x4140_1843_656f_0559;
/// Frozen 2026-11-01 after visual review; same pins as above.
const GOLDEN_SCENE_V4: u64 = 0x6a9e_86c5_263a_f2db;

fn label_scene() -> Scene {
    let region = Region::new(40, 40, 200, 70);
    Scene {
        redactions: vec![Redaction { region, style: RedactStyle::Solid([240, 242, 245, 255]) }],
        labels: vec![Label { region, text: "user-a@example.com".into(), ink: [26, 25, 21, 255] }],
        ..Scene::default()
    }
}

#[test]
fn a_label_stays_inside_its_opaque_cover() {
    let mut img = source(320, 240, 5);
    let before = img.clone();
    render_into(&mut img, &label_scene(), 1.0);
    let (mut ink, mut fill) = (0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if (40..200).contains(&x) && (40..70).contains(&y) {
            assert_eq!(p.0[3], 255, "the cover stays opaque under its text");
            if p.0[..3] == [240, 242, 245] { fill += 1 } else { ink += 1 }
        } else {
            assert_eq!(p, before.get_pixel(x, y), "nothing changes outside the cover");
        }
    }
    assert!(ink > 100 && fill > ink, "ink {ink} fill {fill}");
}

#[test]
fn nothing_of_the_source_shows_through_a_label() {
    let scene = label_scene();
    let (mut a, mut b) = (source(320, 240, 1), source(320, 240, 2));
    render_into(&mut a, &scene, 1.0);
    render_into(&mut b, &scene, 1.0);
    for y in 40..70 { for x in 40..200 { assert_eq!(a.get_pixel(x, y), b.get_pixel(x, y)); } }
}

#[test]
fn a_long_label_shrinks_and_a_tiny_cover_stays_plain() {
    let long = Region::new(0, 0, 60, 30);
    let ((x, _), size) = label_layout("a very long stand-in text", &long).unwrap();
    assert!(size < 30.0 * LABEL_FILL && x >= LABEL_INSET);
    assert!(text_extent("a very long stand-in text", size).0 <= 60.0 - LABEL_INSET * 2.0 + 0.5);
    assert_eq!(label_layout("text", &Region::new(0, 0, 4, 4)), None);
}

/// The magnifier samples the picture after stand-in text is drawn, never before.
#[test]
fn a_magnifier_shows_the_label_not_the_source() {
    let mut scene = label_scene();
    scene.annotations.push(Annotation { shape: Shape::Magnify { region: Region::new(40, 40, 120, 70), center: (160.0, 160.0), zoom: 2.0, width: 2.0 }, color: [255, 0, 0, 255] });
    let (mut a, mut b) = (source(320, 240, 1), source(320, 240, 2));
    render_into(&mut a, &scene, 1.0);
    render_into(&mut b, &scene, 1.0);
    for y in 140..175 { for x in 90..230 { assert_eq!(a.get_pixel(x, y), b.get_pixel(x, y)); } }
}

/// Frozen render golden for stand-in text (tests/golden/render/scene-v4.png).
#[test]
fn golden_scene_v4_digest_is_frozen() {
    let mut img = source(320, 240, 77);
    render_into(&mut img, &label_scene(), 1.0);
    if std::env::var_os("ANSHOT_WRITE_RENDER_GOLDEN").is_some() {
        img.save(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden/render/scene-v4.png")).unwrap();
        println!("DIGEST_V4={:#018x}", digest(&img));
    }
    assert_eq!(digest(&img), GOLDEN_SCENE_V4, "render output changed; review and re-freeze deliberately");
}

/// The golden depends on the exact bundled font file.
#[test]
fn bundled_font_is_the_pinned_file() {
    let mut h = 0u64;
    for c in FONT_BYTES.chunks(8) { let mut b = [0u8; 8]; b[..c.len()].copy_from_slice(c); h = mix(h ^ u64::from_le_bytes(b)); }
    assert_eq!((FONT_BYTES.len(), h), (FONT_LEN, FONT_DIGEST), "font changed: re-review render goldens");
}
const FONT_LEN: usize = 16_437_364;
const FONT_DIGEST: u64 = 8_555_162_396_145_626_508;

pub(crate) fn golden_scene_v2() -> Scene {
    let wave: Vec<(f32, f32)> = (0..24).map(|i| (30.0 + i as f32 * 8.0, 190.0 + (i as f32 * 0.6).sin() * 18.0)).collect();
    Scene {
        labels: Vec::new(),
        redactions: vec![Redaction { region: Region::new(250, 20, 310, 50), style: RedactStyle::SafeMosaic }],
        annotations: vec![
            Annotation { shape: Shape::Ellipse { region: Region::new(20, 20, 140, 90), width: 4.0 }, color: [0xe5, 0x34, 0x2b, 255] },
            Annotation { shape: Shape::Ellipse { region: Region::new(160, 30, 220, 90), width: 2.0 }, color: [0x2f, 0x6f, 0xf0, 255] },
            Annotation { shape: Shape::Line { from: (20.0, 120.0), to: (300.0, 140.0), width: 6.0 }, color: [0x22, 0xa6, 0x52, 255] },
            Annotation { shape: Shape::Pen { points: wave, width: 4.0 }, color: [0x9b, 0x30, 0xd9, 255] },
            Annotation { shape: Shape::Pen { points: vec![(280.0, 200.0)], width: 8.0 }, color: [0x1a, 0x19, 0x15, 255] },
        ],
        crop: None, frame: None,
    }
}

#[test]
fn ellipse_line_and_pen_draw_where_expected() {
    let mut img = RgbaImage::from_pixel(320, 240, Rgba([255, 255, 255, 255]));
    let scene = golden_scene_v2();
    render_into(&mut img, &scene, 1.0);
    // Ellipse: stroke on the left extreme of the oval (x = 22), centre stays white.
    assert_eq!(img.get_pixel(22, 55).0, [0xe5, 0x34, 0x2b, 255]);
    assert_eq!(img.get_pixel(80, 55).0, [255, 255, 255, 255]);
    // Line: centre of the segment is solid green.
    assert_eq!(img.get_pixel(160, 130).0, [0x22, 0xa6, 0x52, 255]);
    // Pen: the stroke passes through its first sample; a single-point pen is a dot.
    assert_eq!(img.get_pixel(30, 190).0, [0x9b, 0x30, 0xd9, 255]);
    assert_eq!(img.get_pixel(280, 200).0, [0x1a, 0x19, 0x15, 255]);
    // Bounds cover the drawn pixels.
    let (x0, y0, x1, y1) = bounds(&scene.annotations[3]);
    assert!(x0 <= 28.0 && x1 >= 214.0 && y0 <= 172.0 && y1 >= 208.0, "{x0} {y0} {x1} {y1}");
    assert_eq!(bounds(&Annotation { shape: Shape::Pen { points: vec![], width: 3.0 }, color: [0; 4] }), (0.0, 0.0, 0.0, 0.0));
}

/// Frozen render golden for the 0.2 shapes (tests/golden/render/scene-v2.png).
#[test]
fn golden_scene_v2_digest_is_frozen() {
    let mut img = source(320, 240, 77);
    render_into(&mut img, &golden_scene_v2(), 1.0);
    if std::env::var_os("ANSHOT_WRITE_RENDER_GOLDEN_V2").is_some() {
        img.save(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden/render/scene-v2.png")).unwrap();
        println!("DIGEST_V2={:#018x}", digest(&img));
    }
    assert_eq!(digest(&img), GOLDEN_SCENE_V2, "render output changed; review and re-freeze deliberately");
}

/// Frozen 2026-10-21 after visual review; same toolchain pins as v1.
const GOLDEN_SCENE_V2: u64 = 0xd654_7732_c6a9_0ce1;

pub(crate) fn golden_scene_v3() -> Scene {
    Scene {
        labels: Vec::new(),
        redactions: vec![Redaction { region: Region::new(40, 40, 90, 60), style: RedactStyle::SafeMosaic }],
        annotations: vec![
            Annotation { shape: Shape::Spotlight { region: Region::new(20, 20, 120, 100) }, color: [0xe5, 0x34, 0x2b, 255] },
            Annotation { shape: Shape::Magnify { region: Region::new(40, 40, 90, 70), center: (220.0, 80.0), zoom: 2.0, width: 3.0 }, color: [0xe5, 0x34, 0x2b, 255] },
        ],
        crop: None, frame: None,
    }
}

#[test]
fn spotlight_dims_outside_and_magnifier_enlarges_the_redacted_picture() {
    let src = source(320, 240, 33);
    let mut img = src.clone();
    let scene = golden_scene_v3();
    render_into(&mut img, &scene, 1.0);
    // Outside the spotlight: darkened; inside (away from the mosaic): unchanged.
    let (o, d) = (src.get_pixel(300, 220).0, img.get_pixel(300, 220).0);
    assert_eq!(d[0], (f32::from(o[0]) * SPOTLIGHT_KEEP).round() as u8);
    assert_eq!(img.get_pixel(30, 90).0, src.get_pixel(30, 90).0);
    // The callout shows the mosaic (never the covered source pixels), at 2x:
    // source (42, 42) is mosaic; it appears at callout (170 + 4..5, 50 + 4..5).
    let (l, t, _, _) = magnify_target(&Region::new(40, 40, 90, 70), (220.0, 80.0), 2.0);
    assert_eq!((l, t), (170.0, 50.0));
    assert_eq!(img.get_pixel(174, 54).0, mosaic_color(42, 42));
    assert_ne!(img.get_pixel(174, 54).0, src.get_pixel(42, 42).0);
    // Bounds span the region and the callout.
    let (x0, y0, x1, y1) = bounds(&scene.annotations[1]);
    assert!(x0 <= 40.0 && y0 <= 40.0 && x1 >= 270.0 && y1 >= 110.0, "{x0} {y0} {x1} {y1}");
    // The preview (scale 0.5) samples the same picture.
    let mut small = image::imageops::resize(&src, 160, 120, image::imageops::FilterType::Nearest);
    render_into(&mut small, &scene, 0.5);
    assert_eq!(small.dimensions(), (160, 120));
}

/// Frozen render golden for the 0.3 shapes (tests/golden/render/scene-v3.png).
#[test]
fn golden_scene_v3_digest_is_frozen() {
    let mut img = source(320, 240, 91);
    render_into(&mut img, &golden_scene_v3(), 1.0);
    if std::env::var_os("ANSHOT_WRITE_RENDER_GOLDEN_V3").is_some() {
        img.save(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden/render/scene-v3.png")).unwrap();
        println!("DIGEST_V3={:#018x}", digest(&img));
    }
    assert_eq!(digest(&img), GOLDEN_SCENE_V3, "render output changed; review and re-freeze deliberately");
}

/// Frozen 2026-10-24 after visual review.
const GOLDEN_SCENE_V3: u64 = 0xe8bc_9325_9bf6_889d;

// ---- 0.5 frame (ADR 0.5-BEAUTIFY) ----

fn framed(chrome: Chrome, ratio: Ratio) -> Frame {
    Frame { background: PRESETS[2], padding: Step::Medium, radius: Step::Medium, shadow: Step::Medium, ratio, chrome, text: "example.com/anshot".into() }
}

#[test]
fn frame_layout_follows_the_steps_and_ratio() {
    let plain = Frame { padding: Step::None, radius: Step::None, shadow: Step::None, ..Frame::default() };
    let l = frame_layout((300, 200), &plain).unwrap();
    assert_eq!((l.canvas, l.image, l.bar.3, l.radius, l.blur), ((300, 200), (0, 0, 300, 200), 0, 0.0, 0.0));
    // Medium padding is 8% of the long side on every side.
    let l = frame_layout((300, 200), &Frame { padding: Step::Medium, ..plain.clone() }).unwrap();
    assert_eq!((l.canvas, l.image), ((348, 248), (24, 24, 300, 200)));
    // Ratios widen or heighten the canvas and centre the image; the image is never cut.
    for ratio in Ratio::ALL {
        let l = frame_layout((300, 200), &Frame { ratio, chrome: Chrome::Browser, ..plain.clone() }).unwrap();
        let (cw, ch) = (u64::from(l.canvas.0), u64::from(l.canvas.1));
        if let Some((rw, rh)) = ratio.wh() { assert!((cw * rh).abs_diff(ch * rw) < rw.max(rh), "{ratio:?} {cw}x{ch}"); }
        assert_eq!((l.image.2, l.image.3), (300, 200));
        assert!(l.image.0 + 300 <= l.canvas.0 && l.image.1 + 200 <= l.canvas.1);
        assert_eq!((l.bar.0, l.bar.1 + l.bar.3, l.bar.2, l.bar.3), (l.image.0, l.image.1, 300, 28));
    }
    // Radius has a floor and never passes half the short side.
    assert_eq!(frame_layout((300, 200), &Frame { radius: Step::Small, ..plain.clone() }).unwrap().radius, 4.0);
    assert_eq!(frame_layout((40, 10), &Frame { radius: Step::Large, ..plain.clone() }).unwrap().radius, 5.0);
}

#[test]
fn frame_layout_shrinks_padding_then_refuses_past_the_pixel_limit() {
    let big = Frame { padding: Step::Large, ..Frame::default() };
    // 6000x4000 is exactly the limit: padding has nowhere to go, so it is dropped.
    let l = frame_layout((6000, 4000), &Frame { ratio: Ratio::Free, ..big.clone() }).unwrap();
    assert_eq!((l.canvas, l.image), ((6000, 4000), (0, 0, 6000, 4000)));
    // 5000x4000 has room for some padding but not 12%.
    let l = frame_layout((5000, 4000), &big).unwrap();
    assert!(u64::from(l.canvas.0) * u64::from(l.canvas.1) <= anshot_core::MAX_PIXELS as u64);
    assert!(l.image.0 > 0 && l.image.0 < 600, "{:?}", l);
    // A square output of a 6000x4000 image cannot fit at all.
    assert!(frame_layout((6000, 4000), &Frame { ratio: Ratio::Square, ..big }).is_err());
    assert!(frame_layout((0, 10), &Frame::default()).is_err());
}

#[test]
fn framed_output_is_opaque_and_keeps_the_export_pixels() {
    let mut src = source(320, 240, 57);
    src.put_pixel(0, 0, Rgba([9, 9, 9, 0])); // a transparent source pixel still ends opaque
    for chrome in Chrome::ALL {
        let f = framed(chrome, Ratio::R4x3);
        let out = render_framed(&src, &f).unwrap();
        let l = frame_layout((320, 240), &f).unwrap();
        assert_eq!(out.dimensions(), l.canvas);
        assert!(out.pixels().all(|p| p[3] == 255), "{chrome:?}");
        // Inside the image rectangle, away from the rounded corners, pixels are copied as is.
        let r = l.radius.ceil() as u32 + 1;
        let (x0, y0, w, h) = l.image;
        for y in 0..h {
            for x in 0..w {
                let corner = (x < r || x + r >= w) && (y < r || y + r >= h);
                if corner || (x, y) == (0, 0) { continue; }
                assert_eq!(out.get_pixel(x0 + x, y0 + y).0, src.get_pixel(x, y).0, "{chrome:?} at {x},{y}");
            }
        }
    }
}

#[test]
fn render_png_frames_after_redaction_and_crop() {
    let src = source(200, 120, 33);
    let crop = Region::new(10, 10, 190, 110);
    let red = Redaction { region: Region::new(20, 20, 80, 50), style: RedactStyle::Solid([0x1a, 0x19, 0x15, 255]) };
    let plain = Scene { redactions: vec![red], crop: Some(crop), ..Scene::default() };
    let f = framed(Chrome::Mac, Ratio::R16x9);
    let scene = Scene { frame: Some(f.clone()), ..plain.clone() };
    let flat = decode_generated_png(&render_png(&png(&src), &plain).unwrap(), (180, 100)).unwrap();
    let dims = output_size((200, 120), &scene).unwrap();
    let out = decode_generated_png(&render_png(&png(&src), &scene).unwrap(), dims).unwrap();
    assert_eq!(out, render_framed(&flat, &f).unwrap());
    let (x0, y0, _, _) = image_rect((200, 120), &scene).unwrap();
    assert_eq!(image_rect((200, 120), &plain), Some((0, 0, 180, 100)));
    // The cover survives framing: crop-relative (10..70, 10..40).
    for (x, y) in [(10, 10), (40, 25), (69, 39)] { assert_eq!(out.get_pixel(x0 + x, y0 + y).0, [0x1a, 0x19, 0x15, 255]); }
    // Without a frame the export is unchanged byte for byte.
    assert_eq!(render_png(&png(&src), &plain).unwrap(), render_png(&png(&src), &Scene { frame: None, ..scene }).unwrap());
}

#[test]
fn auto_colors_come_from_the_outer_ring_only() {
    let mut img = RgbaImage::from_pixel(50, 40, Rgba([100, 150, 200, 255]));
    for y in 5..35 { for x in 5..45 { img.put_pixel(x, y, Rgba([255, 0, 0, 255])); } }
    let (light, dark) = auto_colors(&img);
    assert_eq!(light, [119, 163, 207, 255]);
    assert_eq!(dark, [88, 132, 176, 255]);
    // Cover the ring and the colours follow the cover, not what was under it.
    for (x, y, p) in img.enumerate_pixels_mut() { if x < 2 || y < 2 || x >= 48 || y >= 38 { *p = Rgba([0x1a, 0x19, 0x15, 255]); } }
    assert_eq!(auto_colors(&img).1, [23, 22, 18, 255]);
    let out = render_framed(&img, &Frame { background: Background::Auto, ..Frame::default() }).unwrap();
    assert_eq!(out.get_pixel(0, 0).0[..3], auto_colors(&img).0[..3]);
}

#[test]
fn chrome_text_is_cut_to_fit() {
    assert_eq!(fit_text("short", 12.0, 500.0), "short");
    let cut = fit_text(&"example.com/".repeat(20), 12.0, 120.0);
    assert!(cut.ends_with('…') && text_extent(&cut, 12.0).0 <= 120.0, "{cut}");
    assert_eq!(fit_text("a\nb", 12.0, 500.0), "a b");
    assert_eq!(fit_text("abc", 12.0, 0.0), "");
}

/// Frozen render golden for the 0.5 frame (tests/golden/render/scene-v5.png): gradient,
/// rounded corners, shadow and browser chrome around the v1 scene.
#[test]
fn golden_scene_v5_digest_is_frozen() {
    let mut img = source(320, 240, 57);
    render_into(&mut img, &golden_scene(), 1.0);
    let img = render_framed(&img, &framed(Chrome::Browser, Ratio::R16x9)).unwrap();
    if std::env::var_os("ANSHOT_WRITE_RENDER_GOLDEN_V5").is_some() {
        img.save(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden/render/scene-v5.png")).unwrap();
        println!("DIGEST_V5={:#018x}", digest(&img));
    }
    assert_eq!(digest(&img), GOLDEN_SCENE_V5, "render output changed; review and re-freeze deliberately");
}

/// Frozen 2026-11-02 after visual review.
const GOLDEN_SCENE_V5: u64 = 0x0cb1_1d49_a23f_9748;

/// Timing only (run with --release --ignored): framing a large export.
#[test]
#[ignore]
fn frame_timing_large() {
    let src = source(4800, 3200, 7);
    for f in [Frame::default(), Frame { shadow: Step::Large, chrome: Chrome::Browser, ratio: Ratio::R16x9, ..Frame::default() }] {
        let t = std::time::Instant::now();
        let out = render_framed(&src, &f).unwrap();
        println!("FRAME {:?} -> {:?} in {} ms", src.dimensions(), out.dimensions(), t.elapsed().as_millis());
    }
}

#[test]
fn preview_keeps_the_export_proportions_and_text_cut() {
    // A 4000 px export: the bar is capped at 72 px (1.8%). A quarter-size preview must show
    // the same proportions, not its own 28 px minimum, and the same cut of the text.
    let f = Frame { chrome: Chrome::Browser, text: "https://example.com/a/very/long/path/with/a/token-0123456789abcdef".repeat(3), ..framed(Chrome::Browser, Ratio::Free) };
    let full = frame_layout((4000, 2000), &f).unwrap();
    let small = source(1000, 500, 3);
    let out = render_framed_preview(&small, &f, 0.25).unwrap();
    let (cw, ch) = full.canvas;
    assert!((i64::from(out.width()) - i64::from(cw / 4)).abs() <= 1 && (i64::from(out.height()) - i64::from(ch / 4)).abs() <= 1, "{:?} vs {:?}", out.dimensions(), full.canvas);
    assert_eq!(full.bar.3, 72);
    // The export pixels sit unchanged in the preview too.
    let (ix, iy) = ((full.image.0 as f32 * 0.25).round() as u32, (full.image.1 as f32 * 0.25).round() as u32);
    assert_eq!(out.get_pixel(ix + 500, iy + 250).0, small.get_pixel(500, 250).0);
    // Scale 1 is the export itself.
    assert_eq!(render_framed_preview(&small, &f, 1.0).unwrap(), render_framed(&small, &f).unwrap());
}

#[test]
fn frame_debug_hides_the_typed_text() {
    let f = Frame { text: "secret-token-42".into(), ..Frame::default() };
    let shown = format!("{f:?} {:?}", Scene { frame: Some(f.clone()), ..Scene::default() });
    assert!(!shown.contains("secret") && shown.contains("<15 chars>"), "{shown}");
}

#[test]
fn auto_colors_ignore_transparent_pixels() {
    let img = RgbaImage::from_pixel(20, 20, Rgba([200, 10, 10, 0]));
    let Background::Gradient(a, b) = PRESETS[0] else { unreachable!() };
    assert_eq!(auto_colors(&img), (a, b));
}
