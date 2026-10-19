//! Explicit synthetic GPU readback capability probe; no user image/file input.
#![forbid(unsafe_code)]
use eframe::egui;
use std::{path::PathBuf,sync::{Arc,atomic::{AtomicBool,Ordering}}};

struct Probe {frames:u32,output:PathBuf,captured:Arc<AtomicBool>}
impl eframe::App for Probe {
    fn ui(&mut self,ui:&mut egui::Ui,_frame:&mut eframe::Frame){
        self.frames+=1;let ctx=ui.ctx().clone();
        ui.heading("ShareReady GPU readback probe");
        ui.label("Synthetic content only. No system clipboard or private input.");
        ui.colored_label(egui::Color32::from_rgb(17,24,39),"Known dark cover color");
        ui.horizontal(|ui|{for color in [egui::Color32::RED,egui::Color32::GREEN,egui::Color32::BLUE]{
            let (rect,_)=ui.allocate_exact_size(egui::vec2(80.0,80.0),egui::Sense::hover());ui.painter().rect_filled(rect,0.0,color);
        }});
        let image=ctx.input(|i|i.raw.events.iter().find_map(|event|match event {
            egui::Event::Screenshot{image,..}=>Some(image.clone()),_=>None,
        }));
        if let Some(image)=image {
            let result=(||->Result<(),String>{
                let (w,h)=(image.size[0],image.size[1]);
                if w==0 || h==0 || w as u64*h as u64>shareready_core::MAX_PIXELS{return Err("Readback dimensions invalid".into());}
                // Existing strict Core DIB intake encodes these synthetic pixels
                // without a new image dependency or screenshot export bypass.
                let mut dib=vec![0u8;124];
                for (at,value) in [(0,124u32),(4,w as u32),(8,(-(h as i32)) as u32),(16,3),
                    (40,0x00ff0000),(44,0x0000ff00),(48,0x000000ff),(52,0xff000000),(56,0x73524742)]{
                    dib[at..at+4].copy_from_slice(&value.to_le_bytes());
                }
                dib[12]=1;dib[14]=32;
                for pixel in &image.pixels {let [r,g,b,a]=pixel.to_srgba_unmultiplied();dib.extend([b,g,r,a]);}
                let png=shareready_core::dib_to_png(&dib)?;
                shareready_platform_windows::save_new(&self.output,&png).map_err(|e|e.to_string())?;
                println!("SR_GPU_READBACK_PASS width={w} height={h} pixels={}",image.pixels.len());Ok(())
            })();
            match result{Ok(())=>self.captured.store(true,Ordering::Release),Err(e)=>eprintln!("SR_GPU_READBACK_ERROR {e}")}
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.frames==4{ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));}
        if self.frames>120{eprintln!("SR_GPU_READBACK_TIMEOUT");ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
        ctx.request_repaint();
    }
}
fn main()->eframe::Result {
    let args:Vec<_>=std::env::args().skip(1).collect();
    if args.len()!=2 || args[0]!="--output"{eprintln!("Usage: readback_probe --output NEW_SYNTHETIC_PNG");std::process::exit(2);}
    let output=PathBuf::from(&args[1]);if output.exists(){eprintln!("Output must be a new file");std::process::exit(2);}
    let captured=Arc::new(AtomicBool::new(false));let done=captured.clone();
    let options=eframe::NativeOptions{renderer:eframe::Renderer::Wgpu,
        viewport:egui::ViewportBuilder::default().with_inner_size([640.0,400.0]).with_title("ShareReady · GPU readback probe"),..Default::default()};
    eframe::run_native("ShareReady readback probe",options,Box::new(move|_|Ok(Box::new(Probe{frames:0,output,captured}))))?;
    if !done.load(Ordering::Acquire){std::process::exit(3);}Ok(())
}
