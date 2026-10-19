//! UI-independent controller: turns pointer/keyboard intents into Editor
//! commands and drives the render -> delivery handshake. No Slint types here.
#![forbid(unsafe_code)]
use crate::canvas::{View, Zoom, HANDLES, HANDLE_HIT_HALF};
use shareready_core::Region;
use shareready_editor::{
    Annotation, EditError, Editor, GestureKind, GestureTicket, Handle, OperationKind, PixelRect, Point, RedactStyle,
    Redaction, RenderWork, ReviewKey, ReviewResult, Scene, Shape, SourceCandidate, Stamp, Ticket,
};
use shareready_render::bounds;

/// 04_DESIGN `ink.*` content colours (keys 1-6).
pub const INKS: [[u8; 4]; 6] = [
    [0xe5, 0x34, 0x2b, 255], [0xff, 0xc6, 0x0a, 255], [0x22, 0xa6, 0x52, 255],
    [0x2f, 0x6f, 0xf0, 255], [0xff, 0xff, 0xff, 255], [0x1a, 0x19, 0x15, 255],
];
pub const WIDTHS: [f32; 3] = [2.0, 4.0, 7.0];
pub const TEXT_SIZES: [f32; 5] = [14.0, 18.0, 24.0, 32.0, 48.0];
/// 04_DESIGN `redact.solid` choices.
pub const SOLIDS: [[u8; 4]; 2] = [[0x1a, 0x19, 0x15, 255], [0xff, 0xff, 0xff, 255]];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool { Select, Arrow, Rect, Text, Number, Highlight, Redact, Mosaic, Crop }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery { Copy, Save }

/// Current drawing style; also mirrors the selected object's style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style { pub color: usize, pub width: usize, pub size: usize, pub background: bool, pub solid: usize }
impl Default for Style { fn default() -> Self { Self { color: 0, width: 1, size: 1, background: false, solid: 0 } } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grip { Box(Handle), ArrowFrom, ArrowTo }

#[derive(Debug)]
enum Drag {
    Draw { start: Point, current: Point },
    Crop { start: Point, current: Point },
    Mask { ticket: GestureTicket, start: Point, id: u64, preview: Option<Region> },
    Ann { id: u64, start: Point, base: Annotation, grip: Option<Grip>, preview: Annotation },
}

/// Pending in-place text edit (source-pixel origin; `id` = existing text being edited).
#[derive(Clone, Debug, PartialEq)]
pub struct TextEdit { pub origin: (f32, f32), pub id: Option<u64>, pub initial: String }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice { Failed { title: String, detail: String }, Changed, Ignored }

/// What a pointer press would grab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit { Grip(u64, Grip), Mask(u64), Ann(u64), Empty }

enum Draft { Redact(Region, RedactStyle), Ann(Annotation) }

pub struct Controller {
    pub editor: Option<Editor>,
    pub tool: Tool,
    pub style: Style,
    pub zoom: Zoom,
    area: (f64, f64),
    drag: Option<Drag>,
    delivered: Option<Stamp>,
    busy: Option<(Delivery, ReviewKey)>,
    pub opening: bool,
    pub source_name: Option<String>,
    pub text_edit: Option<TextEdit>,
    /// Crop rectangle being adjusted (Enter applies, Esc discards).
    pub crop_draft: Option<Region>,
}

impl Default for Controller {
    fn default() -> Self {
        Self { editor: None, tool: Tool::Redact, style: Style::default(), zoom: Zoom::Fit, area: (800.0, 500.0), drag: None,
            delivered: None, busy: None, opening: false, source_name: None, text_edit: None, crop_draft: None }
    }
}

fn region_of(a: &Annotation) -> Option<Region> {
    match a.shape { Shape::Rect { region, .. } | Shape::Highlight { region } => Some(region), _ => None }
}
fn with_region(a: &Annotation, r: Region) -> Annotation {
    let mut a = a.clone();
    if let Shape::Rect { region, .. } | Shape::Highlight { region } = &mut a.shape { *region = r; }
    a
}
fn translate(a: &Annotation, dx: f32, dy: f32) -> Annotation {
    let mut a = a.clone();
    let (ix, iy) = (dx.round() as i64, dy.round() as i64);
    match &mut a.shape {
        Shape::Rect { region, .. } | Shape::Highlight { region } => *region = Region::new(region.x0 + ix, region.y0 + iy, region.x1 + ix, region.y1 + iy),
        Shape::Arrow { from, to, .. } => { *from = (from.0 + dx, from.1 + dy); *to = (to.0 + dx, to.1 + dy); }
        Shape::Number { center, .. } => *center = (center.0 + dx, center.1 + dy),
        Shape::Text { origin, .. } => *origin = (origin.0 + dx, origin.1 + dy),
    }
    a
}
fn seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (vx, vy, wx, wy) = (b.0 - a.0, b.1 - a.1, p.0 - a.0, p.1 - a.1);
    let len = vx * vx + vy * vy;
    let t = if len > 0.0 { ((wx * vx + wy * vy) / len).clamp(0.0, 1.0) } else { 0.0 };
    ((wx - t * vx).powi(2) + (wy - t * vy).powi(2)).sqrt()
}
fn box_grips((x, y, w, h): (f64, f64, f64, f64)) -> Vec<(Grip, (f64, f64))> {
    HANDLES.iter().map(|(hd, fx, fy)| (Grip::Box(*hd), (x + w * fx, y + h * fy))).collect()
}

