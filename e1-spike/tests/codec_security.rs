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
    for (x,y) in [(78,539),(626,587),(78,612),(674,663)] {
        assert_eq!(*img.get_pixel(x,y),COVER);
    }
}
