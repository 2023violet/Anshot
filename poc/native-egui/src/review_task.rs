//! One owned worker and an inseparable generated-PNG/display result.
use std::{sync::mpsc::{self,Receiver,TryRecvError},thread::{self,JoinHandle},time::{Duration,Instant}};
use eframe::egui::{self,Rect};
use shareready_editor::{Editor,ReviewKey,ReviewResult,ReviewWork,Ticket};
use crate::display::{self,DisplayImage,PreparedDisplay,Zoom};

#[cfg(feature="performance-evidence")]
pub(super) fn event(ctx:&egui::Context,key:&ReviewKey,name:&str,at:Instant,extra:&str){
    let origin=ctx.data(|d|{
        if d.get_temp::<bool>(egui::Id::new("sr-performance-active"))==Some(true){
            d.get_temp::<Instant>(egui::Id::new("sr-review-origin"))
        }else{None}
    });
    let Some(origin)=origin else{return;};let t=key.ticket();
    let row=format!("{{\"type\":\"review_event\",\"name\":\"{name}\",\"ms\":{},\"frame\":{},\"pass\":{},\"document\":{},\"revision\":{},\"operation\":{}{extra}}}",
        at.saturating_duration_since(origin).as_secs_f64()*1000.0,ctx.cumulative_frame_nr(),ctx.current_pass_index(),t.stamp.document_id,t.stamp.revision,t.operation_id);
    ctx.data_mut(|d|d.get_temp_mut_or_default::<Vec<String>>(egui::Id::new("sr-review-events")).push(row));
}

