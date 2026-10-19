//! All edit/export decisions are delegated to shareready-editor/core.
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};
use crate::display::{self,DisplayImage,Zoom};
use crate::trace::Trace;
use shareready_core::Region;
#[cfg(test)]
use shareready_core::decode_source;
use shareready_editor::{Editor, GestureKind, GestureTicket, Handle, OperationKind, Point, Ticket, Viewport};
#[path="review_task.rs"] mod review_task;
#[path="source_task.rs"] mod source_task;
use review_task::{ReadyReview,ReviewTask};
use source_task::{SourceInput,SourceKey,SourceTask};
#[cfg(test)]
thread_local! {static TEST_DELIVERY:std::cell::Cell<(usize,usize)>=const{std::cell::Cell::new((0,0))};}
use shareready_native_input_bridge::PasteBridge;
use std::borrow::Cow;
use std::path::Path;
#[cfg(feature="performance-evidence")]
#[path="../examples/support/performance_driver.rs"]
pub(crate) mod performance_driver;
#[cfg(feature="performance-evidence")]
#[path="../examples/support/async_driver.rs"]
pub(crate) mod async_driver;
#[cfg(feature="performance-evidence")]
#[path="../examples/support/source_driver.rs"]
pub(crate) mod source_driver;

enum DragMode { Draw, Existing(GestureTicket) }
struct Drag { start:Pos2, mode:DragMode }
struct PreparedSource { source:shareready_editor::SourceCandidate, texture:DisplayImage, label:String, cap:usize }
enum PendingAction { Replace(PreparedSource), Close }
pub struct NativeApp {
    editor:Editor,
    texture:Option<DisplayImage>,
    review_texture:Option<ReadyReview>,
    review_task:Option<ReviewTask>,
    source_task:Option<SourceTask>,
    source_key:Option<SourceKey>,
    delivery_blocked_frame:Option<u64>,
    source_label:String,
    zoom:Zoom,
    drag:Option<Drag>,
    preview:Option<Region>,
    status:String,
    review_view:bool,
    smoke:bool,
    frames:usize,
    trace:Option<Trace>,
    pending:Option<PendingAction>,
    close_confirmed:bool,
    paste_bridge:PasteBridge,
    visual:Option<crate::visual::VisualRun>,
}

const DEMO: &[u8]=include_bytes!("../../../docs/research/fixtures/T04_EDGES.png");

