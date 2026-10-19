//! No fake work or sleeps: fixed fixtures, ordinary worker, buffered diagnostics.
use super::*;
use std::{fs::OpenOptions,io::Write,path::PathBuf,time::Instant};
struct Run {
    app:NativeApp,out:PathBuf,start:Instant,rows:Vec<String>,phase:u8,frames:u64,last:Option<u64>,
    phase_frames:usize,waiting:Option<(String,String)>,busy_shot:bool,cancelled:bool,
    old_copy:Option<Ticket>,cancel_status:String,done:bool,
}
impl Run {
    fn row(&mut self,row:String){self.rows.push(row);}
    fn ms(&self)->f64{self.start.elapsed().as_secs_f64()*1000.0}
    fn request(&mut self,ctx:&egui::Context,name:&str){
        let ticket=self.app.review_task.as_ref().map(|t|t.key().ticket())
            .or_else(||self.app.review_texture.as_ref().map(|r|r.key().ticket())).expect("screenshot needs current task/artifact");
        self.row(format!("{{\"type\":\"screenshot_request\",\"name\":\"{name}\",\"ms\":{},\"frame\":{},\"reviewed\":{},\"busy\":{},\"document\":{},\"revision\":{},\"operation\":{}}}",self.ms(),ctx.cumulative_frame_nr(),self.app.editor.is_reviewed(),self.app.review_busy(),ticket.stamp.document_id,ticket.stamp.revision,ticket.operation_id));
        self.waiting=Some((name.into(),self.app.visual_snapshot(ctx)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(name.to_owned())));
    }
    fn output(&mut self,name:&str)->Result<(),String>{
        let ticket=self.app.editor.begin(OperationKind::Save).map_err(|e|format!("{e:?}"))?;
        self.app.save_choice(ticket,Some(&self.out.join(format!("{name}.png"))));
        if !self.out.join(format!("{name}.png")).is_file(){return Err(self.app.status.clone());}
        shareready_platform_windows::save_new(&self.out.join(format!("{name}.regions.json")),crate::trace::regions(&self.app.editor).as_bytes()).map_err(|e|e.to_string())?;
        let copy=self.app.editor.begin(OperationKind::Copy).map_err(|e|format!("{e:?}"))?;let dimensions=self.app.editor.dimensions();
        let path=self.out.join(format!("{name}.rgba"));let mut called=false;
        self.app.copy_choice(copy,|png|{
            let rgba=NativeApp::clipboard_pixels(png,dimensions)?;
            shareready_platform_windows::save_new(&path,rgba.bytes.as_ref()).map_err(|e|e.to_string())?;called=true;Ok(())
        });
        if !called{return Err(self.app.status.clone());}
        self.app.status="合成取证：受控输出核验完成，未使用系统剪贴板。".into();Ok(())
    }
    fn center_review(&mut self,ctx:&egui::Context){
        self.app.zoom=Zoom::Pixels(1.0);let r=self.app.editor.masks()[0].region;let ppp=ctx.pixels_per_point();
        ctx.data_mut(|d|d.insert_temp(egui::Id::new("sr-performance-offset"),
            (Vec2::new(r.x0 as f32,r.y0 as f32)/ppp-Vec2::new(150.0,150.0)).max(Vec2::ZERO)));
    }
    fn step(&mut self,ctx:&egui::Context)->Result<(),String>{
        if self.start.elapsed()>std::time::Duration::from_secs(120){return Err("bounded async window timeout".into());}
        if let Some((name,snapshot))=self.waiting.clone(){
            let image=ctx.input(|i|i.raw.events.iter().find_map(|e|match e{
                egui::Event::Screenshot{viewport_id,user_data,image} if *viewport_id==egui::ViewportId::ROOT &&
                    user_data.data.as_ref().and_then(|d|d.downcast_ref::<String>())==Some(&name)=>Some(image.clone()),_=>None}));
            if let Some(image)=image{
                if snapshot!=self.app.visual_snapshot(ctx){return Err(format!("state changed during {name} GPU readback"));}
                self.app.trace.as_mut().unwrap().screenshot(&format!("gpu-{name}"),&image,&self.app.editor,self.app.review_view,ctx.pixels_per_point()).map_err(|e|e.to_string())?;
                self.row(format!("{{\"type\":\"screenshot_received\",\"name\":\"{name}\",\"ms\":{},\"frame\":{}}}",self.ms(),ctx.cumulative_frame_nr()));
                self.waiting=None;
                match name.as_str(){"busy"=>self.busy_shot=true,"first"=>{self.phase=3;self.phase_frames=0;},"retry"=>self.done=true,_=>return Err("unknown screenshot".into())}
            }
        }
        match self.phase {
            0 if self.frames>=3=>{self.app.new_mask();self.app.start_review(ctx);self.phase=1;self.phase_frames=0;},
            1 if self.app.editor.is_reviewed() && self.app.delivery_blocked_frame.is_none() && self.busy_shot && self.waiting.is_none()=>{
                self.output("first")?;self.old_copy=Some(self.app.editor.begin(OperationKind::Copy).map_err(|e|format!("{e:?}"))?);
                self.app.editor.finish(self.old_copy.unwrap(),false);self.center_review(ctx);self.phase=2;self.phase_frames=0;
            },
            3=>{
                self.app.back_to_edit();self.app.new_mask();self.app.zoom=Zoom::Fit;
                ctx.data_mut(|d|d.insert_temp(egui::Id::new("sr-performance-offset"),Vec2::ZERO));
                self.app.start_review(ctx);self.phase=4;self.phase_frames=0;
            },
            4 if !self.cancelled && self.phase_frames>=3=>{
                let key=self.app.review_task.as_ref().ok_or("cancel worker disappeared")?.key().clone();
                review_task::event(ctx,&key,"cancel",Instant::now(),"");
                self.app.cancel_review_generation();
                if self.app.editor.review_is_current(&key){return Err("cancel did not revoke its current review key".into());}
                review_task::event(ctx,&key,"cancel_return",Instant::now(),"");self.app.new_mask();
                let id=self.app.editor.selected().ok_or("synthetic edit has no selected mask")?;
                if !self.app.editor.move_mask(id,-50,-30).map_err(|e|format!("{e:?}"))?{return Err("synthetic post-cancel edit was a no-op".into());}
                self.app.mark_edit();self.cancel_status=self.app.status.clone();self.cancelled=true;
            },
            4 if self.cancelled && self.app.review_task.is_none()=>{
                if self.app.editor.is_reviewed() || self.app.editor.output_png().is_some() || self.app.status!=self.cancel_status{return Err("stale cancelled result changed new editing state".into());}
                let status=self.app.status.clone();let mut called=false;self.app.copy_choice(self.old_copy.unwrap(),|_|{called=true;Ok(())});
                if called || self.app.status!=status{return Err("stale copy reached consumer or changed new status".into());}
                self.row(format!("{{\"type\":\"cancel_tail_released\",\"ms\":{}}}",self.ms()));self.phase=5;
            },
            5=>{self.app.start_review(ctx);self.phase=6;self.phase_frames=0;},
            6 if self.app.editor.is_reviewed() && self.app.delivery_blocked_frame.is_none()=>{
                self.output("retry")?;self.center_review(ctx);self.phase=7;self.phase_frames=0;
            },_=>{}
        }
        Ok(())
    }
    fn close(&mut self,ctx:&egui::Context,error:Option<String>){
        if let Some(error)=error{self.row(format!("{{\"type\":\"failed\",\"reason\":{error:?}}}"));}
        else{self.row("{\"type\":\"complete\",\"physical_input\":false,\"os_clipboard\":false}".into());}
        let path=self.out.join("async.jsonl");
        let mut file=OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        for row in &self.rows{writeln!(file,"{row}").unwrap();}file.sync_all().unwrap();
        println!("SR_ASYNC_WINDOW_TERMINAL rows={}",self.rows.len());self.done=true;self.app.close_confirmed=true;ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
impl eframe::App for Run {
    fn clear_color(&self,v:&egui::Visuals)->[f32;4]{self.app.clear_color(v)}
    fn ui(&mut self,ui:&mut egui::Ui,frame:&mut eframe::Frame){
        let callback_start=Instant::now();let callback_begin=self.ms();
        let ctx=ui.ctx().clone();if self.done{ctx.send_viewport_cmd(egui::ViewportCommand::Close);return;}
        let nr=ctx.cumulative_frame_nr();let fresh=self.last!=Some(nr);
        if fresh{self.last=Some(nr);self.frames+=1;self.phase_frames+=1;}
        ctx.input_mut(|i|{i.pointer=Default::default();i.events.retain(|e|matches!(e,egui::Event::Screenshot{..}));i.smooth_scroll_delta=Vec2::ZERO;});
        let controller_start=Instant::now();
        if fresh{if let Err(error)=self.step(&ctx){self.close(&ctx,Some(error));return;}}
        let controller_ms=controller_start.elapsed().as_secs_f64()*1000.0;
        if self.done{self.close(&ctx,None);return;}
        ctx.data_mut(|d|d.remove::<String>(egui::Id::new("sr-performance-metric")));
        let begin=self.ms();let cpu=Instant::now();self.app.ui(ui,frame);let ui_ms=cpu.elapsed().as_secs_f64()*1000.0;let end=self.ms();let callback_ms=callback_start.elapsed().as_secs_f64()*1000.0;
        let display=ctx.data(|d|d.get_temp::<String>(egui::Id::new("sr-performance-metric"))).unwrap_or("null".into());
        let events=ctx.data_mut(|d|std::mem::take(d.get_temp_mut_or_default::<Vec<String>>(egui::Id::new("sr-review-events"))));self.rows.extend(events);
        self.row(format!("{{\"type\":\"frame\",\"frame\":{nr},\"pass\":{},\"discard\":{},\"fresh\":{fresh},\"phase\":{},\"callback_begin_ms\":{callback_begin},\"controller_cpu_ms\":{controller_ms},\"callback_to_app_return_cpu_ms\":{callback_ms},\"begin_ms\":{begin},\"end_ms\":{end},\"ui_cpu_ms\":{ui_ms},\"busy\":{},\"reviewed\":{},\"display\":{display}}}",ctx.current_pass_index(),ctx.will_discard(),self.phase,self.app.review_busy(),self.app.editor.is_reviewed()));
        if !ctx.will_discard() && self.waiting.is_none(){
            if self.phase==1 && !self.busy_shot && self.phase_frames>=3 && self.app.review_task.is_some(){self.request(&ctx,"busy");}
            if self.phase==2 && self.phase_frames>=5{self.request(&ctx,"first");}
            if self.phase==7 && self.phase_frames>=5{self.request(&ctx,"retry");}
        }
        ctx.request_repaint();
    }
}
pub(crate) fn run()->eframe::Result{
    let args:Vec<_>=std::env::args().skip(1).collect();
    if args.len()!=2 || !matches!(args[1].as_str(),"grid"|"noise"){eprintln!("Usage: async_window NEW_DIR grid|noise");std::process::exit(2);}
    let out=PathBuf::from(&args[0]);if out.exists(){eprintln!("Requires a new synthetic directory");std::process::exit(2);}
    let source=if args[1]=="grid"{include_bytes!("../../assets/performance/grid-24mp.png").as_slice()}else{include_bytes!("../../../../tests/performance/fixtures/synthetic-noise-9mp.jpg").as_slice()};
    let mut trace=Trace::create_async(&out,source).unwrap();trace.pause_frame_logging();
    shareready_platform_windows::save_new(&out.join("normalized-source.png"),&shareready_core::preview_png(source).unwrap()).unwrap();
    let bridge=PasteBridge::default();let hook=bridge.clone();let start=Instant::now();
    let options=eframe::NativeOptions{renderer:eframe::Renderer::Wgpu,dithering:false,event_loop_builder:Some(Box::new(move|b|hook.install(b))),
        viewport:egui::ViewportBuilder::default().with_inner_size([1120.0,760.0]).with_title("ShareReady · 固定合成后台复核测试"),..Default::default()};
    eframe::run_native("ShareReady synthetic background review",options,Box::new(move|cc|{
        cc.egui_ctx.data_mut(|d|{d.insert_temp(egui::Id::new("sr-performance-active"),true);d.insert_temp(egui::Id::new("sr-review-origin"),start);});
        cc.egui_ctx.set_visuals(egui::Visuals::light());let app=NativeApp::new(cc,false,Some(trace),bridge,false);
        let mut run=Run{app,out,start,rows:Vec::new(),phase:0,frames:0,last:None,phase_frames:0,waiting:None,busy_shot:false,cancelled:false,old_copy:None,cancel_status:String::new(),done:false};
        let identity_unix_ns=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();let since_origin=start.elapsed();
        run.row(format!("{{\"type\":\"identity\",\"build\":\"{}\",\"debug_assertions\":{},\"fixture\":{:?},\"identity_unix_ns\":{identity_unix_ns},\"origin_unix_ns_estimate\":{},\"origin_to_identity_ms\":{},\"clock_scope\":\"adjacent wall/monotonic samples; approximate cross-clock anchor\"}}",option_env!("SR_BUILD_ID").unwrap_or("unidentified"),cfg!(debug_assertions),args[1],identity_unix_ns.saturating_sub(since_origin.as_nanos()),since_origin.as_secs_f64()*1000.0));
        Ok(Box::new(run))
    }))
}
