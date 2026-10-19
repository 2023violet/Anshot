//! Explicit synthetic-only visual run. No desktop capture or injected OS input.
use eframe::egui;
pub const STATES:[&str;8]=["edit","multi","review","back-edit","error","discard","small","dark"];
pub struct VisualRun {pub index:usize,pub frames:usize,pub waiting:bool,pub total:usize,pub snapshot:Option<String>,pub started:std::time::Instant}
impl VisualRun {
    pub fn new()->Self{Self{index:0,frames:0,waiting:false,total:0,snapshot:None,started:std::time::Instant::now()}}
    pub fn unchanged(&self,snapshot:&str)->bool{self.snapshot.as_deref()==Some(snapshot)}
    pub fn accepts(&self,viewport:egui::ViewportId,data:&egui::UserData)->bool{
        self.waiting && viewport==egui::ViewportId::ROOT && data.data.as_ref()
            .and_then(|d|d.downcast_ref::<usize>())==Some(&self.index)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn unsolicited_wrong_viewport_and_old_stage_images_are_rejected(){
        let mut run=VisualRun::new();let tag=egui::UserData::new(0usize);
        assert!(!run.accepts(egui::ViewportId::ROOT,&tag));run.waiting=true;
        assert!(run.accepts(egui::ViewportId::ROOT,&tag));
        assert!(!run.accepts(egui::ViewportId::from_hash_of("child"),&tag));
        run.index=1;assert!(!run.accepts(egui::ViewportId::ROOT,&tag));
        assert!(!run.accepts(egui::ViewportId::ROOT,&Default::default()));
    }
    #[test] fn changed_state_cannot_certify_a_pending_gpu_image(){
        let mut run=VisualRun::new();assert!(!run.unchanged("r1"));
        run.snapshot=Some("r1".into());assert!(run.unchanged("r1"));assert!(!run.unchanged("r2"));
    }
}