impl NativeApp {
    pub fn new(cc:&eframe::CreationContext<'_>,smoke:bool,trace:Option<Trace>,paste_bridge:PasteBridge,visual:bool)->Self {
        crate::fonts::install(&cc.egui_ctx);
        assert!(!visual || trace.is_some(),"visual evidence requires a synthetic trace");
        if let Some(ref rs)=cc.wgpu_render_state {
            let info=rs.adapter.get_info();
            let lim=rs.adapter.limits();
            println!("SR_V2_03_GPU_ADAPTER backend={:?} type={:?} name={:?} vendor={} device={} max_texture_dimension_2d={}",
                info.backend,info.device_type,info.name,info.vendor,info.device,lim.max_texture_dimension_2d);
        } else {
            println!("SR_V2_03_GPU_ADAPTER wgpu_render_state=absent (glow or unavailable)");
        }
        let source=trace.as_ref().map(|t|t.fixture()).unwrap_or(DEMO);
        let label=trace.as_ref().map(|t|t.label()).unwrap_or("T04_EDGES.png");
        let editor=Editor::from_source(source.to_vec()).expect("bundled synthetic PNG must decode");
        let texture=DisplayImage::decode(&cc.egui_ctx,source).ok();
        let paste_status=paste_bridge.bind(cc,cc.egui_ctx.clone()).err();
        Self {
            editor,texture,review_texture:None,review_task:None,source_task:None,source_key:None,delivery_blocked_frame:None,
            source_label:format!("合成示例 {label} · 无私人图片"),
            zoom:Zoom::Fit,drag:None,preview:None,
            status:paste_status.unwrap_or_else(||"打开或粘贴图片，在画布上拖动以添加遮挡。".into()),
            review_view:false,smoke,frames:0,trace,pending:None,close_confirmed:false,paste_bridge,
            visual:visual.then(crate::visual::VisualRun::new),
        }
    }
    fn load_source(&mut self,ctx:&egui::Context,path:&Path){
        if self.close_confirmed{return;}
        if self.trace.is_some(){self.status="合成取证模式：已禁用私人图片导入。".into();return;}
        if self.pending.is_some(){self.status="请先处理当前确认，再打开其他图片。".into();return;}
        if let Err(e)=self.start_source(ctx,SourceInput::File(path.to_path_buf())){self.status=e;}
    }
    fn offer_source(&mut self,ctx:&egui::Context,bytes:Vec<u8>,label:String)->Result<(),String>{
        self.start_source(ctx,SourceInput::Bytes{bytes,label})
    }
    fn start_source(&mut self,ctx:&egui::Context,input:SourceInput)->Result<(),String>{
        if self.close_confirmed || self.pending.is_some() || self.source_task.is_some() || self.review_busy(){
            return Err("请先结束当前任务或确认，再打开其他图片。".into());
        }
        let task=SourceTask::spawn(input,display::texture_limit(ctx),ctx.clone()).map_err(|e|format!("无法开始导入，请重试：{e}"))?;
        self.source_key=Some(task.key().clone());self.source_task=Some(task);
        self.status="正在打开图片，可以取消或继续编辑。".into();ctx.request_repaint();
        #[cfg(feature="performance-evidence")]
        source_task::event(ctx,self.source_key.as_ref().unwrap(),"trigger_return",std::time::Instant::now(),"");
        Ok(())
    }
    fn cancel_source_generation(&mut self){
        if self.source_key.take().is_some(){self.status="已取消打开，后台任务结束后可以重试；当前图片和编辑已保留。".into();}
    }
    fn poll_source(&mut self,ctx:&egui::Context){
        if self.source_task.is_some(){ctx.request_repaint_after(std::time::Duration::from_millis(16));}
        if self.pending.is_some(){return;}
        let result=self.source_task.as_mut().and_then(SourceTask::poll);
        let Some(result)=result else{return;};let task=self.source_task.take().unwrap();
        #[cfg(feature="performance-evidence")]
        {
            if let Some(at)=task.received_at{source_task::event(ctx,task.key(),"received",at,"");}
            if let Some(at)=task.finished_at{source_task::event(ctx,task.key(),"finished_observed",at,"");}
        }
        let current=self.source_key.as_ref().filter(|key|key.matches(task.key())).cloned();
        if current.is_some(){self.source_key=None;}
        if self.close_confirmed || current.is_none(){
            #[cfg(feature="performance-evidence")]
            source_task::event(ctx,task.key(),"discarded_stale",std::time::Instant::now(),"");
            return;
        }
        if let Some(key)=current{match result.and_then(|prepared|prepared.install(ctx,&key)){
            Ok(prepared)=>{
                self.offer_prepared(ctx,prepared);
                #[cfg(feature="performance-evidence")]
                source_task::event(ctx,&key,"install_return",std::time::Instant::now(),"");
            },
            Err(e) if task.is_clipboard()=>self.status=format!("粘贴失败：{e}。请重试或打开 PNG/JPEG 文件，当前图片已保留。"),
            Err(e)=>self.image_rejected(e),
        }}
    }
    fn offer_prepared(&mut self,ctx:&egui::Context,prepared:PreparedSource){
        if self.has_edits(){self.cancel_drag();self.pending=Some(PendingAction::Replace(prepared));}
        else{self.replace_prepared(ctx,prepared);}
    }
    fn image_rejected(&mut self,error:impl std::fmt::Display){
        self.status=format!("图片无法打开，请选择有效的 PNG/JPEG 文件重试；当前图片和编辑记录已保留。详情：{error}");
    }
    fn has_edits(&self)->bool{
        self.editor.can_undo() || self.editor.can_redo() || self.drag.is_some() || self.editor.active_gesture()
    }
    fn replace_prepared(&mut self,ctx:&egui::Context,source:PreparedSource){
        if source.cap!=display::texture_limit(ctx){self.image_rejected("显示能力已变化，请重新打开图片。");return;}
        match self.editor.replace_candidate(source.source){
            Ok(())=>{
                self.texture=Some(source.texture);self.source_label=source.label;
                self.review_texture=None;self.review_view=false;self.drag=None;self.preview=None;
                self.zoom=Zoom::Fit;self.status="已在本地打开，原始图片未上传。".into();
            }
            Err(e)=>self.image_rejected(format!("{e:?}")),
        }
    }
    fn handle_close_request(&mut self,ctx:&egui::Context){
        if !ctx.input(|i|i.raw.viewport().close_requested()){return;}
        if self.close_confirmed {return;}
        if self.has_edits() || self.pending.is_some(){
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.cancel_drag();
            if self.pending.is_none(){self.pending=Some(PendingAction::Close);}
        }else if self.source_task.is_some(){
            self.cancel_source_generation();self.close_confirmed=true;
        }
    }
    fn resolve_pending(&mut self,ctx:&egui::Context,discard:bool){
        let Some(action)=self.pending.take() else{return;};
        if !discard {self.status="已保留当前图片和编辑记录。".into();return;}
        match action {
            PendingAction::Replace(source)=>self.replace_prepared(ctx,source),
            PendingAction::Close=>{
                self.cancel_source_generation();
                self.cancel_review_generation();self.review_texture=None;self.review_view=false;
                self.close_confirmed=true;ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
    fn show_confirmation(&mut self,ctx:&egui::Context){
        if self.pending.is_none(){return;}
        let response=egui::Modal::new(egui::Id::new("discard-edits")).show(ctx,|ui|{
            ui.heading("是否保留当前修改？");
            ui.label("打开其他图片或关闭窗口将丢弃当前编辑记录。导出的 PNG 不保留可编辑的遮挡框。");
            if let Some(PendingAction::Replace(source))=&self.pending {ui.label(format!("新图片：{}",source.label));}
            ui.horizontal(|ui|{
                if ui.button("继续编辑").clicked(){return Some(false);}
                if ui.button("放弃修改").clicked(){return Some(true);}
                None
            }).inner
        });
        if let Some(discard)=response.inner {self.resolve_pending(ctx,discard);}
        else if response.should_close(){self.resolve_pending(ctx,false);}
    }
    fn dropped_sources(&mut self,ctx:&egui::Context,paths:Vec<std::path::PathBuf>){
        if paths.len()>1 {self.status="请一次打开一张图片。本次未读取拖入的文件。".into();return;}
        if let Some(path)=paths.first(){self.load_source(ctx,path);}
    }
    fn mark_edit(&mut self){
        self.review_view=false;
        self.review_texture=None;
        self.status="修改已更新。请先复核新生成的 PNG，再复制或另存。".into();
    }
    fn back_to_edit(&mut self){
        if self.review_texture.as_ref().is_some_and(ReadyReview::awaiting){
            self.cancel_review_generation();self.status="已返回编辑，请重新生成复核图。".into();return;
        }
        self.review_view=false;
        self.status="已返回编辑。再次修改前，已复核的 PNG 仍可交付。".into();
    }
    fn cancel_drag(&mut self){
        if let Some(Drag{mode:DragMode::Existing(ticket),..})=self.drag.take(){let _=self.editor.cancel_gesture(ticket);}
        self.preview=None;
    }
    fn new_mask(&mut self){
        if self.drag.is_some() || self.pending.is_some(){return;}
        let (w,h)=self.editor.dimensions();
        let (w,h)=(i64::from(w),i64::from(h));
        let (rw,rh)=((w/4).max(1),(h/4).max(1));
        let (x,y)=((w-rw)/2,(h-rh)/2);
        if self.editor.add_mask(Region::new(x,y,x+rw,y+rh)).is_ok(){self.mark_edit();}
    }
    fn start_review(&mut self,ctx:&egui::Context){
        if self.close_confirmed || self.pending.is_some() || self.source_task.is_some() || self.review_busy(){return;}
        #[cfg(feature="performance-evidence")]
        let triggered=std::time::Instant::now();
        let work=match self.editor.prepare_review(){
            Ok(work)=>work,
            Err(shareready_editor::EditError::NoEffectiveMask)=>{self.status="请先添加至少一个有效遮挡，再生成复核图。".into();return;}
            Err(e)=>{self.status=format!("暂时无法复核：{e:?}");return;}
        };
        let key=work.key();self.review_texture=None;self.review_view=false;
        #[cfg(feature="performance-evidence")]
        review_task::event(ctx,&key,"trigger",triggered,"");
        self.delivery_blocked_frame=Some(ctx.cumulative_frame_nr());
        match ReviewTask::spawn(work,display::texture_limit(ctx),ctx.clone()){
            Ok(task)=>{self.review_task=Some(task);self.status="正在生成复核图，可以取消生成或继续编辑。".into();},
            Err(e)=>self.review_failed(&key,format!("无法开始生成，请重试：{e}")),
        }
        ctx.request_repaint();
        #[cfg(feature="performance-evidence")]
        review_task::event(ctx,&key,"trigger_return",std::time::Instant::now(),"");
    }
    fn review_busy(&self)->bool{
        self.review_task.is_some() || self.review_texture.as_ref().is_some_and(ReadyReview::awaiting)
    }
    fn review_failed(&mut self,key:&shareready_editor::ReviewKey,message:String){
        if self.editor.cancel_review(key){self.review_texture=None;self.review_view=false;self.status=message;}
    }
    fn cancel_review_generation(&mut self){
        let key=self.review_task.as_ref().map(|t|t.key().clone()).or_else(||self.review_texture.as_ref()
            .filter(|r|r.awaiting()).map(|r|r.key().clone()));
        if let Some(key)=key{self.review_failed(&key,"已取消生成，后台任务结束后可以重新生成。".into());}
    }
    fn begin_review_frame(&mut self,ctx:&egui::Context){
        let frame=ctx.cumulative_frame_nr();let entered_waiting=self.review_busy();
        if entered_waiting{self.delivery_blocked_frame=Some(frame);}
        else if self.delivery_blocked_frame!=Some(frame){self.delivery_blocked_frame=None;}
        if self.pending.is_none(){
            if let Some(ready)=self.review_texture.as_mut(){
                let key=ready.key().clone();
                match ready.advance(ctx,&mut self.editor,false){
                    Ok(Some(ticket))=>{
                        #[cfg(feature="performance-evidence")]
                        review_task::event(ctx,&key,"promoted",std::time::Instant::now(),"");
                        self.status="正在显示实际编码后的 PNG。复制前请检查所有未遮挡的细节。".into();
                        if let (Some(trace),Some(bytes))=(self.trace.as_mut(),self.editor.output_png()){
                            if let Err(e)=trace.review(&self.editor,bytes,ticket){self.status=format!("PNG 已显示；合成证据写入失败：{e}");}
                        }
                    },
                    Ok(None)=>{},
                    Err(e)=>self.review_failed(&key,e),
                }
            }
            let result=self.review_task.as_mut().and_then(ReviewTask::poll);
            if let Some(result)=result{
                let task=self.review_task.take().unwrap();let key=task.key().clone();
                #[cfg(feature="performance-evidence")]
                {
                    if let Some(at)=task.received_at{review_task::event(ctx,&key,"received",at,"");}
                    if let Some(at)=task.finished_observed_at{review_task::event(ctx,&key,"finished_observed",at,&format!(",\"join_ms\":{}",task.join_duration.unwrap_or_default().as_secs_f64()*1000.0));}
                    if !self.editor.review_is_current(&key){review_task::event(ctx,&key,"discarded_stale",std::time::Instant::now(),"");}
                }
                if self.editor.review_is_current(&key){match result{
                    Ok(prepared)=>{
                        #[cfg(feature="performance-evidence")]
                        let upload_start=std::time::Instant::now();
                        let installed=prepared.install(ctx,&mut self.editor);
                        #[cfg(feature="performance-evidence")]
                        review_task::event(ctx,&key,"install_return",std::time::Instant::now(),&format!(",\"upload_and_accept_cpu_ms\":{}",upload_start.elapsed().as_secs_f64()*1000.0));
                        match installed{
                        Ok(ready)=>{self.review_texture=Some(ready);self.review_view=true;self.status="正在显示生成的复核图，请稍候。".into();},
                        Err(e)=>self.review_failed(&key,e),
                    }},
                    Err(e)=>self.review_failed(&key,e),
                }}
            }
        }
        if let Some(ready)=self.review_texture.as_mut(){ready.begin_pass();}
        if self.review_busy() || entered_waiting{ctx.request_repaint_after(std::time::Duration::from_millis(16));}
    }
    fn save(&mut self){
        if self.trace.is_some(){return;}
        if self.close_confirmed || self.source_task.is_some() || self.review_busy() || self.delivery_blocked_frame.is_some(){return;}
        if self.pending.is_some(){return;}
        let ticket=match self.editor.begin(OperationKind::Save){
            Ok(t)=>t,Err(e)=>{self.status=format!("暂时无法另存：{e:?}");return;}
        };
        #[cfg(not(test))]
        let picked=rfd::FileDialog::new().add_filter("PNG",&["png"])
            .set_file_name("share_ready.png").save_file();
        #[cfg(test)]
        let picked:Option<std::path::PathBuf>={TEST_DELIVERY.with(|c|{let (copy,save)=c.get();c.set((copy,save+1));});None};
        self.save_choice(ticket,picked.as_deref());
    }
    fn save_choice(&mut self,ticket:Ticket,path:Option<&Path>){
        if ticket.kind!=OperationKind::Save {return;}
        let Some(bytes)=self.editor.bytes_for(ticket) else{return;};
        if self.close_confirmed || self.pending.is_some() || self.source_task.is_some() || self.review_busy() || self.delivery_blocked_frame.is_some(){self.editor.finish(ticket,false);return;}
        let Some(path)=path else{
            if self.editor.finish(ticket,false){self.status="已取消另存，未写入文件。".into();}
            return;
        };
        let result=shareready_platform_windows::save_new(path,bytes);
        if !self.editor.finish(ticket,result.is_ok()){return;}
        self.status=match result {
            Ok(())=>"已另存为新的 PNG 文件。请在其他图片查看器中核对结果。".into(),
            Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>"另存失败：文件名已存在，请选择新名称。".into(),
            Err(e)=>format!("另存失败；当前图片与复核结果已保留：{e}"),
        };
    }
    fn copy(&mut self){
        if self.trace.is_some(){return;}
        if self.close_confirmed || self.source_task.is_some() || self.review_busy() || self.delivery_blocked_frame.is_some(){return;}
        if self.pending.is_some(){return;}
        let ticket=match self.editor.begin(OperationKind::Copy){
            Ok(t)=>t,Err(e)=>{self.status=format!("暂时无法复制：{e:?}");return;}
        };
        #[cfg(test)]
        {TEST_DELIVERY.with(|c|{let (copy,save)=c.get();c.set((copy+1,save));});self.copy_choice(ticket,|_|Err("受控测试接收函数，未访问系统剪贴板。".into()));}
        #[cfg(not(test))]
        {
        let dimensions=self.editor.dimensions();
        self.copy_choice(ticket,|png|{
            let data=Self::clipboard_pixels(png,dimensions)?;
            let mut clipboard=arboard::Clipboard::new().map_err(|e|format!("{e}"))?;
            clipboard.set_image(data).map_err(|e|format!("{e}"))
        });
        }
    }
    fn clipboard_pixels(png:&[u8],dimensions:(u32,u32))->Result<arboard::ImageData<'static>,String>{
        let pixels=shareready_core::decode_generated_png(png,dimensions)?;
        Ok(arboard::ImageData{width:pixels.width() as usize,height:pixels.height() as usize,
            bytes:Cow::Owned(pixels.into_raw())})
    }
    fn copy_choice(&mut self,ticket:Ticket,write:impl FnOnce(&[u8])->Result<(),String>){
        if ticket.kind!=OperationKind::Copy{return;}
        let Some(png)=self.editor.bytes_for(ticket) else{return;};
        if self.close_confirmed || self.pending.is_some() || self.source_task.is_some() || self.review_busy() || self.delivery_blocked_frame.is_some(){self.editor.finish(ticket,false);return;}
        let result=write(png);
        if !self.editor.finish(ticket,result.is_ok()){return;}
        self.status=match result {
            Ok(())=>"系统剪贴板已接受图片；跨应用读回尚未验证。".into(),
            Err(e)=>format!("复制失败：{e}。请重试或另存为新 PNG，当前图片已保留。"),
        };
    }
    fn read_clipboard()->Result<shareready_platform_windows::ClipboardImage,String>{
        shareready_platform_windows::read_clipboard_image(shareready_core::MAX_INPUT_BYTES,shareready_core::MAX_DIB_BYTES).map_err(|e|e.to_string())
    }
    fn paste_with(&mut self,ctx:&egui::Context,read:impl FnOnce()->Result<shareready_platform_windows::ClipboardImage,String>){
        if self.close_confirmed{return;}
        if self.trace.is_some(){self.status="合成取证模式：已禁用剪贴板导入。".into();return;}
        if self.pending.is_some(){self.status="请先处理当前确认，再粘贴其他图片。".into();return;}
        if self.source_task.is_some() || self.review_busy(){self.status="请先结束当前任务，再粘贴其他图片。".into();return;}
        let result=read().and_then(|data|self.start_source(ctx,SourceInput::Clipboard(data)));
        if let Err(e)=result{self.status=format!("粘贴失败：{e}。请重试或打开 PNG/JPEG 文件，当前图片已保留。");}
    }
    fn frame_view(w:u32,h:u32,rect:Rect)->Option<Viewport>{
        Viewport::new(w,h,f64::from(rect.min.x),f64::from(rect.min.y),
            f64::from(rect.width()),f64::from(rect.height())).ok()
    }
    fn to_screen(rect:Region,view:Viewport)->Option<Rect>{
        let (l,t,r,b)=rect.clipped(view.image_width,view.image_height)?;
        let a=view.source_to_screen(Point{x:f64::from(l),y:f64::from(t)}).ok()?;
        let z=view.source_to_screen(Point{x:f64::from(r),y:f64::from(b)}).ok()?;
        Some(Rect::from_min_max(Pos2::new(a.x as f32,a.y as f32),
                                Pos2::new(z.x as f32,z.y as f32)))
    }
    fn hit_mask(&self,p:Pos2,view:Viewport)->Option<(u64,GestureKind)>{
        for mask in self.editor.masks().iter().rev(){
            let Some(rect)=Self::to_screen(mask.region,view) else{continue};
            let around=rect.expand(9.0);
            if !around.contains(p){continue;}
            let near_l=(p.x-rect.left()).abs()<=9.0;
            let near_r=(p.x-rect.right()).abs()<=9.0;
            let near_t=(p.y-rect.top()).abs()<=9.0;
            let near_b=(p.y-rect.bottom()).abs()<=9.0;
            let handle=match(near_t,near_b,near_l,near_r){
                (true,_,true,_)=>Some(Handle::Nw),
                (true,_,_,true)=>Some(Handle::Ne),
                (_,true,true,_)=>Some(Handle::Sw),
                (_,true,_,true)=>Some(Handle::Se),
                (true,_,_,_)=>Some(Handle::N),
                (_,true,_,_)=>Some(Handle::S),
                (_,_,true,_)=>Some(Handle::W),
                (_,_,_,true)=>Some(Handle::E),
                _=>None,
            };
            if let Some(handle)=handle{return Some((mask.id,GestureKind::Resize(handle)));}
            if rect.contains(p){return Some((mask.id,GestureKind::Move));}
        }
        None
    }
    fn update_pointer(&mut self,response:&egui::Response,view:Viewport){
        if self.pending.is_some(){return;}
        let hovered=response.interact_pointer_pos();
        if response.drag_started(){
            if let Some(p)=response.ctx.input(|i|i.pointer.press_origin()) {
                self.mark_edit();
                if let Some((id,kind))=self.hit_mask(p,view){
                    if let Ok(ticket)=self.editor.begin_gesture(id,kind){
                        self.drag=Some(Drag{start:p,mode:DragMode::Existing(ticket)});
                    }
                }else{
                    if self.editor.begin_draw_preview().is_err(){return;}
                    self.drag=Some(Drag{start:p,mode:DragMode::Draw});
                }
            }
        }
        if let Some(drag)=self.drag.as_ref(){
            if let Some(p)=hovered{
                match drag.mode {
                    DragMode::Draw=>{
                        self.preview=view.pixel_region(
                            Point{x:f64::from(drag.start.x),y:f64::from(drag.start.y)},
                            Point{x:f64::from(p.x),y:f64::from(p.y)}
                        ).ok();
                    }
                    DragMode::Existing(ticket)=>{
                        let dx=((p.x-drag.start.x)/response.rect.width()
                            *view.image_width as f32).round() as i64;
                        let dy=((p.y-drag.start.y)/response.rect.height()
                            *view.image_height as f32).round() as i64;
                        self.preview=self.editor.preview_gesture(ticket,dx,dy).ok();
                    }
                }
            }
        }
        if response.drag_stopped(){
            if let Some(drag)=self.drag.take(){
                let result=match drag.mode {
                    DragMode::Draw=>self.preview.and_then(|region|self.editor.add_mask(region).ok().map(|_|true)),
                    DragMode::Existing(t)=>self.editor.commit_gesture(t).ok(),
                };
                if result==Some(true){self.mark_edit();self.status="遮挡已更新。".into();}
                self.preview=None;
            }
        }
        if response.clicked(){
            if let Some(p)=hovered{
                let selection=self.hit_mask(p,view).map(|(id,_)|id);
                let _=self.editor.select(selection);
            }
        }
    }
    fn render_canvas(&mut self,ui:&mut egui::Ui){
        if self.texture.is_none(){ui.label("尚未打开图片");return;}
        let (w,h)=self.editor.dimensions();
        let shown=self.review_view && (self.editor.is_reviewed() || self.review_texture.as_ref()
            .is_some_and(|r|r.awaiting() && self.editor.review_is_current(r.key())));
        let size=display::image_size([w as usize,h as usize],self.zoom,ui.available_size(),ui.ctx().pixels_per_point());
        let scroll=egui::ScrollArea::both().auto_shrink([false,false]);
        #[cfg(feature="performance-evidence")]
        let scroll=if let Some(offset)=performance_driver::scroll_offset(ui.ctx()){scroll.scroll_offset(offset)}else{scroll};
        scroll.show(ui,|ui|{
            let (rect,response)=ui.allocate_exact_size(size,Sense::click_and_drag());
            if shown{
                if let Some(texture)=self.review_texture.as_mut(){texture.paint(ui,rect,self.zoom,&self.editor,self.pending.is_some());}
            }else if let Some(texture)=self.texture.as_mut(){texture.paint(ui,rect,self.zoom);}
            if let Some(view)=Self::frame_view(w,h,response.rect){
                if !shown {self.update_pointer(&response,view);}
                if !shown {
                    let painter=ui.painter();
                    for mask in self.editor.masks(){
                        if let Some(rect)=Self::to_screen(mask.region,view){
                            let selected=self.editor.selected()==Some(mask.id);
                            painter.rect_filled(rect,0.0,
                                if selected{Color32::from_rgba_unmultiplied(17,24,39,185)}
                                else{Color32::from_rgba_unmultiplied(17,24,39,135)});
                            if selected {
                                for p in [rect.left_top(),rect.center_top(),rect.right_top(),rect.left_center(),
                                    rect.right_center(),rect.left_bottom(),rect.center_bottom(),rect.right_bottom()] {
                                    painter.rect_filled(Rect::from_center_size(p,Vec2::splat(7.0)),1.0,Color32::WHITE);
                                }
                            }
                        }
                    }
                    if let Some(preview)=self.preview.and_then(|r|Self::to_screen(r,view)){
                        painter.rect_filled(preview,0.0,Color32::from_rgba_unmultiplied(217,112,58,95));
                    }
                }
            }
            if let Some(trace)=self.trace.as_mut(){
                let pointer=ui.ctx().input(|i|i.pointer.hover_pos());
                let (time,events)=ui.ctx().input(|i|(i.time,i.raw.events.clone()));
                if let Err(e)=trace.frame(&self.editor,rect,ui.ctx().pixels_per_point(),pointer,self.preview,time,&events){
                    self.status=format!("合成取证写入失败：{e}");
                }
            }
        });
    }
    fn keyboard(&mut self,ctx:&egui::Context){
        self.keyboard_with(ctx,Self::read_clipboard);
    }
    fn visual_frame(&mut self,ctx:&egui::Context){
        let Some(mut run)=self.visual.take() else{return;};
        run.total+=1;
        let shot=ctx.input(|i|i.raw.events.iter().find_map(|event|match event{
            egui::Event::Screenshot{viewport_id,user_data,image} if run.accepts(*viewport_id,user_data)=>Some(image.clone()),_=>None,
        }));
        if let Some(image)=shot{
            if !run.unchanged(&self.visual_snapshot(ctx)){
                eprintln!("SR_VISUAL_EVIDENCE_FAIL state changed while GPU image pending");self.close_confirmed=true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);return;
            }
            if let Err(e)=self.trace.as_mut().expect("synthetic trace required").screenshot(
                crate::visual::STATES[run.index],&image,&self.editor,self.review_view,ctx.pixels_per_point()){
                eprintln!("SR_VISUAL_EVIDENCE_FAIL {e}");self.close_confirmed=true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);return;
            }
            run.index+=1;run.frames=0;run.waiting=false;run.snapshot=None;
        }
        if run.index==crate::visual::STATES.len(){
            println!("SR_VISUAL_EVIDENCE_PASS states={}",run.index);self.close_confirmed=true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);return;
        }
        if run.started.elapsed()>std::time::Duration::from_secs(120){
            eprintln!("SR_VISUAL_EVIDENCE_FAIL timeout");self.close_confirmed=true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);return;
        }
        if run.frames==0{
            match run.index{
                0=>{ctx.set_visuals(egui::Visuals::light());self.new_mask();},
                1=>{if self.editor.add_mask(Region::new(30,30,750,230)).is_ok(){self.mark_edit();}},
                2=>self.start_review(ctx),
                3=>self.back_to_edit(),
                4=>{if let Err(e)=self.offer_source(ctx,b"invalid synthetic image".to_vec(),"合成错误示例".into()){
                    self.image_rejected(e);
                }},
                5=>{if let Err(e)=self.offer_source(ctx,DEMO.to_vec(),"合成替换示例.png".into()){
                    self.image_rejected(e);
                }},
                6=>{self.resolve_pending(ctx,false);ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(700.0,480.0)));},
                7=>ctx.set_visuals(egui::Visuals::dark()),
                _=>unreachable!(),
            }
        }
        run.frames+=1;
        self.visual=Some(run);ctx.request_repaint();
    }
    fn visual_snapshot(&self,ctx:&egui::Context)->String{
        format!("{:?}|{:?}|{:?}|{:?}|{}|{}|{}|{}|{:?}|{}|{}|{}|{:?}|{}|{:?}",
            self.editor.stamp(),self.editor.masks(),self.editor.selected(),self.zoom,self.review_view,
            self.editor.is_reviewed(),self.pending.is_some(),ctx.pixels_per_point(),ctx.content_rect(),
            ctx.global_style().visuals.dark_mode,self.status,self.source_label,
            self.review_texture.as_ref().map(|r|r.key().ticket()),display::texture_limit(ctx),self.review_task.as_ref().map(|t|t.key().ticket()))
    }
    fn request_visual_image(&mut self,ctx:&egui::Context){
        let snapshot=self.visual_snapshot(ctx);
        if let Some(run)=self.visual.as_mut(){if self.source_task.is_none() && run.frames>=5 && !run.waiting && !ctx.will_discard() &&
            (run.index!=2 || (self.editor.is_reviewed() && self.delivery_blocked_frame.is_none())){
            run.snapshot=Some(snapshot);run.waiting=true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(run.index)));
        }}
    }
    fn keyboard_with(&mut self,ctx:&egui::Context,read:impl FnOnce()->Result<shareready_platform_windows::ClipboardImage,String>){
        let paste=self.paste_bridge.take_pending();
        if self.close_confirmed || self.pending.is_some() || ctx.text_edit_focused(){return;}
        if paste || ctx.input_mut(|i|i.consume_key(egui::Modifiers::COMMAND,egui::Key::V)){
            self.paste_with(ctx,read);return;
        }
        let (undo,redo)=ctx.input_mut(|i|(
            i.consume_key(egui::Modifiers::COMMAND,egui::Key::Z),
            i.consume_key(egui::Modifiers::COMMAND,egui::Key::Y)));
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::Escape)){
            self.cancel_drag();let _=self.editor.select(None);
            if self.review_view{self.back_to_edit();}
            return;
        }
        if undo||redo {self.cancel_drag();}
        if undo {if self.editor.undo().is_ok(){self.mark_edit();}}
        if redo {if self.editor.redo().is_ok(){self.mark_edit();}}
        if self.drag.is_some(){return;}
        // Plain canvas commands must not steal modified platform shortcuts.
        let keys=ctx.input_mut(|i|{
            let mut keys=Vec::new();
            i.events.retain(|event|{
                if let egui::Event::Key {key,pressed:true,modifiers,..}=event {
                    if *modifiers==egui::Modifiers::NONE && matches!(key,egui::Key::N|egui::Key::Delete|egui::Key::ArrowLeft|egui::Key::ArrowRight|egui::Key::ArrowUp|egui::Key::ArrowDown){
                        keys.push(*key);return false;
                    }
                }true
            });keys
        });
        if keys.contains(&egui::Key::N){self.new_mask();}
        let delete=keys.contains(&egui::Key::Delete);
        if delete {
            if let Some(id)=self.editor.selected(){
                if self.editor.remove_mask(id).is_ok(){self.mark_edit();}
            }
        }
        for key in keys {
            let delta=match key {egui::Key::ArrowLeft=>(-1,0),egui::Key::ArrowRight=>(1,0),egui::Key::ArrowUp=>(0,-1),egui::Key::ArrowDown=>(0,1),_=>continue};
            if let Some(id)=self.editor.selected(){if self.editor.move_mask(id,delta.0,delta.1).unwrap_or(false){self.mark_edit();}}
        }
    }
}

