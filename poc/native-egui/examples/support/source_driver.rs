//! Real bounded intake, no sleeps or artificial work; diagnostics are buffered.
use super::*;
use std::{fs::OpenOptions,io::Write,path::PathBuf,time::Instant};
struct Run {
    app:NativeApp,out:PathBuf,source:&'static[u8],source_path:PathBuf,start:Instant,rows:Vec<String>,
    phase:u8,frames:u64,last:Option<u64>,phase_frames:usize,key:Option<SourceKey>,
    waiting:Option<(String,String)>,busy_shot:bool,stamp:shareready_editor::Stamp,png:Vec<u8>,status:String,done:bool,
}
impl Run {
    fn row(&mut self,row:String){self.rows.push(row);}
    fn ms(&self)->f64{self.start.elapsed().as_secs_f64()*1000.0}
    fn output(&mut self,name:&str)->Result<(),String>{
        let ticket=self.app.editor.begin(OperationKind::Save).map_err(|e|format!("{e:?}"))?;
        self.app.save_choice(ticket,Some(&self.out.join(format!("{name}.png"))));
        if !self.out.join(format!("{name}.png")).is_file(){return Err(self.app.status.clone());}
        shareready_platform_windows::save_new(&self.out.join(format!("{name}.regions.json")),crate::trace::regions(&self.app.editor).as_bytes()).map_err(|e|e.to_string())?;
        let ticket=self.app.editor.begin(OperationKind::Copy).map_err(|e|format!("{e:?}"))?;
        let dimensions=self.app.editor.dimensions();let path=self.out.join(format!("{name}.rgba"));let mut called=false;
        self.app.copy_choice(ticket,|png|{let data=NativeApp::clipboard_pixels(png,dimensions)?;
            shareready_platform_windows::save_new(&path,data.bytes.as_ref()).map_err(|e|e.to_string())?;called=true;Ok(())});
        if !called{return Err(self.app.status.clone());}
        self.app.status="合成取证：受控输出核验完成，未使用系统剪贴板。".into();Ok(())
    }
    fn start_source(&mut self,ctx:&egui::Context)->Result<(),String>{
        self.app.back_to_edit();self.app.zoom=Zoom::Fit;
        ctx.data_mut(|d|d.insert_temp(egui::Id::new("sr-performance-offset"),Vec2::ZERO));
        self.stamp=self.app.editor.stamp();
        self.app.start_source(ctx,SourceInput::File(self.source_path.clone()))?;
        self.key=self.app.source_key.clone();self.phase_frames=0;Ok(())
    }
    fn accept(&mut self,ctx:&egui::Context)->Result<(),String>{
        if !matches!(self.app.pending,Some(PendingAction::Replace(_))){return Err(format!("source candidate absent: {}",self.app.status));}
        self.app.resolve_pending(ctx,true);
        if self.app.editor.stamp().document_id!=self.stamp.document_id+1 || self.app.editor.source_bytes()!=Some(self.source){return Err("exact source was not accepted".into());}
        let stamp=self.app.editor.stamp();source_task::event(ctx,self.key.as_ref().unwrap(),"accepted_observed",Instant::now(),
            &format!(",\"document\":{},\"revision\":{}",stamp.document_id,stamp.revision));Ok(())
    }
    fn center(&mut self,ctx:&egui::Context){
        self.app.zoom=Zoom::Pixels(1.0);let r=self.app.editor.masks()[0].region;let ppp=ctx.pixels_per_point();
        ctx.data_mut(|d|d.insert_temp(egui::Id::new("sr-performance-offset"),
            (Vec2::new(r.x0 as f32,r.y0 as f32)/ppp-Vec2::new(150.0,150.0)).max(Vec2::ZERO)));
    }
    fn request(&mut self,ctx:&egui::Context,name:&str){
        let stamp=self.app.editor.stamp();
        self.row(format!("{{\"type\":\"screenshot_request\",\"name\":\"{name}\",\"request\":{},\"ms\":{},\"frame\":{},\"reviewed\":{},\"review_view\":{},\"source_busy\":{},\"document\":{},\"revision\":{}}}",
            self.key.as_ref().unwrap().number(),self.ms(),ctx.cumulative_frame_nr(),self.app.editor.is_reviewed(),self.app.review_view,self.app.source_task.is_some(),stamp.document_id,stamp.revision));
        self.waiting=Some((name.into(),self.app.visual_snapshot(ctx)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(name.to_owned())));
    }
    fn step(&mut self,ctx:&egui::Context)->Result<(),String>{
        if self.start.elapsed()>std::time::Duration::from_secs(120){return Err("bounded source window timeout".into());}
        if let Some((name,snapshot))=self.waiting.clone(){
            let image=ctx.input(|i|i.raw.events.iter().find_map(|e|match e{egui::Event::Screenshot{viewport_id,user_data,image}
                if *viewport_id==egui::ViewportId::ROOT && user_data.data.as_ref().and_then(|d|d.downcast_ref::<String>())==Some(&name)=>Some(image.clone()),_=>None}));
            if let Some(image)=image{
                if snapshot!=self.app.visual_snapshot(ctx){return Err(format!("state changed during {name} GPU readback"));}
                self.app.trace.as_mut().unwrap().screenshot(&format!("gpu-{name}"),&image,&self.app.editor,self.app.review_view,ctx.pixels_per_point()).map_err(|e|e.to_string())?;
                self.row(format!("{{\"type\":\"screenshot_received\",\"name\":\"{name}\",\"request\":{},\"ms\":{},\"frame\":{}}}",self.key.as_ref().unwrap().number(),self.ms(),ctx.cumulative_frame_nr()));
                self.waiting=None;
                match name.as_str(){"busy"=>self.busy_shot=true,"first"=>self.phase=5,"retry"=>self.done=true,_=>return Err("unknown source screenshot".into())}
            }
        }
        match self.phase {
            0 if self.frames>=3=>{self.app.new_mask();self.app.start_review(ctx);self.phase=1;},
            1 if self.app.editor.is_reviewed() && self.app.delivery_blocked_frame.is_none()=>{
                self.output("old")?;self.start_source(ctx)?;self.phase=2;
            },
            2 if self.app.source_task.is_none() && self.waiting.is_none()=>{
                if !self.busy_shot{return Err("source completed before actual busy screenshot".into());}
                self.accept(ctx)?;self.app.new_mask();self.app.start_review(ctx);self.phase=3;
            },
            3 if self.app.editor.is_reviewed() && self.app.delivery_blocked_frame.is_none()=>{
                self.output("first")?;self.png=self.app.editor.output_png().unwrap().to_vec();self.center(ctx);self.phase=4;self.phase_frames=0;
            },
            5=>{self.start_source(ctx)?;self.phase=6;},
            6 if self.phase_frames>=3=>{
                let key=self.key.as_ref().unwrap();
                if self.app.source_task.is_none(){return Err("cancel task already finished".into());}
                source_task::event(ctx,key,"cancel",Instant::now(),"");self.app.cancel_source_generation();
                if self.app.source_key.is_some() || self.app.editor.stamp()!=self.stamp || self.app.editor.source_bytes()!=Some(self.source) || self.app.editor.output_png()!=Some(self.png.as_slice()){
                    return Err("cancel changed old source, stamp or PNG".into());
                }
                source_task::event(ctx,key,"cancel_return",Instant::now(),"");
                self.app.new_mask();let id=self.app.editor.selected().ok_or("no selected post-cancel edit")?;
                if !self.app.editor.move_mask(id,-50,-30).map_err(|e|format!("{e:?}"))?{return Err("post-cancel edit was a no-op".into());}
                self.app.mark_edit();self.stamp=self.app.editor.stamp();self.status=self.app.status.clone();self.phase=7;
            },
            7 if self.app.source_task.is_none()=>{
                if self.app.pending.is_some() || self.app.editor.stamp()!=self.stamp || self.app.editor.source_bytes()!=Some(self.source) || self.app.editor.output_png().is_some() || self.app.status!=self.status{
                    return Err("stale source result changed subsequent editing".into());
                }
                self.row(format!("{{\"type\":\"cancel_tail_released\",\"request\":{},\"ms\":{}}}",self.key.as_ref().unwrap().number(),self.ms()));
                self.start_source(ctx)?;self.phase=8;
            },
            8 if self.app.source_task.is_none()=>{
                self.accept(ctx)?;self.app.new_mask();let id=self.app.editor.selected().ok_or("no retry mask")?;
                if !self.app.editor.move_mask(id,-50,-30).map_err(|e|format!("{e:?}"))?{return Err("retry movement was a no-op".into());}
                self.app.mark_edit();self.app.start_review(ctx);self.phase=9;
            },
            9 if self.app.editor.is_reviewed() && self.app.delivery_blocked_frame.is_none()=>{
                self.output("retry")?;self.center(ctx);self.phase=10;self.phase_frames=0;
            },_=>{}
        }
        Ok(())
    }
    fn close(&mut self,ctx:&egui::Context,error:Option<String>){
        if let Some(error)=error{self.row(format!("{{\"type\":\"failed\",\"reason\":{error:?}}}"));}
        else{self.row("{\"type\":\"complete\",\"physical_input\":false,\"os_clipboard\":false}".into());}
        let mut file=OpenOptions::new().write(true).create_new(true).open(self.out.join("source.jsonl")).unwrap();
        for row in &self.rows{writeln!(file,"{row}").unwrap();}file.sync_all().unwrap();
        println!("SR_SOURCE_WINDOW_TERMINAL rows={}",self.rows.len());self.done=true;self.app.close_confirmed=true;ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
impl eframe::App for Run {
    fn clear_color(&self,v:&egui::Visuals)->[f32;4]{self.app.clear_color(v)}
    fn ui(&mut self,ui:&mut egui::Ui,frame:&mut eframe::Frame){
        let callback_start=Instant::now();let callback_begin=self.ms();let ctx=ui.ctx().clone();
        if self.done{ctx.send_viewport_cmd(egui::ViewportCommand::Close);return;}
        let nr=ctx.cumulative_frame_nr();let fresh=self.last!=Some(nr);
        if fresh{self.last=Some(nr);self.frames+=1;self.phase_frames+=1;}
        ctx.input_mut(|i|{i.pointer=Default::default();i.events.retain(|e|matches!(e,egui::Event::Screenshot{..}));i.smooth_scroll_delta=Vec2::ZERO;});
        let controller_start=Instant::now();
        if fresh{if let Err(error)=self.step(&ctx){self.close(&ctx,Some(error));return;}}
        let controller_ms=controller_start.elapsed().as_secs_f64()*1000.0;
        if self.done{self.close(&ctx,None);return;}
        ctx.data_mut(|d|d.remove::<String>(egui::Id::new("sr-performance-metric")));
        let begin=self.ms();let cpu=Instant::now();self.app.ui(ui,frame);let ui_ms=cpu.elapsed().as_secs_f64()*1000.0;let end=self.ms();
        let display=ctx.data(|d|d.get_temp::<String>(egui::Id::new("sr-performance-metric"))).unwrap_or("null".into());
        for id in ["sr-source-events","sr-review-events"]{self.rows.extend(ctx.data_mut(|d|std::mem::take(d.get_temp_mut_or_default::<Vec<String>>(egui::Id::new(id)))));}
        let app_return_ms=callback_start.elapsed().as_secs_f64()*1000.0;
        if !ctx.will_discard() && self.waiting.is_none(){
            if self.phase==2 && !self.busy_shot && self.phase_frames>=3 && self.app.source_task.is_some(){self.request(&ctx,"busy");}
            if self.phase==4 && self.phase_frames>=5{self.request(&ctx,"first");}
            if self.phase==10 && self.phase_frames>=5{self.request(&ctx,"retry");}
        }
        ctx.request_repaint();
        let request=self.app.source_task.as_ref().map(|t|t.key().number().to_string()).unwrap_or("null".into());
        self.row(format!("{{\"type\":\"frame\",\"frame\":{nr},\"pass\":{},\"discard\":{},\"fresh\":{fresh},\"phase\":{},\"callback_begin_ms\":{callback_begin},\"callback_end_ms\":{},\"controller_cpu_ms\":{controller_ms},\"callback_to_app_return_cpu_ms\":{app_return_ms},\"begin_ms\":{begin},\"end_ms\":{end},\"ui_cpu_ms\":{ui_ms},\"source_request\":{request},\"source_busy\":{},\"reviewed\":{},\"display\":{display}}}",ctx.current_pass_index(),ctx.will_discard(),self.phase,self.ms(),self.app.source_task.is_some(),self.app.editor.is_reviewed()));
    }
}
pub(crate) fn run()->eframe::Result{
    let args:Vec<_>=std::env::args().skip(1).collect();
    if args.len()!=2 || !matches!(args[1].as_str(),"grid"|"noise"){eprintln!("Usage: source_window NEW_DIR grid|noise");std::process::exit(2);}
    let out=PathBuf::from(&args[0]);if out.exists(){eprintln!("Requires a new synthetic directory");std::process::exit(2);}
    let source=if args[1]=="grid"{include_bytes!("../../assets/performance/grid-24mp.png").as_slice()}else{include_bytes!("../../../../tests/performance/fixtures/synthetic-noise-9mp.jpg").as_slice()};
    let mut trace=Trace::create_async(&out,source).unwrap();trace.pause_frame_logging();
    shareready_platform_windows::save_new(&out.join("normalized-source.png"),&shareready_core::preview_png(source).unwrap()).unwrap();
    shareready_platform_windows::save_new(&out.join("old-source.png"),DEMO).unwrap();
    let source_path=out.join(if args[1]=="grid"{"source.png"}else{"source.jpg"});
    let bridge=PasteBridge::default();let hook=bridge.clone();let start=Instant::now();
    let options=eframe::NativeOptions{renderer:eframe::Renderer::Wgpu,dithering:false,event_loop_builder:Some(Box::new(move|b|hook.install(b))),
        viewport:egui::ViewportBuilder::default().with_inner_size([1120.0,760.0]).with_title("ShareReady · 固定合成后台导入测试"),..Default::default()};
    eframe::run_native("ShareReady synthetic background source",options,Box::new(move|cc|{
        cc.egui_ctx.data_mut(|d|{d.insert_temp(egui::Id::new("sr-performance-active"),true);d.insert_temp(egui::Id::new("sr-source-origin"),start);d.insert_temp(egui::Id::new("sr-review-origin"),start);});
        cc.egui_ctx.set_visuals(egui::Visuals::light());let mut app=NativeApp::new(cc,false,None,bridge,false);app.trace=Some(trace);
        let stamp=app.editor.stamp();let mut run=Run{app,out,source,source_path,start,rows:Vec::new(),phase:0,frames:0,last:None,phase_frames:0,key:None,waiting:None,busy_shot:false,stamp,png:Vec::new(),status:String::new(),done:false};
        run.row(format!("{{\"type\":\"identity\",\"build\":\"{}\",\"debug_assertions\":{},\"fixture\":{:?},\"old_source\":\"T04_EDGES.png\",\"scope\":\"FIXED_SYNTHETIC_FILE_CPU_AND_GPU_NOT_PHYSICAL_ACCEPTANCE\"}}",option_env!("SR_BUILD_ID").unwrap_or("unidentified"),cfg!(debug_assertions),args[1]));Ok(Box::new(run))
    }))
}
