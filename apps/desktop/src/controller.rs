//! UI-independent controller: turns pointer/keyboard intents into Editor
//! commands and drives the review -> delivery handshake. No Slint types here.
#![forbid(unsafe_code)]
use crate::canvas::{hit_test, Hit, View, Zoom};
use shareready_core::Region;
use shareready_editor::{
    EditError, Editor, GestureKind, GestureTicket, OperationKind, Point, ReviewKey, ReviewResult,
    ReviewWork, SourceCandidate, Stamp, Ticket,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool { Select, Redact }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery { Copy, Save }

#[derive(Debug)]
enum Drag {
    Draw { start: Point, current: Point },
    Gesture { ticket: GestureTicket, start: Point, id: u64, preview: Option<Region> },
}

/// What the UI should tell the user after an async step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    Failed { title: String, detail: String },
    Changed,
    /// A result from an older worker; the current operation is untouched.
    Ignored,
}

pub struct Controller {
    pub editor: Option<Editor>,
    pub tool: Tool,
    pub zoom: Zoom,
    area: (f64, f64),
    drag: Option<Drag>,
    delivered: Option<Stamp>,
    busy: Option<(Delivery, ReviewKey)>,
    pub opening: bool,
    pub source_name: Option<String>,
}

impl Default for Controller {
    fn default() -> Self {
        Self { editor: None, tool: Tool::Redact, zoom: Zoom::Fit, area: (800.0, 500.0), drag: None,
            delivered: None, busy: None, opening: false, source_name: None }
    }
}

impl Controller {
    pub fn view(&self) -> Option<View> {
        let (w, h) = self.editor.as_ref()?.dimensions();
        Some(View::layout(w, h, self.area.0, self.area.1, self.zoom))
    }
    pub fn area(&self) -> (f64, f64) { self.area }
    pub fn set_area(&mut self, w: f64, h: f64) { if w > 1.0 && h > 1.0 { self.area = (w, h); } }
    pub fn busy(&self) -> Option<Delivery> { self.busy.as_ref().map(|b| b.0) }
    pub fn dragging(&self) -> bool { self.drag.is_some() }

    /// Unsaved = edited since the last successful copy/save of this document.
    pub fn has_unsaved_edits(&self) -> bool {
        self.editor.as_ref().is_some_and(|e| e.stamp().revision > 0 && Some(e.stamp()) != self.delivered)
    }
    pub fn can_deliver(&self) -> bool {
        self.busy.is_none() && self.drag.is_none() && self.editor.as_ref().is_some_and(|e| !e.masks().is_empty())
    }

    /// Replace (or create) the document. The caller has already confirmed discard.
    pub fn accept_source(&mut self, candidate: SourceCandidate, name: Option<String>) -> Result<(), EditError> {
        self.cancel_drag();
        match self.editor.as_mut() {
            Some(e) => e.replace_candidate(candidate)?,
            None => self.editor = Some(Editor::from_candidate(candidate)?),
        }
        self.delivered = None;
        self.busy = None;
        self.source_name = name;
        self.zoom = Zoom::Fit;
        Ok(())
    }