impl eframe::App for NativeApp {
    fn clear_color(&self,visuals:&egui::Visuals)->[f32;4]{visuals.panel_fill.to_normalized_gamma_f32()}
    fn ui(&mut self,ui:&mut egui::Ui,_frame:&mut eframe::Frame){
        self.show(ui);
    }
}
impl NativeApp {
    fn show(&mut self,ui:&mut egui::Ui){
        let ctx=ui.ctx().clone();
        self.handle_close_request(&ctx);
        self.poll_source(&ctx);
        self.begin_review_frame(&ctx);
        let visual=self.visual.is_some();
        self.visual_frame(&ctx);
        if visual{ctx.input_mut(|i|{
            i.pointer=Default::default();i.smooth_scroll_delta=Vec2::ZERO;
            i.events.retain(|e|matches!(e,egui::Event::Screenshot{..}));
        });}
        if self.trace.is_some(){ctx.request_repaint_after(std::time::Duration::from_millis(100));}
        if self.smoke {
            self.frames+=1;
            if self.frames==3 {
                ctx.data_mut(|d|d.insert_temp(egui::Id::new("sr-smoke-start"),std::time::Instant::now()));
                self.new_mask();self.start_review(&ctx);
            }
            let timed_out=ctx.data(|d|d.get_temp::<std::time::Instant>(egui::Id::new("sr-smoke-start")))
                .is_some_and(|t|t.elapsed()>std::time::Duration::from_secs(120));
            if timed_out{eprintln!("SR_V2_03_NATIVE_WINDOW_LIFECYCLE_FAIL review timeout");self.close_confirmed=true;ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
            else if self.frames>7 && self.editor.is_reviewed() && !self.review_busy() && self.delivery_blocked_frame.is_none(){println!("SR_V2_03_NATIVE_WINDOW_LIFECYCLE_PASS");self.close_confirmed=true;ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
            else {ctx.request_repaint();}
        }
        if !visual && !self.close_confirmed{
            self.keyboard(&ctx);
            self.dropped_sources(&ctx,ctx.input(|i|i.raw.dropped_files.iter().map(|f|f.path().to_path_buf()).collect()));
        }
        if self.pending.is_some() || self.close_confirmed{ui.disable();}
        ui.group(|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.heading("ShareReady · 图片遮挡");
                ui.separator();
                let intake=self.trace.is_none() && self.source_task.is_none() && !self.review_busy();
                if ui.add_enabled(intake,egui::Button::new("打开图片")).clicked(){
                    if let Some(path)=rfd::FileDialog::new().add_filter("图片",&["png","jpg","jpeg"]).pick_file(){
                        self.load_source(&ctx,&path);
                    }
                }
                if ui.add_enabled(intake,egui::Button::new("粘贴图片 (Ctrl+V)")).clicked(){self.paste_with(&ctx,Self::read_clipboard);}
                if ui.add_enabled(self.drag.is_none(),egui::Button::new("新增遮挡 (N)")).clicked(){self.new_mask();}
                if ui.add_enabled(self.editor.can_undo() && self.drag.is_none(),egui::Button::new("撤销")).clicked() && self.editor.undo().is_ok(){self.mark_edit();}
                if ui.add_enabled(self.editor.can_redo() && self.drag.is_none(),egui::Button::new("重做")).clicked() && self.editor.redo().is_ok(){self.mark_edit();}
                if ui.add_enabled(self.drag.is_none() && self.source_task.is_none() && !self.review_busy(),egui::Button::new("生成复核图")).clicked(){self.start_review(&ctx);}
                if ui.add_enabled(self.review_view,egui::Button::new("返回编辑")).clicked(){self.back_to_edit();}
                let deliver=!self.close_confirmed && self.trace.is_none() && self.editor.is_reviewed() && self.source_task.is_none() && !self.review_busy() && self.delivery_blocked_frame.is_none();
                if ui.add_enabled(deliver,egui::Button::new("复制图片")).clicked(){self.copy();}
                if ui.add_enabled(deliver,egui::Button::new("另存为新 PNG")).clicked(){self.save();}
                ui.selectable_value(&mut self.zoom,Zoom::Fit,"适合窗口");
                ui.selectable_value(&mut self.zoom,Zoom::Pixels(1.0),"100%");
                ui.selectable_value(&mut self.zoom,Zoom::Pixels(2.0),"200%");
            });
            ui.label(&self.status);
            if self.source_task.is_some(){ui.horizontal(|ui|{
                ui.spinner();let current=self.source_key.is_some();
                ui.label(if current{"正在打开图片"}else{"正在结束上一次打开，请稍候"});
                if ui.add_enabled(current,egui::Button::new("取消打开")).clicked(){self.cancel_source_generation();}
            });}
            if self.review_busy(){ui.horizontal(|ui|{
                ui.spinner();
                let current=self.review_task.as_ref().map(|t|self.editor.review_is_current(t.key()))
                    .unwrap_or_else(||self.review_texture.as_ref().is_some_and(|r|self.editor.review_is_current(r.key())));
                ui.label(if current{"正在生成或显示复核图"}else{"正在结束上一次生成，请稍候"});
                if ui.add_enabled(current,egui::Button::new("取消生成")).clicked(){self.cancel_review_generation();}
            });}
        });
        // Keep image corners and toolbar visible at the minimum window size.
        ui.horizontal_top(|ui|{
            let inspector_width=260.0_f32.min(ui.available_width()*0.4);
            let height=ui.available_height();
            let canvas_width=(ui.available_width()-inspector_width-ui.spacing().item_spacing.x*2.0-1.0).max(1.0);
            ui.allocate_ui_with_layout(Vec2::new(canvas_width,height),egui::Layout::top_down(egui::Align::Min),|ui|self.render_canvas(ui));
            ui.separator();
            ui.allocate_ui_with_layout(Vec2::new(inspector_width,height),egui::Layout::top_down(egui::Align::Min),|ui|{
            ui.set_max_width(inspector_width);
            egui::ScrollArea::vertical().id_salt("inspector").show(ui,|ui|{
            ui.heading("检查器");
            ui.label(&self.source_label);
            ui.label(format!("原图尺寸：{}×{}",self.editor.dimensions().0,self.editor.dimensions().1));
            ui.label(format!("修改版本：{}",self.editor.stamp().revision));
            ui.label(format!("遮挡数量：{}",self.editor.masks().len()));
            if self.review_view{ui.strong("正在复核实际编码的 PNG");}
            ui.separator();
            let masks=self.editor.masks().to_vec();
            for m in masks {
                let picked=self.editor.selected()==Some(m.id);
                if ui.selectable_label(picked,format!("#{} [{},{},{},{}]",m.id,
                    m.region.x0,m.region.y0,m.region.x1,m.region.y1)).clicked(){
                    let _=self.editor.select(Some(m.id));
                }
            }
            ui.separator();
            if let Some(id)=self.editor.selected(){
                ui.label("每次移动 1 个原图像素（可用方向键）");
                ui.horizontal_wrapped(|ui|{
                    for (label,dx,dy) in [("左",-1,0),("右",1,0),("上",0,-1),("下",0,1)]{
                        if ui.button(label).clicked() && self.editor.move_mask(id,dx,dy).unwrap_or(false){self.mark_edit();}
                    }
                });
                ui.label("调整选中框边缘，每次 1 个原图像素");
                ui.horizontal_wrapped(|ui|{
                    for (label,h,dx,dy) in [
                        ("左上",Handle::Nw,-1,-1),("上",Handle::N,0,-1),("右上",Handle::Ne,1,-1),
                        ("左",Handle::W,-1,0),("右",Handle::E,1,0),
                        ("左下",Handle::Sw,-1,1),("下",Handle::S,0,1),("右下",Handle::Se,1,1)
                    ] {
                        if ui.button(label).clicked() {
                            if self.editor.resize_mask(id,h,dx,dy).unwrap_or(false) {self.mark_edit();}
                        }
                    }
                });
                if ui.button("删除选中遮挡").clicked(){
                    if self.editor.remove_mask(id).is_ok(){self.mark_edit();}
                }
            }
            if ui.button("清空遮挡").clicked() {
                if self.editor.clear_masks().unwrap_or(false){self.mark_edit();}
            }
            ui.separator();
            ui.small("当前为开发样机。系统剪贴板、跨屏缩放与辅助功能仍待真机验证。");
            });
            });
        });
        self.show_confirmation(&ctx);
        if visual{self.request_visual_image(&ctx);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn accepted_jpeg_with_large_generated_png_can_be_reviewed(){
        let source=include_bytes!("../../../tests/performance/fixtures/synthetic-noise-9mp.jpg");
        assert!(source.len()<shareready_core::MAX_INPUT_BYTES);
        let mut app=app();let ctx=egui::Context::default();
        app.editor=Editor::from_source(source.to_vec()).unwrap();app.new_mask();app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        assert!(app.editor.is_reviewed(),"actual Native start_review rejected accepted JPEG result: {}",app.status);
        assert!(app.review_view && app.review_texture.is_some());
        assert!(app.editor.output_png().unwrap().len()>shareready_core::MAX_INPUT_BYTES);
        let png=app.editor.output_png().unwrap().to_vec();let stamp=app.editor.stamp();
        let copy=app.editor.begin(OperationKind::Copy).unwrap();let size=app.editor.dimensions();
        let normalized=decode_source(source).unwrap();let region=app.editor.masks()[0].region;
        let mut called=false;
        app.copy_choice(copy,|bytes|{
            let data=NativeApp::clipboard_pixels(bytes,size)?;called=true;
            assert_eq!((data.width,data.height),(3000,3000));
            for (index,(got,original)) in data.bytes.chunks_exact(4).zip(normalized.pixels()).enumerate(){
                let (x,y)=((index%3000) as i64,(index/3000) as i64);
                let expected=if x>=region.x0 && x<region.x1 && y>=region.y0 && y<region.y1{[17,24,39,255]}else{original.0};
                assert_eq!(got,expected,"actual Copy pixel helper at {x},{y}");
            }
            Ok(())
        });
        assert!(called && app.editor.is_reviewed());
        app.offer_source(&ctx,png.clone(),"oversized source rejected".into()).unwrap();finish_source(&mut app,&ctx);
        assert!(app.pending.is_none());assert_eq!(app.editor.output_png(),Some(png.as_slice()));
        app.paste_with(&ctx,||Ok(shareready_platform_windows::ClipboardImage{
            format:shareready_platform_windows::ClipboardImageFormat::Png,bytes:png,
        }));
        finish_source(&mut app,&ctx);
        assert_eq!(app.editor.stamp(),stamp);assert!(app.editor.is_reviewed());
        let stale=app.editor.begin(OperationKind::Copy).unwrap();app.editor.undo().unwrap();
        app.copy_choice(stale,|_|panic!("stale large artifact must not reach a consumer"));
    }
    #[test] fn clear_color_matches_both_themes_and_is_opaque(){
        use eframe::App;
        for visuals in [egui::Visuals::light(),egui::Visuals::dark()]{
            assert_eq!(app().clear_color(&visuals),visuals.panel_fill.to_normalized_gamma_f32());
            assert_eq!(app().clear_color(&visuals)[3],1.0);
        }
    }
    #[test] fn visual_pending_image_is_invalidated_by_document_or_display_changes(){
        let ctx=egui::Context::default();let mut app=app();
        let mut run=crate::visual::VisualRun::new();run.snapshot=Some(app.visual_snapshot(&ctx));
        assert!(run.unchanged(&app.visual_snapshot(&ctx)));app.new_mask();
        assert!(!run.unchanged(&app.visual_snapshot(&ctx)));
        run.snapshot=Some(app.visual_snapshot(&ctx));app.zoom=Zoom::Pixels(2.0);
        assert!(!run.unchanged(&app.visual_snapshot(&ctx)));
        run.snapshot=Some(app.visual_snapshot(&ctx));app.pending=Some(PendingAction::Close);
        assert!(!run.unchanged(&app.visual_snapshot(&ctx)));
    }
    #[test] fn visual_capture_waits_for_real_source_completion(){
        let ctx=egui::Context::default();let mut app=app();
        let (task,sent,release)=SourceTask::blocked_after_send(SourceInput::Bytes{bytes:ASYNC_FIX.to_vec(),label:"synthetic".into()},display::texture_limit(&ctx));
        sent.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        app.source_key=Some(task.key().clone());app.source_task=Some(task);
        let mut run=crate::visual::VisualRun::new();run.index=4;run.frames=5;app.visual=Some(run);
        ctx.run_ui(Default::default(),|_|app.request_visual_image(&ctx)).drop_without_applying_deltas();
        let requested=app.visual.as_ref().unwrap().waiting;release.send(()).unwrap();
        assert!(!requested,"GPU state must not be requested while its real source task can change it");
    }
    fn app()->NativeApp {
        NativeApp {editor:Editor::new(100,80).unwrap(),texture:None,review_texture:None,
            review_task:None,source_task:None,source_key:None,delivery_blocked_frame:None,
            source_label:"synthetic test".into(),zoom:Zoom::Pixels(1.0),drag:None,preview:None,
            status:String::new(),review_view:false,smoke:false,frames:0,trace:None,pending:None,close_confirmed:false,paste_bridge:PasteBridge::default(),visual:None}
    }
    const ASYNC_FIX:&[u8]=include_bytes!("../../../docs/research/fixtures/T01_CHAT.png");
    #[test] fn source_intake_stays_pending_before_app_poll(){
        let ctx=egui::Context::default();let mut app=app();
        app.editor=Editor::from_source(ASYNC_FIX.to_vec()).unwrap();let stamp=app.editor.stamp();
        let path=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/fixtures/T01_CHAT.png");
        app.load_source(&ctx,&path);
        assert!(app.source_task.is_some(),"file intake must return while owned worker slot remains");
        assert_eq!(app.editor.stamp(),stamp);assert!(app.pending.is_none());
    }
    #[test] fn source_intake_busy_prevents_review_and_system_delivery(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);
        app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        let (task,sent,release)=SourceTask::blocked_after_send(SourceInput::Bytes{bytes:ASYNC_FIX.to_vec(),label:"synthetic".into()},display::texture_limit(&ctx));
        sent.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        app.source_key=Some(task.key().clone());app.source_task=Some(task);
        TEST_DELIVERY.with(|calls|calls.set((0,0)));app.copy();app.save();
        let calls=TEST_DELIVERY.with(|calls|calls.get());app.start_review(&ctx);
        let concurrent=app.review_task.is_some();release.send(()).unwrap();
        assert_eq!(calls,(0,0));assert!(!concurrent,"source and review must not run concurrently");
    }
    #[test] fn source_intake_close_without_edits_invalidates_request(){
        let ctx=egui::Context::default();let mut app=app();
        app.editor=Editor::from_source(ASYNC_FIX.to_vec()).unwrap();
        let (task,sent,release)=SourceTask::blocked_after_send(SourceInput::Bytes{bytes:ASYNC_FIX.to_vec(),label:"synthetic".into()},display::texture_limit(&ctx));
        sent.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        app.source_key=Some(task.key().clone());app.source_task=Some(task);
        let mut raw=async_input();raw.viewports.get_mut(&egui::ViewportId::ROOT).unwrap().events.push(egui::ViewportEvent::Close);
        ctx.run_ui(raw,|_|app.handle_close_request(&ctx)).drop_without_applying_deltas();
        release.send(()).unwrap();assert!(app.close_confirmed && app.source_key.is_none());
        let stamp=app.editor.stamp();let status=app.status.clone();
        for _ in 0..3{
            let mut raw=async_input();raw.viewports.get_mut(&egui::ViewportId::ROOT).unwrap().events.push(egui::ViewportEvent::Close);
            raw.events.push(key(egui::Key::N,egui::Modifiers::NONE));
            ctx.run_ui(raw,|_|{app.handle_close_request(&ctx);app.keyboard_with(&ctx,||panic!("closed intake must not read"));}).drop_without_applying_deltas();
            app.poll_source(&ctx);app.load_source(&ctx,Path::new("not-read-after-close.png"));
            app.paste_with(&ctx,||panic!("closed intake must not read"));
            assert!(app.close_confirmed && app.source_key.is_none());
            assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.status,status);
        }
    }
    #[test] fn source_intake_refuses_reads_while_review_waits(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.start_review(&ctx);
        let mut read=false;app.paste_with(&ctx,||{read=true;Err("must not reach reader".into())});
        assert!(!read,"guard must run before reading clipboard bytes");
    }
    fn finish_source(app:&mut NativeApp,ctx:&egui::Context){
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
        while app.source_task.is_some(){
            assert!(std::time::Instant::now()<deadline,"source task did not finish");
            async_frame(app,ctx);std::thread::yield_now();
        }
    }
    fn source_finished(app:&NativeApp){
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
        while !app.source_task.as_ref().unwrap().is_finished(){
            assert!(std::time::Instant::now()<deadline);std::thread::yield_now();
        }
    }
    #[test] fn source_intake_cancel_keeps_review_and_retains_slot(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);
        app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        let stamp=app.editor.stamp();let output=app.editor.output_png().unwrap().to_vec();
        app.offer_source(&ctx,ASYNC_FIX.to_vec(),"synthetic".into()).unwrap();
        app.cancel_source_generation();
        assert!(app.source_key.is_none() && app.source_task.is_some());
        assert!(app.offer_source(&ctx,ASYNC_FIX.to_vec(),"must wait".into()).is_err());
        finish_source(&mut app,&ctx);
        assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.output_png(),Some(output.as_slice()));
        assert!(app.editor.is_reviewed() && app.pending.is_none());
    }
    #[test] fn source_intake_checks_current_edits_before_replacement(){
        let ctx=egui::Context::default();let mut app=app();
        app.editor=Editor::from_source(ASYNC_FIX.to_vec()).unwrap();
        app.offer_source(&ctx,ASYNC_FIX.to_vec(),"synthetic".into()).unwrap();
        app.editor.add_mask(Region::new(10,10,30,30)).unwrap();let stamp=app.editor.stamp();
        finish_source(&mut app,&ctx);
        assert!(app.pending.is_some());assert_eq!(app.editor.stamp(),stamp);
        app.resolve_pending(&ctx,false);assert_eq!(app.editor.masks().len(),1);
        app.offer_source(&ctx,ASYNC_FIX.to_vec(),"retry".into()).unwrap();finish_source(&mut app,&ctx);
        assert!(app.pending.is_some());app.resolve_pending(&ctx,true);
        assert_eq!(app.editor.stamp().document_id,stamp.document_id+1);
        assert!(app.editor.masks().is_empty() && !app.editor.can_undo() && !app.editor.is_reviewed());
    }
    #[test] fn source_intake_errors_preserve_gesture_and_ignore_stale_status(){
        let ctx=egui::Context::default();
        for panics in [false,true]{for current in [false,true]{
            let mut app=async_app(&ctx);let id=app.editor.selected().unwrap();
            let gesture=app.editor.begin_gesture(id,GestureKind::Move).unwrap();let stamp=app.editor.stamp();
            let task=SourceTask::failed(panics);
            app.source_key=current.then(||task.key().clone());app.source_task=Some(task);
            app.status="latest edit status".into();source_finished(&app);app.poll_source(&ctx);
            assert!(app.source_task.is_none() && app.source_key.is_none());
            assert_eq!(app.editor.stamp(),stamp);assert!(app.editor.active_gesture());
            app.editor.cancel_gesture(gesture).unwrap();
            if !current{assert_eq!(app.status,"latest edit status");}
        }}
        let mut app=async_app(&ctx);app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        let output=app.editor.output_png().unwrap().to_vec();let stamp=app.editor.stamp();
        app.offer_source(&ctx,b"bad image".to_vec(),"bad".into()).unwrap();finish_source(&mut app,&ctx);
        assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.output_png(),Some(output.as_slice()));
        assert!(app.editor.is_reviewed() && app.pending.is_none());
    }
    #[test] fn source_intake_modal_defers_ready_candidate(){
        let ctx=egui::Context::default();let mut app=app();app.editor=Editor::from_source(ASYNC_FIX.to_vec()).unwrap();
        let stamp=app.editor.stamp();app.offer_source(&ctx,ASYNC_FIX.to_vec(),"synthetic".into()).unwrap();
        app.pending=Some(PendingAction::Close);source_finished(&app);app.poll_source(&ctx);
        assert!(app.source_task.is_some());assert_eq!(app.editor.stamp(),stamp);
        app.resolve_pending(&ctx,false);finish_source(&mut app,&ctx);
        assert_eq!(app.editor.stamp().document_id,stamp.document_id+1);assert!(!app.close_confirmed);
    }
    #[test] fn source_intake_cap_change_keeps_old_review(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        let stamp=app.editor.stamp();let output=app.editor.output_png().unwrap().to_vec();
        app.offer_source(&ctx,ASYNC_FIX.to_vec(),"synthetic".into()).unwrap();source_finished(&app);
        let mut raw=async_input();raw.max_texture_side=Some(display::texture_limit(&ctx)*2);
        ctx.run_ui(raw,|_|{}).drop_without_applying_deltas();app.poll_source(&ctx);
        assert!(app.source_task.is_none() && app.source_key.is_none() && app.pending.is_none());
        assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.output_png(),Some(output.as_slice()));
    }
    #[test] fn source_accept_rechecks_cap_after_discard_confirmation(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        let stamp=app.editor.stamp();let output=app.editor.output_png().unwrap().to_vec();let masks=app.editor.masks().to_vec();
        app.offer_source(&ctx,ASYNC_FIX.to_vec(),"synthetic".into()).unwrap();finish_source(&mut app,&ctx);
        assert!(app.pending.is_some());
        let mut raw=async_input();raw.max_texture_side=Some(display::texture_limit(&ctx)/2);
        ctx.run_ui(raw,|_|{}).drop_without_applying_deltas();app.resolve_pending(&ctx,true);
        assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.masks(),masks);
        assert_eq!(app.editor.output_png(),Some(output.as_slice()));assert!(app.pending.is_none() && app.editor.can_undo());
    }
    #[test] fn source_accept_old_error_does_not_clear_different_current_key(){
        let ctx=egui::Context::default();let mut app=app();let old=SourceTask::failed(false);let newer=SourceTask::failed(false);
        let current=newer.key().clone();app.source_key=Some(current.clone());app.source_task=Some(old);app.status="new request status".into();
        source_finished(&app);app.poll_source(&ctx);
        assert!(app.source_key.as_ref().is_some_and(|key|key.matches(&current)));assert_eq!(app.status,"new request status");
    }
    #[test] fn source_accept_clipboard_error_keeps_paste_context(){
        let ctx=egui::Context::default();let mut app=save_app();let stamp=app.editor.stamp();let output=app.editor.output_png().unwrap().to_vec();
        app.paste_with(&ctx,||Ok(shareready_platform_windows::ClipboardImage{format:shareready_platform_windows::ClipboardImageFormat::Png,bytes:b"bad PNG".to_vec()}));
        finish_source(&mut app,&ctx);
        assert!(app.status.contains("粘贴失败"));assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.output_png(),Some(output.as_slice()));
    }
    #[test] fn source_intake_rejects_prequeued_delivery_callbacks(){
        let ctx=egui::Context::default();let mut app=save_app();let output=app.editor.output_png().unwrap().to_vec();
        let copy=app.editor.begin(OperationKind::Copy).unwrap();
        app.offer_source(&ctx,ASYNC_FIX.to_vec(),"synthetic".into()).unwrap();
        app.copy_choice(copy,|_|panic!("queued Copy must not reach OS during intake"));
        assert_eq!(app.editor.last_delivery(),Some((copy,false)));app.cancel_source_generation();finish_source(&mut app,&ctx);
        let save=app.editor.begin(OperationKind::Save).unwrap();let path=save_path();
        app.offer_source(&ctx,ASYNC_FIX.to_vec(),"synthetic retry".into()).unwrap();app.save_choice(save,Some(&path));
        assert!(!path.exists());assert_eq!(app.editor.last_delivery(),Some((save,false)));
        app.cancel_source_generation();finish_source(&mut app,&ctx);
        assert_eq!(app.editor.output_png(),Some(output.as_slice()));
    }
    fn async_input()->egui::RawInput{
        egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1120.0,760.0))),max_texture_side:Some(4096),..Default::default()}
    }
    fn async_app(ctx:&egui::Context)->NativeApp{
        ctx.run_ui(async_input(),|_|{}).drop_without_applying_deltas();
        let mut app=app();app.editor=Editor::from_source(ASYNC_FIX.to_vec()).unwrap();
        app.texture=Some(DisplayImage::decode(ctx,ASYNC_FIX).unwrap());app.zoom=Zoom::Fit;app.new_mask();app
    }
    fn async_frame(app:&mut NativeApp,ctx:&egui::Context){
        let mut input=async_input();input.max_texture_side=Some(display::texture_limit(ctx));
        ctx.run_ui(input,|ui|app.show(ui)).drop_without_applying_deltas();
    }
    fn wait_uploaded(app:&mut NativeApp,ctx:&egui::Context){
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(15);
        while app.review_texture.is_none(){assert!(std::time::Instant::now()<deadline,"{}",app.status);async_frame(app,ctx);std::thread::yield_now();}
    }
    fn finish_async(app:&mut NativeApp,ctx:&egui::Context){
        wait_uploaded(app,ctx);
        for _ in 0..4 {async_frame(app,ctx);}
        assert!(app.editor.is_reviewed(),"{}",app.status);
    }
    fn finish_current_review(app:&mut NativeApp,ctx:&egui::Context){
        if app.texture.is_none(){app.texture=Some(DisplayImage::decode(ctx,app.editor.source_bytes().unwrap()).unwrap());}
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(120);
        while app.review_busy(){
            assert!(std::time::Instant::now()<deadline,"{}",app.status);
            async_frame(app,ctx);std::thread::yield_now();
        }
        async_frame(app,ctx);
        assert!(app.editor.is_reviewed(),"{}",app.status);
    }
    #[test] fn async_cancel_keeps_sent_worker_slot_until_actual_exit(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);
        let work=app.editor.prepare_review().unwrap();let key=work.key();
        let (task,sent,release)=review_task::ReviewTask::blocked_after_send(work,display::texture_limit(&ctx));
        app.review_task=Some(task);app.cancel_review_generation();
        sent.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        async_frame(&mut app,&ctx);app.start_review(&ctx);
        assert_eq!(app.review_task.as_ref().unwrap().key().ticket(),key.ticket());
        assert!(!app.editor.review_is_current(&key));assert!(!app.editor.is_reviewed());
        release.send(()).unwrap();
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
        while app.review_task.is_some(){assert!(std::time::Instant::now()<deadline);async_frame(&mut app,&ctx);std::thread::yield_now();}
        app.start_review(&ctx);finish_async(&mut app,&ctx);
    }
    #[test] fn async_hidden_canvas_cannot_finish_a_review(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.texture=None;
        app.start_review(&ctx);wait_uploaded(&mut app,&ctx);
        for _ in 0..4 {async_frame(&mut app,&ctx);}
        assert!(!app.editor.is_reviewed());assert!(app.review_texture.as_ref().unwrap().awaiting());
        app.texture=Some(DisplayImage::decode(&ctx,ASYNC_FIX).unwrap());finish_async(&mut app,&ctx);
    }
    #[test] fn async_discarded_pass_does_not_authorize_an_undrawn_final_pass(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.start_review(&ctx);
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
        while !app.review_task.as_ref().unwrap().is_finished(){assert!(std::time::Instant::now()<deadline);std::thread::yield_now();}
        let mut passes=0;
        ctx.run_ui(async_input(),|ui|{
            app.show(ui);passes+=1;
            if ui.ctx().current_pass_index()==0{
                assert!(app.review_texture.is_some());assert!(!app.editor.is_reviewed());
                app.texture=None;ui.ctx().request_discard("last pass has no canvas");
            }
        }).drop_without_applying_deltas();
        assert_eq!(passes,2);assert!(!app.editor.is_reviewed());
        for _ in 0..3{async_frame(&mut app,&ctx);assert!(!app.editor.is_reviewed());}
        app.texture=Some(DisplayImage::decode(&ctx,ASYNC_FIX).unwrap());finish_async(&mut app,&ctx);
    }
    #[test] fn async_current_worker_failure_and_panic_allow_retry_but_old_failure_is_ignored(){
        for (panics,stale) in [(false,false),(true,false),(true,true)]{
            let ctx=egui::Context::default();let mut app=async_app(&ctx);
            let work=app.editor.prepare_review().unwrap();let key=work.key();
            app.review_task=Some(ReviewTask::failed(work,panics));
            if stale{app.new_mask();app.status="newer editing status".into();}
            let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
            while !app.review_task.as_ref().unwrap().is_finished(){assert!(std::time::Instant::now()<deadline);std::thread::yield_now();}
            async_frame(&mut app,&ctx);
            assert!(!app.review_busy());assert!(!app.editor.review_is_current(&key));assert!(!app.editor.is_reviewed());
            if stale{assert_eq!(app.status,"newer editing status");}else{assert!(app.status.contains("failure") || app.status.contains("异常"));}
            app.start_review(&ctx);finish_async(&mut app,&ctx);
        }
    }
    #[test] fn async_start_failure_cleanup_uses_current_key_and_allows_retry(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);
        let work=app.editor.prepare_review().unwrap();let key=work.key();drop(work);
        app.review_failed(&key,"controlled spawn failure".into());
        assert!(!app.editor.review_is_current(&key));assert!(!app.review_busy());
        app.start_review(&ctx);finish_async(&mut app,&ctx);
    }
    #[test] fn async_confirmed_close_revokes_running_and_awaiting_results_before_late_frames(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);
        let work=app.editor.prepare_review().unwrap();let key=work.key();
        let (task,sent,release)=ReviewTask::blocked_after_send(work,display::texture_limit(&ctx));app.review_task=Some(task);
        sent.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        app.pending=Some(PendingAction::Close);app.resolve_pending(&ctx,true);
        assert!(app.close_confirmed);assert!(!app.editor.review_is_current(&key));
        assert!(app.review_task.is_some(),"confirmed close must not drop a live worker slot");release.send(()).unwrap();
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
        while app.review_task.is_some(){assert!(std::time::Instant::now()<deadline);async_frame(&mut app,&ctx);std::thread::yield_now();}
        assert!(!app.editor.is_reviewed());assert!(app.review_texture.is_none());
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.start_review(&ctx);wait_uploaded(&mut app,&ctx);
        let key=app.review_texture.as_ref().unwrap().key().clone();
        app.pending=Some(PendingAction::Close);app.resolve_pending(&ctx,true);
        assert!(!app.editor.review_is_current(&key));assert!(app.review_texture.is_none());assert!(!app.review_view);
        for _ in 0..3{async_frame(&mut app,&ctx);assert!(!app.editor.is_reviewed());}
        let mut app=save_app();let copy=app.editor.begin(OperationKind::Copy).unwrap();
        app.pending=Some(PendingAction::Close);app.resolve_pending(&ctx,true);
        app.copy_choice(copy,|_|panic!("confirmed close must not deliver a late Copy"));
        TEST_DELIVERY.with(|calls|calls.set((0,0)));app.copy();app.save();
        TEST_DELIVERY.with(|calls|assert_eq!(calls.get(),(0,0)));
    }
    #[test] fn async_waiting_frame_blocks_delivery_across_all_passes(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.start_review(&ctx);wait_uploaded(&mut app,&ctx);
        TEST_DELIVERY.with(|calls|calls.set((0,0)));
        let mut passes=0;
        ctx.run_ui(async_input(),|ui|{
            app.show(ui);app.copy();app.save();passes+=1;
            if ui.ctx().current_pass_index()==0 {ui.ctx().request_discard("controlled second pass after review finish");}
        }).drop_without_applying_deltas();
        assert_eq!(passes,2);assert!(app.editor.is_reviewed());
        TEST_DELIVERY.with(|calls|assert_eq!(calls.get(),(0,0)));
        async_frame(&mut app,&ctx);app.copy();app.save();
        TEST_DELIVERY.with(|calls|assert_eq!(calls.get(),(1,1)));
    }
    #[test] fn async_confirmed_close_is_terminal_across_repeated_viewport_close_events(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.start_review(&ctx);finish_async(&mut app,&ctx);
        let copy=app.editor.begin(OperationKind::Copy).unwrap();app.pending=Some(PendingAction::Close);app.resolve_pending(&ctx,true);
        TEST_DELIVERY.with(|calls|calls.set((0,0)));let status=app.status.clone();
        for _ in 0..3{
            assert!(!close_frame(&mut app,&ctx).iter().any(|c|matches!(c,egui::ViewportCommand::CancelClose)));
            assert!(app.close_confirmed,"actual repeated Close event must not reopen delivery");
            async_frame(&mut app,&ctx);app.copy_choice(copy,|_|panic!("late Copy after confirmed Close"));
            let save=app.editor.begin(OperationKind::Save).unwrap();app.save_choice(save,None);
            app.copy();app.save();app.start_review(&ctx);
            assert!(app.review_task.is_none());assert_eq!(app.status,status);
        }
        TEST_DELIVERY.with(|calls|assert_eq!(calls.get(),(0,0)));
    }
    #[test] fn async_modal_defers_result_and_discard_rejects_old_messages(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.start_review(&ctx);
        // Directly inject a validated candidate to exercise late review defense;
        // ordinary intake/review mutual exclusion has its own boundary test.
        let source=shareready_editor::SourceCandidate::decode(ASYNC_FIX.to_vec()).unwrap();
        let texture=DisplayImage::decode(&ctx,source.bytes()).unwrap();
        app.offer_prepared(&ctx,PreparedSource{source,texture,label:"synthetic replacement".into(),cap:display::texture_limit(&ctx)});
        assert!(app.pending.is_some());
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
        while !app.review_task.as_ref().unwrap().is_finished(){assert!(std::time::Instant::now()<deadline);std::thread::yield_now();}
        async_frame(&mut app,&ctx);assert!(app.review_texture.is_none());assert!(!app.editor.is_reviewed());
        app.resolve_pending(&ctx,true);let stamp=app.editor.stamp();let status=app.status.clone();
        async_frame(&mut app,&ctx);assert!(app.review_task.is_none());assert_eq!(app.editor.stamp(),stamp);
        assert_eq!(app.status,status);assert!(!app.editor.is_reviewed());
        app.new_mask();app.start_review(&ctx);app.pending=Some(PendingAction::Close);
        while !app.review_task.as_ref().unwrap().is_finished(){assert!(std::time::Instant::now()<deadline);std::thread::yield_now();}
        async_frame(&mut app,&ctx);assert!(app.review_texture.is_none());app.resolve_pending(&ctx,false);
        finish_async(&mut app,&ctx);
    }
    #[test] fn async_cap_change_rejects_upload_and_can_retry(){
        let ctx=egui::Context::default();let mut app=async_app(&ctx);app.start_review(&ctx);
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
        while !app.review_task.as_ref().unwrap().is_finished(){assert!(std::time::Instant::now()<deadline);std::thread::yield_now();}
        let mut input=async_input();input.max_texture_side=Some(1024);
        ctx.run_ui(input,|ui|app.show(ui)).drop_without_applying_deltas();
        assert!(app.review_task.is_none());assert!(app.review_texture.is_none());assert!(!app.editor.is_reviewed());
        assert!(app.status.contains("显示能力"));
        async_frame(&mut app,&ctx);app.start_review(&ctx);finish_async(&mut app,&ctx);
    }
    #[test] fn hundred_percent_image_uses_physical_pixels(){
        let ctx=egui::Context::default();
        ctx.set_pixels_per_point(2.0);
        let mut app=app();
        app.texture=Some(DisplayImage::from_color(&ctx,egui::ColorImage::filled([100,80],Color32::WHITE)));
        let input=egui::RawInput {screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(800.0,600.0))),..Default::default()};
        let mut output=ctx.run_ui(input,|ui|app.render_canvas(ui));
        output.textures_delta.clear();
        let primitives=ctx.tessellate(output.shapes,output.pixels_per_point);
        let image=primitives.iter().find_map(|s|match &s.primitive {
            egui::epaint::Primitive::Mesh(m) if m.texture_id!=egui::TextureId::default()=>Some(m.calc_bounds()),_=>None,
        }).expect("actual canvas image mesh");
        assert!((image.width()-50.0).abs()<0.6 && (image.height()-40.0).abs()<0.6,"actual image: {image:?}");
    }
    #[test] fn synthetic_trace_blocks_import_before_reading_or_replacing_source(){
        let dir=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/trace-tests")
            .join(format!("{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut app=app();
        app.trace=Some(Trace::create(&dir,DEMO).unwrap());
        app.editor.add_mask(Region::new(1,2,3,4)).unwrap();
        let stamp=app.editor.stamp();
        app.load_source(&egui::Context::default(),Path::new("nonexistent-private-image.png"));
        assert_eq!(app.editor.stamp(),stamp);
        assert_eq!(app.editor.masks().len(),1);
        assert!(app.status.contains("已禁用私人图片导入"));
        assert_eq!(std::fs::read(dir.join("source-T04.png")).unwrap(),DEMO);
    }
    fn key(key:egui::Key,modifiers:egui::Modifiers)->egui::Event {
        egui::Event::Key {key,physical_key:None,pressed:true,repeat:false,modifiers}
    }
    fn input(app:&mut NativeApp,ctx:&egui::Context,events:Vec<egui::Event>){
        let mut output=ctx.run_ui(egui::RawInput {events,..Default::default()},|_|app.keyboard(ctx));
        output.textures_delta.clear();
    }
    #[test] fn undo_uses_event_modifiers_after_ctrl_is_released_in_same_frame(){
        let mut app=app();let ctx=egui::Context::default();
        app.editor.add_mask(Region::new(10,10,30,30)).unwrap();
        input(&mut app,&ctx,vec![key(egui::Key::Z,egui::Modifiers::COMMAND)]);
        assert!(app.editor.masks().is_empty());
        input(&mut app,&ctx,vec![key(egui::Key::Y,egui::Modifiers::COMMAND)]);
        assert_eq!(app.editor.masks().len(),1);
        let windows=egui::Modifiers{ctrl:true,command:true,..egui::Modifiers::NONE};
        input(&mut app,&ctx,vec![key(egui::Key::Z,windows)]);
        assert!(app.editor.masks().is_empty());
        input(&mut app,&ctx,vec![key(egui::Key::Y,windows)]);
        assert_eq!(app.editor.masks().len(),1);
    }
    #[test] fn editing_clears_previous_review_success_message(){
        let mut app=app();let ctx=egui::Context::default();
        app.editor.add_mask(Region::new(10,10,30,30)).unwrap();
        app.status="正在显示实际编码后的 PNG。复制前请检查所有未遮挡的细节。".into();
        app.editor.undo().unwrap();app.mark_edit();
        assert!(!app.status.contains("正在显示实际编码后的 PNG"));
        assert!(!app.review_view);
        input(&mut app,&ctx,vec![]);
    }
    #[test] fn text_edit_focus_keeps_delete_and_undo_out_of_canvas(){
        let mut app=app();let ctx=egui::Context::default();
        app.editor.add_mask(Region::new(10,10,30,30)).unwrap();
        let mut text="abc".to_owned();
        let mut output=ctx.run_ui(egui::RawInput::default(),|ui|{ui.text_edit_singleline(&mut text).request_focus();});
        output.textures_delta.clear();
        assert!(ctx.text_edit_focused());
        input(&mut app,&ctx,vec![key(egui::Key::Delete,egui::Modifiers::NONE),key(egui::Key::Z,egui::Modifiers::COMMAND)]);
        assert_eq!(app.editor.masks().len(),1);
    }
    #[test] fn escape_cancels_active_gesture_without_committing_history(){
        let mut app=app();let ctx=egui::Context::default();
        let id=app.editor.add_mask(Region::new(10,10,30,30)).unwrap();
        let stamp=app.editor.stamp();
        let ticket=app.editor.begin_gesture(id,GestureKind::Move).unwrap();
        app.preview=Some(app.editor.preview_gesture(ticket,4,5).unwrap());
        app.drag=Some(Drag {start:Pos2::ZERO,mode:DragMode::Existing(ticket)});
        input(&mut app,&ctx,vec![key(egui::Key::Escape,egui::Modifiers::NONE)]);
        assert!(app.drag.is_none() && app.preview.is_none() && !app.editor.active_gesture());
        assert_eq!(app.editor.stamp(),stamp);
        assert_eq!(app.editor.masks()[0].region,Region::new(10,10,30,30));
    }
    #[test] fn keyboard_creates_selects_and_moves_one_source_pixel(){
        let mut app=app();let ctx=egui::Context::default();
        input(&mut app,&ctx,vec![key(egui::Key::N,egui::Modifiers::NONE)]);
        assert_eq!(app.editor.masks().len(),1);
        let before=app.editor.masks()[0].region;
        input(&mut app,&ctx,vec![key(egui::Key::ArrowRight,egui::Modifiers::NONE)]);
        assert_eq!(app.editor.masks()[0].region,Region::new(before.x0+1,before.y0,before.x1+1,before.y1));
        input(&mut app,&ctx,vec![key(egui::Key::Escape,egui::Modifiers::NONE)]);
        assert_eq!(app.editor.selected(),None);
    }
    #[test] fn real_pointer_draw_preserves_press_origin_and_invalidates_pending_copy(){
        let ctx=egui::Context::default();let mut app=app();
        app.editor=Editor::from_source(DEMO.to_vec()).unwrap();
        app.editor.add_mask(Region::new(10,10,30,30)).unwrap();
        let review=app.editor.begin(OperationKind::Review).unwrap();assert!(app.editor.finish(review,true));
        let stale=app.editor.begin(OperationKind::Copy).unwrap();
        let frame=|app:&mut NativeApp,events:Vec<egui::Event>|{
            let raw=egui::RawInput{events,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1200.0,1000.0))),..Default::default()};
            let mut output=ctx.run_ui(raw,|ui|{
                let (rect,response)=ui.allocate_exact_size(Vec2::new(1000.0,800.0),Sense::click_and_drag());
                app.update_pointer(&response,NativeApp::frame_view(2400,1600,rect).unwrap());
            });output.textures_delta.clear();
        };
        frame(&mut app,vec![]);
        let down=Pos2::new(300.0,200.0);let end=Pos2::new(500.0,400.0);
        frame(&mut app,vec![egui::Event::PointerMoved(down),egui::Event::PointerButton {pos:down,button:egui::PointerButton::Primary,pressed:true,modifiers:egui::Modifiers::NONE}]);
        frame(&mut app,vec![egui::Event::PointerMoved(end)]);
        assert_eq!(app.drag.as_ref().unwrap().start,down);
        assert_eq!(app.preview,Some(Region::new(720,400,1200,800)));
        assert!(!app.editor.is_reviewed() && !app.editor.finish(stale,true));
        frame(&mut app,vec![egui::Event::PointerButton {pos:end,button:egui::PointerButton::Primary,pressed:false,modifiers:egui::Modifiers::NONE}]);
        assert_eq!(app.editor.masks()[1].region,Region::new(720,400,1200,800));
        app.editor.undo().unwrap();assert_eq!(app.editor.masks().len(),1);
    }
    #[test] fn eight_handles_resize_through_real_pointer_sequences(){
        for (press,delta,expected) in [
            ((200.0,200.0),(-10.0,-10.0),Region::new(19,19,80,60)),
            ((500.0,200.0),(0.0,-10.0),Region::new(20,19,80,60)),
            ((800.0,200.0),(10.0,-10.0),Region::new(20,19,81,60)),
            ((200.0,400.0),(-10.0,0.0),Region::new(19,20,80,60)),
            ((800.0,400.0),(10.0,0.0),Region::new(20,20,81,60)),
            ((200.0,600.0),(-10.0,10.0),Region::new(19,20,80,61)),
            ((500.0,600.0),(0.0,10.0),Region::new(20,20,80,61)),
            ((800.0,600.0),(10.0,10.0),Region::new(20,20,81,61)),
        ] {
            let ctx=egui::Context::default();let mut app=app();
            app.editor.add_mask(Region::new(20,20,80,60)).unwrap();
            let frame=|app:&mut NativeApp,events|{
                let mut output=ctx.run_ui(egui::RawInput {events,..Default::default()},|ui|{
                    let (rect,response)=ui.allocate_exact_size(Vec2::new(1000.0,800.0),Sense::click_and_drag());
                    app.update_pointer(&response,NativeApp::frame_view(100,80,rect).unwrap());
                });output.textures_delta.clear();
            };
            let p=Pos2::new(press.0,press.1);let end=p+Vec2::new(delta.0,delta.1);
            frame(&mut app,vec![]);
            frame(&mut app,vec![egui::Event::PointerMoved(p),egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed:true,modifiers:egui::Modifiers::NONE}]);
            frame(&mut app,vec![egui::Event::PointerMoved(end)]);
            frame(&mut app,vec![egui::Event::PointerButton{pos:end,button:egui::PointerButton::Primary,pressed:false,modifiers:egui::Modifiers::NONE}]);
            assert_eq!(app.editor.masks()[0].region,expected,"handle at {p:?}");
            app.editor.undo().unwrap();assert_eq!(app.editor.masks()[0].region,Region::new(20,20,80,60));
        }
    }
    #[test] fn return_to_edit_keeps_png_until_next_drawing_preview(){
        let ctx=egui::Context::default();let mut app=app();
        app.editor=Editor::from_source(DEMO.to_vec()).unwrap();
        app.new_mask();app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        assert!(app.review_view && app.editor.is_reviewed());
        let png=app.editor.output_png().unwrap().to_vec();
        app.back_to_edit();
        assert!(!app.review_view);assert_eq!(app.editor.output_png(),Some(png.as_slice()));
        app.editor.begin_draw_preview().unwrap();
        assert!(!app.editor.is_reviewed());
    }
    #[test] fn rejected_file_inputs_preserve_document_history_and_review_ticket(){
        let ctx=egui::Context::default();let mut app=app();
        app.editor=Editor::from_source(DEMO.to_vec()).unwrap();app.new_mask();app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        let copy=app.editor.begin(OperationKind::Copy).unwrap();
        let stamp=app.editor.stamp();let masks=app.editor.masks().to_vec();
        let png=app.editor.output_png().unwrap().to_vec();
        let dir=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/input-tests")
            .join(format!("{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("invalid.png"),b"synthetic invalid image").unwrap();
        std::fs::File::create(dir.join("oversized.png")).unwrap().set_len(shareready_core::MAX_INPUT_BYTES as u64+1).unwrap();
        for name in ["missing.png","invalid.png","oversized.png"] {
            app.load_source(&ctx,&dir.join(name));
            finish_source(&mut app,&ctx);
            assert_eq!(app.editor.stamp(),stamp,"{name}");
            assert_eq!(app.editor.masks(),masks);
            assert_eq!(app.editor.source_bytes(),Some(DEMO));
            assert_eq!(app.editor.output_png(),Some(png.as_slice()));
            assert_eq!(app.editor.bytes_for(copy),Some(png.as_slice()));
            assert!(app.editor.can_undo());
        }
        assert!(app.editor.finish(copy,true));
        app.editor.undo().unwrap();assert!(app.editor.masks().is_empty());
    }
    #[test] fn valid_replacement_waits_before_discarding_committed_edits(){
        let ctx=egui::Context::default();let mut app=app();
        app.editor=Editor::from_source(DEMO.to_vec()).unwrap();app.new_mask();app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        let stamp=app.editor.stamp();let masks=app.editor.masks().to_vec();let png=app.editor.output_png().unwrap().to_vec();
        let copy=app.editor.begin(OperationKind::Copy).unwrap();
        let replacement=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/fixtures/T01_CHAT.png");
        app.load_source(&ctx,&replacement);
        finish_source(&mut app,&ctx);
        assert_eq!(app.editor.stamp(),stamp);
        assert_eq!(app.editor.source_bytes(),Some(DEMO));
        assert_eq!(app.editor.masks(),masks);
        assert_eq!(app.editor.output_png(),Some(png.as_slice()));
        input(&mut app,&ctx,vec![key(egui::Key::N,egui::Modifiers::NONE),key(egui::Key::Z,egui::Modifiers::COMMAND)]);
        assert_eq!(app.editor.stamp(),stamp);
        app.resolve_pending(&ctx,false);
        assert!(app.pending.is_none());assert_eq!(app.editor.bytes_for(copy),Some(png.as_slice()));
        app.load_source(&ctx,&replacement);
        finish_source(&mut app,&ctx);
        app.resolve_pending(&ctx,true);
        assert!(app.pending.is_none());
        assert_eq!(app.editor.stamp().document_id,stamp.document_id+1);
        assert_eq!(app.editor.dimensions(),(1200,800));assert!(app.editor.masks().is_empty());
        assert!(!app.editor.finish(copy,true));
    }
    fn close_frame(app:&mut NativeApp,ctx:&egui::Context)->Vec<egui::ViewportCommand>{
        let mut raw=egui::RawInput::default();
        raw.viewports.get_mut(&egui::ViewportId::ROOT).unwrap().events.push(egui::ViewportEvent::Close);
        let mut output=ctx.run_ui(raw,|_|app.handle_close_request(ctx));
        output.textures_delta.clear();
        output.viewport_output.get(&egui::ViewportId::ROOT).unwrap().commands.clone()
    }
    #[test] fn repeated_close_cannot_bypass_pending_preview_confirmation(){
        let ctx=egui::Context::default();let mut app=app();
        app.drag=Some(Drag{start:Pos2::ZERO,mode:DragMode::Draw});app.preview=Some(Region::new(1,1,2,2));
        assert!(close_frame(&mut app,&ctx).iter().any(|c|matches!(c,egui::ViewportCommand::CancelClose)));
        assert!(app.drag.is_none() && app.preview.is_none());
        assert!(matches!(app.pending,Some(PendingAction::Close)));
        assert!(close_frame(&mut app,&ctx).iter().any(|c|matches!(c,egui::ViewportCommand::CancelClose)));
    }
    #[test] fn close_cancel_retains_history_and_explicit_discard_emits_one_close(){
        let ctx=egui::Context::default();let mut app=app();
        app.editor.add_mask(Region::new(20,20,50,50)).unwrap();let stamp=app.editor.stamp();
        assert!(close_frame(&mut app,&ctx).iter().any(|c|matches!(c,egui::ViewportCommand::CancelClose)));
        app.resolve_pending(&ctx,false);
        assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.masks().len(),1);
        assert!(app.editor.can_undo() && app.pending.is_none() && !app.close_confirmed);
        close_frame(&mut app,&ctx);
        let mut output=ctx.run_ui(egui::RawInput::default(),|_|app.resolve_pending(&ctx,true));
        output.textures_delta.clear();
        assert!(output.viewport_output[&egui::ViewportId::ROOT].commands.iter().any(|c|matches!(c,egui::ViewportCommand::Close)));
        assert!(!close_frame(&mut app,&ctx).iter().any(|c|matches!(c,egui::ViewportCommand::CancelClose)));
        assert!(app.close_confirmed);
    }
    #[test] fn multiple_dropped_files_are_rejected_without_replacing_or_preparing_any(){
        let ctx=egui::Context::default();let mut app=app();
        let source=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/fixtures/T01_CHAT.png");
        let stamp=app.editor.stamp();
        app.dropped_sources(&ctx,vec![source.clone(),source]);
        assert_eq!(app.editor.stamp(),stamp);assert!(app.pending.is_none() && app.texture.is_none());
        assert!(app.status.contains("未读取拖入的文件"));
    }
    #[test] fn redo_history_requires_confirmation_even_when_no_masks_remain(){
        let ctx=egui::Context::default();let mut app=app();
        app.new_mask();app.editor.undo().unwrap();assert!(app.editor.masks().is_empty());
        assert!(app.editor.can_redo());
        let stamp=app.editor.stamp();
        let source=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/fixtures/T01_CHAT.png");
        app.dropped_sources(&ctx,vec![source]);
        finish_source(&mut app,&ctx);
        assert_eq!(app.editor.stamp(),stamp);assert!(app.pending.is_some());
        app.resolve_pending(&ctx,false);app.editor.redo().unwrap();assert_eq!(app.editor.masks().len(),1);
    }
    #[test] fn replacement_cancels_only_uncommitted_gesture_and_keeps_committed_history(){
        let ctx=egui::Context::default();let mut app=app();
        let region=Region::new(20,20,50,50);let id=app.editor.add_mask(region).unwrap();
        let stamp=app.editor.stamp();let gesture=app.editor.begin_gesture(id,GestureKind::Move).unwrap();
        app.preview=Some(app.editor.preview_gesture(gesture,5,8).unwrap());
        app.drag=Some(Drag{start:Pos2::ZERO,mode:DragMode::Existing(gesture)});
        let source=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/fixtures/T01_CHAT.png");
        app.load_source(&ctx,&source);
        finish_source(&mut app,&ctx);
        assert!(!app.editor.active_gesture() && app.drag.is_none() && app.preview.is_none());
        assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.masks()[0].region,region);
        app.resolve_pending(&ctx,false);app.editor.undo().unwrap();assert!(app.editor.masks().is_empty());
    }
    #[test] fn clean_document_import_does_not_require_discard_and_modal_escape_preserves_edits(){
        let ctx=egui::Context::default();let mut app=app();
        let source=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/fixtures/T01_CHAT.png");
        app.dropped_sources(&ctx,vec![source.clone()]);
        finish_source(&mut app,&ctx);
        assert!(app.pending.is_none());assert_eq!(app.editor.dimensions(),(1200,800));
        app.new_mask();let stamp=app.editor.stamp();
        app.dropped_sources(&ctx,vec![source]);finish_source(&mut app,&ctx);assert!(app.pending.is_some());
        // The modal must have been shown before testing a user's Escape response.
        let mut shown=ctx.run_ui(egui::RawInput::default(),|ui|{ui.disable();app.show_confirmation(&ctx);});
        shown.textures_delta.clear();
        let mut output=ctx.run_ui(egui::RawInput {events:vec![key(egui::Key::Escape,egui::Modifiers::NONE)],..Default::default()},|ui|{
            ui.disable();app.keyboard(&ctx);app.show_confirmation(&ctx);
        });output.textures_delta.clear();
        assert!(app.pending.is_none());assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.masks().len(),1);
    }
    fn save_app()->NativeApp {
        let mut app=app();
        app.editor=Editor::from_source(include_bytes!("../../../docs/research/fixtures/T01_CHAT.png").to_vec()).unwrap();
        let ctx=egui::Context::default();app.new_mask();app.start_review(&ctx);finish_current_review(&mut app,&ctx);
        assert!(app.editor.is_reviewed());app
    }
    fn clipboard_png()->shareready_platform_windows::ClipboardImage{
        let mut dib=vec![0u8;40];dib[0]=40;dib[4]=2;dib[8]=1;dib[12]=1;dib[14]=24;
        dib.extend([0,0,255,0,255,0,99,99]);
        shareready_platform_windows::ClipboardImage{format:shareready_platform_windows::ClipboardImageFormat::Png,
            bytes:shareready_core::dib_to_png(&dib).unwrap()}
    }
    #[test] fn paste_failures_preserve_review_history_and_allow_new_candidate_retry(){
        use shareready_platform_windows::{ClipboardImage,ClipboardImageFormat};
        let ctx=egui::Context::default();let mut app=save_app();let stamp=app.editor.stamp();
        let source=app.editor.source_bytes().unwrap().to_vec();let png=app.editor.output_png().unwrap().to_vec();
        for input in [Err("clipboard busy".into()),Ok(ClipboardImage{format:ClipboardImageFormat::Png,bytes:b"bad png".to_vec()}),
            Ok(ClipboardImage{format:ClipboardImageFormat::Dib,bytes:vec![0;12]})]{
            app.paste_with(&ctx,||input);
            finish_source(&mut app,&ctx);
            assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.source_bytes(),Some(source.as_slice()));
            assert_eq!(app.editor.output_png(),Some(png.as_slice()));assert!(app.editor.can_undo());assert!(app.pending.is_none());
            assert!(app.status.contains("粘贴失败"));
        }
        app.paste_with(&ctx,||Ok(clipboard_png()));finish_source(&mut app,&ctx);assert!(app.pending.is_some());assert_eq!(app.editor.stamp(),stamp);
    }
    #[test] fn registered_png_must_not_accept_jpeg_bytes_as_a_candidate(){
        let ctx=egui::Context::default();let mut app=save_app();let stamp=app.editor.stamp();
        let source=app.editor.source_bytes().unwrap().to_vec();let png=app.editor.output_png().unwrap().to_vec();
        let jpeg=include_bytes!("../../../docs/research/fixtures/T05_JPEG_EXIF.jpg").to_vec();
        assert!(decode_source(&jpeg).is_ok());
        app.paste_with(&ctx,||Ok(shareready_platform_windows::ClipboardImage{format:shareready_platform_windows::ClipboardImageFormat::Png,bytes:jpeg}));
        finish_source(&mut app,&ctx);
        assert!(app.pending.is_none());assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.source_bytes(),Some(source.as_slice()));
        assert_eq!(app.editor.output_png(),Some(png.as_slice()));assert!(app.editor.can_undo());assert!(app.status.contains("粘贴失败"));
    }
    #[test] fn paste_candidate_uses_existing_discard_confirmation_and_cancel_keeps_png(){
        let ctx=egui::Context::default();let mut app=save_app();let stamp=app.editor.stamp();let png=app.editor.output_png().unwrap().to_vec();
        app.paste_with(&ctx,||Ok(clipboard_png()));finish_source(&mut app,&ctx);assert!(app.pending.is_some());
        app.resolve_pending(&ctx,false);assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.output_png(),Some(png.as_slice()));
        app.paste_with(&ctx,||Ok(clipboard_png()));finish_source(&mut app,&ctx);app.resolve_pending(&ctx,true);
        assert_eq!(app.editor.dimensions(),(2,1));assert!(app.editor.masks().is_empty());assert!(app.editor.output_png().is_none());
        assert_eq!(decode_source(app.editor.source_bytes().unwrap()).unwrap().into_raw(),[255,0,0,255,0,255,0,255]);
    }
    #[test] fn dib_paste_enters_the_same_candidate_path_with_explicit_alpha(){
        let ctx=egui::Context::default();let mut app=app();
        let mut dib=vec![0u8;124];dib[0]=124;dib[4]=2;dib[8..12].copy_from_slice(&(-1i32).to_le_bytes());dib[12]=1;dib[14]=32;dib[16]=3;
        for (at,n) in [(40,0x00ff0000u32),(44,0x0000ff00),(48,0x000000ff),(52,0xff000000),(56,0x73524742)]{dib[at..at+4].copy_from_slice(&n.to_le_bytes());}
        dib.extend([3,2,1,0,30,20,10,128]);
        app.paste_with(&ctx,||Ok(shareready_platform_windows::ClipboardImage{format:shareready_platform_windows::ClipboardImageFormat::Dib,bytes:dib}));
        finish_source(&mut app,&ctx);
        assert_eq!(app.editor.dimensions(),(2,1));assert!(app.pending.is_none());
        assert_eq!(decode_source(app.editor.source_bytes().unwrap()).unwrap().into_raw(),[1,2,3,0,10,20,30,128]);
    }
    #[test] fn paste_keeps_redo_and_cancels_only_uncommitted_gesture(){
        let ctx=egui::Context::default();let mut app=app();app.new_mask();app.editor.undo().unwrap();
        let stamp=app.editor.stamp();app.paste_with(&ctx,||Ok(clipboard_png()));finish_source(&mut app,&ctx);assert!(app.pending.is_some());
        app.resolve_pending(&ctx,false);assert_eq!(app.editor.stamp(),stamp);assert!(app.editor.can_redo());
        app.editor.redo().unwrap();let id=app.editor.masks()[0].id;let gesture=app.editor.begin_gesture(id,GestureKind::Move).unwrap();
        app.drag=Some(Drag{start:Pos2::ZERO,mode:DragMode::Existing(gesture)});let stamp=app.editor.stamp();
        app.paste_with(&ctx,||Ok(clipboard_png()));finish_source(&mut app,&ctx);assert!(app.pending.is_some());assert!(app.drag.is_none());assert!(!app.editor.active_gesture());
        app.resolve_pending(&ctx,false);assert_eq!(app.editor.stamp(),stamp);assert_eq!(app.editor.masks().len(),1);
    }
    #[test] fn pending_confirmation_and_trace_block_paste_before_reading(){
        let ctx=egui::Context::default();let mut app=app();app.pending=Some(PendingAction::Close);
        app.paste_with(&ctx,||panic!("must not read clipboard during confirmation"));app.pending=None;
        let dir=save_path().with_file_name("trace");app.trace=Some(Trace::create(&dir,DEMO).unwrap());
        app.paste_with(&ctx,||panic!("must not read clipboard during trace"));
    }
    #[test] fn copy_failure_preserves_png_and_retry_delivers_the_exact_current_artifact(){
        let mut app=save_app();let stamp=app.editor.stamp();let png=app.editor.output_png().unwrap().to_vec();
        let ticket=app.editor.begin(OperationKind::Copy).unwrap();app.copy_choice(ticket,|bytes|{assert_eq!(bytes,png);Err("busy".into())});
        assert_eq!(app.editor.last_delivery(),Some((ticket,false)));assert_eq!(app.editor.stamp(),stamp);
        assert_eq!(app.editor.output_png(),Some(png.as_slice()));assert!(app.editor.can_undo());assert!(app.status.contains("另存为新 PNG"));
        let retry=app.editor.begin(OperationKind::Copy).unwrap();let mut called=false;
        app.copy_choice(retry,|bytes|{called=true;assert_eq!(bytes,png);Ok(())});
        assert!(called);assert_eq!(app.editor.last_delivery(),Some((retry,true)));assert!(app.status.contains("尚未验证"));
    }
    #[test] fn stale_wrong_kind_and_blocked_copy_never_call_os_or_change_new_status(){
        let mut app=save_app();let wrong=app.editor.begin(OperationKind::Save).unwrap();
        app.copy_choice(wrong,|_|panic!("wrong operation"));assert!(app.editor.bytes_for(wrong).is_some());
        app.editor.finish(wrong,false);let stale=app.editor.begin(OperationKind::Copy).unwrap();app.new_mask();
        let status=app.status.clone();app.copy_choice(stale,|_|panic!("stale copy"));assert_eq!(app.status,status);
        let ctx=egui::Context::default();app.start_review(&ctx);finish_current_review(&mut app,&ctx);let review=app.editor.begin(OperationKind::Review).unwrap();
        app.copy_choice(stale,|_|panic!("old callback"));assert!(app.editor.bytes_for(review).is_some());app.editor.finish(review,true);
        let current=app.editor.begin(OperationKind::Copy).unwrap();app.pending=Some(PendingAction::Close);
        app.copy_choice(current,|_|panic!("confirmation pending"));assert!(app.editor.output_png().is_some());
    }
    #[test] fn keyboard_paste_is_not_stolen_from_text_focus(){
        let ctx=egui::Context::default();let mut app=app();let mut text=String::new();
        let mut out=ctx.run_ui(Default::default(),|ui|{ui.add(egui::TextEdit::singleline(&mut text)).request_focus();});out.textures_delta.clear();
        assert!(ctx.text_edit_focused());
        let mut out=ctx.run_ui(egui::RawInput{events:vec![key(egui::Key::V,egui::Modifiers::COMMAND),egui::Event::Paste("ordinary text".into())],..Default::default()},|ui|{
            app.keyboard_with(&ctx,||panic!("text focus must not read image"));ui.add(egui::TextEdit::singleline(&mut text));
        });out.textures_delta.clear();assert_eq!(text,"ordinary text");
    }
    #[test] fn canvas_ctrl_v_uses_bounded_reader_once_and_imports_a_clean_candidate(){
        let ctx=egui::Context::default();let mut app=app();let mut reads=0;
        let mut out=ctx.run_ui(egui::RawInput{events:vec![key(egui::Key::V,egui::Modifiers::COMMAND)],..Default::default()},|_|{
            app.keyboard_with(&ctx,||{reads+=1;Ok(clipboard_png())});
        });out.textures_delta.clear();finish_source(&mut app,&ctx);assert_eq!(reads,1);assert_eq!(app.editor.dimensions(),(2,1));assert!(app.pending.is_none());
    }
    fn save_path()->std::path::PathBuf {
        let dir=Path::new(env!("CARGO_MANIFEST_DIR")).join("target/save-ui-tests")
            .join(format!("{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();dir.join("new.png")
    }
    #[test] fn cancelled_save_keeps_png_history_and_can_retry(){
        let mut app=save_app();let path=save_path();let png=app.editor.output_png().unwrap().to_vec();let stamp=app.editor.stamp();
        let ticket=app.editor.begin(OperationKind::Save).unwrap();app.save_choice(ticket,None);
        assert_eq!(app.editor.last_delivery(),Some((ticket,false)));
        assert_eq!(app.editor.stamp(),stamp);assert!(app.editor.can_undo());
        assert_eq!(app.editor.output_png(),Some(png.as_slice()));assert!(!path.exists());
        assert!(app.status.contains("已取消另存"));
        let retry=app.editor.begin(OperationKind::Save).unwrap();app.save_choice(retry,Some(&path));
        assert_eq!(std::fs::read(path).unwrap(),png);assert_eq!(app.editor.last_delivery(),Some((retry,true)));
    }
    #[test] fn existing_target_is_unchanged_and_new_name_retry_succeeds(){
        let mut app=save_app();let path=save_path();std::fs::write(&path,b"existing synthetic target").unwrap();
        let ticket=app.editor.begin(OperationKind::Save).unwrap();app.save_choice(ticket,Some(&path));
        assert_eq!(std::fs::read(&path).unwrap(),b"existing synthetic target");
        assert_eq!(app.editor.last_delivery(),Some((ticket,false)));assert!(app.editor.is_reviewed());
        assert!(!app.status.contains("已另存为新的 PNG"));
        let new_path=path.with_file_name("retry.png");let retry=app.editor.begin(OperationKind::Save).unwrap();
        app.save_choice(retry,Some(&new_path));
        assert_eq!(std::fs::read(new_path).unwrap(),app.editor.output_png().unwrap());
    }
    #[test] fn stale_save_choice_never_writes_or_changes_current_status(){
        let mut app=save_app();let path=save_path();
        let stale=app.editor.begin(OperationKind::Save).unwrap();app.new_mask();
        let current=app.editor.stamp();let status=app.status.clone();
        app.save_choice(stale,Some(&path));
        assert!(!path.exists());assert_eq!(app.editor.stamp(),current);assert_eq!(app.status,status);
        assert_eq!(app.editor.last_delivery(),None);
        let review=app.editor.begin(OperationKind::Review).unwrap();
        app.save_choice(stale,None);
        assert!(app.editor.bytes_for(review).is_some());assert_eq!(app.status,status);
    }
    #[test] fn source_file_path_is_rejected_without_changing_original_bytes(){
        let mut app=save_app();
        let source=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/fixtures/T01_CHAT.png");
        let before=std::fs::read(&source).unwrap();
        let ticket=app.editor.begin(OperationKind::Save).unwrap();app.save_choice(ticket,Some(&source));
        assert_eq!(std::fs::read(source).unwrap(),before);assert_eq!(app.editor.last_delivery(),Some((ticket,false)));
        assert!(app.editor.is_reviewed());
    }
    #[test] fn filesystem_failure_keeps_png_and_retries_after_location_is_available(){
        let mut app=save_app();let path=save_path().with_file_name("missing").join("new.png");
        let png=app.editor.output_png().unwrap().to_vec();let stamp=app.editor.stamp();
        let ticket=app.editor.begin(OperationKind::Save).unwrap();app.save_choice(ticket,Some(&path));
        assert!(!path.exists());assert_eq!(app.editor.stamp(),stamp);
        assert_eq!(app.editor.output_png(),Some(png.as_slice()));assert_eq!(app.editor.last_delivery(),Some((ticket,false)));
        assert!(app.status.contains("另存失败"));
        std::fs::create_dir(path.parent().unwrap()).unwrap();
        let retry=app.editor.begin(OperationKind::Save).unwrap();app.save_choice(retry,Some(&path));
        assert_eq!(std::fs::read(path).unwrap(),png);assert_eq!(app.editor.last_delivery(),Some((retry,true)));
    }
    #[test] fn wrong_operation_or_pending_confirmation_cannot_save(){
        let mut app=save_app();let path=save_path();
        let copy=app.editor.begin(OperationKind::Copy).unwrap();
        app.save_choice(copy,Some(&path));
        assert!(!path.exists() && app.editor.bytes_for(copy).is_some());assert!(app.editor.finish(copy,false));
        let save=app.editor.begin(OperationKind::Save).unwrap();app.pending=Some(PendingAction::Close);
        app.save_choice(save,Some(&path));
        assert!(!path.exists() && app.editor.is_reviewed());assert_eq!(app.editor.last_delivery(),Some((save,false)));
        app.resolve_pending(&egui::Context::default(),false);
        let retry=app.editor.begin(OperationKind::Save).unwrap();app.save_choice(retry,Some(&path));
        assert!(path.exists());assert_eq!(app.editor.last_delivery(),Some((retry,true)));
    }
}
