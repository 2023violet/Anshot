//! Strict packed DIB intake. No palette, profile paths or ambiguous alpha inference.
#![forbid(unsafe_code)]
use crate::{MAX_INPUT_BYTES,MAX_PIXELS};
use image::{DynamicImage,ImageFormat,Rgba,RgbaImage};
use std::io::Cursor;

pub const MAX_DIB_BYTES:usize=MAX_PIXELS as usize*4+124;

/// Normalize supported clipboard DIB pixels into a fresh source PNG. This is
/// intake, not a reviewed/redacted export; delivery still requires Editor review.
pub fn dib_to_png(input:&[u8])->Result<Vec<u8>,String>{
    let invalid=||"Unsupported or invalid clipboard DIB; copy a PNG image instead".to_string();
    if input.len()<40 || input.len()>MAX_DIB_BYTES{return Err(invalid());}
    // Every read below lies within the validated fixed header or mask block.
    let word=|at|u32::from_le_bytes(input[at..at+4].try_into().unwrap());
    let header=word(0) as usize;
    if ![40,108,124].contains(&header) || input.len()<header{return Err(invalid());}
    let (width,height)=(word(4) as i32,word(8) as i32);
    let Some(height_abs)=height.checked_abs() else{return Err(invalid());};
    if width<=0 || height_abs==0 || width as u64*height_abs as u64>MAX_PIXELS{return Err(invalid());}
    let planes=u16::from_le_bytes(input[12..14].try_into().unwrap());
    let bits=u16::from_le_bytes(input[14..16].try_into().unwrap());
    let compression=word(16);
    if planes!=1 || ![24,32].contains(&bits) || word(32)!=0 ||
        !(compression==0 || (compression==3 && bits==32)){return Err(invalid());}
    if header>=108 && ![0x73524742,0x57696e20].contains(&word(56)){return Err(invalid());}
    if header==124 && (word(112)!=0 || word(116)!=0){return Err(invalid());}
    let mut offset=header;
    let mut alpha=false;
    if compression==3 {
        if header==40{offset+=12;}
        if input.len()<offset || (word(40),word(44),word(48))!=(0x00ff0000,0x0000ff00,0x000000ff){return Err(invalid());}
        if header>=108 {
            match word(52){0=>{},0xff000000=>alpha=true,_=>return Err(invalid())}
        }
    }
    let (w,h)=(width as usize,height_abs as usize);
    let bytes_per_pixel=usize::from(bits/8);
    let stride=(w*bytes_per_pixel+3)&!3;
    // w*h was bounded above; stride <= 4*w, so these products fit all targets.
    let pixels_end=offset.checked_add(stride*h).ok_or_else(invalid)?;
    if input.len()<pixels_end{return Err(invalid());}
    let mut image=RgbaImage::new(w as u32,h as u32);
    for y in 0..h {
        let row=if height<0{y}else{h-1-y};
        for x in 0..w {
            let at=offset+row*stride+x*bytes_per_pixel;
            image.put_pixel(x as u32,y as u32,Rgba([input[at+2],input[at+1],input[at],
                if alpha{input[at+3]}else{255}]));
        }
    }
    let mut out=Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image).write_to(&mut out,ImageFormat::Png)
        .map_err(|e|format!("Clipboard PNG encode failed: {e}"))?;
    let png=out.into_inner();
    if png.len()>MAX_INPUT_BYTES{return Err("Normalized clipboard PNG exceeds 25 MiB; use a smaller image".into());}
    Ok(png)
}