impl Controller {
    pub fn view(&self) -> Option<View> {
        let (w, h) = self.editor.as_ref()?.dimensions();
        Some(View::layout(w, h, self.area.0, self.area.1, self.zoom))
    }
    pub fn area(&self) -> (f64, f64) { self.area }
    /// Hide numeric fields when the window is too narrow for them.
    pub fn narrow_propbar(&self) -> bool { self.area.0 < 760.0 }
    pub fn set_area(&mut self, w: f64, h: f64) { if w > 1.0 && h > 1.0 { self.area = (w, h); } }
    pub fn busy(&self) -> Option<Delivery> { self.busy.as_ref().map(|b| b.0) }
    pub fn dragging(&self) -> bool { self.drag.is_some() }

    /// Unsaved = edited since the last successful copy/save of this document.
    pub fn has_unsaved_edits(&self) -> bool {
        self.editor.as_ref().is_some_and(|e| e.stamp().revision > 0 && Some(e.stamp()) != self.delivered)
    }
    pub fn can_deliver(&self) -> bool {
        self.busy.is_none() && self.drag.is_none() && self.text_edit.is_none() && self.editor.is_some()
    }
    pub fn redaction_count(&self) -> usize { self.editor.as_ref().map_or(0, |e| e.masks().len()) }

