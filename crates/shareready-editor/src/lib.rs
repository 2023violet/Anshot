//! Rust-owned editing state. GUI renders state and submits intents; it never owns export truth.
#![forbid(unsafe_code)]

pub mod geometry;
pub mod viewport;
pub use geometry::{Handle, PixelRect};
pub use viewport::{Point, Viewport, ViewportError};

use shareready_core::{decode_source, redact, Region, COVER, MAX_PIXELS};
pub use shareready_render::{Annotation, RedactStyle, Redaction, Scene, Shape};
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
pub struct Mask {pub id: u64, pub region: Region, pub style: RedactStyle}
/// An annotation object in the same id space and history as masks.
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationObject {pub id: u64, pub annotation: Annotation}
/// A redaction is never translucent: force solid fills to alpha 255.
fn opaque(style:RedactStyle)->RedactStyle{match style{RedactStyle::Solid([r,g,b,_])=>RedactStyle::Solid([r,g,b,255]),s=>s}}
/// V1-compatible default: Core `COVER` solid fill.
pub const DEFAULT_REDACTION: RedactStyle = RedactStyle::Solid(COVER.0);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureKind { Move, Resize(Handle) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureTicket {pub document_id:u64,pub gesture_id:u64}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    InvalidImageDimensions, NoEffectiveMask, MaskLimit, InvalidMaskIndex, EmptyHistory,
    OperationPending, OutputNotReviewed, IdentityOverflow, MissingSource,
    GestureInProgress, WrongGesture, NoChange, StaleReview, CoreFailure(String),
}
#[derive(Clone, Debug, PartialEq)]
struct Snapshot {masks:Vec<Mask>,annotations:Vec<AnnotationObject>,crop:Option<Region>,selected:Option<u64>}
#[derive(Clone, Debug)]
struct Gesture {ticket:GestureTicket,id:u64,base:Region,kind:GestureKind,preview:Region}
#[derive(Clone, Debug)]
struct Artifact {stamp:Stamp,bytes:Arc<[u8]>}

/// Opaque ownership of one review, including failure/cancellation cleanup.
/// Equal source bytes and numeric tickets in another Editor are not authority.
#[derive(Clone)]
pub struct ReviewKey {ticket:Ticket,source:Arc<[u8]>}
impl ReviewKey {
    pub fn ticket(&self)->Ticket{self.ticket}
}
pub struct ReviewWork {key:ReviewKey,dimensions:(u32,u32),regions:Vec<Region>}
/// Full-scene export work (redactions with styles, annotations, crop). May run off-thread.
pub struct RenderWork {key:ReviewKey,dimensions:(u32,u32),scene:Scene}
pub struct ReviewResult {key:ReviewKey,dimensions:(u32,u32),bytes:Arc<[u8]>}
/// A validated source, not a reviewed output or an authority to discard edits.
/// Presentation code cannot forge dimensions or duplicate this owned candidate.
/// ```compile_fail
/// use shareready_editor::SourceCandidate;
/// let forged=SourceCandidate{bytes:std::sync::Arc::from([]),dimensions:(1,1)};
/// ```
/// ```compile_fail
/// use shareready_editor::SourceCandidate;
/// fn duplicate(source:SourceCandidate){let second=source.clone();}
/// ```
pub struct SourceCandidate {bytes:Arc<[u8]>,dimensions:(u32,u32)}
impl SourceCandidate {
    /// Run Core validation before touching an existing Editor; may run off-thread.
    pub fn decode(bytes:Vec<u8>)->Result<Self,EditError>{
        let dimensions=decode_source(&bytes).map_err(EditError::CoreFailure)?.dimensions();
        Ok(Self{bytes:Arc::from(bytes),dimensions})
    }
    pub fn bytes(&self)->&[u8]{self.bytes.as_ref()}
    pub fn dimensions(&self)->(u32,u32){self.dimensions}
}
impl ReviewWork {
    pub fn key(&self)->ReviewKey{self.key.clone()}
    /// Core owns transformation and complete PNG validation. No Editor is shared.
    pub fn run(self)->Result<ReviewResult,EditError>{
        let bytes=redact(self.key.source.as_ref(),&self.regions).map_err(EditError::CoreFailure)?;
        Ok(ReviewResult{key:self.key,dimensions:self.dimensions,bytes:Arc::from(bytes)})
    }
}
impl RenderWork {
    pub fn key(&self)->ReviewKey{self.key.clone()}
    pub fn scene(&self)->&Scene{&self.scene}
    /// The render crate owns composition; Core owns decoding and PNG validation.
    pub fn run(self)->Result<ReviewResult,EditError>{
        let bytes=shareready_render::render_png(self.key.source.as_ref(),&self.scene).map_err(EditError::CoreFailure)?;
        Ok(ReviewResult{key:self.key,dimensions:self.dimensions,bytes:Arc::from(bytes)})
    }
}
impl ReviewResult {
    pub fn bytes(&self)->&[u8]{self.bytes.as_ref()}
    pub fn dimensions(&self)->(u32,u32){self.dimensions}
}

