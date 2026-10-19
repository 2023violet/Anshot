//! One owned intake worker; validated source and CPU display stay inseparable.
use std::{path::PathBuf,sync::{Arc,mpsc::{self,Receiver,TryRecvError}},thread::{self,JoinHandle}};
use eframe::egui;
use shareready_editor::SourceCandidate;
use crate::display::{DisplayImage,PreparedDisplay};
#[cfg(feature="performance-evidence")]
use std::{sync::atomic::{AtomicU64,Ordering},time::Instant};

#[cfg(feature="performance-evidence")]
pub(super) fn event(ctx:&egui::Context,key:&SourceKey,name:&str,at:Instant,extra:&str){
    let origin=ctx.data(|d|if d.get_temp::<bool>(egui::Id::new("sr-performance-active"))==Some(true){d.get_temp::<Instant>(egui::Id::new("sr-source-origin"))}else{None});
    let Some(origin)=origin else{return;};
    let row=format!("{{\"type\":\"source_event\",\"name\":\"{name}\",\"request\":{},\"ms\":{}{extra}}}",key.number(),at.saturating_duration_since(origin).as_secs_f64()*1000.0);
    ctx.data_mut(|d|d.get_temp_mut_or_default::<Vec<String>>(egui::Id::new("sr-source-events")).push(row));
}