    // ---- pointer ----
    pub fn pointer_down(&mut self, p: Point) {
        if self.busy.is_some() || self.drag.is_some() { return; }
        let Some(view) = self.view() else { return };
        let Some(editor) = self.editor.as_mut() else { return };
        let hit = hit_test(view, editor.masks(), editor.selected(), p);
        let gesture = match (hit, self.tool) {
            (Hit::Handle(id, h), _) => Some((id, GestureKind::Resize(h))),
            (Hit::Body(id), _) => Some((id, GestureKind::Move)),
            (Hit::Empty, Tool::Redact) => {
                if editor.begin_draw_preview().is_ok() {
                    let _ = editor.select(None);
                    self.drag = Some(Drag::Draw { start: p, current: p });
                }
                None
            }
            (Hit::Empty, Tool::Select) => { let _ = editor.select(None); None }
        };
        if let Some((id, kind)) = gesture {
            if let Ok(ticket) = editor.begin_gesture(id, kind) {
                self.drag = Some(Drag::Gesture { ticket, start: p, id, preview: None });
            }
        }
    }
    pub fn pointer_move(&mut self, p: Point) {
        let Some(view) = self.view() else { return };
        match self.drag.as_mut() {
            Some(Drag::Draw { current, .. }) => *current = p,
            Some(Drag::Gesture { ticket, start, preview, .. }) => {
                let (dx, dy) = view.source_delta(*start, p);
                if let Some(e) = self.editor.as_mut() { *preview = e.preview_gesture(*ticket, dx, dy).ok(); }
            }
            None => {}
        }
    }
    pub fn pointer_up(&mut self) {
        let Some(view) = self.view() else { self.drag = None; return };
        match self.drag.take() {
            Some(Drag::Draw { start, current }) => {
                if let (Ok(r), Some(e)) = (view.viewport().pixel_region(start, current), self.editor.as_mut()) {
                    // A click without a drag adds nothing (zero-area region is rejected).
                    let _ = e.add_mask(r);
                }
            }
            Some(Drag::Gesture { ticket, .. }) => {
                if let Some(e) = self.editor.as_mut() { let _ = e.commit_gesture(ticket); }
            }
            None => {}
        }
    }
    pub fn cancel_drag(&mut self) {
        if let Some(Drag::Gesture { ticket, .. }) = self.drag.take() {
            if let Some(e) = self.editor.as_mut() { let _ = e.cancel_gesture(ticket); }
        }
    }
    /// The rectangle being drawn, in source pixels, for the overlay.
    pub fn draft(&self) -> Option<Region> {
        let view = self.view()?;
        match self.drag.as_ref()? {
            Drag::Draw { start, current } => view.viewport().pixel_region(*start, *current).ok(),
            Drag::Gesture { .. } => None,
        }
    }
    /// Masks as displayed, including a live gesture preview.
    pub fn display_masks(&self) -> Vec<(u64, Region, bool)> {
        let Some(e) = self.editor.as_ref() else { return Vec::new() };
        let preview = match &self.drag {
            Some(Drag::Gesture { id, preview: Some(r), .. }) => Some((*id, *r)),
            _ => None,
        };
        e.masks().iter().map(|m| {
            let region = preview.filter(|(id, _)| *id == m.id).map(|(_, r)| r).unwrap_or(m.region);
            (m.id, region, e.selected() == Some(m.id))
        }).collect()
    }

    // ---- commands ----
    pub fn delete_selected(&mut self) -> bool {
        if self.drag.is_some() { return false; }
        let Some(e) = self.editor.as_mut() else { return false };
        e.selected().is_some_and(|id| e.remove_mask(id).is_ok())
    }
    pub fn nudge(&mut self, dx: i64, dy: i64) -> bool {
        if self.drag.is_some() { return false; }
        let Some(e) = self.editor.as_mut() else { return false };
        e.selected().is_some_and(|id| e.move_mask(id, dx, dy).unwrap_or(false))
    }
    pub fn undo(&mut self) -> bool { self.drag.is_none() && self.editor.as_mut().is_some_and(|e| e.undo().is_ok()) }
    pub fn redo(&mut self) -> bool { self.drag.is_none() && self.editor.as_mut().is_some_and(|e| e.redo().is_ok()) }
    pub fn deselect(&mut self) { if let Some(e) = self.editor.as_mut() { let _ = e.select(None); } }

