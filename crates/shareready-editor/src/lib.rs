//! Rust-owned editing state. GUI renders state and submits intents; it never owns export truth.
#![forbid(unsafe_code)]

pub mod geometry;
pub mod viewport;
pub use geometry::{Handle, PixelRect};
pub use viewport::{Point, Viewport, ViewportError};

use shareready_core::{decode_source, redact, Region, MAX_PIXELS};
use std::sync::Arc;

const MAX_MASKS: usize = 1024;
const HISTORY_LIMIT: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp { pub document_id: u64, pub revision: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationKind { Review, Copy, Save }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {pub stamp: Stamp, pub operation_id: u64, pub kind: OperationKind}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mask {pub id: u64, pub region: Region}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureKind { Move, Resize(Handle) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureTicket {pub document_id:u64,pub gesture_id:u64}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    InvalidImageDimensions, NoEffectiveMask, MaskLimit, InvalidMaskIndex, EmptyHistory,
    OperationPending, OutputNotReviewed, IdentityOverflow, MissingSource,
    GestureInProgress, WrongGesture, NoChange, CoreFailure(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {masks:Vec<Mask>,selected:Option<u64>}
#[derive(Clone, Debug)]
struct Gesture {ticket:GestureTicket,id:u64,base:Region,kind:GestureKind,preview:Region}
#[derive(Clone, Debug)]
struct Artifact {stamp:Stamp,bytes:Arc<[u8]>}

/// This type deliberately does not implement Clone: a second editor must not share
/// an operation token, pending OS action, or an implicitly valid output version.
#[derive(Debug)]
pub struct Editor {
    stamp:Stamp,
    width:u32,height:u32,
    source:Option<Arc<[u8]>>,
    masks:Vec<Mask>,
    selected:Option<u64>,
    undo:Vec<Snapshot>, redo:Vec<Snapshot>,
    next_mask_id:u64,
    active:Option<Gesture>, next_gesture_id:u64,
    output:Option<Artifact>, pending_png:Option<Arc<[u8]>>,
    pending:Option<Ticket>, next_op:u64,
    last_delivery:Option<(Ticket,bool)>,
}

impl Editor {
    /// Geometry-only document for UI layout/tests. Review requires from_source.
    pub fn new(width:u32,height:u32)->Result<Self,EditError>{
        Self::check_dimensions(width,height)?;
        Ok(Self{
            stamp:Stamp{document_id:1,revision:0},width,height,source:None,
            masks:Vec::new(),selected:None,undo:Vec::new(),redo:Vec::new(),
            next_mask_id:1,active:None,next_gesture_id:1,
            output:None,pending_png:None,pending:None,next_op:1,last_delivery:None,
        })
    }
    /// Decode before changing existing document. An invalid input cannot erase a good edit.
    pub fn from_source(bytes:Vec<u8>)->Result<Self,EditError>{
        let image=decode_source(&bytes).map_err(EditError::CoreFailure)?;
        let (width,height)=image.dimensions();
        let mut doc=Self::new(width,height)?;
        doc.source=Some(Arc::from(bytes));
        Ok(doc)
    }
    fn check_dimensions(w:u32,h:u32)->Result<(),EditError>{
        if w==0 || h==0 || u64::from(w)*u64::from(h)>MAX_PIXELS {
            Err(EditError::InvalidImageDimensions)
        } else {Ok(())}
    }
    pub fn stamp(&self)->Stamp{self.stamp}
    pub fn dimensions(&self)->(u32,u32){(self.width,self.height)}
    pub fn masks(&self)->&[Mask]{&self.masks}
    pub fn selected(&self)->Option<u64>{self.selected}
    pub fn can_undo(&self)->bool{!self.undo.is_empty()}
    pub fn can_redo(&self)->bool{!self.redo.is_empty()}
    pub fn active_gesture(&self)->bool{self.active.is_some()}
    pub fn last_delivery(&self)->Option<(Ticket,bool)>{self.last_delivery}
    /// "reviewed" means Rust encoded and validated the current PNG. It does not
    /// certify that a human inspected every unmasked personal detail.
    pub fn is_reviewed(&self)->bool{self.output.as_ref().is_some_and(|v|v.stamp==self.stamp)}
    pub fn output_png(&self)->Option<&[u8]>{
        self.output.as_ref().filter(|v|v.stamp==self.stamp).map(|v|v.bytes.as_ref())
    }
    pub fn source_bytes(&self)->Option<&[u8]>{self.source.as_deref()}
    pub fn select(&mut self,id:Option<u64>)->Result<(),EditError>{
        if let Some(id)=id {
            if !self.masks.iter().any(|m|m.id==id){return Err(EditError::InvalidMaskIndex);}
        }
        self.selected=id;
        Ok(())
    }
    fn snapshot(&self)->Snapshot {Snapshot{masks:self.masks.clone(),selected:self.selected}}
    fn invalidate(&mut self){
        self.output=None;self.pending_png=None;self.pending=None;self.last_delivery=None;
    }
    fn ensure_idle(&self)->Result<(),EditError>{
        if self.active.is_some(){Err(EditError::GestureInProgress)}else{Ok(())}
    }
    fn change(&mut self,next:Snapshot)->Result<bool,EditError>{
        self.ensure_idle()?;
        if self.masks==next.masks {
            self.selected=next.selected;
            return Ok(false);
        }
        let revision=self.stamp.revision.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let before=self.snapshot();
        if self.undo.len()==HISTORY_LIMIT{self.undo.remove(0);}
        self.undo.push(before);
        self.redo.clear();
        self.masks=next.masks;self.selected=next.selected;
        self.stamp.revision=revision;
        self.invalidate();
        Ok(true)
    }
    fn normalized(&self,candidate:Region)->Result<Region,EditError>{
        PixelRect::from_region(candidate,self.width,self.height)
            .map(PixelRect::region).ok_or(EditError::NoEffectiveMask)
    }
    /// Every real edit is one undo entry, even if initiated from a keyboard command.
    pub fn add_mask(&mut self,candidate:Region)->Result<u64,EditError>{
        self.ensure_idle()?;
        if self.masks.len()>=MAX_MASKS {return Err(EditError::MaskLimit);}
        let region=self.normalized(candidate)?;
        let id=self.next_mask_id;
        let next_id=id.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let mut next=self.snapshot();
        next.masks.push(Mask{id,region});next.selected=Some(id);
        self.change(next)?;
        self.next_mask_id=next_id;
        Ok(id)
    }
    pub fn remove_mask(&mut self,id:u64)->Result<(),EditError>{
        self.ensure_idle()?;
        let mut next=self.snapshot();
        let pos=next.masks.iter().position(|m|m.id==id).ok_or(EditError::InvalidMaskIndex)?;
        next.masks.remove(pos);
        if next.selected==Some(id){next.selected=None;}
        self.change(next)?;
        Ok(())
    }
    pub fn clear_masks(&mut self)->Result<bool,EditError>{
        self.ensure_idle()?;
        self.change(Snapshot{masks:Vec::new(),selected:None})
    }
    pub fn move_mask(&mut self,id:u64,dx:i64,dy:i64)->Result<bool,EditError>{
        self.ensure_idle()?;
        let mut next=self.snapshot();
        let mask=next.masks.iter_mut().find(|m|m.id==id).ok_or(EditError::InvalidMaskIndex)?;
        let rect=PixelRect::from_region(mask.region,self.width,self.height)
            .ok_or(EditError::NoEffectiveMask)?;
        mask.region=rect.translate(dx,dy,self.width,self.height).region();
        next.selected=Some(id);
        self.change(next)
    }
    pub fn resize_mask(&mut self,id:u64,handle:Handle,dx:i64,dy:i64)->Result<bool,EditError>{
        self.ensure_idle()?;
        let mut next=self.snapshot();
        let mask=next.masks.iter_mut().find(|m|m.id==id).ok_or(EditError::InvalidMaskIndex)?;
        let rect=PixelRect::from_region(mask.region,self.width,self.height)
            .ok_or(EditError::NoEffectiveMask)?;
        mask.region=rect.resize(handle,dx,dy,self.width,self.height).region();
        next.selected=Some(id);
        self.change(next)
    }
    /// Pointer movement only updates a preview. Commit creates exactly one history item.
    /// The gesture start invalidates old output/operations, so an old OS callback
    /// can never claim success after a user begins changing a different image region.
    pub fn begin_gesture(&mut self,id:u64,kind:GestureKind)->Result<GestureTicket,EditError>{
        self.ensure_idle()?;
        let base=self.masks.iter().find(|m|m.id==id)
            .ok_or(EditError::InvalidMaskIndex)?.region;
        let g=self.next_gesture_id;
        self.next_gesture_id=g.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let ticket=GestureTicket{document_id:self.stamp.document_id,gesture_id:g};
        self.selected=Some(id);
        self.invalidate();
        self.active=Some(Gesture{ticket,id,base,kind,preview:base});
        Ok(ticket)
    }
    pub fn preview_gesture(&mut self,ticket:GestureTicket,dx:i64,dy:i64)
        ->Result<Region,EditError>
    {
        let g=self.active.as_ref().filter(|g|g.ticket==ticket).ok_or(EditError::WrongGesture)?;
        let base=PixelRect::from_region(g.base,self.width,self.height).ok_or(EditError::NoEffectiveMask)?;
        let result=match g.kind{
            GestureKind::Move=>base.translate(dx,dy,self.width,self.height),
            GestureKind::Resize(handle)=>base.resize(handle,dx,dy,self.width,self.height),
        }.region();
        if let Some(ref mut current)=self.active{current.preview=result;}
        Ok(result)
    }
    pub fn cancel_gesture(&mut self,ticket:GestureTicket)->Result<(),EditError>{
        if self.active.as_ref().map(|g|g.ticket)!=Some(ticket){
            return Err(EditError::WrongGesture);
        }
        self.active=None;
        Ok(())
    }
    pub fn commit_gesture(&mut self,ticket:GestureTicket)->Result<bool,EditError>{
        let g=self.active.as_ref().filter(|g|g.ticket==ticket)
            .ok_or(EditError::WrongGesture)?.clone();
        let mut next=self.snapshot();
        let mask=next.masks.iter_mut().find(|m|m.id==g.id).ok_or(EditError::InvalidMaskIndex)?;
        mask.region=g.preview;
        next.selected=Some(g.id);
        // Validate preconditions before consuming a gesture; no partial changes.
        self.stamp.revision.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        self.active=None;
        self.change(next)
    }
    pub fn undo(&mut self)->Result<(),EditError>{
        self.ensure_idle()?;
        let revision=self.stamp.revision.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let previous=self.undo.pop().ok_or(EditError::EmptyHistory)?;
        self.redo.push(self.snapshot());
        self.masks=previous.masks;self.selected=previous.selected;
        self.stamp.revision=revision;self.invalidate();
        Ok(())
    }
    pub fn redo(&mut self)->Result<(),EditError>{
        self.ensure_idle()?;
        let revision=self.stamp.revision.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let next=self.redo.pop().ok_or(EditError::EmptyHistory)?;
        if self.undo.len()==HISTORY_LIMIT{self.undo.remove(0);}
        self.undo.push(self.snapshot());
        self.masks=next.masks;self.selected=next.selected;
        self.stamp.revision=revision;self.invalidate();
        Ok(())
    }
    /// Dimension-only replacement is safe but deliberately loses the source image.
    pub fn replace_document(&mut self,w:u32,h:u32)->Result<(),EditError>{
        Self::check_dimensions(w,h)?;
        let next_id=self.stamp.document_id.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        self.width=w;self.height=h;self.source=None;
        self.stamp=Stamp{document_id:next_id,revision:0};
        self.masks.clear();self.selected=None;self.undo.clear();self.redo.clear();
        self.active=None;self.invalidate();
        Ok(())
    }
    pub fn replace_source(&mut self,bytes:Vec<u8>)->Result<(),EditError>{
        let img=decode_source(&bytes).map_err(EditError::CoreFailure)?;
        let (w,h)=img.dimensions();
        self.replace_document(w,h)?;
        self.source=Some(Arc::from(bytes));
        Ok(())
    }
    /// Core builds bytes BEFORE a review can complete. The caller must still
    /// show the actual decoded PNG to the person before an OS Save/Copy.
    pub fn begin(&mut self,kind:OperationKind)->Result<Ticket,EditError>{
        self.ensure_idle()?;
        if self.pending.is_some(){return Err(EditError::OperationPending);}
        let fresh_png=match kind {
            OperationKind::Review=>{
                if self.masks.is_empty(){return Err(EditError::NoEffectiveMask);}
                let src=self.source.as_ref().ok_or(EditError::MissingSource)?;
                let regions:Vec<Region>=self.masks.iter().map(|m|m.region).collect();
                Some(Arc::from(redact(src.as_ref(),&regions).map_err(EditError::CoreFailure)?))
            },
            OperationKind::Copy|OperationKind::Save=>{
                if !self.is_reviewed(){return Err(EditError::OutputNotReviewed);}
                None
            },
        };
        let op=self.next_op;
        self.next_op=op.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let ticket=Ticket{stamp:self.stamp,operation_id:op,kind};
        self.pending_png=fresh_png;self.pending=Some(ticket);
        self.last_delivery=None;
        Ok(ticket)
    }
    /// Rust-generated artifact belongs to exactly this live operation.
    /// Stale review bytes and stale copy/save requests cannot escape through here.
    pub fn bytes_for(&self,ticket:Ticket)->Option<&[u8]>{
        if self.pending!=Some(ticket)||ticket.stamp!=self.stamp{return None;}
        match ticket.kind {
            OperationKind::Review=>self.pending_png.as_deref(),
            OperationKind::Copy|OperationKind::Save=>self.output_png(),
        }
    }
    /// OS success is a claim by the trusted platform adapter, not by the GUI.
    /// A result for a stale document, revision, or operation is always ignored.
    pub fn finish(&mut self,ticket:Ticket,succeeded:bool)->bool{
        if self.pending!=Some(ticket)||ticket.stamp!=self.stamp{return false;}
        self.pending=None;
        match ticket.kind{
            OperationKind::Review=>{
                let bytes=self.pending_png.take();
                self.output=if succeeded {
                    bytes.map(|bytes|Artifact{stamp:self.stamp,bytes})
                }else{None};
            },
            OperationKind::Copy|OperationKind::Save=>{
                self.pending_png=None;
                self.last_delivery=Some((ticket,succeeded));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    const FIX:&[u8]=include_bytes!("../../../docs/research/fixtures/T01_CHAT.png");
    fn editor()->Editor{Editor::from_source(FIX.to_vec()).unwrap()}
    fn region()->Region{Region::new(84,154,166,237)}
    fn reviewed(e:&mut Editor){
        e.add_mask(region()).unwrap();
        let t=e.begin(OperationKind::Review).unwrap();
        assert!(e.bytes_for(t).unwrap().starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(e.finish(t,true));
    }
    #[test] fn invalid_input_never_replaces_current_document(){
        let mut e=editor();reviewed(&mut e);let stamp=e.stamp();
        assert!(matches!(e.replace_source(b"bad image".to_vec()),Err(EditError::CoreFailure(_))));
        assert_eq!(e.stamp(),stamp);assert!(e.output_png().is_some());
        assert_eq!(e.replace_document(0,2),Err(EditError::InvalidImageDimensions));
    }
    #[test] fn dimension_only_document_cannot_claim_review_success(){
        let mut e=Editor::new(1200,800).unwrap();e.add_mask(region()).unwrap();
        assert_eq!(e.begin(OperationKind::Review),Err(EditError::MissingSource));
        assert!(!e.is_reviewed());
    }
    #[test] fn source_replace_invalidates_all_old_tickets_and_bytes(){
        let mut e=editor();reviewed(&mut e);
        let stale=e.begin(OperationKind::Copy).unwrap();
        let old_id=e.selected().unwrap();
        e.replace_source(FIX.to_vec()).unwrap();
        assert!(!e.finish(stale,true));assert!(e.bytes_for(stale).is_none());
        assert!(!e.is_reviewed());assert!(e.masks().is_empty());
        let new_mask=e.add_mask(region()).unwrap();assert_ne!(old_id,new_mask);
        let t=e.begin(OperationKind::Review).unwrap();assert!(e.finish(t,true));
        assert!(e.output_png().is_some());
    }
    #[test] fn review_must_be_rust_encoded_not_visual_only(){
        let mut e=editor();assert_eq!(e.begin(OperationKind::Review),Err(EditError::NoEffectiveMask));
        e.add_mask(region()).unwrap();
        let t=e.begin(OperationKind::Review).unwrap();
        let actual=e.bytes_for(t).unwrap().to_vec();
        assert!(actual.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(e.finish(t,true));
        assert_eq!(e.output_png(),Some(actual.as_slice()));
        let t2=e.begin(OperationKind::Copy).unwrap();
        assert_eq!(e.bytes_for(t2),Some(actual.as_slice()));
        assert!(e.finish(t2,false));
        assert!(e.output_png().is_some());
        assert_eq!(e.last_delivery(),Some((t2,false)));
    }
    #[test] fn full_drag_commits_one_undo_item_and_cancel_does_not_change_pixels(){
        let mut e=editor();let id=e.add_mask(region()).unwrap();let rev=e.stamp().revision;
        let g=e.begin_gesture(id,GestureKind::Move).unwrap();
        assert_eq!(e.preview_gesture(g,20,30).unwrap(),Region::new(104,184,186,267));
        assert_eq!(e.preview_gesture(g,60,10).unwrap(),Region::new(144,164,226,247));
        assert_eq!(e.stamp().revision,rev);
        assert!(e.commit_gesture(g).unwrap());
        assert_eq!(e.stamp().revision,rev+1);
        assert_eq!(e.masks()[0].region,Region::new(144,164,226,247));
        e.undo().unwrap();assert_eq!(e.masks()[0].region,region());
        let cancel=e.begin_gesture(id,GestureKind::Resize(Handle::Nw)).unwrap();
        e.preview_gesture(cancel,400,400).unwrap();e.cancel_gesture(cancel).unwrap();
        assert_eq!(e.masks()[0].region,region());
    }
    #[test] fn preview_start_cancels_pending_copy_before_any_mutation(){
        let mut e=editor();reviewed(&mut e);let id=e.selected().unwrap();
        let copy=e.begin(OperationKind::Copy).unwrap();
        let g=e.begin_gesture(id,GestureKind::Move).unwrap();
        assert!(!e.finish(copy,true));assert!(e.output_png().is_none());
        assert_eq!(e.begin(OperationKind::Review),Err(EditError::GestureInProgress));
        e.cancel_gesture(g).unwrap();
        assert_eq!(e.begin(OperationKind::Copy),Err(EditError::OutputNotReviewed));
    }
    #[test] fn resize_eight_handles_and_noop_edit_does_not_change_revision(){
        let mut e=editor();let id=e.add_mask(region()).unwrap();let stamp=e.stamp();
        for h in [Handle::N,Handle::Ne,Handle::E,Handle::Se,Handle::S,Handle::Sw,Handle::W,Handle::Nw]{
            let g=e.begin_gesture(id,GestureKind::Resize(h)).unwrap();
            let r=e.preview_gesture(g,i64::MAX,i64::MIN).unwrap();
            assert!(r.clipped(1200,800).is_some());
            e.cancel_gesture(g).unwrap();
        }
        assert!(!e.move_mask(id,0,0).unwrap());
        assert_eq!(e.stamp(),stamp);
        assert!(e.resize_mask(id,Handle::Nw,10000,10000).unwrap());
        let rect=e.masks()[0].region;assert_eq!(rect.clipped(1200,800).unwrap().0+1,rect.clipped(1200,800).unwrap().2);
    }
    #[test] fn selection_is_non_destructive_and_stable_across_deletion(){
        let mut e=editor();let first=e.add_mask(region()).unwrap();let second=e.add_mask(Region::new(1,2,4,6)).unwrap();
        let rev=e.stamp().revision;e.select(Some(first)).unwrap();assert_eq!(e.stamp().revision,rev);
        assert!(e.select(Some(99999)).is_err());assert_eq!(e.selected(),Some(first));
        e.remove_mask(first).unwrap();assert_eq!(e.selected(),None);
        assert_eq!(e.masks()[0].id,second);
        e.undo().unwrap();assert_eq!(e.selected(),Some(first));assert_eq!(e.masks().len(),2);
    }
    #[test] fn undo_redo_clear_and_branch_do_not_reuse_output_revision(){
        let mut e=editor();reviewed(&mut e);let st=e.stamp();
        e.clear_masks().unwrap();assert!(e.output_png().is_none());
        e.undo().unwrap();assert_eq!(e.masks().len(),1);assert!(e.stamp().revision>st.revision);
        e.redo().unwrap();assert!(e.masks().is_empty());
        e.undo().unwrap();e.add_mask(Region::new(1,1,2,2)).unwrap();
        assert_eq!(e.redo(),Err(EditError::EmptyHistory));
    }
    #[test] fn late_review_save_and_reused_ticket_cannot_advance_status(){
        let mut e=editor();e.add_mask(region()).unwrap();
        let old=e.begin(OperationKind::Review).unwrap();
        e.add_mask(Region::new(1,1,2,2)).unwrap();
        assert!(!e.finish(old,true));assert!(!e.is_reviewed());
        let current=e.begin(OperationKind::Review).unwrap();
        assert!(e.finish(current,true));assert!(!e.finish(current,true));
        let save=e.begin(OperationKind::Save).unwrap();
        assert_eq!(e.begin(OperationKind::Copy),Err(EditError::OperationPending));
        assert!(e.finish(save,true));
        let copy=e.begin(OperationKind::Copy).unwrap();
        assert!(!e.finish(save,true));assert!(e.finish(copy,false));
        assert_eq!(e.last_delivery(),Some((copy,false)));
    }
    #[test] fn old_gesture_from_replaced_document_is_rejected(){
        let mut e=editor();let id=e.add_mask(region()).unwrap();
        let g=e.begin_gesture(id,GestureKind::Move).unwrap();
        e.replace_source(FIX.to_vec()).unwrap();
        assert_eq!(e.preview_gesture(g,8,9),Err(EditError::WrongGesture));
        assert_eq!(e.commit_gesture(g),Err(EditError::WrongGesture));
        assert_eq!(e.selected(),None);
    }
    #[test] fn true_bad_zero_area_guard_and_out_of_bounds(){
        let mut e=editor();assert_eq!(e.add_mask(Region::new(1,1,1,4)),Err(EditError::NoEffectiveMask));
        assert_eq!(e.add_mask(Region::new(9999,9999,10000,10000)),Err(EditError::NoEffectiveMask));
        let id=e.add_mask(Region::new(15,16,14,15)).unwrap();
        assert_eq!(e.masks()[0].region,Region::new(14,15,15,16));
        let r=e.move_mask(id,i64::MIN,i64::MAX).unwrap();assert!(r);
        assert_eq!(e.masks()[0].region,Region::new(0,799,1,800));
    }
}