pub(super) struct PreparedReview {
    key:ReviewKey,
    result:ReviewResult,
    display:PreparedDisplay,
    cap:usize,
    pub(super) started:Instant,
    pub(super) encoded:Instant,
    pub(super) prepared:Instant,
    pub(super) send_invoked:Option<Instant>,
}
impl PreparedReview {
    fn prepare(work:ReviewWork,cap:usize)->Result<Self,String>{
        let started=Instant::now();let key=work.key();
        let result=work.run().map_err(|e|format!("PNG 生成失败：{e:?}"))?;let encoded=Instant::now();
        let display=PreparedDisplay::generated(result.bytes(),result.dimensions(),cap)?;
        Ok(Self{key,result,display,cap,started,encoded,prepared:Instant::now(),send_invoked:None})
    }
    pub(super) fn install(self,ctx:&egui::Context,editor:&mut Editor)->Result<ReadyReview,String>{
        if !editor.review_is_current(&self.key){return Err("复核任务已过期。".into());}
        let texture=DisplayImage::from_prepared(ctx,self.display)?;
        editor.accept_review(self.result).map_err(|e|format!("拒绝复核结果：{e:?}"))?;
        Ok(ReadyReview{key:self.key,texture,cap:self.cap,awaiting:true,drawn:None})
    }
}
struct DrawProof {key:ReviewKey,frame:u64,texture:egui::TextureId,cap:usize}
pub(super) struct ReadyReview {
    key:ReviewKey,
    texture:DisplayImage,
    cap:usize,
    awaiting:bool,
    drawn:Option<DrawProof>,
}
impl ReadyReview {
    pub(super) fn key(&self)->&ReviewKey{&self.key}
    pub(super) fn awaiting(&self)->bool{self.awaiting}
    pub(super) fn begin_pass(&mut self){self.drawn=None;}
    pub(super) fn paint(&mut self,ui:&mut egui::Ui,rect:Rect,zoom:Zoom,editor:&Editor,modal:bool){
        self.texture.paint(ui,rect,zoom);
        if self.awaiting && !modal && editor.review_is_current(&self.key) &&
            self.cap==display::texture_limit(ui.ctx()) && rect.intersect(ui.clip_rect()).is_positive(){
            self.drawn=Some(DrawProof{key:self.key.clone(),frame:ui.ctx().cumulative_frame_nr(),texture:self.texture.identity(),cap:self.cap});
            #[cfg(feature="performance-evidence")]
            event(ui.ctx(),&self.key,"draw",Instant::now(),"");
        }
    }
    pub(super) fn advance(&mut self,ctx:&egui::Context,editor:&mut Editor,modal:bool)->Result<Option<Ticket>,String>{
        if !self.awaiting || modal || !editor.review_is_current(&self.key){return Ok(None);}
        if self.cap!=display::texture_limit(ctx){return Err("显示能力已变化，请重新生成复核图。".into());}
        let valid=self.drawn.as_ref().is_some_and(|p|p.frame<ctx.cumulative_frame_nr() &&
            p.texture==self.texture.identity() && p.cap==self.cap && editor.review_is_current(&p.key));
        if valid && editor.finish(self.key.ticket(),true){self.awaiting=false;self.drawn=None;Ok(Some(self.key.ticket()))}
        else{Ok(None)}
    }
}
pub(super) struct ReviewTask {
    key:ReviewKey,
    receiver:Receiver<Result<PreparedReview,String>>,
    handle:Option<JoinHandle<()>>,
    received:Option<Result<PreparedReview,String>>,
    pub(super) received_at:Option<Instant>,
    pub(super) finished_observed_at:Option<Instant>,
    pub(super) join_duration:Option<Duration>,
}
impl ReviewTask {
    fn from_parts(key:ReviewKey,receiver:Receiver<Result<PreparedReview,String>>,handle:JoinHandle<()>)->Self{
        Self{key,receiver,handle:Some(handle),received:None,received_at:None,finished_observed_at:None,join_duration:None}
    }
    pub(super) fn spawn(work:ReviewWork,cap:usize,ctx:egui::Context)->std::io::Result<Self>{
        let key=work.key();let (sender,receiver)=mpsc::sync_channel(1);
        let handle=thread::Builder::new().name("ShareReady PNG review".into()).spawn(move||{
            #[cfg(feature="performance-evidence")]
            let diagnostic_key=work.key();
            #[cfg(feature="performance-evidence")]
            event(&ctx,&diagnostic_key,"worker_start",Instant::now(),"");
            let mut result=PreparedReview::prepare(work,cap);
            if let Ok(ref mut prepared)=result{
                #[cfg(feature="performance-evidence")]
                {
                    event(&ctx,&diagnostic_key,"encoded",prepared.encoded,&format!(",\"encode_ms\":{}",prepared.encoded.duration_since(prepared.started).as_secs_f64()*1000.0));
                    event(&ctx,&diagnostic_key,"cpu_prepared",prepared.prepared,"");
                }
                prepared.send_invoked=Some(Instant::now());
                #[cfg(feature="performance-evidence")]
                event(&ctx,&diagnostic_key,"send_invoked",prepared.send_invoked.unwrap(),"");
            }
            let _=sender.send(result);ctx.request_repaint();
        })?;
        Ok(Self::from_parts(key,receiver,handle))
    }
    #[cfg(test)]
    pub(super) fn blocked_after_send(work:ReviewWork,cap:usize)->(Self,mpsc::Receiver<()>,mpsc::SyncSender<()>){
        let key=work.key();let (sender,receiver)=mpsc::sync_channel(1);
        let (sent,observed)=mpsc::sync_channel(0);let (release,wait)=mpsc::sync_channel(0);
        let handle=thread::spawn(move||{let _=sender.send(PreparedReview::prepare(work,cap));let _=sent.send(());let _=wait.recv();});
        (Self::from_parts(key,receiver,handle),observed,release)
    }
    #[cfg(test)]
    pub(super) fn failed(work:ReviewWork,panics:bool)->Self{
        let key=work.key();let (sender,receiver)=mpsc::sync_channel(1);
        let handle=thread::spawn(move||{
            drop(work);
            if panics{panic!("controlled review worker panic");}
            let _=sender.send(Err("controlled review failure".into()));
        });
        Self::from_parts(key,receiver,handle)
    }
    pub(super) fn key(&self)->&ReviewKey{&self.key}
    pub(super) fn is_finished(&self)->bool{self.handle.as_ref().is_none_or(JoinHandle::is_finished)}
    pub(super) fn poll(&mut self)->Option<Result<PreparedReview,String>>{
        self.handle.as_ref()?;
        if self.received.is_none(){match self.receiver.try_recv(){
            Ok(result)=>{self.received_at=Some(Instant::now());self.received=Some(result);},
            Err(TryRecvError::Empty)=>{},
            Err(TryRecvError::Disconnected)=>self.received=Some(Err("后台生成任务意外结束，请重试。".into())),
        }}
        self.finish_poll()
    }
    fn finish_poll(&mut self)->Option<Result<PreparedReview,String>>{
        if !self.is_finished(){return None;}
        self.finished_observed_at=Some(Instant::now());let start=Instant::now();
        let joined=self.handle.take().unwrap().join();self.join_duration=Some(start.elapsed());
        if joined.is_err(){self.received=Some(Err("后台生成任务异常结束，请重试。".into()));}
        if self.received.is_none(){
            if let Ok(result)=self.receiver.try_recv(){self.received_at=Some(Instant::now());self.received=Some(result);}
        }
        self.received.take().or_else(||Some(Err("未收到复核结果，请重试。".into())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    use shareready_editor::Editor;
    use shareready_core::Region;
    use std::{sync::mpsc,thread,time::{Duration,Instant}};
    const FIX:&[u8]=include_bytes!("../../../docs/research/fixtures/T01_CHAT.png");
    fn editor()->Editor{
        let mut e=Editor::from_source(FIX.to_vec()).unwrap();e.add_mask(Region::new(1,1,20,20)).unwrap();e
    }
    #[test] fn sent_result_keeps_slot_occupied_until_worker_finishes(){
        let mut e=editor();let work=e.prepare_review().unwrap();let key=work.key();drop(work);
        let (sender,receiver)=mpsc::sync_channel(1);
        let (sent,observed)=mpsc::sync_channel(0);let (release,wait)=mpsc::sync_channel(0);
        let handle=thread::spawn(move||{assert!(sender.send(Err("controlled failure".into())).is_ok());sent.send(()).unwrap();let _=wait.recv();});
        let mut task=ReviewTask::from_parts(key,receiver,handle);
        observed.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(task.poll().is_none());assert!(!task.is_finished());
        release.send(()).unwrap();
        let deadline=Instant::now()+Duration::from_secs(2);
        while !task.is_finished(){assert!(Instant::now()<deadline);thread::yield_now();}
        assert!(matches!(task.poll(),Some(Err(_))));assert!(task.poll().is_none());
    }
    #[test] fn disconnected_panicked_worker_yields_failure_after_exit(){
        let mut e=editor();let work=e.prepare_review().unwrap();let key=work.key();drop(work);
        let (sender,receiver)=mpsc::sync_channel(1);
        let handle=thread::spawn(move||{drop(sender);panic!("controlled review worker panic");});
        let mut task=ReviewTask::from_parts(key,receiver,handle);
        let deadline=Instant::now()+Duration::from_secs(2);
        while !task.is_finished(){assert!(Instant::now()<deadline);thread::yield_now();}
        assert!(matches!(task.poll(),Some(Err(_))));assert!(task.poll().is_none());
    }
    #[test] fn final_result_sent_after_empty_receive_is_not_lost_when_worker_exits(){
        let ctx=egui::Context::default();let cap=crate::display::texture_limit(&ctx);
        let mut e=editor();let work=e.prepare_review().unwrap();let key=work.key();
        let (sender,receiver)=mpsc::sync_channel(1);let (release,wait)=mpsc::sync_channel(0);
        let handle=thread::spawn(move||{wait.recv().unwrap();sender.send(PreparedReview::prepare(work,cap)).ok();});
        let mut task=ReviewTask::from_parts(key,receiver,handle);
        // The actual channel state at poll's receive/finished boundary.
        assert!(matches!(task.receiver.try_recv(),Err(TryRecvError::Empty)));
        release.send(()).unwrap();let deadline=Instant::now()+Duration::from_secs(10);
        while !task.is_finished(){assert!(Instant::now()<deadline);thread::yield_now();}
        let result=task.finish_poll().expect("finished worker must yield its queued result");
        assert!(result.is_ok(),"queued success was replaced by missing-result error");
        assert!(task.poll().is_none());
    }
    #[test] fn success_envelope_cannot_install_into_equal_numbered_editor(){
        let ctx=egui::Context::default();let mut first=editor();let mut second=editor();
        let work=first.prepare_review().unwrap();let a=work.key();
        let current=second.prepare_review().unwrap();let b=current.key();assert_eq!(a.ticket(),b.ticket());
        let prepared=PreparedReview::prepare(work,crate::display::texture_limit(&ctx)).unwrap();
        assert!(prepared.install(&ctx,&mut second).is_err());
        assert!(second.review_is_current(&b));assert!(second.bytes_for(b.ticket()).is_none());
        let prepared=PreparedReview::prepare(current,crate::display::texture_limit(&ctx)).unwrap();
        let _ready=prepared.install(&ctx,&mut second).unwrap();
        assert!(second.review_is_current(&b));assert!(!second.is_reviewed());
        assert!(second.bytes_for(b.ticket()).is_some());
    }
}
