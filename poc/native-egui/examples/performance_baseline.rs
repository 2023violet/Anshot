//! Release component baseline; no OS input, clipboard, window, or private files.
#![forbid(unsafe_code)]
#[path="../src/display.rs"] mod display;
use std::{io::Write,path::Path,time::Instant};
use eframe::egui;
use shareready_core::Region;
use shareready_editor::{Editor,OperationKind};
const GRID:&[u8]=include_bytes!("../assets/performance/grid-24mp.png");
const WIDE:&[u8]=include_bytes!("../assets/performance/wide-24mp.png");
fn ms(start:Instant)->f64{start.elapsed().as_secs_f64()*1000.0}
fn phase(fixture:&str,iteration:usize,name:&str){
    let unix_ns=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    println!("{{\"type\":\"phase\",\"fixture\":\"{fixture}\",\"iteration\":{iteration},\"phase\":\"{name}\",\"unix_ns\":{unix_ns}}}");
    std::io::stdout().flush().unwrap();
}
fn new_file(path:&Path,data:&[u8])->Result<(),String>{
    let mut file=std::fs::OpenOptions::new().create_new(true).write(true).open(path).map_err(|e|e.to_string())?;
    file.write_all(data).and_then(|_|file.sync_all()).map_err(|e|e.to_string())
}
fn run(out:&Path)->Result<(),String>{
    std::fs::create_dir_all(out).map_err(|e|e.to_string())?;
    println!("{{\"type\":\"identity\",\"build\":\"{}\",\"mode\":\"release-component-no-gpu-no-window\",\"debug_assertions\":{}}}",option_env!("SR_BUILD_ID").unwrap_or("unidentified"),cfg!(debug_assertions));
    for (name,source,w,h) in [("grid",GRID,6000u32,4000u32),("wide",WIDE,24000,1000)]{
        // Embedded fixed inputs only; no arbitrary file intake or clipboard.
        new_file(&out.join(format!("{name}-source.png")),source)?;
        for iteration in 0..3{
            let ctx=egui::Context::default();phase(name,iteration,"editor-decode");let start=Instant::now();
            let mut ed=Editor::from_source(source.to_vec()).map_err(|e|format!("{e:?}"))?;
            let editor_ms=ms(start);assert_eq!(ed.dimensions(),(w,h));
            phase(name,iteration,"display-decode-prepare");let start=Instant::now();let mut image=display::DisplayImage::decode(&ctx,source)?;let display_ms=ms(start);
            let region=Region::new(i64::from(w/2),i64::from(h/2),i64::from(w/2+120),i64::from(h/2+80));
            ed.add_mask(region).map_err(|e|format!("{e:?}"))?;
            phase(name,iteration,"review-decode-paint-encode-readback");let start=Instant::now();let ticket=ed.begin(OperationKind::Review).map_err(|e|format!("{e:?}"))?;let review_ms=ms(start);
            let png=ed.bytes_for(ticket).ok_or("no review bytes")?;
            phase(name,iteration,"review-display-decode-prepare");let start=Instant::now();let mut reviewed=display::DisplayImage::decode_generated(&ctx,png,(w,h))?;let review_display_ms=ms(start);
            phase(name,iteration,"cpu-paint-no-gpu");
            for (label,zoom) in [("fit",display::Zoom::Fit),("100",display::Zoom::Pixels(1.0)),("200",display::Zoom::Pixels(2.0))]{
                let start=Instant::now();
                let mut output=ctx.run_ui(egui::RawInput{max_texture_side:Some(16384),screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(850.0,650.0))),..Default::default()},|ui|{
                    let size=display::image_size([w as usize,h as usize],zoom,ui.available_size(),1.0);
                    let origin=if zoom==display::Zoom::Fit{egui::Pos2::ZERO}else{egui::pos2(-size.x/2.0,-size.y/2.0)};
                    let rect=egui::Rect::from_min_size(origin,size);
                    image.paint(ui,rect,zoom);reviewed.paint(ui,rect,zoom);
                });
                let paint_ms=ms(start);let uploads=output.textures_delta.set.len();output.textures_delta.clear();
                println!("{{\"type\":\"component_paint\",\"fixture\":\"{name}\",\"iteration\":{iteration},\"zoom\":\"{label}\",\"cpu_ms\":{paint_ms},\"upload_entries\":{uploads},\"gpu_uploaded\":false}}");
            }
            let read=shareready_core::decode_generated_png(png,(w,h))?;assert_eq!(read.dimensions(),(w,h));drop(read);
            if iteration==0{
                new_file(&out.join(format!("{name}-review.png")),png)?;
                new_file(&out.join(format!("{name}.regions.json")),format!("[[{},{},{},{}]]",region.x0,region.y0,region.x1,region.y1).as_bytes())?;
            }
            assert!(ed.finish(ticket,true));
            println!("{{\"type\":\"component\",\"fixture\":\"{name}\",\"iteration\":{iteration},\"editor_decode_ms\":{editor_ms},\"display_decode_prepare_ms\":{display_ms},\"review_encode_validate_ms\":{review_ms},\"review_display_decode_prepare_ms\":{review_display_ms},\"png_bytes\":{}}}",ed.output_png().unwrap().len());
            std::io::stdout().flush().map_err(|e|e.to_string())?;
            phase(name,iteration,"drop-component-scope-not-app-reimport");
        }
    }
    println!("{{\"type\":\"complete\",\"component_only\":true}}");Ok(())
}
fn main(){
    let args:Vec<_>=std::env::args_os().skip(1).collect();
    if args.len()!=1{eprintln!("Usage: performance_baseline NEW_SYNTHETIC_OUTPUT_DIR");std::process::exit(2);}
    if let Err(e)=run(Path::new(&args[0])){eprintln!("component baseline failed: {e}");std::process::exit(3);}
}