/// This type deliberately does not implement Clone: a second editor must not share
/// an operation token, pending OS action, or an implicitly valid output version.
#[derive(Debug)]
pub struct Editor {
    stamp:Stamp,
    width:u32,height:u32,
    source:Option<Arc<[u8]>>,
    masks:Vec<Mask>,
    annotations:Vec<AnnotationObject>,
    crop:Option<Region>,
    selected:Option<u64>,
    undo:Vec<Snapshot>, redo:Vec<Snapshot>,
    next_mask_id:u64,
    active:Option<Gesture>, next_gesture_id:u64,
    output:Option<Artifact>, pending_png:Option<Arc<[u8]>>,
    pending:Option<Ticket>, next_op:u64,
    last_delivery:Option<(Ticket,bool)>,
}

impl Editor {
    /// Consume a validated source without decoding again or marking it reviewed.
    pub fn from_candidate(source:SourceCandidate)->Result<Self,EditError>{
        let mut doc=Self::new(source.dimensions.0,source.dimensions.1)?;
        doc.source=Some(source.bytes);Ok(doc)
    }
    /// The caller owns discard confirmation. Overflow leaves the old document intact.
    pub fn replace_candidate(&mut self,source:SourceCandidate)->Result<(),EditError>{
        self.replace_document(source.dimensions.0,source.dimensions.1)?;
        self.source=Some(source.bytes);Ok(())
    }
    /// Geometry-only document for UI layout/tests. Review requires from_source.
    pub fn new(width:u32,height:u32)->Result<Self,EditError>{
        Self::check_dimensions(width,height)?;
        Ok(Self{
            stamp:Stamp{document_id:1,revision:0},width,height,source:None,
            masks:Vec::new(),annotations:Vec::new(),crop:None,selected:None,undo:Vec::new(),redo:Vec::new(),
            next_mask_id:1,active:None,next_gesture_id:1,
            output:None,pending_png:None,pending:None,next_op:1,last_delivery:None,
        })
    }
    /// Decode before changing existing document. An invalid input cannot erase a good edit.
    pub fn from_source(bytes:Vec<u8>)->Result<Self,EditError>{
        Self::from_candidate(SourceCandidate::decode(bytes)?)
    }
    fn check_dimensions(w:u32,h:u32)->Result<(),EditError>{
        if w==0 || h==0 || u64::from(w)*u64::from(h)>MAX_PIXELS {
            Err(EditError::InvalidImageDimensions)
        } else {Ok(())}
    }
    pub fn stamp(&self)->Stamp{self.stamp}
    pub fn dimensions(&self)->(u32,u32){(self.width,self.height)}
    pub fn masks(&self)->&[Mask]{&self.masks}
    pub fn annotations(&self)->&[AnnotationObject]{&self.annotations}
    pub fn crop(&self)->Option<Region>{self.crop}
    /// Exported image size (after crop).
    pub fn output_dimensions(&self)->(u32,u32){
        shareready_render::output_size(self.dimensions(),&self.scene()).unwrap_or(self.dimensions())
    }
    /// The exact scene an export of the current revision renders.
    pub fn scene(&self)->Scene{
        Scene{
            redactions:self.masks.iter().map(|m|Redaction{region:m.region,style:m.style}).collect(),
            annotations:self.annotations.iter().map(|a|a.annotation.clone()).collect(),
            crop:self.crop,
        }
    }
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
            if !self.masks.iter().any(|m|m.id==id) && !self.annotations.iter().any(|a|a.id==id){return Err(EditError::InvalidMaskIndex);}
        }
        self.selected=id;
        Ok(())
    }
    fn snapshot(&self)->Snapshot {Snapshot{masks:self.masks.clone(),annotations:self.annotations.clone(),crop:self.crop,selected:self.selected}}
    fn invalidate(&mut self){
        self.output=None;self.pending_png=None;self.pending=None;self.last_delivery=None;
    }
    fn ensure_idle(&self)->Result<(),EditError>{
        if self.active.is_some(){Err(EditError::GestureInProgress)}else{Ok(())}
    }
    fn change(&mut self,next:Snapshot)->Result<bool,EditError>{
        self.ensure_idle()?;
        if self.masks==next.masks && self.annotations==next.annotations && self.crop==next.crop {
            self.selected=next.selected;
            return Ok(false);
        }
        let revision=self.stamp.revision.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let before=self.snapshot();
        if self.undo.len()==HISTORY_LIMIT{self.undo.remove(0);}
        self.undo.push(before);
        self.redo.clear();
        self.masks=next.masks;self.annotations=next.annotations;self.crop=next.crop;self.selected=next.selected;
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
        self.add_redaction(candidate,DEFAULT_REDACTION)
    }
    pub fn add_redaction(&mut self,candidate:Region,style:RedactStyle)->Result<u64,EditError>{
        let style=opaque(style);
        self.ensure_idle()?;
        if self.masks.len()+self.annotations.len()>=MAX_MASKS {return Err(EditError::MaskLimit);}
        let region=self.normalized(candidate)?;
        let id=self.next_mask_id;
        let next_id=id.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let mut next=self.snapshot();
        next.masks.push(Mask{id,region,style});next.selected=Some(id);
        self.change(next)?;
        self.next_mask_id=next_id;
        Ok(id)
    }
    pub fn set_redaction_style(&mut self,id:u64,style:RedactStyle)->Result<bool,EditError>{
        let style=opaque(style);
        self.ensure_idle()?;
        let mut next=self.snapshot();
        next.masks.iter_mut().find(|m|m.id==id).ok_or(EditError::InvalidMaskIndex)?.style=style;
        self.change(next)
    }
    /// One undo entry per added annotation.
    pub fn add_annotation(&mut self,annotation:Annotation)->Result<u64,EditError>{
        self.ensure_idle()?;
        if self.masks.len()+self.annotations.len()>=MAX_MASKS {return Err(EditError::MaskLimit);}
        let id=self.next_mask_id;
        let next_id=id.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let mut next=self.snapshot();
        next.annotations.push(AnnotationObject{id,annotation});next.selected=Some(id);
        self.change(next)?;
        self.next_mask_id=next_id;
        Ok(id)
    }
    /// Move / resize / restyle an annotation as one undo entry (GUI owns the live preview).
    pub fn replace_annotation(&mut self,id:u64,annotation:Annotation)->Result<bool,EditError>{
        self.ensure_idle()?;
        let mut next=self.snapshot();
        next.annotations.iter_mut().find(|a|a.id==id).ok_or(EditError::InvalidMaskIndex)?.annotation=annotation;
        next.selected=Some(id);
        self.change(next)
    }
    /// Remove a mask or an annotation.
    pub fn remove_object(&mut self,id:u64)->Result<(),EditError>{
        if self.masks.iter().any(|m|m.id==id){return self.remove_mask(id);}
        self.ensure_idle()?;
        let mut next=self.snapshot();
        let pos=next.annotations.iter().position(|a|a.id==id).ok_or(EditError::InvalidMaskIndex)?;
        next.annotations.remove(pos);
        if next.selected==Some(id){next.selected=None;}
        self.change(next)?;
        Ok(())
    }
    /// Crop is part of the history; `None` restores the full image.
    pub fn set_crop(&mut self,crop:Option<Region>)->Result<bool,EditError>{
        self.ensure_idle()?;
        let crop=match crop {Some(r)=>Some(self.normalized(r)?),None=>None};
        let mut next=self.snapshot();
        next.crop=crop;
        self.change(next)
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
        let mut next=self.snapshot();next.masks.clear();next.selected=None;
        self.change(next)
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
    /// Starting a new rectangle revokes output and OS tickets before any committed edit.
    /// The GUI owns the uncommitted rectangle; cancellation creates no history entry.
    pub fn begin_draw_preview(&mut self)->Result<(),EditError>{
        self.ensure_idle()?;
        self.invalidate();
        Ok(())
    }
    /// Pointer movement only updates a preview. Commit creates exactly one history item.
    /// Starting the gesture revokes output and pending OS tickets.
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
        self.masks=previous.masks;self.annotations=previous.annotations;self.crop=previous.crop;self.selected=previous.selected;
        self.stamp.revision=revision;self.invalidate();
        Ok(())
    }
    pub fn redo(&mut self)->Result<(),EditError>{
        self.ensure_idle()?;
        let revision=self.stamp.revision.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let next=self.redo.pop().ok_or(EditError::EmptyHistory)?;
        if self.undo.len()==HISTORY_LIMIT{self.undo.remove(0);}
        self.undo.push(self.snapshot());
        self.masks=next.masks;self.annotations=next.annotations;self.crop=next.crop;self.selected=next.selected;
        self.stamp.revision=revision;self.invalidate();
        Ok(())
    }
    /// Dimension-only replacement is safe but deliberately loses the source image.
    pub fn replace_document(&mut self,w:u32,h:u32)->Result<(),EditError>{
        Self::check_dimensions(w,h)?;
        let next_id=self.stamp.document_id.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        self.width=w;self.height=h;self.source=None;
        self.stamp=Stamp{document_id:next_id,revision:0};
        self.masks.clear();self.annotations.clear();self.crop=None;self.selected=None;self.undo.clear();self.redo.clear();
        self.active=None;self.invalidate();
        Ok(())
    }
    pub fn replace_source(&mut self,bytes:Vec<u8>)->Result<(),EditError>{
        self.replace_candidate(SourceCandidate::decode(bytes)?)
    }
    /// Reserve work without encoding. The caller retains its key for any failure.
    /// Work/results cannot be constructed or cloned by presentation code.
    pub fn prepare_review(&mut self)->Result<ReviewWork,EditError>{
        self.ensure_idle()?;
        if self.pending.is_some(){return Err(EditError::OperationPending);}
        if self.masks.is_empty(){return Err(EditError::NoEffectiveMask);}
        let source=self.source.as_ref().ok_or(EditError::MissingSource)?.clone();
        let regions=self.masks.iter().map(|m|m.region).collect();
        let op=self.next_op;
        let next=op.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let ticket=Ticket{stamp:self.stamp,operation_id:op,kind:OperationKind::Review};
        self.invalidate();self.next_op=next;self.pending=Some(ticket);
        Ok(ReviewWork{key:ReviewKey{ticket,source},dimensions:self.dimensions(),regions})
    }
    /// Reserve a full-scene export. Unlike `prepare_review`, an image with only
    /// annotations (or nothing) may be exported: the GUI states the redaction count.
    pub fn prepare_render(&mut self)->Result<RenderWork,EditError>{
        self.ensure_idle()?;
        if self.pending.is_some(){return Err(EditError::OperationPending);}
        let source=self.source.as_ref().ok_or(EditError::MissingSource)?.clone();
        let op=self.next_op;
        let next=op.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let ticket=Ticket{stamp:self.stamp,operation_id:op,kind:OperationKind::Review};
        let scene=self.scene();let dimensions=self.output_dimensions();
        self.invalidate();self.next_op=next;self.pending=Some(ticket);
        Ok(RenderWork{key:ReviewKey{ticket,source},dimensions,scene})
    }
    pub fn review_is_current(&self,key:&ReviewKey)->bool{
        key.ticket.kind==OperationKind::Review && self.pending==Some(key.ticket) &&
            key.ticket.stamp==self.stamp && self.source.as_ref().is_some_and(|s|Arc::ptr_eq(s,&key.source))
    }
    /// A stale failure must not cancel a different Editor or newer operation.
    pub fn cancel_review(&mut self,key:&ReviewKey)->bool{
        if !self.review_is_current(key){return false;}
        self.invalidate();true
    }
    /// Accept only Core-produced bytes for this exact owner and operation.
    /// This does not grant delivery: the adapter must show this PNG, then finish.
    pub fn accept_review(&mut self,result:ReviewResult)->Result<Ticket,EditError>{
        if !self.review_is_current(&result.key) || result.dimensions!=self.output_dimensions(){
            return Err(EditError::StaleReview);
        }
        let ticket=result.key.ticket;self.pending_png=Some(result.bytes);Ok(ticket)
    }
    /// Synchronous compatibility path. Adapters still own actual display/finish.
    pub fn begin(&mut self,kind:OperationKind)->Result<Ticket,EditError>{
        if kind==OperationKind::Review {
            let work=self.prepare_review()?;let key=work.key();
            let result=work.run().and_then(|result|self.accept_review(result));
            if result.is_err(){self.cancel_review(&key);}
            return result;
        }
        self.ensure_idle()?;
        if self.pending.is_some(){return Err(EditError::OperationPending);}
        if !self.is_reviewed(){return Err(EditError::OutputNotReviewed);}
        let op=self.next_op;
        self.next_op=op.checked_add(1).ok_or(EditError::IdentityOverflow)?;
        let ticket=Ticket{stamp:self.stamp,operation_id:op,kind};
        self.pending_png=None;self.pending=Some(ticket);
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
        if ticket.kind==OperationKind::Review && succeeded && self.pending_png.is_none(){return false;}
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
mod scene_tests{
    use super::*;
    const FIX:&[u8]=include_bytes!("../../../docs/research/fixtures/T01_CHAT.png");
    fn ann(x:f32)->Annotation{Annotation{shape:Shape::Arrow{from:(x,10.0),to:(x+50.0,60.0),width:4.0},color:[229,52,43,255]}}
    #[test] fn annotations_styles_and_crop_share_one_history_and_invalidate_output(){
        let mut e=Editor::from_source(FIX.to_vec()).unwrap();
        let m=e.add_redaction(Region::new(10,10,60,40),RedactStyle::SafeMosaic).unwrap();
        let a=e.add_annotation(ann(100.0)).unwrap();
        assert_eq!(e.selected(),Some(a));
        assert!(e.set_redaction_style(m,RedactStyle::Solid([0,0,0,255])).unwrap());
        assert!(e.replace_annotation(a,ann(120.0)).unwrap());
        assert!(e.set_crop(Some(Region::new(0,0,600,400))).unwrap());
        assert_eq!(e.output_dimensions(),(600,400));
        let scene=e.scene();
        assert_eq!(scene.redactions[0].style,RedactStyle::Solid([0,0,0,255]));
        assert_eq!(scene.annotations[0],ann(120.0));
        for _ in 0..3 {e.undo().unwrap();}
        assert_eq!(e.masks()[0].style,RedactStyle::SafeMosaic);
        assert_eq!(e.annotations()[0].annotation,ann(100.0));
        assert_eq!(e.crop(),None);
        e.remove_object(a).unwrap();
        assert!(e.annotations().is_empty());
        e.undo().unwrap();
        assert_eq!(e.annotations().len(),1);
    }
    #[test] fn render_exports_annotation_only_scenes_with_crop_and_rejects_stale_results(){
        let mut e=Editor::from_source(FIX.to_vec()).unwrap();
        e.add_annotation(ann(100.0)).unwrap();
        e.set_crop(Some(Region::new(50,0,450,300))).unwrap();
        let work=e.prepare_render().unwrap();let key=work.key();
        let result=work.run().unwrap();
        assert_eq!(result.dimensions(),(400,300));
        let review=e.accept_review(result).unwrap();
        assert!(e.finish(review,true));
        let copy=e.begin(OperationKind::Copy).unwrap();
        let png=e.bytes_for(copy).unwrap();
        assert_eq!(shareready_core::decode_generated_png(png,(400,300)).unwrap().dimensions(),(400,300));
        assert!(e.finish(copy,true));
        // A newer edit makes an older render unacceptable.
        let work=e.prepare_render().unwrap();let result=work.run().unwrap();
        e.add_annotation(ann(200.0)).unwrap();
        assert!(e.accept_review(result).is_err());
        assert!(!e.review_is_current(&key));
    }
    #[test] fn translucent_solid_redactions_are_forced_opaque(){
        let mut e=Editor::from_source(FIX.to_vec()).unwrap();
        let id=e.add_redaction(Region::new(0,0,10,10),RedactStyle::Solid([1,2,3,0])).unwrap();
        assert_eq!(e.masks()[0].style,RedactStyle::Solid([1,2,3,255]));
        e.set_redaction_style(id,RedactStyle::Solid([9,9,9,10])).unwrap();
        assert_eq!(e.masks()[0].style,RedactStyle::Solid([9,9,9,255]));
    }
    #[test] fn legacy_review_path_still_requires_a_mask(){
        let mut e=Editor::from_source(FIX.to_vec()).unwrap();
        e.add_annotation(ann(1.0)).unwrap();
        assert_eq!(e.prepare_review().err(),Some(EditError::NoEffectiveMask));
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
    #[test] fn source_candidate_keeps_exact_validated_bytes_and_dimensions(){
        let candidate=SourceCandidate::decode(FIX.to_vec()).unwrap();
        assert_eq!(candidate.bytes(),FIX);assert_eq!(candidate.dimensions(),(1200,800));
        let mut prepared=Editor::from_candidate(candidate).unwrap();
        assert_eq!(prepared.source_bytes(),Some(FIX));assert_eq!(prepared.dimensions(),(1200,800));
        assert!(!prepared.is_reviewed());
        let mut synchronous=editor();reviewed(&mut prepared);reviewed(&mut synchronous);
        assert_eq!(prepared.output_png(),synchronous.output_png());
    }
    #[test] fn source_candidate_replacement_revokes_old_review_and_history(){
        let mut e=editor();reviewed(&mut e);let copy=e.begin(OperationKind::Copy).unwrap();
        e.replace_candidate(SourceCandidate::decode(FIX.to_vec()).unwrap()).unwrap();
        assert_eq!(e.stamp(),Stamp{document_id:2,revision:0});
        assert!(!e.finish(copy,true) && e.bytes_for(copy).is_none());
        assert!(e.masks().is_empty() && e.selected().is_none());
        assert!(!e.can_undo() && !e.can_redo() && !e.is_reviewed());
        assert!(e.output_png().is_none() && e.last_delivery().is_none());
        e.add_mask(region()).unwrap();let work=e.prepare_review().unwrap();let key=work.key();
        let result=work.run().unwrap();
        e.replace_candidate(SourceCandidate::decode(FIX.to_vec()).unwrap()).unwrap();
        assert!(!e.review_is_current(&key) && !e.cancel_review(&key));
        assert_eq!(e.accept_review(result),Err(EditError::StaleReview));
        let id=e.add_mask(region()).unwrap();let gesture=e.begin_gesture(id,GestureKind::Move).unwrap();
        e.replace_candidate(SourceCandidate::decode(FIX.to_vec()).unwrap()).unwrap();
        assert!(!e.active_gesture());assert_eq!(e.cancel_gesture(gesture),Err(EditError::WrongGesture));
        assert_eq!(e.stamp(),Stamp{document_id:4,revision:0});
    }
    #[test] fn source_candidate_overflow_preserves_entire_existing_document(){
        let mut e=editor();reviewed(&mut e);
        e.stamp.document_id=u64::MAX;e.output.as_mut().unwrap().stamp=e.stamp;
        let source=e.source.as_ref().unwrap().clone();let stamp=e.stamp();
        let output=e.output_png().unwrap().to_vec();let snapshot=e.snapshot();
        let undo=e.undo.clone();let redo=e.redo.clone();let next_mask=e.next_mask_id;
        let copy=e.begin(OperationKind::Copy).unwrap();
        assert_eq!(e.replace_candidate(SourceCandidate::decode(FIX.to_vec()).unwrap()),Err(EditError::IdentityOverflow));
        assert_eq!(e.stamp(),stamp);assert_eq!(e.dimensions(),(1200,800));
        assert!(Arc::ptr_eq(e.source.as_ref().unwrap(),&source));
        assert_eq!(e.snapshot(),snapshot);assert_eq!(e.undo,undo);assert_eq!(e.redo,redo);
        assert_eq!(e.next_mask_id,next_mask);assert_eq!(e.bytes_for(copy),Some(output.as_slice()));
        assert!(e.is_reviewed() && e.finish(copy,false));
    }
    #[test] fn source_candidate_invalid_input_preserves_existing_review(){
        let mut e=editor();reviewed(&mut e);let stamp=e.stamp();let snapshot=e.snapshot();
        let output=e.output_png().unwrap().to_vec();let copy=e.begin(OperationKind::Copy).unwrap();
        assert!(matches!(SourceCandidate::decode(b"bad image".to_vec()),Err(EditError::CoreFailure(_))));
        assert!(matches!(e.replace_source(b"bad image".to_vec()),Err(EditError::CoreFailure(_))));
        assert_eq!(e.stamp(),stamp);assert_eq!(e.snapshot(),snapshot);
        assert_eq!(e.bytes_for(copy),Some(output.as_slice()));assert!(e.finish(copy,true));
        assert_eq!(e.last_delivery(),Some((copy,true)));
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
    #[test] fn new_rectangle_preview_revokes_output_without_changing_history(){
        let mut e=editor();reviewed(&mut e);let stamp=e.stamp();
        let copy=e.begin(OperationKind::Copy).unwrap();
        e.begin_draw_preview().unwrap();
        assert!(e.bytes_for(copy).is_none() && !e.finish(copy,true));
        assert!(!e.is_reviewed() && e.last_delivery().is_none());
        assert_eq!(e.stamp(),stamp);assert_eq!(e.masks()[0].region,region());
        e.undo().unwrap();assert!(e.masks().is_empty());
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
    #[test] fn deferred_review_requires_accept_and_finish_before_delivery(){
        let mut e=editor();e.add_mask(region()).unwrap();
        let work=e.prepare_review().unwrap();let key=work.key();let ticket=key.ticket();
        assert!(e.review_is_current(&key));assert!(e.bytes_for(ticket).is_none());
        assert!(!e.is_reviewed());assert!(e.output_png().is_none());
        assert!(!e.finish(ticket,true));assert!(e.review_is_current(&key));
        assert_eq!(e.begin(OperationKind::Copy),Err(EditError::OperationPending));
        let result=work.run().unwrap();assert_eq!(result.dimensions(),(1200,800));
        let expected=redact(FIX,&[region()]).unwrap();assert_eq!(result.bytes(),expected);
        assert_eq!(e.accept_review(result),Ok(ticket));assert_eq!(e.bytes_for(ticket),Some(expected.as_slice()));
        assert!(!e.is_reviewed());assert!(e.finish(ticket,true));assert!(e.is_reviewed());
        assert!(!e.review_is_current(&key));assert!(!e.finish(ticket,true));
    }
    #[test] fn deferred_equal_numbers_do_not_authorize_another_editor(){
        let mut first=editor();let mut second=editor();
        first.add_mask(region()).unwrap();second.add_mask(region()).unwrap();
        let a=first.prepare_review().unwrap();let a_key=a.key();
        let b=second.prepare_review().unwrap();let b_key=b.key();
        assert_eq!(a_key.ticket(),b_key.ticket());
        assert!(!second.review_is_current(&a_key));assert!(!second.cancel_review(&a_key));
        assert!(second.review_is_current(&b_key));
        assert_eq!(second.accept_review(a.run().unwrap()),Err(EditError::StaleReview));
        assert!(second.review_is_current(&b_key));assert!(second.bytes_for(b_key.ticket()).is_none());
        assert_eq!(second.accept_review(b.run().unwrap()),Ok(b_key.ticket()));
        assert!(second.finish(b_key.ticket(),true));assert!(second.is_reviewed());
        assert!(first.review_is_current(&a_key));
    }
    #[test] fn deferred_old_result_and_cleanup_preserve_new_pending(){
        let mut e=editor();e.add_mask(region()).unwrap();
        let old=e.prepare_review().unwrap();let old_key=old.key();let result=old.run().unwrap();
        e.add_mask(Region::new(10,10,20,20)).unwrap();
        let current=e.prepare_review().unwrap();let key=current.key();
        assert_eq!(e.accept_review(result),Err(EditError::StaleReview));
        assert!(!e.cancel_review(&old_key));assert!(e.review_is_current(&key));
        assert_eq!(e.accept_review(current.run().unwrap()),Ok(key.ticket()));
        assert!(e.finish(key.ticket(),true));
    }
    #[test] fn deferred_source_replacement_rejects_old_owned_result(){
        let mut e=editor();e.add_mask(region()).unwrap();
        let old=e.prepare_review().unwrap();let old_key=old.key();
        e.replace_source(FIX.to_vec()).unwrap();e.add_mask(region()).unwrap();
        let current=e.prepare_review().unwrap();let key=current.key();
        assert_eq!(e.accept_review(old.run().unwrap()),Err(EditError::StaleReview));
        assert!(!e.cancel_review(&old_key));assert!(e.review_is_current(&key));
        assert_eq!(e.accept_review(current.run().unwrap()),Ok(key.ticket()));
        assert!(e.finish(key.ticket(),true));
    }
    #[test] fn deferred_gesture_revokes_result_before_revision_changes(){
        let mut e=editor();let id=e.add_mask(region()).unwrap();
        let work=e.prepare_review().unwrap();let key=work.key();let stamp=e.stamp();
        let gesture=e.begin_gesture(id,GestureKind::Move).unwrap();assert_eq!(e.stamp(),stamp);
        assert!(!e.review_is_current(&key));assert!(!e.cancel_review(&key));
        assert_eq!(e.accept_review(work.run().unwrap()),Err(EditError::StaleReview));
        assert_eq!(e.active_gesture(),true);e.cancel_gesture(gesture).unwrap();
        let retry=e.prepare_review().unwrap();let next=retry.key();
        assert_eq!(e.accept_review(retry.run().unwrap()),Ok(next.ticket()));assert!(e.finish(next.ticket(),true));
    }
    #[test] fn deferred_cancelled_work_cannot_replay_into_retry(){
        let mut e=editor();e.add_mask(region()).unwrap();
        let old=e.prepare_review().unwrap();let old_key=old.key();
        assert!(e.cancel_review(&old_key));assert!(!e.cancel_review(&old_key));
        let retry=e.prepare_review().unwrap();let key=retry.key();
        assert_eq!(e.accept_review(old.run().unwrap()),Err(EditError::StaleReview));
        assert!(!e.cancel_review(&old_key));assert!(e.review_is_current(&key));
        assert_eq!(e.accept_review(retry.run().unwrap()),Ok(key.ticket()));assert!(e.finish(key.ticket(),true));
    }
    #[test] fn deferred_failure_key_and_sync_failure_are_recoverable(){
        let mut e=editor();e.add_mask(region()).unwrap();
        // Controlled impossible-in-normal-intake corruption exercises Core failure cleanup.
        e.source=Some(Arc::from(&b"synthetic invalid source"[..]));
        let work=e.prepare_review().unwrap();let key=work.key();
        assert!(matches!(work.run(),Err(EditError::CoreFailure(_))));
        let mut other=editor();other.add_mask(region()).unwrap();let current=other.prepare_review().unwrap();let current_key=current.key();
        assert_eq!(key.ticket(),current_key.ticket());
        assert!(!other.cancel_review(&key));assert!(other.review_is_current(&current_key));
        assert!(e.cancel_review(&key));
        assert!(matches!(e.begin(OperationKind::Review),Err(EditError::CoreFailure(_))));
        e.source=Some(Arc::from(FIX));
        let recovered=e.begin(OperationKind::Review).unwrap();assert!(e.finish(recovered,true));
        assert_eq!(other.accept_review(current.run().unwrap()),Ok(current_key.ticket()));assert!(other.finish(current_key.ticket(),true));
    }
    #[test] fn deferred_work_result_and_key_are_send_without_editor_clone(){
        fn send<T:Send>(){}
        send::<ReviewWork>();send::<ReviewResult>();send::<ReviewKey>();
    }
}
