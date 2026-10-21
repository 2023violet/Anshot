use anshot_core::{decode_source,dib_to_png};

fn put32(b:&mut [u8],at:usize,n:u32){b[at..at+4].copy_from_slice(&n.to_le_bytes());}
fn dib24(top_down:bool)->Vec<u8>{
    let mut b=vec![0;40];put32(&mut b,0,40);put32(&mut b,4,2);
    put32(&mut b,8,if top_down{(-2i32) as u32}else{2});
    b[12]=1;b[14]=24;
    let top=[0,0,255,0,255,0,99,99];
    let bottom=[255,0,0,30,20,10,88,88];
    if top_down{b.extend(top);b.extend(bottom);}else{b.extend(bottom);b.extend(top);}
    b
}
fn expected_rgb()->Vec<u8>{vec![255,0,0,255,0,255,0,255,0,0,255,255,10,20,30,255]}
#[test] fn bottom_up_24bit_rows_and_padding_have_independent_pixel_truth(){
    let p=decode_source(&dib_to_png(&dib24(false)).unwrap()).unwrap();
    assert_eq!(p.dimensions(),(2,2));assert_eq!(p.into_raw(),expected_rgb());
}
#[test] fn top_down_24bit_rows_preserve_orientation(){
    assert_eq!(decode_source(&dib_to_png(&dib24(true)).unwrap()).unwrap().into_raw(),expected_rgb());
}
fn dib32(header:u32,alpha:bool)->Vec<u8>{
    let mut b=vec![0;header as usize];put32(&mut b,0,header);put32(&mut b,4,2);put32(&mut b,8,(-2i32) as u32);
    b[12]=1;b[14]=32;
    if header>=108{
        put32(&mut b,16,3);put32(&mut b,40,0x00ff0000);put32(&mut b,44,0x0000ff00);put32(&mut b,48,0x000000ff);
        if alpha{put32(&mut b,52,0xff000000);}put32(&mut b,56,0x73524742);
    }
    b.extend([0,0,255,0,0,255,0,128,255,0,0,255,30,20,10,64]);b
}
#[test] fn v4_and_v5_explicit_alpha_are_preserved_in_all_pixels(){
    for header in [108,124]{
        assert_eq!(decode_source(&dib_to_png(&dib32(header,true)).unwrap()).unwrap().into_raw(),
            [255,0,0,0,0,255,0,128,0,0,255,255,10,20,30,64]);
    }
}
#[test] fn rgb_unused_high_byte_and_absent_alpha_are_opaque(){
    for b in [dib32(40,false),dib32(124,false)]{
        assert_eq!(decode_source(&dib_to_png(&b).unwrap()).unwrap().into_raw(),expected_rgb());
    }
}
#[test] fn info_header_bitfields_use_three_external_rgb_masks(){
    let mut b=dib32(40,false);put32(&mut b,16,3);
    let masks=[0x00ff0000u32,0x0000ff00,0x000000ff].into_iter().flat_map(u32::to_le_bytes);
    b.splice(40..40,masks);
    assert_eq!(decode_source(&dib_to_png(&b).unwrap()).unwrap().into_raw(),expected_rgb());
}
#[test] fn malformed_headers_dimensions_and_truncated_rows_are_rejected(){
    let good=dib24(false);
    for len in 0..good.len(){assert!(dib_to_png(&good[..len]).is_err(),"len={len}");}
    for (at,value) in [(0,12),(0,125),(4,0),(4,u32::MAX),(8,0),(8,i32::MIN as u32),(4,24_000_001),(8,24_000_001),(16,1),(32,1)]{
        let mut b=good.clone();put32(&mut b,at,value);assert!(dib_to_png(&b).is_err(),"at={at} value={value}");
    }
    let mut b=good;b[12]=2;assert!(dib_to_png(&b).is_err());
    b[12]=1;b[14]=8;assert!(dib_to_png(&b).is_err());
}
#[test] fn nonstandard_masks_profiles_and_color_spaces_fail_without_guessing(){
    for (at,value) in [(40,0xff),(44,0xff),(48,0xff0000),(52,0x0f000000),(56,0),(56,0x4d424544),(112,124),(116,10)]{
        let mut b=dib32(124,true);put32(&mut b,at,value);assert!(dib_to_png(&b).is_err(),"at={at}");
    }
}