    // ---- delivery: review on a worker, then deliver the exact reviewed PNG ----
    pub fn begin_delivery(&mut self, kind: Delivery) -> Result<ReviewWork, EditError> {
        if self.busy.is_some() { return Err(EditError::OperationPending); }
        if self.drag.is_some() { return Err(EditError::GestureInProgress); }
        let e = self.editor.as_mut().ok_or(EditError::MissingSource)?;
        let work = e.prepare_review()?;
        self.busy = Some((kind, work.key()));
        Ok(work)
    }
    /// Accept a finished review only if nothing changed meanwhile, then open the
    /// delivery ticket. `Err(Notice::Changed)` means the result was discarded.
    pub fn review_finished(&mut self, from: &ReviewKey, result: Result<ReviewResult, EditError>) -> Result<(Delivery, Ticket), Notice> {
        // Only the worker started for the current busy operation may consume it.
        if !self.busy.as_ref().is_some_and(|(_, k)| k.ticket() == from.ticket()) { return Err(Notice::Ignored); }
        let Some((kind, key)) = self.busy.take() else { return Err(Notice::Ignored) };
        let Some(e) = self.editor.as_mut() else { return Err(Notice::Changed) };
        let fail = |detail: String| Notice::Failed { title: "无法生成图片".into(), detail };
        let result = match result {
            Ok(r) => r,
            Err(err) => { e.cancel_review(&key); return Err(fail(format!("{err:?}"))); }
        };
        if !e.review_is_current(&key) { return Err(Notice::Changed); }
        let review = e.accept_review(result).map_err(|_| Notice::Changed)?;
        if !e.finish(review, true) { return Err(Notice::Changed); }
        let op = match kind { Delivery::Copy => OperationKind::Copy, Delivery::Save => OperationKind::Save };
        let ticket = e.begin(op).map_err(|err| fail(format!("{err:?}")))?;
        Ok((kind, ticket))
    }
    /// Hand the exact reviewed bytes to the platform writer and record the outcome.
    pub fn deliver(&mut self, ticket: Ticket, write: impl FnOnce(&[u8], (u32, u32)) -> Result<(), String>) -> Result<(), String> {
        let e = self.editor.as_mut().ok_or("文档已关闭")?;
        let dims = e.dimensions();
        let Some(bytes) = e.bytes_for(ticket) else { e.finish(ticket, false); return Err("内容已变化".into()) };
        let outcome = write(bytes, dims);
        if e.finish(ticket, outcome.is_ok()) && outcome.is_ok() { self.delivered = Some(e.stamp()); }
        outcome
    }
    pub fn abandon_delivery(&mut self, ticket: Ticket) {
        if let Some(e) = self.editor.as_mut() { e.finish(ticket, false); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(w, h, |x, y| Rgba([(x % 256) as u8, (y % 256) as u8, 200, 255]));
        let mut out = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(img).write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }
    fn opened(w: u32, h: u32) -> Controller {
        let mut c = Controller::default();
        c.set_area(1000.0, 600.0);
        c.accept_source(SourceCandidate::decode(png(w, h)).unwrap(), Some("t.png".into())).unwrap();
        c
    }
    fn at(c: &Controller, sx: f64, sy: f64) -> Point {
        let v = c.view().unwrap();
        Point { x: v.x + sx * v.scale, y: v.y + sy * v.scale }
    }

    #[test]
    fn draw_move_resize_undo_through_pointer_intents() {
        let mut c = opened(400, 200); // scale 1
        c.pointer_down(at(&c, 10.0, 20.0));
        c.pointer_move(at(&c, 60.0, 50.0));
        assert_eq!(c.draft(), Some(Region::new(10, 20, 60, 50)));
        c.pointer_up();
        let e = c.editor.as_ref().unwrap();
        assert_eq!(e.masks().len(), 1);
        assert_eq!(e.masks()[0].region, Region::new(10, 20, 60, 50));
        assert!(c.has_unsaved_edits());
        // Move by dragging the body.
        c.pointer_down(at(&c, 30.0, 30.0));
        c.pointer_move(at(&c, 40.0, 35.0));
        assert_eq!(c.display_masks()[0].1, Region::new(20, 25, 70, 55));
        c.pointer_up();
        assert_eq!(c.editor.as_ref().unwrap().masks()[0].region, Region::new(20, 25, 70, 55));
        // Resize from the Se handle, grabbing 6px off the corner.
        c.pointer_down(at(&c, 76.0, 61.0));
        c.pointer_move(at(&c, 86.0, 71.0));
        c.pointer_up();
        assert_eq!(c.editor.as_ref().unwrap().masks()[0].region, Region::new(20, 25, 80, 65));
        assert!(c.undo());
        assert_eq!(c.editor.as_ref().unwrap().masks()[0].region, Region::new(20, 25, 70, 55));
        assert!(c.redo());
        assert!(c.nudge(1, 0));
        assert_eq!(c.editor.as_ref().unwrap().masks()[0].region, Region::new(21, 25, 81, 65));
        assert!(c.delete_selected());
        assert!(c.editor.as_ref().unwrap().masks().is_empty());
    }

    #[test]
    fn click_without_drag_adds_nothing_and_select_tool_never_draws() {
        let mut c = opened(400, 200);
        c.pointer_down(at(&c, 10.0, 10.0));
        c.pointer_up();
        assert!(c.editor.as_ref().unwrap().masks().is_empty());
        c.tool = Tool::Select;
        c.pointer_down(at(&c, 10.0, 10.0));
        c.pointer_move(at(&c, 90.0, 90.0));
        c.pointer_up();
        assert!(c.editor.as_ref().unwrap().masks().is_empty());
        assert!(!c.can_deliver());
    }

    #[test]
    fn delivery_writes_the_exact_reviewed_png_with_cover_pixels() {
        let mut c = opened(64, 48);
        c.editor.as_mut().unwrap().add_mask(Region::new(8, 8, 24, 16)).unwrap();
        let work = c.begin_delivery(Delivery::Copy).unwrap();
        assert!(!c.can_deliver());
        let (kind, ticket) = c.review_finished(&work.key(), work.run()).unwrap();
        assert_eq!(kind, Delivery::Copy);
        let mut seen = None;
        c.deliver(ticket, |bytes, dims| {
            let img = shareready_core::decode_generated_png(bytes, dims)?;
            seen = Some((img.get_pixel(8, 8).0, img.get_pixel(23, 15).0, img.get_pixel(24, 16).0));
            Ok(())
        }).unwrap();
        let cover = shareready_core::COVER.0;
        assert_eq!(seen, Some((cover, cover, [24, 16, 200, 255])));
        assert!(!c.has_unsaved_edits());
        assert!(c.can_deliver());
    }

    #[test]
    fn edit_during_review_discards_the_stale_result() {
        let mut c = opened(64, 48);
        c.editor.as_mut().unwrap().add_mask(Region::new(8, 8, 24, 16)).unwrap();
        let work = c.begin_delivery(Delivery::Save).unwrap();
        let key = work.key();
        let result = work.run();
        // The user edits while the worker is encoding.
        let id = c.editor.as_ref().unwrap().masks()[0].id;
        c.editor.as_mut().unwrap().move_mask(id, 1, 0).unwrap();
        assert_eq!(c.review_finished(&key, result), Err(Notice::Changed));
        assert!(c.has_unsaved_edits());
        assert!(c.can_deliver());
    }

    #[test]
    fn an_older_worker_result_cannot_consume_or_strand_the_current_review() {
        let mut c = opened(64, 48);
        c.editor.as_mut().unwrap().add_mask(Region::new(0, 0, 4, 4)).unwrap();
        let old = c.begin_delivery(Delivery::Copy).unwrap();
        let (old_key, old_result) = (old.key(), old.run());
        // A new source arrives mid-review, the user redacts and copies again.
        c.accept_source(SourceCandidate::decode(png(32, 32)).unwrap(), None).unwrap();
        c.editor.as_mut().unwrap().add_mask(Region::new(1, 1, 5, 5)).unwrap();
        let current = c.begin_delivery(Delivery::Copy).unwrap();
        assert_eq!(c.review_finished(&old_key, old_result), Err(Notice::Ignored));
        assert_eq!(c.busy(), Some(Delivery::Copy));
        let (_, ticket) = c.review_finished(&current.key(), current.run()).unwrap();
        assert!(c.deliver(ticket, |_, dims| { assert_eq!(dims, (32, 32)); Ok(()) }).is_ok());
        assert!(c.can_deliver());
    }

    #[test]
    fn failed_write_keeps_edits_unsaved_and_allows_retry() {
        let mut c = opened(64, 48);
        c.editor.as_mut().unwrap().add_mask(Region::new(0, 0, 4, 4)).unwrap();
        let work = c.begin_delivery(Delivery::Copy).unwrap();
        let (_, ticket) = c.review_finished(&work.key(), work.run()).unwrap();
        assert_eq!(c.deliver(ticket, |_, _| Err("busy".into())), Err("busy".into()));
        assert!(c.has_unsaved_edits());
        let work = c.begin_delivery(Delivery::Copy).unwrap();
        let (_, ticket) = c.review_finished(&work.key(), work.run()).unwrap();
        assert!(c.deliver(ticket, |_, _| Ok(())).is_ok());
        assert!(!c.has_unsaved_edits());
    }

    #[test]
    fn replacing_the_source_resets_history_and_delivery_state() {
        let mut c = opened(64, 48);
        c.editor.as_mut().unwrap().add_mask(Region::new(0, 0, 4, 4)).unwrap();
        c.accept_source(SourceCandidate::decode(png(32, 32)).unwrap(), None).unwrap();
        let e = c.editor.as_ref().unwrap();
        assert!(e.masks().is_empty() && !e.can_undo());
        assert_eq!(e.dimensions(), (32, 32));
        assert!(!c.has_unsaved_edits());
    }
}