pub(super) enum SourceInput {
    File(PathBuf),
    Bytes{bytes:Vec<u8>,label:String},
    Clipboard(shareready_platform_windows::ClipboardImage),
}
impl SourceInput {
    fn read(self)->Result<(Vec<u8>,String),String>{
        match self {
            Self::File(path)=>{
                let bytes=crate::input::read_source(&path)?;
                let label=path.file_name().map(|p|p.to_string_lossy().into_owned()).unwrap_or_else(||"已导入图片".into());
                Ok((bytes,label))
            },
            Self::Bytes{bytes,label}=>Ok((bytes,label)),
            Self::Clipboard(data)=>{
                let bytes=match data.format {
                    shareready_platform_windows::ClipboardImageFormat::Png=>{
                        if !data.bytes.starts_with(b"\x89PNG\r\n\x1a\n") || data.bytes.len()>shareready_core::MAX_INPUT_BYTES {
                            return Err("剪贴板 PNG 格式无效或超过 25 MiB".into());
                        }
                        data.bytes
                    },
                    shareready_platform_windows::ClipboardImageFormat::Dib=>shareready_core::dib_to_png(&data.bytes)?,
                };
                Ok((bytes,"剪贴板图片".into()))
            },
        }
    }
}
#[derive(Clone)]
pub(super) struct SourceKey(Arc<()>,#[cfg(feature="performance-evidence")] u64);
impl SourceKey {
    fn new()->Self{
        #[cfg(feature="performance-evidence")]
        {static NEXT:AtomicU64=AtomicU64::new(1);Self(Arc::new(()),NEXT.fetch_add(1,Ordering::Relaxed))}
        #[cfg(not(feature="performance-evidence"))]
        {Self(Arc::new(()))}
    }
    pub(super) fn matches(&self,other:&Self)->bool{Arc::ptr_eq(&self.0,&other.0)}
    #[cfg(feature="performance-evidence")]
    pub(super) fn number(&self)->u64{self.1}
}
pub(super) struct PreparedSource {
    key:SourceKey,
    source:SourceCandidate,
    display:PreparedDisplay,
    label:String,
    cap:usize,
    #[cfg(feature="performance-evidence")]
    phases:[Instant;3],
}
impl PreparedSource {
    fn prepare(key:SourceKey,input:SourceInput,cap:usize)->Result<Self,String>{
        let (bytes,label)=input.read()?;
        #[cfg(feature="performance-evidence")]
        let read=Instant::now();
        let source=SourceCandidate::decode(bytes).map_err(|e|format!("{e:?}"))?;
        #[cfg(feature="performance-evidence")]
        let decoded=Instant::now();
        let display=PreparedDisplay::source(source.bytes(),cap)?;
        Ok(Self{key,source,display,label,cap:cap.max(1),#[cfg(feature="performance-evidence")] phases:[read,decoded,Instant::now()]})
    }
    pub(super) fn install(self,ctx:&egui::Context,current:&SourceKey)->Result<super::PreparedSource,String>{
        if !self.key.matches(current){return Err("导入任务已过期。".into());}
        let texture=DisplayImage::from_prepared(ctx,self.display)?;
        Ok(super::PreparedSource{source:self.source,texture,label:self.label,cap:self.cap})
    }
}
pub(super) struct SourceTask {
    key:SourceKey,
    receiver:Receiver<Result<PreparedSource,String>>,
    handle:Option<JoinHandle<()>>,
    received:Option<Result<PreparedSource,String>>,
    clipboard:bool,
    #[cfg(feature="performance-evidence")]
    pub(super) received_at:Option<Instant>,
    #[cfg(feature="performance-evidence")]
    pub(super) finished_at:Option<Instant>,
}
impl SourceTask {
    fn from_parts(key:SourceKey,receiver:Receiver<Result<PreparedSource,String>>,handle:JoinHandle<()>,clipboard:bool)->Self{
        Self{key,receiver,handle:Some(handle),received:None,clipboard,
            #[cfg(feature="performance-evidence")] received_at:None,#[cfg(feature="performance-evidence")] finished_at:None}
    }
    pub(super) fn spawn(input:SourceInput,cap:usize,ctx:egui::Context)->std::io::Result<Self>{
        let key=SourceKey::new();let owned_key=key.clone();let (sender,receiver)=mpsc::sync_channel(1);
        let clipboard=matches!(&input,SourceInput::Clipboard(_));
        #[cfg(feature="performance-evidence")]
        event(&ctx,&key,"trigger",Instant::now(),"");
        let handle=thread::Builder::new().name("ShareReady image intake".into()).spawn(move||{
            #[cfg(feature="performance-evidence")]
            event(&ctx,&owned_key,"worker_start",Instant::now(),"");
            let result=PreparedSource::prepare(owned_key.clone(),input,cap);
            #[cfg(feature="performance-evidence")]
            if let Ok(ref ready)=result{
                for (name,at) in ["read_complete","decoded","cpu_prepared"].into_iter().zip(ready.phases){event(&ctx,&owned_key,name,at,"");}
                event(&ctx,&owned_key,"send_invoked",Instant::now(),"");
            }
            let _=sender.send(result);ctx.request_repaint();
        })?;
        Ok(Self::from_parts(key,receiver,handle,clipboard))
    }
    pub(super) fn key(&self)->&SourceKey{&self.key}
    pub(super) fn is_clipboard(&self)->bool{self.clipboard}
    pub(super) fn is_finished(&self)->bool{self.handle.as_ref().is_none_or(JoinHandle::is_finished)}
    pub(super) fn poll(&mut self)->Option<Result<PreparedSource,String>>{
        self.handle.as_ref()?;
        if self.received.is_none(){match self.receiver.try_recv(){
            Ok(result)=>{
                #[cfg(feature="performance-evidence")]
                {self.received_at=Some(Instant::now());}
                self.received=Some(result);
            },
            Err(TryRecvError::Empty)=>{},
            Err(TryRecvError::Disconnected)=>self.received=Some(Err("后台导入任务意外结束，请重试。".into())),
        }}
        self.finish_poll()
    }
    fn finish_poll(&mut self)->Option<Result<PreparedSource,String>>{
        if !self.is_finished(){return None;}
        #[cfg(feature="performance-evidence")]
        {self.finished_at=Some(Instant::now());}
        let joined=self.handle.take().unwrap().join();
        if joined.is_err(){self.received=Some(Err("后台导入任务异常结束，请重试。".into()));}
        if self.received.is_none(){if let Ok(result)=self.receiver.try_recv(){
            #[cfg(feature="performance-evidence")]
            {self.received_at=Some(Instant::now());}
            self.received=Some(result);
        }}
        self.received.take().or_else(||Some(Err("未收到导入结果，请重试。".into())))
    }
    #[cfg(test)]
    pub(super) fn blocked_after_send(input:SourceInput,cap:usize)->(Self,mpsc::Receiver<()>,mpsc::SyncSender<()>){
        let key=SourceKey::new();let owned=key.clone();let (sender,receiver)=mpsc::sync_channel(1);
        let clipboard=matches!(&input,SourceInput::Clipboard(_));
        let (sent,observed)=mpsc::sync_channel(0);let (release,wait)=mpsc::sync_channel(0);
        let handle=thread::spawn(move||{let _=sender.send(PreparedSource::prepare(owned,input,cap));let _=sent.send(());let _=wait.recv();});
        (Self::from_parts(key,receiver,handle,clipboard),observed,release)
    }
    #[cfg(test)]
    pub(super) fn failed(panics:bool)->Self{
        let key=SourceKey::new();let (sender,receiver)=mpsc::sync_channel(1);
        let handle=thread::spawn(move||{drop(sender);if panics{panic!("controlled source worker panic");}});
        Self::from_parts(key,receiver,handle,false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIX:&[u8]=include_bytes!("../../../docs/research/fixtures/T01_CHAT.png");
    fn input()->SourceInput{SourceInput::Bytes{bytes:FIX.to_vec(),label:"synthetic source".into()}}
    fn finished(task:&SourceTask){
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
        while !task.is_finished(){assert!(std::time::Instant::now()<deadline);std::thread::yield_now();}
    }
    #[test] fn received_source_keeps_slot_until_worker_finishes(){
        let ctx=eframe::egui::Context::default();let cap=crate::display::texture_limit(&ctx);
        let (mut task,sent,release)=SourceTask::blocked_after_send(input(),cap);
        sent.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert!(task.poll().is_none() && !task.is_finished());
        release.send(()).unwrap();finished(&task);
        let ready=task.poll().unwrap().unwrap().install(&ctx,task.key()).unwrap();
        assert_eq!(ready.source.dimensions(),(1200,800));assert_eq!(ready.source.bytes(),FIX);
        assert!(task.poll().is_none());
    }
    #[test] fn source_final_drain_keeps_send_after_initial_empty(){
        let ctx=eframe::egui::Context::default();
        let key=SourceKey::new();let owned=key.clone();let cap=crate::display::texture_limit(&ctx);
        let (sender,receiver)=mpsc::sync_channel(1);let (release,wait)=mpsc::sync_channel(0);
        let handle=thread::spawn(move||{wait.recv().unwrap();sender.send(PreparedSource::prepare(owned,input(),cap)).ok();});
        let mut task=SourceTask::from_parts(key,receiver,handle,false);
        assert!(matches!(task.receiver.try_recv(),Err(TryRecvError::Empty)));
        release.send(()).unwrap();
        finished(&task);
        assert!(task.received.is_none());
        let ready=task.finish_poll().unwrap().unwrap().install(&ctx,task.key()).unwrap();
        assert_eq!(ready.source.bytes(),FIX);
    }
    #[test] fn prepared_source_rejects_another_request_and_changed_cap(){
        let ctx=eframe::egui::Context::default();let cap=crate::display::texture_limit(&ctx);
        let key=SourceKey::new();
        let prepared=PreparedSource::prepare(key.clone(),input(),cap).unwrap();
        assert!(prepared.install(&ctx,&SourceKey::new()).is_err());
        let prepared=PreparedSource::prepare(key.clone(),input(),cap).unwrap();
        ctx.run_ui(eframe::egui::RawInput{max_texture_side:Some(cap*2),..Default::default()},|_|{}).drop_without_applying_deltas();
        assert!(prepared.install(&ctx,&key).is_err());
    }
    #[test] fn source_panic_disconnect_and_invalid_input_return_errors(){
        for panics in [false,true]{
            let mut task=SourceTask::failed(panics);finished(&task);
            assert!(task.poll().unwrap().is_err());assert!(task.poll().is_none());
        }
        assert!(PreparedSource::prepare(SourceKey::new(),SourceInput::Bytes{bytes:b"bad image".to_vec(),label:"bad".into()},8192).is_err());
        assert!(PreparedSource::prepare(SourceKey::new(),SourceInput::Clipboard(shareready_platform_windows::ClipboardImage{
            format:shareready_platform_windows::ClipboardImageFormat::Png,bytes:b"bad PNG".to_vec()}),8192).is_err());
    }
}
