//! Compiled only by the explicit performance-evidence feature. No private intake.
use super::*;
use std::{fs::File,io::Write,path::PathBuf,time::Instant};
const ACTIVE:&str="sr-performance-active";
const OFFSET:&str="sr-performance-offset";
const METRIC:&str="sr-performance-metric";
pub(crate) fn scroll_offset(ctx:&egui::Context)->Option<Vec2>{
    ctx.data(|d|if d.get_temp::<bool>(egui::Id::new(ACTIVE))==Some(true){d.get_temp(egui::Id::new(OFFSET))}else{None})
}
fn unix_ns()->u128{std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()}
fn ms(start:Instant)->f64{start.elapsed().as_secs_f64()*1000.0}
fn new_frame(last:&mut Option<u64>,frame:u64)->bool{
    if *last==Some(frame){false}else{*last=Some(frame);true}
}
fn edge_offset(source:[u32;2],zoom:f32,ppp:f32,viewport:Vec2)->Vec2{
    (Vec2::new(source[0] as f32,source[1] as f32)*zoom/ppp-viewport).max(Vec2::ZERO)
}
struct Bench {
    app:NativeApp,out:PathBuf,log:File,source:&'static[u8],fixture:&'static str,rounds:usize,
    stage:usize,stage_frames:usize,frame:usize,waiting:bool,snapshot:String,last_time:Option<f64>,start:Instant,
    repeat:usize,repeat_phase:usize,done:bool,
    last_frame:Option<u64>,interval:Option<f64>,rows:Vec<String>,
}
impl Bench {
    fn row(&mut self,row:String){self.rows.push(row);}
    fn flush_rows(&mut self){
        for row in &self.rows{writeln!(self.log,"{row}").unwrap();println!("{row}");}
        self.log.flush().unwrap();std::io::stdout().flush().unwrap();self.rows.clear();
    }
    fn phase(&mut self,name:&str){
        self.row(format!("{{\"type\":\"phase\",\"name\":\"{name}\",\"round\":{},\"unix_ns\":{}}}",self.repeat,unix_ns()));
        // Reimport/review phases only, after all 30 interactive frames. Timed
        // controller stages start after the before-phase heartbeat returns.
        println!("SR_SYNTHETIC_PHASE round={} name={name}",self.repeat);std::io::stdout().flush().unwrap();
    }
    fn save_current(&mut self,name:&str)->Result<(),String>{
        let ticket=self.app.editor.begin(OperationKind::Save).map_err(|e|format!("{e:?}"))?;
        let png=self.app.editor.bytes_for(ticket).ok_or("missing current PNG")?;
        let start=Instant::now();let result=shareready_platform_windows::save_new(&self.out.join(format!("{name}.png")),png);
        let elapsed=ms(start);let accepted=self.app.editor.finish(ticket,result.is_ok());result.map_err(|e|e.to_string())?;
        if !accepted{return Err("stale benchmark save".into());}
        shareready_platform_windows::save_new(&self.out.join(format!("{name}.regions.json")),crate::trace::regions(&self.app.editor).as_bytes()).map_err(|e|e.to_string())?;
        self.row(format!("{{\"type\":\"save_new\",\"name\":\"{name}\",\"ms\":{elapsed},\"round\":{},\"document\":{},\"revision\":{},\"operation\":{}}}",self.repeat,ticket.stamp.document_id,ticket.stamp.revision,ticket.operation_id));Ok(())
    }
    fn review(&mut self,ctx:&egui::Context){
        // Exclude trace PNG duplication/disk I/O from actual start_review timing.
        let trace=self.app.trace.take();let start=Instant::now();self.app.start_review(ctx);let elapsed=ms(start);self.app.trace=trace;
        self.row(format!("{{\"type\":\"review\",\"scope\":\"ASYNC_TRIGGER_ONLY_NOT_COMPLETION\",\"round\":{},\"inclusive_controller_ms\":{elapsed},\"reviewed\":{},\"document\":{},\"revision\":{},\"operation\":\"NOT_OBSERVED\",\"unix_ns\":{}}}",self.repeat,self.app.editor.is_reviewed(),self.app.editor.stamp().document_id,self.app.editor.stamp().revision,unix_ns()));
    }
    fn repeat_step(&mut self,ctx:&egui::Context)->Result<bool,String>{
        if self.repeat>=self.rounds{return Ok(true);}
        match self.repeat_phase{
            0=>{
                self.phase("reimport-before");let start=Instant::now();
                let bytes=crate::input::read_source(&self.out.join(format!("source-{}-24mp.png",self.fixture)))?;
                if bytes!=self.source{return Err("fixed disk source changed; private intake refused".into());}
                let read_ms=ms(start);let start=Instant::now();
                self.app.offer_source(ctx,bytes,format!("合成24MP {}",self.fixture))?;let trigger_ms=ms(start);
                self.row(format!("{{\"type\":\"reimport_trigger\",\"scope\":\"REQUEST_ONLY_NOT_COMPLETION\",\"round\":{},\"bounded_read_and_exact_fixture_check_ms\":{read_ms},\"trigger_cpu_ms\":{trigger_ms},\"document\":{},\"unix_ns\":{}}}",self.repeat,self.app.editor.stamp().document_id,unix_ns()));
                self.repeat_phase=11;self.phase("reimport-trigger-returned");
            },
            11=>{
                if self.app.source_task.is_some(){return Ok(false);}
                if !matches!(self.app.pending,Some(PendingAction::Replace(_))){return Err(format!("reimport candidate absent: {}",self.app.status));}
                let stamp=self.app.editor.stamp();let start=Instant::now();self.app.resolve_pending(ctx,true);let replace_ms=ms(start);
                if self.app.editor.stamp().document_id!=stamp.document_id+1 || self.app.editor.source_bytes()!=Some(self.source){return Err("reimport did not accept the exact new source".into());}
                self.row(format!("{{\"type\":\"reimport\",\"round\":{},\"replace_editor_ms\":{replace_ms},\"had_discard_candidate\":true,\"document\":{},\"unix_ns\":{}}}",self.repeat,self.app.editor.stamp().document_id,unix_ns()));
                self.app.new_mask();self.repeat_phase=1;self.phase("reimport-returned-before-stabilize");
            },
            5=>{self.phase("review-before");self.review(ctx);self.repeat_phase=6;self.phase("review-trigger-returned");},
            6=>{
                if !self.app.review_busy() && self.app.delivery_blocked_frame.is_none(){
                    if !self.app.editor.is_reviewed(){return Err(format!("24MP review failed: {}",self.app.status));}
                    self.repeat_phase=7;self.phase("review-finished-before-stabilize");
                }
            },
            10=>{
                if self.repeat==0 || self.repeat+1==self.rounds{self.save_current(&format!("round-{}",self.repeat))?;}
                // Controlled consumer measures controller delivery; no OS clipboard.
                let ticket=self.app.editor.begin(OperationKind::Copy).map_err(|e|format!("{e:?}"))?;
                let start=Instant::now();self.app.copy_choice(ticket,|png|if png.starts_with(b"\x89PNG\r\n\x1a\n"){Ok(())}else{Err("invalid current artifact".into())});
                let elapsed=ms(start);self.row(format!("{{\"type\":\"controlled_copy\",\"round\":{},\"ms\":{elapsed},\"os_clipboard\":false,\"operation\":{}}}",self.repeat,ticket.operation_id));
                self.app.back_to_edit();self.repeat+=1;self.repeat_phase=0;self.phase("released-review-view-artifact-retained");
                return Ok(self.repeat==self.rounds);
            },_=>self.repeat_phase+=1,
        }
        Ok(false)
    }
}
impl eframe::App for Bench {
    fn clear_color(&self,visuals:&egui::Visuals)->[f32;4]{self.app.clear_color(visuals)}
    fn ui(&mut self,ui:&mut egui::Ui,frame:&mut eframe::Frame){
        let ctx=ui.ctx().clone();let fresh=new_frame(&mut self.last_frame,ctx.cumulative_frame_nr());
        if fresh{self.frame+=1;let time=ctx.input(|i|i.time);self.interval=self.last_time.map(|last|(time-last)*1000.0);self.last_time=Some(time);}
        if self.done{ctx.send_viewport_cmd(egui::ViewportCommand::Close);return;}
        ctx.input_mut(|i|{i.pointer=Default::default();i.smooth_scroll_delta=Vec2::ZERO;i.events.retain(|e|matches!(e,egui::Event::Screenshot{..}));});
        let mut finished=false;
        let controller_start=Instant::now();
        let result=if fresh{(||->Result<(),String>{
            if self.start.elapsed()>std::time::Duration::from_secs(120){return Err("bounded synthetic window timeout".into());}
            let shot=ctx.input(|i|i.raw.events.iter().find_map(|e|match e{egui::Event::Screenshot{viewport_id,user_data,image}
                if self.waiting && *viewport_id==egui::ViewportId::ROOT && user_data.data.as_ref().and_then(|d|d.downcast_ref::<usize>())==Some(&self.stage)=>Some(image.clone()),_=>None}));
            if let Some(image)=shot{
                let ready_ms=ms(self.start);let ready_unix_ns=unix_ns();
                if self.snapshot!=self.app.visual_snapshot(&ctx){return Err("screenshot state changed".into());}
                let evidence_start=Instant::now();
                self.app.trace.as_mut().unwrap().screenshot(&format!("stage-{}",self.stage),&image,&self.app.editor,self.app.review_view,ctx.pixels_per_point()).map_err(|e|e.to_string())?;
                self.row(format!("{{\"type\":\"screenshot_complete\",\"stage\":{},\"readback_ready_since_run_native_ms\":{ready_ms},\"readback_ready_unix_ns\":{ready_unix_ns},\"evidence_encode_write_ms\":{},\"frame\":{}}}",self.stage,ms(evidence_start),self.frame));
                self.stage+=1;self.stage_frames=0;self.waiting=false;
            }
            if self.stage<5{
                if self.stage_frames==0{
                    let (w,h)=self.app.editor.dimensions();
                    self.app.zoom=match self.stage{0=>Zoom::Fit,1=>Zoom::Pixels(1.0),2=>Zoom::Pixels(2.0),_=>Zoom::Pixels(1.0)};
                    let viewport=ctx.data(|d|d.get_temp::<Rect>(egui::Id::new("sr-performance-clip"))).unwrap_or(ctx.content_rect()).size();
                    let offset=match self.stage{0|1=>Vec2::ZERO,2=>edge_offset([w,h],2.0,ctx.pixels_per_point(),viewport),3=>edge_offset([w,h],1.0,ctx.pixels_per_point(),viewport),_=>{
                        let mask=self.app.editor.masks()[0].region;
                        (Vec2::new(mask.x0 as f32,h as f32*0.5)/ctx.pixels_per_point()-viewport*0.5).max(Vec2::ZERO)
                    }};
                    ctx.data_mut(|d|d.insert_temp(egui::Id::new(OFFSET),offset));
                    if self.stage==0{self.app.new_mask();}
                    if self.stage==3{self.review(&ctx);}
                }
                if self.stage==3 && (!self.app.editor.is_reviewed() || self.app.delivery_blocked_frame.is_some()){
                    self.stage_frames=(self.stage_frames+1).min(4);
                }else{self.stage_frames+=1;}
            }else if self.stage==5{
                self.app.back_to_edit();
                // Thirty actual egui frames with changing source crop/zoom/mask.
                let n=self.stage_frames;let (w,h)=self.app.editor.dimensions();
                self.app.zoom=if n%3==0{Zoom::Pixels(2.0)}else{Zoom::Pixels(1.0)};
                let offset=Vec2::new(((n*173)%((w-900) as usize)) as f32,((n*97)%((h-700) as usize)) as f32);
                ctx.data_mut(|d|d.insert_temp(egui::Id::new(OFFSET),offset));
                if let Some(id)=self.app.editor.selected(){if self.app.editor.move_mask(id,1,0).unwrap_or(false){self.app.mark_edit();}}
                self.stage_frames+=1;if self.stage_frames>30{self.stage=6;self.stage_frames=0;ctx.data_mut(|d|d.insert_temp(egui::Id::new(OFFSET),Vec2::ZERO));}
            }else{finished=self.repeat_step(&ctx)?;}
            Ok(())
        })()}else{Ok(())};
        let controller_ms=ms(controller_start);
        if let Err(error)=result{self.row(format!("{{\"type\":\"failed\",\"reason\":{:?}}}",error));self.flush_rows();self.done=true;self.app.close_confirmed=true;ctx.send_viewport_cmd(egui::ViewportCommand::Close);return;}
        ctx.data_mut(|d|d.remove::<String>(egui::Id::new(METRIC)));
        let start=Instant::now();self.app.ui(ui,frame);let cpu=ms(start);
        let display=ctx.data(|d|d.get_temp::<String>(egui::Id::new(METRIC))).unwrap_or("null".into());
        self.row(format!("{{\"type\":\"frame\",\"frame\":{},\"cumulative_frame_nr\":{},\"cumulative_pass_nr\":{},\"pass_index\":{},\"will_discard\":{},\"controller_advanced\":{fresh},\"controller_cpu_ms\":{controller_ms},\"stage\":{},\"stage_frame\":{},\"round\":{},\"repeat_phase\":{},\"app_ui_cpu_ms\":{cpu},\"frame_interval_ms\":{},\"previous_integration_cpu_ms\":{},\"pixels_per_point\":{},\"display\":{display},\"unix_ns\":{}}}",self.frame,ctx.cumulative_frame_nr(),ctx.cumulative_pass_nr(),ctx.current_pass_index(),ctx.will_discard(),self.stage,self.stage_frames,self.repeat,self.repeat_phase,
            self.interval.map(|v|v.to_string()).unwrap_or("null".into()),frame.info().cpu_usage.map(|v|(v*1000.0).to_string()).unwrap_or("null".into()),ctx.pixels_per_point(),unix_ns()));
        if self.stage<5 && self.stage_frames==5 && !self.waiting && !ctx.will_discard(){
            self.snapshot=self.app.visual_snapshot(&ctx);self.waiting=true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(self.stage)));
        }
        if finished{self.row("{\"type\":\"complete\",\"gpu_time\":\"NOT_MEASURED\",\"physical_input\":false}".into());self.flush_rows();self.done=true;self.app.close_confirmed=true;ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
        ctx.request_repaint();
    }
}
pub(crate) fn run()->eframe::Result{
    let args:Vec<_>=std::env::args().skip(1).collect();
    if args.len()!=4 || !matches!(args[1].as_str(),"grid"|"wide") || !matches!(args[2].as_str(),"native"|"512") || !matches!(args[3].as_str(),"1"|"20"){
        eprintln!("Usage: performance_window NEW_DIR grid|wide native|512 1|20");std::process::exit(2);
    }
    let out=PathBuf::from(&args[0]);if out.exists(){eprintln!("Performance evidence requires a new directory");std::process::exit(2);}
    let (fixture,source)=if args[1]=="grid"{("grid",include_bytes!("../../assets/performance/grid-24mp.png").as_slice())}else{("wide",include_bytes!("../../assets/performance/wide-24mp.png").as_slice())};
    let mut trace=Trace::create_performance(&out,source).unwrap();trace.pause_frame_logging();let log=std::fs::OpenOptions::new().write(true).create_new(true).open(out.join("performance.jsonl")).unwrap();
    let cap=(args[2]=="512").then_some(512usize);let rounds=args[3].parse().unwrap();let start=Instant::now();
    let bridge=PasteBridge::default();let hook=bridge.clone();
    let options=eframe::NativeOptions{renderer:eframe::Renderer::Wgpu,dithering:false,event_loop_builder:Some(Box::new(move|b|hook.install(b))),
        viewport:egui::ViewportBuilder::default().with_inner_size([1120.0,760.0]).with_title("ShareReady · 24MP 合成性能测试"),..Default::default()};
    eframe::run_native("ShareReady synthetic performance",options,Box::new(move|cc|{
        cc.egui_ctx.data_mut(|d|{d.insert_temp(egui::Id::new(ACTIVE),true);d.insert_temp(egui::Id::new(OFFSET),Vec2::ZERO);});
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        if let Some(cap)=cap{cc.egui_ctx.data_mut(|d|d.insert_temp(egui::Id::new("sr-performance-image-cap"),cap));}
        let ctor=Instant::now();let app=NativeApp::new(cc,false,Some(trace),bridge,false);let ctor_ms=ms(ctor);
        let mut bench=Bench{app,out,log,source,fixture,rounds,stage:0,stage_frames:0,frame:0,waiting:false,snapshot:String::new(),last_time:None,start,repeat:0,repeat_phase:0,done:false,last_frame:None,interval:None,rows:Vec::new()};
        bench.row(format!("{{\"type\":\"identity\",\"build\":\"{}\",\"fixture\":\"{fixture}\",\"debug_assertions\":{},\"rounds\":{rounds},\"constructor_ms\":{ctor_ms},\"process_setup_ms\":{},\"unix_ns\":{}}}",option_env!("SR_BUILD_ID").unwrap_or("unidentified"),cfg!(debug_assertions),ms(start),unix_ns()));
        bench.row("{\"type\":\"diagnostic_settings\",\"dithering\":false,\"trace_frame_logging\":false,\"frame_log_io\":\"buffered_until_complete\",\"stdout_heartbeat\":\"repeat_phases_only_after_interactions\",\"display_metric_cpu_included\":true}".into());
        Ok(Box::new(bench))
    }))
}

