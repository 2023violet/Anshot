use shareready_e1::{decode_source,redact,Region,COVER};
use image::ImageFormat;
use std::{fs,path::PathBuf};
fn fixture(name:&str)->Vec<u8>{
    let p=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/research/fixtures").join(name);
    fs::read(p).unwrap()
}
fn verify(name:&str,masks:&[[i64;4]],preserved:&[[u32;4]]) {
    let source=fixture(name);
    let original=decode_source(&source).unwrap();
    let regions:Vec<_>=masks.iter().map(|r|Region::new(r[0],r[1],r[2],r[3])).collect();
    let png=redact(&source,&regions).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(!png.windows(4).any(|w|w==b"eXIf"),"EXIF chunk copied");
    let actual=image::load_from_memory_with_format(&png,ImageFormat::Png).unwrap().to_rgba8();
    assert_eq!(actual.dimensions(),original.dimensions());
    let (w,h)=actual.dimensions();
    for rect in regions {
        let (l,t,r,b)=rect.clipped(w,h).unwrap();
        for y in t..b { for x in l..r {
            assert_eq!(*actual.get_pixel(x,y),COVER,"unmasked pixel: {name}@{x},{y}");
        }}
    }
    for &[l,t,r,b] in preserved {
        for y in t..b { for x in l..r {
            assert_eq!(actual.get_pixel(x,y),original.get_pixel(x,y),"unintended change {name}@{x},{y}");
        }}
    }
}
#[test]
fn chat_all_four_masks_and_keep() {
    verify("T01_CHAT.png",&[[84,154,166,237],[184,154,387,197],[184,205,379,249],[858,148,1119,262]],&[[135,320,755,374],[261,454,798,509]]);
}
#[test]
fn order_all_three_masks_and_keep() {
    verify("T02_ORDER.png",&[[105,469,568,518],[105,536,508,586],[105,604,679,654]],&[[103,169,650,312]]);
}
#[test]
fn edges_four_corners_and_keep() {
    verify("T04_EDGES.png",&[[15,12,950,90],[1550,12,2394,90],[8,1497,905,1595],[1490,1497,2398,1595]],&[[315,605,2050,813]]);
}
#[test]
fn exif_gps_and_jpeg_to_new_png_without_private_metadata() {
    let source=fixture("T05_JPEG_EXIF.jpg");
    assert!(source.windows(6).any(|w|w==b"Exif\0\0"));
    let png=redact(&source,&[Region::new(78,539,627,588),Region::new(78,612,675,664)]).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    for keyword in [b"Exif".as_slice(),b"eXIf".as_slice(),b"GPSInfo".as_slice()] {
        assert!(!png.windows(keyword.len()).any(|w|w==keyword),"source metadata leaked: {keyword:?}");
    }
    let img=image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(img.dimensions(),(1200,800));
    let decoded_source=decode_source(&source).unwrap();
    for y in 277..451 {
        for x in 780..1110 {
            assert_eq!(img.get_pixel(x,y),decoded_source.get_pixel(x,y),"Rust codec altered JPEG preserve pixel {x},{y}");
        }
    }
    for (x,y) in [(78,539),(626,587),(78,612),(674,663)] {
        assert_eq!(*img.get_pixel(x,y),COVER);
    }
}

#[test]
fn rejects_empty_or_ineffective_masks_without_exporting_source() {
    let source = fixture("T01_CHAT.png");
    // Every one of these used to return an apparently successful new PNG of the source.
    let invalid: &[(&str, &[Region])] = &[
        ("empty region list", &[]),
        ("outside positive bounds", &[Region::new(1500, 1600, 2000, 2100)]),
        ("outside negative bounds", &[Region::new(-2000, -1200, -1, -1)]),
        ("zero horizontal extent", &[Region::new(200, 100, 200, 180)]),
        ("zero vertical extent", &[Region::new(200, 100, 250, 100)]),
        ("two outside masks", &[Region::new(1500, 1500, 1600, 1600),
                                 Region::new(-10, -10, -1, -1)]),
        ("overlap boundary only", &[Region::new(-10, -10, 0, 0)]),
        ("extreme signed bounds outside", &[Region::new(i64::MAX, i64::MAX, i64::MAX - 1, i64::MAX - 1)]),
    ];
    for &(scenario, regions) in invalid {
        let failure = redact(&source, regions).expect_err(scenario);
        assert!(failure.contains("effective redaction"), "{scenario}: {failure}");
    }
}

#[test]
fn valid_pixel_amid_invalid_regions_still_masks_and_preserves_others() {
    let source = fixture("T01_CHAT.png");
    let original = decode_source(&source).unwrap();
    let output = redact(&source, &[
        Region::new(1500, 1500, 1550, 1580),
        Region::new(14, 16, 13, 15), // reverse direction, exactly one pixel
        Region::new(70, 70, 70, 90), // zero area must not count
    ]).expect("a single truly effective pixel should be sufficient");
    let result = image::load_from_memory_with_format(&output, ImageFormat::Png)
        .unwrap().to_rgba8();
    assert_eq!(*result.get_pixel(13, 15), COVER);
    for &(x, y) in &[(12, 15), (14, 15), (13, 14), (13, 16), (320, 350)] {
        assert_eq!(result.get_pixel(x, y), original.get_pixel(x, y), "unexpected mask at {x},{y}");
    }
}

#[test]
fn malformed_image_decoder_and_pixel_limit_are_exercised() {
    let png = fixture("T01_CHAT.png");
    let jpeg = fixture("T05_JPEG_EXIF.jpg");
    let mask = [Region::new(0, 0, 1, 1)];
    // These all retain valid magic bytes, so failure must happen in header
    // parsing or image decoding, not in the format or zero-mask guards.
    for truncated in [&png[..8], &png[..24], &jpeg[..3], &jpeg[..16]] {
        let error = redact(truncated, &mask).expect_err("truncated input must fail");
        assert!(error.contains("invalid image header") || error.contains("image decode failed"),
                "{error}");
    }

    // Build a small compressed PNG claiming an oversized image. Recalculate
    // IHDR CRC to ensure the decoder sees valid dimensions rather than bad CRC.
    fn crc32(payload: &[u8]) -> u32 {
        let mut value = !0u32;
        for &b in payload {
            value ^= u32::from(b);
            for _ in 0..8 {
                value = (value >> 1) ^ (if value & 1 == 1 { 0xedb88320 } else { 0 });
            }
        }
        !value
    }
    let mut bomb = png;
    assert_eq!(&bomb[12..16], b"IHDR");
    bomb[16..20].copy_from_slice(&6000u32.to_be_bytes());
    bomb[20..24].copy_from_slice(&6000u32.to_be_bytes());
    let crc = crc32(&bomb[12..29]);
    bomb[29..33].copy_from_slice(&crc.to_be_bytes());
    let error = redact(&bomb, &mask).expect_err("36MP image must be refused before decode");
    assert!(error.contains("decoded pixel dimensions out of bounds"), "{error}");
}