    /// Replace (or create) the document. The caller has already confirmed discard.
    pub fn accept_source(&mut self, candidate: SourceCandidate, name: Option<String>) -> Result<(), EditError> {
        self.cancel_drag();
        self.text_edit = None;
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
    /// Drop the document after the user chose to discard it.
    pub fn close_document(&mut self) {
        self.drag = None;
        self.text_edit = None;
        self.editor = None;
        self.busy = None;
        self.delivered = None;
        self.source_name = None;
    }

    fn src(&self, p: Point) -> Option<(f64, f64)> {
        let v = self.view()?;
        Some(((p.x - v.x) / v.scale, (p.y - v.y) / v.scale))
    }

    // ---- hit testing (canvas px) ----
    pub fn grips(&self) -> Vec<(Grip, (f64, f64))> {
        let (Some(e), Some(v)) = (self.editor.as_ref(), self.view()) else { return Vec::new() };
        let Some(id) = e.selected() else { return Vec::new() };
        if let Some(r) = self.display_masks().into_iter().find(|m| m.0 == id).map(|m| m.1) { return box_grips(v.rect(r)); }
        let sc = |x: f32, y: f32| (v.x + f64::from(x) * v.scale, v.y + f64::from(y) * v.scale);
        match self.display_annotation(id).map(|a| a.shape) {
            Some(Shape::Arrow { from, to, .. }) => vec![(Grip::ArrowFrom, sc(from.0, from.1)), (Grip::ArrowTo, sc(to.0, to.1))],
            Some(Shape::Rect { region, .. } | Shape::Highlight { region }) => box_grips(v.rect(region)),
            _ => Vec::new(),
        }
    }
    pub fn hit(&self, p: Point) -> Hit {
        let (Some(e), Some(v), Some(s)) = (self.editor.as_ref(), self.view(), self.src(p)) else { return Hit::Empty };
        if let Some(id) = e.selected() {
            for (g, (gx, gy)) in self.grips() {
                if (p.x - gx).abs() < HANDLE_HIT_HALF && (p.y - gy).abs() < HANDLE_HIT_HALF { return Hit::Grip(id, g); }
            }
        }
        // Smallest object under the pointer wins, so a large highlight or frame never
        // hides the redactions beneath it; ties go to the topmost (later) object.
        let tol = 6.0 / v.scale;
        let mut best: Option<(f64, Hit)> = None;
        let mut offer = |area: f64, hit: Hit| { if best.is_none_or(|(a, _)| area <= a) { best = Some((area, hit)); } };
        for m in e.masks() {
            let r = m.region;
            if s.0 >= r.x0 as f64 && s.0 < r.x1 as f64 && s.1 >= r.y0 as f64 && s.1 < r.y1 as f64 { offer(((r.x1 - r.x0) * (r.y1 - r.y0)) as f64, Hit::Mask(m.id)); }
        }
        for a in e.annotations() {
            let (x0, y0, x1, y1) = bounds(&a.annotation);
            let inside = match &a.annotation.shape {
                Shape::Arrow { from, to, width } => seg_dist(s, (from.0.into(), from.1.into()), (to.0.into(), to.1.into())) <= f64::from(*width) / 2.0 + tol,
                _ => s.0 >= f64::from(x0) && s.0 < f64::from(x1) && s.1 >= f64::from(y0) && s.1 < f64::from(y1),
            };
            if inside { offer(f64::from((x1 - x0) * (y1 - y0)), Hit::Ann(a.id)); }
        }
        if let Some((_, hit)) = best { return hit; }
        Hit::Empty
    }

    // ---- pointer ----
    pub fn pointer_down(&mut self, p: Point) {
        if self.busy.is_some() || self.drag.is_some() || self.text_edit.is_some() { return; }
        if self.tool == Tool::Crop {
            if self.editor.as_mut().is_some_and(|e| e.begin_draw_preview().is_ok()) { self.drag = Some(Drag::Crop { start: p, current: p }); }
            return;
        }
        let hit = self.hit(p);
        let Some(src) = self.src(p) else { return };
        let is_mask = |c: &Self, id: u64| c.editor.as_ref().is_some_and(|e| e.masks().iter().any(|m| m.id == id));
        match hit {
            Hit::Grip(id, Grip::Box(h)) if is_mask(self, id) => self.begin_mask_drag(id, GestureKind::Resize(h), p),
            Hit::Mask(id) => { self.begin_mask_drag(id, GestureKind::Move, p); self.sync_style(); }
            Hit::Grip(id, g) => self.begin_ann_drag(id, p, Some(g)),
            Hit::Ann(id) => { self.begin_ann_drag(id, p, None); self.sync_style(); }
            Hit::Empty => self.press_empty(p, src),
        }
    }
    fn begin_mask_drag(&mut self, id: u64, kind: GestureKind, p: Point) {
        if let Some(Ok(ticket)) = self.editor.as_mut().map(|e| e.begin_gesture(id, kind)) {
            self.drag = Some(Drag::Mask { ticket, start: p, id, preview: None });
        }
    }
    fn begin_ann_drag(&mut self, id: u64, p: Point, grip: Option<Grip>) {
        let Some(e) = self.editor.as_mut() else { return };
        let Some(base) = e.annotations().iter().find(|a| a.id == id).map(|a| a.annotation.clone()) else { return };
        let _ = e.select(Some(id));
        if e.begin_draw_preview().is_ok() { self.drag = Some(Drag::Ann { id, start: p, preview: base.clone(), base, grip }); }
    }
    fn press_empty(&mut self, p: Point, src: (f64, f64)) {
        let (tool, style) = (self.tool, self.style);
        let Some(editor) = self.editor.as_mut() else { return };
        let _ = editor.select(None);
        match tool {
            Tool::Select => {}
            Tool::Text => self.text_edit = Some(TextEdit { origin: (src.0 as f32, src.1 as f32), id: None, initial: String::new() }),
            Tool::Number => {
                let n = editor.annotations().iter().filter_map(|a| match a.annotation.shape { Shape::Number { n, .. } => Some(n), _ => None }).max().unwrap_or(0) + 1;
                let radius = 9.0 + 2.0 * WIDTHS[style.width];
                let _ = editor.add_annotation(Annotation { shape: Shape::Number { center: (src.0 as f32, src.1 as f32), n, radius }, color: INKS[style.color] });
            }
            _ => { if editor.begin_draw_preview().is_ok() { self.drag = Some(Drag::Draw { start: p, current: p }); } }
        }
    }
    pub fn pointer_move(&mut self, p: Point) {
        let Some(view) = self.view() else { return };
        let (w, h) = self.editor.as_ref().map_or((1, 1), |e| e.dimensions());
        match self.drag.as_mut() {
            Some(Drag::Draw { current, .. }) | Some(Drag::Crop { current, .. }) => *current = p,
            Some(Drag::Mask { ticket, start, preview, .. }) => {
                let (dx, dy) = view.source_delta(*start, p);
                if let Some(e) = self.editor.as_mut() { *preview = e.preview_gesture(*ticket, dx, dy).ok(); }
            }
            Some(Drag::Ann { start, base, grip, preview, .. }) => {
                let (dx, dy) = (((p.x - start.x) / view.scale) as f32, ((p.y - start.y) / view.scale) as f32);
                *preview = match grip {
                    None => translate(base, dx, dy),
                    Some(Grip::Box(hd)) => match region_of(base).and_then(|r| PixelRect::from_region(r, w, h)) {
                        Some(pr) => with_region(base, pr.resize(*hd, dx.round() as i64, dy.round() as i64, w, h).region()),
                        None => base.clone(),
                    },
                    Some(g) => {
                        let mut a = base.clone();
                        if let Shape::Arrow { from, to, .. } = &mut a.shape {
                            if *g == Grip::ArrowFrom { *from = (from.0 + dx, from.1 + dy); } else { *to = (to.0 + dx, to.1 + dy); }
                        }
                        a
                    }
                };
            }
            None => {}
        }
    }
    pub fn pointer_up(&mut self) {
        let Some(view) = self.view() else { self.drag = None; return };
        match self.drag.take() {
            Some(Drag::Draw { start, current }) => {
                let draft = self.draft_from(start, current, view);
                if let (Some(d), Some(e)) = (draft, self.editor.as_mut()) {
                    let _ = match d { Draft::Redact(r, style) => e.add_redaction(r, style), Draft::Ann(a) => e.add_annotation(a) };
                }
            }
            Some(Drag::Crop { start, current }) => {
                if let Ok(r) = view.viewport().pixel_region(start, current) {
                    if (r.x1 - r.x0) > 1 && (r.y1 - r.y0) > 1 { self.crop_draft = Some(r); }
                }
            }
            Some(Drag::Mask { ticket, .. }) => { if let Some(e) = self.editor.as_mut() { let _ = e.commit_gesture(ticket); } }
            Some(Drag::Ann { id, preview, base, .. }) => {
                if preview != base { if let Some(e) = self.editor.as_mut() { let _ = e.replace_annotation(id, preview); } }
            }
            None => {}
        }
    }
    pub fn cancel_drag(&mut self) {
        if let Some(Drag::Mask { ticket, .. }) = self.drag.take() {
            if let Some(e) = self.editor.as_mut() { let _ = e.cancel_gesture(ticket); }
        }
    }

    fn draft_from(&self, start: Point, current: Point, view: View) -> Option<Draft> {
        let r = view.viewport().pixel_region(start, current).ok()?;
        let area = (r.x1 - r.x0) * (r.y1 - r.y0);
        let color = INKS[self.style.color];
        let (s, c) = (self.src(start)?, self.src(current)?);
        match self.tool {
            Tool::Redact if area > 0 => Some(Draft::Redact(r, RedactStyle::Solid(SOLIDS[self.style.solid]))),
            Tool::Mosaic if area > 0 => Some(Draft::Redact(r, RedactStyle::SafeMosaic)),
            Tool::Rect if area > 0 => Some(Draft::Ann(Annotation { shape: Shape::Rect { region: r, width: WIDTHS[self.style.width] }, color })),
            Tool::Highlight if area > 0 => Some(Draft::Ann(Annotation { shape: Shape::Highlight { region: r }, color })),
            Tool::Arrow if ((c.0 - s.0).powi(2) + (c.1 - s.1).powi(2)).sqrt() * view.scale >= 6.0 => Some(Draft::Ann(Annotation {
                shape: Shape::Arrow { from: (s.0 as f32, s.1 as f32), to: (c.0 as f32, c.1 as f32), width: WIDTHS[self.style.width] }, color })),
            _ => None,
        }
    }

    // ---- what the canvas shows ----
    fn display_masks(&self) -> Vec<(u64, Region)> {
        let Some(e) = self.editor.as_ref() else { return Vec::new() };
        let preview = match &self.drag { Some(Drag::Mask { id, preview: Some(r), .. }) => Some((*id, *r)), _ => None };
        e.masks().iter().map(|m| (m.id, preview.filter(|(id, _)| *id == m.id).map(|(_, r)| r).unwrap_or(m.region))).collect()
    }
    fn display_annotation(&self, id: u64) -> Option<Annotation> {
        if let Some(Drag::Ann { id: d, preview, .. }) = &self.drag { if *d == id { return Some(preview.clone()); } }
        self.editor.as_ref()?.annotations().iter().find(|a| a.id == id).map(|a| a.annotation.clone())
    }
    /// The scene the canvas renders: committed state + live gesture + draft.
    /// Excludes crop (shown as an overlay) and a text annotation being re-edited.
    pub fn display_scene(&self) -> Scene {
        let Some(e) = self.editor.as_ref() else { return Scene::default() };
        let mut scene = e.scene();
        scene.crop = None;
        for (red, (_, r)) in scene.redactions.iter_mut().zip(self.display_masks()) { red.region = r; }
        if let Some(Drag::Ann { id, preview, .. }) = &self.drag {
            if let Some(pos) = e.annotations().iter().position(|a| a.id == *id) { scene.annotations[pos] = preview.clone(); }
        }
        if let Some(edit_id) = self.text_edit.as_ref().and_then(|t| t.id) {
            if let Some(pos) = e.annotations().iter().position(|a| a.id == edit_id) { scene.annotations.remove(pos); }
        }
        if let (Some(Drag::Draw { start, current }), Some(view)) = (&self.drag, self.view()) {
            match self.draft_from(*start, *current, view) {
                Some(Draft::Redact(r, style)) => scene.redactions.push(Redaction { region: r, style }),
                Some(Draft::Ann(a)) => scene.annotations.push(a),
                None => {}
            }
        }
        scene
    }
    /// Selected object's canvas bounds.
    pub fn selection_box(&self) -> Option<(f64, f64, f64, f64)> {
        let (e, v) = (self.editor.as_ref()?, self.view()?);
        let id = e.selected()?;
        if let Some(r) = self.display_masks().into_iter().find(|m| m.0 == id).map(|m| m.1) { return Some(v.rect(r)); }
        let (x0, y0, x1, y1) = bounds(&self.display_annotation(id)?);
        Some((v.x + f64::from(x0) * v.scale, v.y + f64::from(y0) * v.scale, f64::from(x1 - x0) * v.scale, f64::from(y1 - y0) * v.scale))
    }
    pub fn selected_region(&self) -> Option<Region> {
        let id = self.editor.as_ref()?.selected()?;
        self.display_masks().into_iter().find(|m| m.0 == id).map(|m| m.1)
    }
    /// Kind of the selection for the property bar.
    pub fn selected_kind(&self) -> Option<Tool> {
        let e = self.editor.as_ref()?;
        let id = e.selected()?;
        if let Some(m) = e.masks().iter().find(|m| m.id == id) {
            return Some(if m.style == RedactStyle::SafeMosaic { Tool::Mosaic } else { Tool::Redact });
        }
        e.annotations().iter().find(|a| a.id == id).map(|a| match a.annotation.shape {
            Shape::Arrow { .. } => Tool::Arrow, Shape::Rect { .. } => Tool::Rect, Shape::Highlight { .. } => Tool::Highlight,
            Shape::Number { .. } => Tool::Number, Shape::Text { .. } => Tool::Text,
        })
    }
    /// Pull the selected object's style into the current style.
    fn sync_style(&mut self) {
        let Some(e) = self.editor.as_ref() else { return };
        let Some(id) = e.selected() else { return };
        if let Some(m) = e.masks().iter().find(|m| m.id == id) {
            if let RedactStyle::Solid(c) = m.style { if let Some(i) = SOLIDS.iter().position(|s| *s == c) { self.style.solid = i; } }
            return;
        }
        let Some(a) = e.annotations().iter().find(|a| a.id == id) else { return };
        if let Some(i) = INKS.iter().position(|c| *c == a.annotation.color) { self.style.color = i; }
        match &a.annotation.shape {
            Shape::Arrow { width, .. } | Shape::Rect { width, .. } => { if let Some(i) = WIDTHS.iter().position(|w| w == width) { self.style.width = i; } }
            Shape::Text { size, background, .. } => {
                if let Some(i) = TEXT_SIZES.iter().position(|s| s == size) { self.style.size = i; }
                self.style.background = *background;
            }
            _ => {}
        }
    }
    /// Apply a style change to the current tool and, if any, the selected object (one undo entry).
    pub fn apply_style(&mut self, f: impl FnOnce(&mut Style)) -> bool {
        f(&mut self.style);
        if self.drag.is_some() { return false; }
        let st = self.style;
        let Some(e) = self.editor.as_mut() else { return false };
        let Some(id) = e.selected() else { return false };
        if let Some(m) = e.masks().iter().find(|m| m.id == id) {
            let style = if m.style == RedactStyle::SafeMosaic { RedactStyle::SafeMosaic } else { RedactStyle::Solid(SOLIDS[st.solid]) };
            return e.set_redaction_style(id, style).unwrap_or(false);
        }
        let Some(mut a) = e.annotations().iter().find(|a| a.id == id).map(|a| a.annotation.clone()) else { return false };
        a.color = INKS[st.color];
        match &mut a.shape {
            Shape::Arrow { width, .. } | Shape::Rect { width, .. } => *width = WIDTHS[st.width],
            Shape::Text { size, background, .. } => { *size = TEXT_SIZES[st.size]; *background = st.background; }
            _ => {}
        }
        e.replace_annotation(id, a).unwrap_or(false)
    }
    /// Choose solid / safe mosaic for new redactions and the selected one.
    pub fn set_redaction_kind(&mut self, mosaic: bool) -> bool {
        self.tool = if mosaic { Tool::Mosaic } else { Tool::Redact };
        if self.drag.is_some() { return false; }
        let solid = SOLIDS[self.style.solid];
        let Some(e) = self.editor.as_mut() else { return false };
        let Some(id) = e.selected().filter(|id| e.masks().iter().any(|m| m.id == *id)) else { return false };
        e.set_redaction_style(id, if mosaic { RedactStyle::SafeMosaic } else { RedactStyle::Solid(solid) }).unwrap_or(false)
    }

    // ---- crop ----
    pub fn enter_crop(&mut self) {
        self.tool = Tool::Crop;
        self.deselect();
        self.crop_draft = self.editor.as_ref().and_then(|e| e.crop());
    }
    /// Apply the draft (one undo entry) and return to selection.
    pub fn confirm_crop(&mut self) -> bool {
        let draft = self.crop_draft.take();
        self.tool = Tool::Select;
        self.editor.as_mut().is_some_and(|e| e.set_crop(draft).unwrap_or(false))
    }
    pub fn cancel_crop(&mut self) { self.crop_draft = None; self.tool = Tool::Select; }
    /// Crop rectangle to show on the canvas (live drag, draft, or applied crop).
    pub fn crop_box(&self) -> Option<Region> {
        let view = self.view()?;
        if let Some(Drag::Crop { start, current }) = &self.drag { return view.viewport().pixel_region(*start, *current).ok(); }
        if self.tool == Tool::Crop { return self.crop_draft; }
        self.editor.as_ref()?.crop()
    }

    // ---- text ----
    /// Begin editing an existing text annotation (double-click).
    pub fn edit_text_at(&mut self, p: Point) -> bool {
        self.cancel_drag();
        let Hit::Ann(id) = self.hit(p) else { return false };
        let Some(a) = self.editor.as_ref().and_then(|e| e.annotations().iter().find(|a| a.id == id)).map(|a| a.annotation.clone()) else { return false };
        let Shape::Text { origin, text, size, background } = a.shape else { return false };
        if let Some(i) = TEXT_SIZES.iter().position(|s| *s == size) { self.style.size = i; }
        if let Some(i) = INKS.iter().position(|c| *c == a.color) { self.style.color = i; }
        self.style.background = background;
        self.text_edit = Some(TextEdit { origin, id: Some(id), initial: text });
        true
    }
    pub fn commit_text(&mut self, text: &str) -> bool {
        let Some(edit) = self.text_edit.take() else { return false };
        let Some(e) = self.editor.as_mut() else { return false };
        let text = text.trim_end().to_string();
        let ann = Annotation {
            shape: Shape::Text { origin: edit.origin, text: text.clone(), size: TEXT_SIZES[self.style.size], background: self.style.background },
            color: INKS[self.style.color],
        };
        match (edit.id, text.is_empty()) {
            (Some(id), true) => e.remove_object(id).is_ok(),
            (Some(id), false) => e.replace_annotation(id, ann).unwrap_or(false),
            (None, true) => false,
            (None, false) => e.add_annotation(ann).is_ok(),
        }
    }
    pub fn cancel_text(&mut self) { self.text_edit = None; }

    // ---- commands ----
    pub fn delete_selected(&mut self) -> bool {
        if self.drag.is_some() { return false; }
        let Some(e) = self.editor.as_mut() else { return false };
        e.selected().is_some_and(|id| e.remove_object(id).is_ok())
    }
    pub fn nudge(&mut self, dx: i64, dy: i64) -> bool {
        if self.drag.is_some() { return false; }
        let Some(e) = self.editor.as_mut() else { return false };
        let Some(id) = e.selected() else { return false };
        if e.masks().iter().any(|m| m.id == id) { return e.move_mask(id, dx, dy).unwrap_or(false); }
        let Some(a) = e.annotations().iter().find(|a| a.id == id).map(|a| a.annotation.clone()) else { return false };
        e.replace_annotation(id, translate(&a, dx as f32, dy as f32)).unwrap_or(false)
    }
    pub fn undo(&mut self) -> bool { self.drag.is_none() && self.editor.as_mut().is_some_and(|e| e.undo().is_ok()) }
    pub fn redo(&mut self) -> bool { self.drag.is_none() && self.editor.as_mut().is_some_and(|e| e.redo().is_ok()) }
    pub fn deselect(&mut self) { if let Some(e) = self.editor.as_mut() { let _ = e.select(None); } }

    // ---- delivery: render on a worker, then deliver the exact rendered PNG ----
    pub fn begin_delivery(&mut self, kind: Delivery) -> Result<RenderWork, EditError> {
        if self.busy.is_some() { return Err(EditError::OperationPending); }
        if self.drag.is_some() || self.text_edit.is_some() { return Err(EditError::GestureInProgress); }
        let e = self.editor.as_mut().ok_or(EditError::MissingSource)?;
        let work = e.prepare_render()?;
        self.busy = Some((kind, work.key()));
        Ok(work)
    }
    /// Accept a finished render only if nothing changed meanwhile, then open the delivery ticket.
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
    /// Hand the exact rendered bytes to the platform writer and record the outcome.
    pub fn deliver(&mut self, ticket: Ticket, write: impl FnOnce(&[u8], (u32, u32)) -> Result<(), String>) -> Result<(), String> {
        let e = self.editor.as_mut().ok_or("文档已关闭")?;
        let dims = e.output_dimensions();
        let Some(bytes) = e.bytes_for(ticket) else { e.finish(ticket, false); return Err("内容已变化".into()) };
        let outcome = write(bytes, dims);
        if e.finish(ticket, outcome.is_ok()) && outcome.is_ok() { self.delivered = Some(e.stamp()); }
        outcome
    }
    pub fn abandon_delivery(&mut self, ticket: Ticket) { if let Some(e) = self.editor.as_mut() { e.finish(ticket, false); } }
}

#[cfg(test)]
mod tests;