/// Release controller/readback regression. No window, input hook or OS clipboard.
pub(crate) fn run_large_artifact()->Result<(),String>{
    let args:Vec<_>=std::env::args_os().skip(1).collect();
    if args.len()!=1{return Err("Usage: generated_artifact_check NEW_SYNTHETIC_DIR".into());}
    let out=PathBuf::from(&args[0]);
    if out.exists(){return Err("Requires a new synthetic evidence directory".into());}
    std::fs::create_dir(&out).map_err(|e|e.to_string())?;
    let source=include_bytes!("../../../../tests/performance/fixtures/synthetic-noise-9mp.jpg");
    println!("SR_LARGE_ARTIFACT_START build={} debug_assertions={}",option_env!("SR_BUILD_ID").unwrap_or("unidentified"),cfg!(debug_assertions));
    let write=|name:&str,bytes:&[u8]|shareready_platform_windows::save_new(&out.join(name),bytes).map_err(|e|e.to_string());
    write("source.jpg",source)?;
    let normalized=shareready_core::preview_png(source)?;write("normalized-source.png",&normalized)?;drop(normalized);
    let ctx=egui::Context::default();
    let mut app=NativeApp{editor:Editor::from_source(source.to_vec()).map_err(|e|format!("{e:?}"))?,
          texture:Some(DisplayImage::decode(&ctx,source)?),review_texture:None,review_task:None,source_task:None,source_key:None,delivery_blocked_frame:None,source_label:"固定合成9MP JPEG".into(),
        zoom:Zoom::Fit,drag:None,preview:None,status:String::new(),review_view:false,smoke:false,frames:0,
        trace:None,pending:None,close_confirmed:false,paste_bridge:PasteBridge::default(),visual:None};
    app.new_mask();app.start_review(&ctx);
    let deadline=Instant::now()+std::time::Duration::from_secs(120);
    while app.review_busy(){
        if Instant::now()>deadline{return Err("headless async controller timeout".into());}
        let input=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1120.0,760.0))),
            max_texture_side:Some(display::texture_limit(&ctx)),..Default::default()};
        ctx.run_ui(input,|ui|app.show(ui)).drop_without_applying_deltas();std::thread::yield_now();
    }
    ctx.run_ui(Default::default(),|ui|app.show(ui)).drop_without_applying_deltas();
    if !app.editor.is_reviewed() || !app.review_view || app.review_texture.is_none(){return Err(app.status);}
    let png=app.editor.output_png().ok_or("no reviewed artifact")?.to_vec();
    if png.len()<=shareready_core::MAX_INPUT_BYTES{return Err("counterexample no longer exceeds source cap".into());}
    if DisplayImage::decode(&ctx,&png).is_ok(){return Err("untrusted source limit widened".into());}
    let size=app.editor.dimensions();let stamp=app.editor.stamp();
    let copy=app.editor.begin(OperationKind::Copy).map_err(|e|format!("{e:?}"))?;
    let mut copied=false;
    app.copy_choice(copy,|bytes|{
        let data=NativeApp::clipboard_pixels(bytes,size)?;
        if (data.width,data.height)!=(size.0 as usize,size.1 as usize){return Err("Copy dimensions changed".into());}
        write("controlled-copy.rgba",data.bytes.as_ref())?;copied=true;Ok(())
    });
    if !copied{return Err(app.status);}
    let save=app.editor.begin(OperationKind::Save).map_err(|e|format!("{e:?}"))?;
    app.save_choice(save,Some(&out.join("output.png")));
    if std::fs::read(out.join("output.png")).map_err(|e|e.to_string())?!=png{return Err("Save changed current PNG".into());}
    write("regions.json",crate::trace::regions(&app.editor).as_bytes())?;
    let finish_source=|app:&mut NativeApp|->Result<(),String>{
        let deadline=Instant::now()+std::time::Duration::from_secs(120);let cap=display::texture_limit(&ctx);
        while app.source_task.is_some(){
            if Instant::now()>deadline{return Err("headless source rejection timeout".into());}
            let input=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1120.0,760.0))),max_texture_side:Some(cap),..Default::default()};
            ctx.run_ui(input,|ui|app.show(ui)).drop_without_applying_deltas();std::thread::yield_now();
        }
        if app.pending.is_some() || app.editor.stamp()!=stamp || app.editor.output_png()!=Some(png.as_slice()){return Err("rejected oversized source changed current artifact".into());}
        Ok(())
    };
    app.offer_source(&ctx,png.clone(),"oversized source refused".into())?;finish_source(&mut app)?;
    app.paste_with(&ctx,||Ok(shareready_platform_windows::ClipboardImage{format:shareready_platform_windows::ClipboardImageFormat::Png,bytes:png.clone()}));
    finish_source(&mut app)?;
    if app.editor.stamp()!=stamp || !app.editor.is_reviewed(){return Err("rejected source lost current review".into());}
    let old=app.editor.begin(OperationKind::Copy).map_err(|e|format!("{e:?}"))?;
    app.editor.undo().map_err(|e|format!("{e:?}"))?;
    let mut stale_called=false;app.copy_choice(old,|_|{stale_called=true;Ok(())});
    if stale_called{return Err("stale consumer called".into());}
    println!("{{\"type\":\"large_artifact_complete\",\"build\":\"{}\",\"debug_assertions\":{},\"source_bytes\":{},\"png_bytes\":{},\"dimensions\":[{},{}],\"copy_operation\":{},\"save_operation\":{},\"source_intake_cap\":{},\"generated_cap\":{},\"os_clipboard\":false,\"window\":false}}",
        option_env!("SR_BUILD_ID").unwrap_or("unidentified"),cfg!(debug_assertions),source.len(),png.len(),size.0,size.1,copy.operation_id,save.operation_id,shareready_core::MAX_INPUT_BYTES,shareready_core::MAX_GENERATED_PNG_BYTES);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn discarded_pass_does_not_advance_controller_twice(){
        let ctx=egui::Context::default();let mut last=None;let mut mutations=0;let mut passes=0;
        let mut output=ctx.run_ui(Default::default(),|ui|{
            passes+=1;if new_frame(&mut last,ui.ctx().cumulative_frame_nr()){mutations+=1;}
            if ui.ctx().current_pass_index()==0{ui.request_discard("exercise real multi-pass");}
        });
        output.textures_delta.clear();assert_eq!(passes,2);assert_eq!(mutations,1);
    }
    #[test] fn right_bottom_scroll_keeps_last_source_pixel_visible(){
        assert_eq!(edge_offset([6000,4000],2.0,1.0,Vec2::new(843.0,704.0)),Vec2::new(11157.0,7296.0));
        assert_eq!(edge_offset([24000,1000],1.0,2.0,Vec2::new(843.0,704.0)),Vec2::new(11157.0,0.0));
    }
    #[test] fn completed_repeat_never_reads_or_replaces_another_source(){
        let dir=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/perf-controller-tests").join(unix_ns().to_string());
        std::fs::create_dir_all(&dir).unwrap();let log=File::create(dir.join("events.jsonl")).unwrap();
        let app=NativeApp{editor:Editor::new(100,80).unwrap(),texture:None,review_texture:None,review_task:None,source_task:None,source_key:None,delivery_blocked_frame:None,source_label:String::new(),zoom:Zoom::Fit,
            drag:None,preview:None,status:String::new(),review_view:false,smoke:false,frames:0,trace:None,pending:None,
            close_confirmed:false,paste_bridge:PasteBridge::default(),visual:None};
        let stamp=app.editor.stamp();let mut bench=Bench{app,out:dir,log,source:b"no source should be read",fixture:"grid",rounds:1,
            stage:6,stage_frames:0,frame:0,waiting:false,snapshot:String::new(),last_time:None,start:Instant::now(),repeat:1,repeat_phase:0,done:true,last_frame:None,interval:None,rows:Vec::new()};
        assert_eq!(bench.repeat_step(&egui::Context::default()),Ok(true));assert_eq!(bench.app.editor.stamp(),stamp);
    }
    #[test] fn repeated_import_waits_for_source_before_editing(){
        let dir=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/perf-controller-tests").join(unix_ns().to_string());
        std::fs::create_dir_all(&dir).unwrap();let log=File::create(dir.join("events.jsonl")).unwrap();
        let ctx=egui::Context::default();
        let (task,sent,release)=SourceTask::blocked_after_send(SourceInput::Bytes{bytes:DEMO.to_vec(),label:"synthetic".into()},display::texture_limit(&ctx));
        sent.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        let app=NativeApp{editor:Editor::new(100,80).unwrap(),texture:None,review_texture:None,review_task:None,source_key:Some(task.key().clone()),source_task:Some(task),delivery_blocked_frame:None,source_label:String::new(),zoom:Zoom::Fit,
            drag:None,preview:None,status:String::new(),review_view:false,smoke:false,frames:0,trace:None,pending:None,
            close_confirmed:false,paste_bridge:PasteBridge::default(),visual:None};
        let stamp=app.editor.stamp();let mut bench=Bench{app,out:dir,log,source:DEMO,fixture:"grid",rounds:1,
            stage:6,stage_frames:0,frame:0,waiting:false,snapshot:String::new(),last_time:None,start:Instant::now(),repeat:0,repeat_phase:11,done:false,last_frame:None,interval:None,rows:Vec::new()};
        let result=bench.repeat_step(&ctx);release.send(()).unwrap();
        assert_eq!(result,Ok(false));assert_eq!(bench.repeat_phase,11,"controller must await actual source completion");
        assert_eq!(bench.app.editor.stamp(),stamp);assert!(bench.app.editor.masks().is_empty());
    }
}
