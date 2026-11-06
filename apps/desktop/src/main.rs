//! Anshot (安截) desktop: capture / open -> redact -> copy / save. Slint UI over the Rust editor;
//! a tray-resident process with a global hotkey (Ctrl+Shift+A).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Slint's generated code needs `unsafe`; our own modules each `forbid` it.
#![deny(unsafe_code)]
use crate::i18n::{t, tf};
use anshot_core::sensitive::SensitiveKind;
use std::cell::Cell;

mod beautify;
mod canvas;
mod capture;
mod check;
mod color;
mod controller;
mod diag;
mod display;
mod history;
mod i18n;
mod messages;
mod platform;
mod reveal;
mod settings;
mod steps;
mod stitch;

use canvas::Zoom;
use controller::{Controller, Delivery, Notice, Tool, CUSTOM_INK, TEXT_SIZES};
use image::RgbaImage;
use anshot_editor::{EditError, Point, ReviewKey, ReviewResult, SourceCandidate};
use slint::{CloseRequestResponse, ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::cell::RefCell;
use std::path::PathBuf;
use anshot_platform_windows::{Monitor, ScreenRect};
use std::time::Duration;

slint::include_modules!();

type Decoded = Result<(SourceCandidate, RgbaImage), String>;

enum Pending { Replace(SourceCandidate, RgbaImage, Option<String>, Option<Delivery>), Close, Quit }

/// What to do once a confirm-dialog copy succeeds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AfterCopy { Stay, Close, Quit }

#[derive(Clone, Copy)]
enum ToastAction { Retry(Delivery), Review, MuteHdr, OpenSteps, UndoStep }

/// One image that left, with what is needed to check it, taken when it left: a later
/// change (frame, crop, covers) must not move the area that is read.
struct Delivered {
    png: Vec<u8>,
    kind: Delivery,
    document: u64,
    source: Vec<u8>,
    crop: Option<anshot_core::Region>,
    /// Size of the delivered image and where the screenshot sits in it.
    size: (u32, u32),
    rect: (u32, u32, u32, u32),
    covers: Vec<anshot_core::Region>,
}

struct App {
    ctl: Controller,
    ui: slint::Weak<AppWindow>,
    pending: Option<Pending>,
    save_path: Option<PathBuf>,
    after_copy: AfterCopy,
    toast_seq: u64,
    toast_action: Option<ToastAction>,
    /// Tray + hotkey owner. A second instance runs editor-only and quits on close.
    resident: bool,
    tray: Option<Tray>,
    session: Option<capture::Session>,
    overlays: Vec<(CaptureOverlay, Monitor)>,
    /// Overlay windows are kept (hidden) and reused: creating a window and its GL
    /// context per capture cost ~1 s. Index i always serves monitor i.
    overlay_pool: Vec<CaptureOverlay>,
    toast_win: Option<ToastWindow>,
    /// Decoded source downscaled for the screen, and its scale (preview px per source px).
    base: Option<(RgbaImage, f32)>,
    settings: settings::Settings,
    hotkey: Option<anshot_platform_windows::HotkeyHandle>,
    /// The editor was hidden for a capture and comes back if the capture is cancelled.
    restore_editor: bool,
    /// A capture waits for the hidden editor to leave the screen.
    capture_pending: bool,
    /// Follows the pointer while an overlay is open (R7: the magnifier must track the real
    /// cursor even when no mouse-move event reaches the overlay).
    cursor_timer: slint::Timer,
    last_cursor: Option<(i32, i32)>,
    /// The region of the last capture this run (memory only, never saved): R puts it back.
    last_region: Option<ScreenRect>,
    /// Bumped on every overlay notice; an older notice's timer does not clear a newer one.
    notice_seq: u64,
    /// Pinned images (top-most windows); ids stay stable while others close.
    pins: Vec<Pin>,
    next_pin: u64,
    /// Pin being dragged: id, cursor and window position at the press.
    pin_drag: Option<(u64, (i32, i32), (i32, i32))>,
    /// History entries currently listed in the history dialog (same order as the UI).
    history_shown: Vec<history::Entry>,
    /// Bumped on every history refresh; a slower, older thumbnail load is discarded.
    history_gen: u64,
    /// A running long screenshot.
    long: Option<LongShot>,
    /// A delayed capture counting down: its window, the tick timer, seconds left.
    countdown: Option<(CountdownWindow, slint::Timer, u32)>,
    /// The text recognition window (reused) and the run it waits for.
    ocr_win: Option<OcrWindow>,
    ocr_seq: u64,
    /// Pins (ids) and the OCR window hidden while the screen is frozen; shown again after.
    hidden_for_capture: Vec<u64>,
    ocr_hidden: bool,
    /// A text recognition job is running.
    ocr_running: bool,
    check_running: bool,
    auto_check_running: bool,
    /// The newest delivery waiting for its check.
    auto_check_next: Option<Delivered>,
    /// The system toast's own timer: an editor toast must not keep it on screen.
    system_toast_seq: u64,
    /// The redaction the stand-in input belongs to.
    replace_target: Cell<Option<u64>>,
    /// The beautify panel is open: the canvas shows the framed export and editing pauses.
    frame_open: bool,
    /// Chrome text of this document: typed by the user, memory only, never saved.
    frame_text: String,
    /// The framed preview of one document revision.
    framed_cache: RefCell<Option<((u64, anshot_editor::Stamp), Image)>>,
    /// The HDR notice was shown this run.
    hdr_noticed: bool,
    /// Step image (ADR 0.6-STEPS): the steps (memory only), the panel, burst mode.
    steps: steps::Steps,
    steps_open: bool,
    burst: bool,
    /// Steps still being encoded on a worker (they count against the limit).
    steps_adding: usize,
    /// A step image is being joined.
    steps_busy: bool,
    steps_preview_gen: u64,
    /// The document is this step: Ctrl+J puts it back in its place.
    editing_step: Option<u64>,
    /// The step being opened for editing (linked once it really opens).
    pending_step_edit: Option<u64>,
}

struct LongShot {
    /// Pins and the OCR window stay hidden while live frames are captured.
    hidden: (Vec<u64>, bool),
    region: ScreenRect,
    stitcher: Option<stitch::Stitcher>,
    panel: LongShotPanel,
    timer: slint::Timer,
}

struct Pin { id: u64, win: PinWindow, img: RgbaImage, zoom: f32 }

fn to_image(img: &RgbaImage) -> Image {
    let (w, h) = img.dimensions();
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    buf.make_mut_bytes().copy_from_slice(img.as_raw());
    Image::from_rgba8(buf)
}

#[derive(Clone, Copy)]
enum Phase { Down, Move, Up }

thread_local! { static APP: RefCell<Option<App>> = const { RefCell::new(None) }; }

/// Run `f` with the app state. Never call blocking OS dialogs inside `f`:
/// their nested message loop could re-enter a callback while the state is borrowed.
fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| cell.try_borrow_mut().ok().and_then(|mut a| a.as_mut().map(f)))
}

impl App {
    fn ui(&self) -> Option<AppWindow> { self.ui.upgrade() }

    fn refresh(&self) {
        let Some(ui) = self.ui() else { return };
        let ctl = &self.ctl;
        ui.set_has_document(ctl.editor.is_some());
        ui.set_opening(ctl.opening);
        ui.set_tool(tool_name(ctl.tool).into());
        ui.set_busy(match ctl.busy() { Some(Delivery::Copy) => "copy", Some(Delivery::Save) => "save", None => "" }.into());
        ui.set_can_deliver(ctl.can_deliver());
        self.push_check(&ui);
        self.push_steps_state();
        ui.set_narrow(ctl.area().0 < 900.0);
        ui.set_tiny(ctl.area().0 < 720.0);
        let st = ctl.style;
        ui.set_color_index(st.color as i32);
        let cc = ctl.custom;
        ui.set_custom_color(slint::Color::from_rgb_u8(cc[0], cc[1], cc[2]));
        ui.set_shape_kind(match ctl.selected_kind().unwrap_or(ctl.tool) {
            Tool::Rect | Tool::Ellipse => "box", Tool::Arrow | Tool::Line => "line", _ => "",
        }.into());
        ui.set_width_index(st.width as i32);
        ui.set_size_index(st.size as i32);
        ui.set_text_background(st.background);
        ui.set_solid_index(st.solid as i32);
        let (Some(e), Some(view)) = (ctl.editor.as_ref(), ctl.view()) else {
            ui.set_grips(ModelRc::default());
            ui.set_sel_visible(false);
            ui.set_crop_visible(false);
            ui.set_has_selection(false);
            ui.set_mask_selected(false);
            ui.set_replace_editing(false);
            ui.set_has_region(false);
            ui.set_text_editing(false);
            ui.set_dims_text(SharedString::new());
            ui.set_can_undo(false);
            ui.set_can_redo(false);
            return;
        };
        let busy = ctl.dragging() || ctl.text_edit.is_some();
        ui.set_can_undo(e.can_undo() && !busy);
        ui.set_can_redo(e.can_redo() && !busy);
        ui.set_img_x(view.x as f32);
        ui.set_img_y(view.y as f32);
        ui.set_img_w(view.width() as f32);
        ui.set_img_h(view.height() as f32);
        let (w, h) = e.dimensions();
        let (ow, oh) = e.output_dimensions();
        ui.set_dims_text(if (ow, oh) == (w, h) { format!("{w} × {h}") } else { format!("{w} × {h} → {ow} × {oh}") }.into());
        ui.set_zoom_fit(ctl.zoom == Zoom::Fit);
        ui.set_zoom_text(format!("{}%", (view.scale * 100.0).round()).into());
        ui.set_mask_count(e.masks().len() as i32);
        // WYSIWYG canvas: the same Rust renderer as export, at preview scale.
        if let Some((base, k)) = self.base.as_ref() {
            let mut img = base.clone();
            anshot_render::render_into(&mut img, &ctl.display_scene(), *k);
            ui.set_preview(to_image(&img));
        }
        self.push_frame(&ui);
        if self.frame_open {
            if let Some((img, (pw, ph))) = self.framed_preview() {
                // Centred in the room left of the panel (300 px wide, 12 px from the edge).
                let (aw, ah) = ctl.area();
                let aw = if aw > 700.0 { aw - 324.0 } else { aw };
                let s = ((aw - 48.0) / pw).min((ah - 48.0) / ph).clamp(0.01, 1.0);
                ui.set_img_x(((aw - pw * s) / 2.0) as f32);
                ui.set_img_y(((ah - ph * s) / 2.0) as f32);
                ui.set_img_w((pw * s) as f32);
                ui.set_img_h((ph * s) as f32);
                ui.set_preview(img);
            }
        }
        let mb = |(x, y, w, h): (f64, f64, f64, f64)| MaskBox { x: x as f32, y: y as f32, w: w as f32, h: h as f32, selected: true };
        match ctl.selection_box() {
            Some(b) => { ui.set_sel(mb(b)); ui.set_sel_visible(true); }
            None => ui.set_sel_visible(false),
        }
        let grips: Vec<MaskBox> = ctl.grips().into_iter().map(|(_, (x, y))| mb((x, y, 0.0, 0.0))).collect();
        ui.set_grips(ModelRc::new(VecModel::from(grips)));
        match ctl.crop_box() {
            Some(r) => { ui.set_crop(mb(view.rect(r))); ui.set_crop_visible(true); }
            None => ui.set_crop_visible(false),
        }
        ui.set_crop_active(ctl.tool == Tool::Crop);
        let kind = ctl.selected_kind();
        ui.set_has_selection(kind.is_some());
        let target = ctl.selected_mask().map(|m| m.0);
        ui.set_mask_selected(target.is_some());
        // The input belongs to one redaction: it must not write to the next one selected.
        if self.replace_target.replace(target) != target && ui.get_replace_editing() { ui.set_replace_editing(false); ui.invoke_focus_keys(); }
        let mode = match kind.unwrap_or(ctl.tool) {
            Tool::Redact => "redact", Tool::Mosaic => "mosaic",
            Tool::Arrow | Tool::Rect | Tool::Ellipse | Tool::Line | Tool::Pen | Tool::Magnify => "stroke",
            // A spotlight has no colour or width: the property bar only shows the hint.
            Tool::Spotlight => "spotlight",
            Tool::Number | Tool::Highlight => "fill", Tool::Text => "text", Tool::Select => "hint-select", Tool::Crop => "crop",
        };
        ui.set_prop_mode(mode.into());
        let region = ctl.selected_region();
        ui.set_has_region(region.is_some() && !ctl.narrow_propbar());
        if let Some(r) = region {
            ui.set_sel_x(r.x0.to_string().into());
            ui.set_sel_y(r.y0.to_string().into());
            ui.set_sel_w((r.x1 - r.x0).to_string().into());
            ui.set_sel_h((r.y1 - r.y0).to_string().into());
        }
        match ctl.text_edit.as_ref() {
            Some(t) => {
                ui.set_text_x((view.x + f64::from(t.origin.0) * view.scale) as f32);
                ui.set_text_y((view.y + f64::from(t.origin.1) * view.scale) as f32);
                ui.set_text_size((f64::from(TEXT_SIZES[st.size]) * view.scale) as f32);
                let c = ctl.ink();
                ui.set_text_color(slint::Color::from_rgb_u8(c[0], c[1], c[2]));
                if !ui.get_text_editing() { ui.set_text_initial(t.initial.clone().into()); }
                ui.set_text_editing(true);
            }
            None => ui.set_text_editing(false),
        }
        let title = ctl.source_name.clone().unwrap_or_else(|| t("未命名").into());
        ui.set_doc_title(if ctl.has_unsaved_edits() { format!("● {title}") } else { title }.into());
    }

    fn toast(&mut self, kind: &str, title: &str, detail: &str, action: Option<(&str, ToastAction)>) {
        let Some(ui) = self.ui() else { return };
        // A quick copy from the capture toolbar runs with the editor hidden: never report
        // into an invisible window.
        if !ui.window().is_visible() {
            self.toast_action = action.as_ref().map(|a| a.1);
            self.system_toast_with(kind, title, detail, action.map_or("", |a| a.0));
            return;
        }
        self.toast_seq += 1;
        let seq = self.toast_seq;
        self.toast_action = action.map(|a| a.1);
        ui.set_toast_kind(kind.into());
        ui.set_toast_title(title.into());
        ui.set_toast_detail(detail.into());
        ui.set_toast_action(action.map(|a| a.0).unwrap_or_default().into());
        if kind == "done" {
            slint::Timer::single_shot(Duration::from_secs(3), move || {
                with(|a| if a.toast_seq == seq { a.clear_toast() });
            });
        }
    }
    fn clear_toast(&mut self) {
        self.toast_action = None;
        if let Some(ui) = self.ui() { ui.set_toast_kind(SharedString::new()); }
    }

    // ---- intake ----
    /// `then` (overlay toolbar copy / save) belongs to this image only: it is dropped
    /// with the image on any failure, refusal or cancelled replace prompt.
    fn source_ready(&mut self, decoded: Decoded, name: Option<String>, then: Option<Delivery>) {
        self.ctl.opening = false;
        match decoded {
            Err(e) => { self.show_editor(); self.toast("error", t("无法打开"), &e, None) }
            Ok((candidate, preview)) => {
                if self.ctl.has_unsaved_edits() {
                    self.ctl.cancel_drag();
                    self.pending = Some(Pending::Replace(candidate, preview, name, then));
                    self.show_editor();
                    if let Some(ui) = self.ui() { ui.set_dialog("replace".into()); }
                } else {
                    self.apply_source(candidate, preview, name, then);
                }
            }
        }
        self.refresh();
    }
    fn apply_source(&mut self, candidate: SourceCandidate, preview: RgbaImage, name: Option<String>, then: Option<Delivery>) {
        if let Err(e) = self.ctl.accept_source(candidate, name) {
            self.show_editor();
            self.toast("error", t("无法打开"), &messages::intake(&e), None);
            return;
        }
        self.after_copy = AfterCopy::Stay;
        // A new document is shown at once; it is a step being edited only when opened as one.
        self.steps_open = false;
        self.editing_step = self.pending_step_edit.take();
        let k = self.ctl.editor.as_ref().map_or(1.0, |e| preview.width() as f32 / e.dimensions().0 as f32);
        self.base = Some((preview, k));
        self.forget_frame();
        if self.settings.frame_default {
            let frame = self.settings.frame_style.frame("");
            // A frame that cannot fit this image is skipped quietly; the panel says why.
            if let Some(e) = self.ctl.editor.as_mut() { let _ = e.set_initial_frame(frame); }
        }
        self.clear_toast();
        match then {
            Some(Delivery::Copy) => { self.start(Delivery::Copy); }
            // The save dialog is blocking: run it outside the state borrow.
            Some(Delivery::Save) => { slint::Timer::single_shot(Duration::ZERO, || command("save")); }
            None => {}
        }
    }

    // ---- delivery ----
    fn start(&mut self, kind: Delivery) -> bool {
        if self.ctl.editor.as_ref().is_some_and(|e| !e.frame_fits()) {
            if let Some(e) = self.ctl.editor.as_mut() { let _ = e.set_frame(None); }
            self.toast("warn", t("图片太大，已关闭美化"), t("美化后会超过 2400 万像素；请再复制或保存一次"), None);
            self.refresh();
            return false;
        }
        let started = match self.ctl.begin_delivery(kind) {
            Ok(work) => {
                self.clear_toast();
                let key = work.key();
                std::thread::spawn(move || {
                    let result = work.run();
                    let _ = slint::invoke_from_event_loop(move || { with(|a| a.review_done(&key, result)); });
                });
                true
            }
            Err(EditError::NoEffectiveMask) => { self.toast("warn", t("还没有遮挡"), t("先用遮挡工具框住要隐藏的内容"), None); false }
            Err(EditError::OperationPending) => { self.toast("warn", t("正在处理"), t("请等上一次复制或保存完成"), None); false }
            Err(EditError::GestureInProgress) => { self.toast("warn", t("正在拖动"), t("请先松开鼠标或按 Esc"), None); false }
            Err(e) => { self.toast("error", t("无法开始"), &messages::edit(&e), None); false }
        };
        self.refresh();
        started
    }
    fn review_done(&mut self, key: &ReviewKey, result: Result<ReviewResult, EditError>) {
        match self.ctl.review_finished(key, result) {
            Ok((Delivery::Copy, ticket)) => {
                let mut kept = None;
                let outcome = self.ctl.deliver(ticket, |png, dims| { let r = platform::write_clipboard(png, dims); if r.is_ok() { kept = Some(png.to_vec()); } r });
                if let Some(png) = kept { self.record_history(&png); self.auto_check_delivered(png, Delivery::Copy); }
                self.after_delivery(Delivery::Copy, outcome, None);
            }
            Ok((Delivery::Save, ticket)) => {
                let Some(path) = self.save_path.take() else { self.ctl.abandon_delivery(ticket); self.refresh(); return };
                let path = platform::free_path(&path);
                let mut kept = None;
                let outcome = self.ctl.deliver(ticket, |png, _| { let r = platform::write_file(&path, png); if r.is_ok() { kept = Some(png.to_vec()); } r });
                if let Some(png) = kept { self.record_history(&png); self.auto_check_delivered(png, Delivery::Save); }
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
                self.after_delivery(Delivery::Save, outcome, name);
            }
            Err(Notice::Changed) => {
                self.after_copy = AfterCopy::Stay;
                self.toast("warn", t("内容已变化"), t("请重新复制或保存"), None);
            }
            Err(Notice::Failed { title, detail }) => {
                self.after_copy = AfterCopy::Stay;
                self.toast("error", &title, &detail, None);
            }
            Err(Notice::Ignored) => {}
        }
        self.sync_frame_text();
        self.refresh();
    }
    fn after_delivery(&mut self, kind: Delivery, outcome: Result<(), String>, name: Option<String>) {
        let n = self.ctl.redactions_in_output();
        let (w, h) = self.ctl.editor.as_ref().map_or((0, 0), |e| e.output_dimensions());
        match (kind, outcome) {
            (Delivery::Copy, Ok(())) if n == 0 && self.settings.warn_unredacted => {
                // Nothing is covered: say so plainly (the copy already happened; nothing is blocked).
                let detail = tf("{} × {} · 如有手机号、地址等，请先遮挡再分享", &[&w, &h]);
                if matches!(std::mem::replace(&mut self.after_copy, AfterCopy::Stay), AfterCopy::Quit) { let _ = slint::quit_event_loop(); return; }
                if self.ui().is_some_and(|ui| ui.window().is_visible()) { self.toast("warn", t("已复制 · 图中没有遮挡"), &detail, None); }
                else { self.system_toast("warn", t("已复制 · 图中没有遮挡"), &detail); }
            }
            (Delivery::Copy, Ok(())) => {
                let detail = if n == 0 { tf("{} × {} · 未遮挡任何内容", &[&w, &h]) } else { tf("{} × {} · 已遮挡 {} 处", &[&w, &h, &n]) };
                match std::mem::replace(&mut self.after_copy, AfterCopy::Stay) {
                    AfterCopy::Quit => { let _ = slint::quit_event_loop(); }
                    // 02 U07: a successful copy ends the task; the editor steps aside.
                    _ if self.resident && self.settings.hide_after_copy => { self.close_editor(false); self.system_toast("done", t("已复制到剪贴板"), &detail); }
                    AfterCopy::Close if self.resident => { self.close_editor(false); self.system_toast("done", t("已复制到剪贴板"), &detail); }
                    AfterCopy::Close => { let _ = slint::quit_event_loop(); }
                    AfterCopy::Stay if !self.ui().is_some_and(|ui| ui.window().is_visible()) => self.system_toast("done", t("已复制到剪贴板"), &detail),
                    AfterCopy::Stay => self.toast("done", t("已复制到剪贴板"), &detail, None),
                }
            }
            (Delivery::Save, Ok(())) if n == 0 && self.settings.warn_unredacted => { self.after_copy = AfterCopy::Stay; self.toast("warn", t("已保存 · 图中没有遮挡"), &name.unwrap_or_default(), None) }
            (Delivery::Save, Ok(())) => { self.after_copy = AfterCopy::Stay; self.toast("done", t("已保存"), &name.unwrap_or_default(), None) }
            (Delivery::Copy, Err(e)) => {
                self.after_copy = AfterCopy::Stay;
                self.toast("error", t("复制失败"), &e, Some((t("重试"), ToastAction::Retry(Delivery::Copy))));
            }
            (Delivery::Save, Err(e)) => { self.after_copy = AfterCopy::Stay; self.toast("error", t("保存失败"), &e, None) }
        }
    }

    fn key(&mut self, text: &str, ctrl: bool, shift: bool, other: bool) -> bool {
        use slint::platform::Key;
        let is = |k: Key| text == SharedString::from(k).as_str();
        if let Some(ui) = self.ui() {
            if ui.get_dialog() != "" {
                if is(Key::Escape) { self.dialog_choice("cancel"); return true; }
                if is(Key::Return) && ui.get_dialog() == "close" && self.ctl.can_deliver() { self.dialog_choice("copy-close"); return true; }
                if is(Key::Return) && (ui.get_dialog() == "about" || ui.get_dialog() == "settings") { self.dialog_choice("cancel"); return true; }
                if is(Key::Return) && ui.get_dialog() == "color" { self.apply_picker(); return true; }
                return false;
            }
        }
        if is(Key::Escape) && self.ctl.review.is_some() && !self.ctl.dragging() && self.ctl.tool != Tool::Crop { self.check_command("check-close"); return true; }
        if is(Key::Escape) && self.frame_open { self.frame_command("frame"); return true; }
        if is(Key::Escape) && self.steps_open { self.open_steps(false); return true; }
        if other { return false; } // Alt / Win combinations are not ours.
        if ctrl {
            // Some platforms deliver Ctrl+letter as a control character.
            let step = if shift { 10 } else { 1 };
            let resize = if is(Key::LeftArrow) { Some((-step, 0)) } else if is(Key::RightArrow) { Some((step, 0)) }
                else if is(Key::UpArrow) { Some((0, -step)) } else if is(Key::DownArrow) { Some((0, step)) } else { None };
            if let Some((dx, dy)) = resize { self.ctl.resize_selected(dx, dy); self.refresh(); return true; }
            let c = text.chars().next().map(|c| if (c as u32) < 27 && c as u32 > 0 { (b'a' + c as u8 - 1) as char } else { c.to_ascii_lowercase() });
            match c {
                Some('z') if shift => { self.ctl.redo(); }
                Some('z') => { self.ctl.undo(); }
                Some('y') => { self.ctl.redo(); }
                Some('c') => { self.start(Delivery::Copy); return true; }
                Some('0') => self.ctl.zoom_to(Zoom::Fit),
                Some('1') => self.ctl.zoom_to(Zoom::Scale(1.0)),
                Some('=') | Some('+') => { let c = canvas_centre(&self.ctl); self.ctl.zoom_by(1.25, c); }
                Some('-') => { let c = canvas_centre(&self.ctl); self.ctl.zoom_by(0.8, c); }
                _ => return false, // Ctrl+O / Ctrl+V / Ctrl+S are handled outside the borrow.
            }
            self.refresh();
            return true;
        }
        // The preview shows the framed result: nothing on the canvas can be edited there.
        if self.frame_open || self.steps_open { return false; }
        let step = if shift { 10 } else { 1 };
        if self.ctl.tool == Tool::Crop {
            if is(Key::Return) { self.ctl.confirm_crop(); self.refresh(); return true; }
            if is(Key::Escape) { self.ctl.cancel_crop(); self.refresh(); return true; }
        }
        if text == " " { self.ctl.space_pan = true; return true; }
        if is(Key::Tab) || is(Key::Backtab) { let back = shift || is(Key::Backtab); self.ctl.cycle_selection(back); self.refresh(); return true; }
        let handled = if is(Key::Delete) || is(Key::Backspace) { self.ctl.delete_selected() }
            else if is(Key::LeftArrow) { self.ctl.nudge(-step, 0) }
            else if is(Key::RightArrow) { self.ctl.nudge(step, 0) }
            else if is(Key::UpArrow) { self.ctl.nudge(0, -step) }
            else if is(Key::DownArrow) { self.ctl.nudge(0, step) }
            else if is(Key::Escape) { if self.ctl.dragging() { self.ctl.cancel_drag() } else { self.ctl.deselect() }; true }
            else {
                match text.chars().next().map(|c| c.to_ascii_lowercase()) {
                    Some(c @ '1'..='7') => { let i = c as usize - '1' as usize; self.ctl.apply_style(|s| s.color = i); true }
                    Some(c) => match tool_for_key(c) {
                        Some(Tool::Crop) => { self.ctl.enter_crop(); true }
                        Some(Tool::Mosaic) => { self.ctl.set_redaction_kind(true); true }
                        Some(Tool::Redact) => { self.ctl.set_redaction_kind(false); true }
                        Some(t @ (Tool::Rect | Tool::Ellipse | Tool::Arrow | Tool::Line)) => { self.ctl.set_shape_tool(t); true }
                        Some(t) => { self.ctl.tool = t; true }
                        None => false,
                    },
                    None => false,
                }
            };
        self.refresh();
        handled
    }

    // ---- window lifecycle ----
    /// Hide the editor (resident) or end the process. `discard` drops the document.
    fn close_editor(&mut self, discard: bool) {
        if discard { self.ctl.close_document(); self.base = None; self.forget_frame(); self.editing_step = None; self.refresh(); }
        if !self.resident { let _ = slint::quit_event_loop(); return; }
        if let Some(ui) = self.ui() { let _ = ui.hide(); }
    }
    fn show_editor(&mut self) {
        // While the screen is about to be frozen, defer: the capture restores it on cancel.
        if self.capture_pending { self.restore_editor = true; return; }
        if let Some(ui) = self.ui() { let _ = ui.show(); }
        self.refresh();
    }
    fn request_quit(&mut self) {
        if self.ctl.has_unsaved_edits() || !self.steps.is_empty() {
            self.show_editor();
            self.ctl.cancel_drag();
            self.pending = Some(Pending::Quit);
            let dialog = if self.ctl.has_unsaved_edits() { "close" } else { "steps-quit" };
            if let Some(ui) = self.ui() { ui.set_dialog(dialog.into()); }
        } else { let _ = slint::quit_event_loop(); }
    }

    /// S4 outside the editor: bottom-right of the primary work area, 3 s.
    fn system_toast(&mut self, kind: &str, heading: &str, detail: &str) { self.system_toast_with(kind, heading, detail, ""); }
    fn system_toast_with(&mut self, kind: &str, heading: &str, detail: &str, action: &str) {
        let wide = !action.is_empty() || heading.chars().count() > 24;
        if self.toast_win.is_none() {
            self.toast_win = ToastWindow::new().ok();
            if let Some(t) = self.toast_win.as_ref() {
                t.on_act(|| { with(|a| { if let Some(t) = a.toast_win.as_ref() { let _ = t.hide(); } }); command("toast-action"); });
            }
        }
        let Some(t) = self.toast_win.as_ref() else { return };
        t.set_action(action.into());
        t.set_wide(wide);
        t.set_kind(kind.into());
        t.set_heading(heading.into());
        t.set_detail(detail.into());
        // The toast must not take keyboard focus (e.g. from a pin the user is working with).
        let focus = anshot_platform_windows::foreground_window();
        let _ = t.show();
        if let Some(h) = hwnd_of(t.window()) { anshot_platform_windows::exclude_from_capture(h); }
        // The window system shows it on a later frame: hand focus back after that.
        slint::Timer::single_shot(Duration::from_millis(60), move || anshot_platform_windows::restore_foreground(focus));
        if let Some(m) = anshot_platform_windows::monitors().ok().and_then(|ms| ms.into_iter().find(|m| m.primary)) {
            let s = t.window().scale_factor();
            let (w, h) = ((if wide { 420.0 } else { 300.0 } * s) as i32, (64.0 * s) as i32);
            t.window().set_position(slint::PhysicalPosition::new(m.work.x + m.work.w - w - (16.0 * s) as i32, m.work.y + m.work.h - h - (16.0 * s) as i32));
        }
        self.system_toast_seq += 1;
        let seq = self.system_toast_seq;
        // A toast with a button stays long enough to reach it.
        slint::Timer::single_shot(Duration::from_secs(if action.is_empty() { 3 } else { 10 }), move || {
            with(|a| if a.system_toast_seq == seq { if let Some(t) = a.toast_win.as_ref() { let _ = t.hide(); } });
        });
    }

    // ---- privacy check: suggestions only; nothing changes until the user applies them ----
    fn run_check(&mut self) {
        if self.check_running { return; }
        self.frame_open = false;
        self.steps_open = false;
        let Some((source, scene)) = self.ctl.ocr_snapshot() else { return };
        let document = self.ctl.generation();
        let covers = self.ctl.covers_in_output();
        self.check_running = true;
        self.ctl.review = None;
        self.refresh();
        std::thread::spawn(move || {
            let crop = scene.crop;
            let items = anshot_render::render_png(&source, &scene).map_err(|_| t("无法生成图片，请重试。").to_string())
                .and_then(|png| anshot_core::decode_source(&png).map_err(|_| t("无法生成图片，请重试。").to_string()))
                .and_then(|img| check::scan(&img).map(|found| check::to_items(found, crop, &img)).map_err(|e| check::ocr_message(&e).to_string()))
                ;
            let _ = slint::invoke_from_event_loop(move || {
                with(|a| {
                    a.check_running = false;
                    // The document was replaced or closed while the worker ran.
                    if a.ctl.generation() == document {
                        match items {
                            Ok(items) => a.ctl.review = Some(check::Review { document, items: check::without_dismissed(items, &covers), like: None, replace: false, delivered: None }),
                            Err(e) => a.toast("error", t("无法检查"), &e, None),
                        }
                    }
                    a.refresh();
                });
            });
        });
    }
    /// After a copy or save: read the bytes that left, on a worker, and speak up only if
    /// something new turned up. Never blocks and never changes anything; a missing OCR
    /// language stays silent here (the button explains it).
    /// Right after a delivery: the document is still exactly what left.
    fn auto_check_delivered(&mut self, png: Vec<u8>, kind: Delivery) {
        if !self.settings.check_on_share || self.after_copy == AfterCopy::Quit { return; }
        let Some((source, crop, rect)) = self.ctl.delivery_source() else { return };
        let Some(size) = self.ctl.editor.as_ref().map(|e| e.output_dimensions()) else { return };
        let d = Delivered { png, kind, document: self.ctl.generation(), source, crop, size, rect, covers: self.ctl.covers_in_output() };
        self.auto_check(d);
    }
    fn auto_check(&mut self, d: Delivered) {
        // One check at a time: a newer delivery waits and replaces any older one still waiting.
        if self.auto_check_running { self.auto_check_next = Some(d); return; }
        let Delivered { png, kind, document, source, crop, size, rect: (ix, iy, iw, ih), covers } = d;
        self.auto_check_running = true;
        std::thread::spawn(move || {
            let decode = |bytes: &[u8]| anshot_core::decode_source(bytes).ok();
            // A framed delivery: only the screenshot inside it (not the background or chrome text).
            let inner = |img: RgbaImage| if img.dimensions() != size || ix + iw > img.width() || iy + ih > img.height() { None }
                else if (iw, ih) == size { Some(img) } else { Some(image::imageops::crop_imm(&img, ix, iy, iw, ih).to_image()) };
            let done = decode(&png).and_then(inner).zip(decode(&source)).and_then(|(export, source)| check::check_export(&export, crop, &source, &covers).ok());
            drop(source);
            let _ = slint::invoke_from_event_loop(move || {
                // An edit made meanwhile changes nothing: this image has already left.
                with(|a| {
                    a.auto_check_running = false;
                    match a.auto_check_next.take() {
                        // A newer image has left since: only its result matters.
                        Some(next) => a.auto_check(next),
                        None => if a.ctl.generation() == document { if let Some((items, audit)) = done { a.auto_check_done(items, audit, document, kind); } },
                    }
                });
            });
        });
    }
    fn auto_check_done(&mut self, items: Vec<check::Item>, audit: check::Audit, document: u64, kind: Delivery) {
        let mut items = check::without_dismissed(items, &self.ctl.dismissed);
        // Covered while the worker ran. Text still readable on a cover stays listed.
        let covers = self.ctl.covers_in_output();
        items.retain(|i| i.kind == SensitiveKind::SameText || check::without_dismissed(vec![i.clone()], &covers).len() == 1);
        if items.is_empty() {
            // Only now can the covers be vouched for, and only as far as this reader goes.
            if audit.vouches() {
                let detail = if audit.no_text > 0 { tf("另有 {} 处框内未识别到文字 · 这是辅助检查", &[&audit.no_text]) } else { t("这是辅助检查").to_string() };
                self.toast("done", &tf("已核对 {} 处遮挡：成品里识别不出", &[&audit.checked]), &detail, None);
            }
            return;
        }
        let n = items.len();
        let title = if kind == Delivery::Copy { tf("已复制 · 发现 {} 处可能的隐私内容", &[&n]) } else { tf("已保存 · 发现 {} 处可能的隐私内容", &[&n]) };
        self.ctl.pending_review = Some(check::Review { document, items, like: None, replace: false, delivered: Some(kind) });
        self.toast("warn", &title, t("这是辅助检查"), Some((t("查看"), ToastAction::Review)));
    }
    fn open_pending_review(&mut self) {
        self.frame_open = false;
        self.ctl.review = self.ctl.pending_review.take();
        self.clear_toast();
        self.show_editor();
    }
    /// Show typed text on the selected redaction, on a fill taken from around it.
    fn replace_selected(&mut self, text: &str) {
        let text = text.trim();
        let (Some((_, r)), Some((base, k))) = (self.ctl.selected_mask(), self.base.as_ref()) else { return };
        if text.is_empty() { return; }
        let at = |v: i64| (v as f32 * k).round() as i64;
        let fill = check::backdrop(base, anshot_core::Region::new(at(r.x0), at(r.y0), at(r.x1), at(r.y1)));
        if let Err(e) = self.ctl.replace_selected(text, fill) { self.toast("error", t("无法遮挡"), &messages::edit(&e), None); }
        self.refresh();
    }
    /// Find the selected redaction's text elsewhere. It reads the source pixels, because the
    /// text is covered in the export; only the suggestions leave the worker.
    fn run_same_text(&mut self) {
        if self.check_running { return; }
        self.frame_open = false;
        let Some((mask, region)) = self.ctl.selected_mask() else { return };
        let Some((source, _)) = self.ctl.ocr_snapshot() else { return };
        let Some(e) = self.ctl.editor.as_ref() else { return };
        let document = self.ctl.generation();
        let covered: Vec<anshot_core::Region> = e.masks().iter().map(|m| m.region).collect();
        self.check_running = true;
        self.ctl.review = None;
        self.refresh();
        std::thread::spawn(move || {
            let found = anshot_core::decode_source(&source).map_err(|_| t("无法读取图片").to_string())
                .and_then(|img| check::same_as(&img, region).map(|found| found.map(|f| check::without_dismissed(check::to_items(f, None, &img), &covered))).map_err(|e| check::ocr_message(&e).to_string()));
            let _ = slint::invoke_from_event_loop(move || {
                with(|a| {
                    a.check_running = false;
                    if a.ctl.generation() == document {
                        match found {
                            Ok(Some(items)) if !items.is_empty() => a.ctl.review = Some(check::Review { document, items, like: Some(mask), replace: false, delivered: None }),
                            Ok(Some(_)) => a.toast("warn", t("没有找到相同内容"), t("写法不同或识别有误时找不到"), None),
                            Ok(None) => a.toast("warn", t("框内没有识别到文字"), t("至少要两个字才能查找"), None),
                            Err(e) => a.toast("error", t("无法检查"), &e, None),
                        }
                    }
                    a.refresh();
                });
            });
        });
    }
    fn check_command(&mut self, c: &str) {
        match c {
            "check-mode-cover" | "check-mode-replace" => if let Some(r) = self.ctl.review.as_mut() { r.replace = c == "check-mode-replace"; },
            "check-close" => self.ctl.dismiss_review(),
            "check-apply" => {
                // The image that left is still the uncovered one: replace it.
                let recopy = self.ctl.review.as_ref().is_some_and(|r| r.delivered == Some(Delivery::Copy));
                let resave = self.ctl.review.as_ref().is_some_and(|r| r.delivered == Some(Delivery::Save));
                // Not yet: the list stays so that the clipboard's old image is not forgotten.
                if recopy && !self.ctl.can_deliver() { self.toast("warn", t("正在处理"), t("请等上一次复制或保存完成"), None); self.refresh(); return; }
                match self.ctl.apply_review() {
                Ok(_) if recopy => { self.start(Delivery::Copy); }
                Ok(n) if self.ctl.review.is_none() && resave => self.toast("warn", &tf("已遮挡 {} 处", &[&n]), t("保存的文件还是刚才那张图，请再保存一次"), None),
                Ok(n) => self.toast("done", &tf("已遮挡 {} 处", &[&n]), t("可以撤销；检查可能有遗漏"), None),
                Err(e) => self.toast("error", t("无法遮挡"), &messages::edit(&e), None),
                }
            }
            _ => if let (Some(i), Some(r)) = (c.strip_prefix("check-toggle-").and_then(|i| i.parse::<usize>().ok()), self.ctl.review.as_mut()) {
                if let Some(item) = r.items.get_mut(i) { item.on = !item.on; }
            },
        }
        self.refresh();
    }
    // ---- beautify (ADR 0.5-BEAUTIFY): a frame around the export, drawn by Rust ----
    /// The document's choices, or the remembered ones while it is unframed.
    fn frame_style(&self) -> beautify::Style {
        self.ctl.editor.as_ref().and_then(|e| e.frame()).map_or(self.settings.frame_style, beautify::Style::of)
    }
    fn set_frame(&mut self, on: bool, style: beautify::Style) {
        let Some(e) = self.ctl.editor.as_mut() else { return };
        match e.set_frame(on.then(|| style.frame(&self.frame_text))) {
            Ok(_) => if style != self.settings.frame_style { self.settings.frame_style = style; self.persist(); },
            Err(EditError::InvalidImageDimensions) => {
                // The switch goes off rather than leaving an older style on.
                let _ = e.set_frame(None);
                self.toast("warn", t("图片太大，无法美化"), t("美化后会超过 2400 万像素；复制和保存不受影响"), None);
            }
            Err(err) => self.toast("error", t("无法美化"), &messages::edit(&err), None),
        }
    }
    /// Panel state and chrome text go with their document.
    fn forget_frame(&mut self) {
        self.frame_open = false;
        self.frame_text.clear();
        *self.framed_cache.borrow_mut() = None;
    }
    fn frame_command(&mut self, c: &str) {
        let mut s = self.frame_style();
        let pick = |prefix: &str| c.strip_prefix(prefix).and_then(|i| i.parse::<usize>().ok());
        match c {
            "frame" if self.ctl.text_edit.is_some() && !self.frame_open => {
                self.toast("warn", t("请先完成文字编辑"), t("按 Enter 完成或 Esc 取消"), None);
            }
            "frame" => {
                self.frame_open = !self.frame_open && self.ctl.editor.is_some();
                if self.frame_open {
                    self.steps_open = false;
                    if self.ctl.review.is_some() { self.ctl.dismiss_review(); }
                    self.ctl.cancel_drag();
                    self.ctl.deselect();
                }
            }
            "frame-on" => self.set_frame(true, s),
            "frame-off" => self.set_frame(false, s),
            _ => {
                if let Some(i) = pick("frame-bg-") { s.background = i.min(beautify::AUTO); }
                else if let Some(i) = pick("frame-pad-") { s.padding = i.min(3); }
                else if let Some(i) = pick("frame-radius-") { s.radius = i.min(3); }
                else if let Some(i) = pick("frame-shadow-") { s.shadow = i.min(3); }
                else if let Some(i) = pick("frame-ratio-") { s.ratio = i.min(anshot_render::Ratio::ALL.len() - 1); }
                else if let Some(i) = pick("frame-chrome-") { s.chrome = i.min(3); }
                else { return; }
                // Picking any option means "beautify with this".
                self.set_frame(true, s);
            }
        }
        // Keys belong to the canvas again: a focused text box would take Ctrl+C for itself.
        if let Some(ui) = self.ui() { ui.invoke_focus_keys(); }
        self.refresh();
    }
    fn frame_text_edited(&mut self, text: &str) {
        self.frame_text = text.chars().filter(|c| !c.is_control()).take(200).collect();
        self.sync_frame_text();
        self.refresh();
    }
    /// Put the typed chrome text on the frame. While a copy or save runs the document cannot
    /// change: the text waits and goes on once that delivery is done.
    fn sync_frame_text(&mut self) {
        if !self.ctl.can_deliver() { return; }
        let stale = self.ctl.editor.as_ref().and_then(|e| e.frame()).is_some_and(|f| f.text != self.frame_text);
        if stale { let s = self.frame_style(); self.set_frame(true, s); }
    }
    /// The framed export at preview resolution (same renderer as the export), cached per revision.
    fn framed_preview(&self) -> Option<(Image, (f64, f64))> {
        let e = self.ctl.editor.as_ref()?;
        let frame = e.scene().frame?;
        let key = (self.ctl.generation(), e.stamp());
        let size = |img: &Image| (f64::from(img.size().width), f64::from(img.size().height));
        if let Some((k, img)) = self.framed_cache.borrow().as_ref() { if *k == key { return Some((img.clone(), size(img))); } }
        let (base, k) = self.base.as_ref()?;
        let mut img = base.clone();
        anshot_render::render_into(&mut img, &self.ctl.display_scene(), *k);
        if let Some(c) = e.crop() {
            let at = |v: i64| (v as f32 * k).round() as i64;
            let (l, t, r, b) = anshot_core::Region::new(at(c.x0), at(c.y0), at(c.x1), at(c.y1)).clipped(img.width(), img.height())?;
            img = image::imageops::crop_imm(&img, l, t, r - l, b - t).to_image();
        }
        let out = to_image(&anshot_render::render_framed_preview(&img, &frame, *k).ok()?);
        let dims = size(&out);
        *self.framed_cache.borrow_mut() = Some((key, out.clone()));
        Some((out, dims))
    }
    fn push_frame(&self, ui: &AppWindow) {
        let e = self.ctl.editor.as_ref();
        let s = self.frame_style();
        ui.set_frame_open(self.frame_open && e.is_some());
        ui.set_frame_on(e.is_some_and(|e| e.frame().is_some()));
        ui.set_frame_fits(e.is_none_or(|e| e.frame_fits()));
        ui.set_frame_bg(s.background as i32);
        ui.set_frame_padding(s.padding as i32);
        ui.set_frame_radius(s.radius as i32);
        ui.set_frame_shadow(s.shadow as i32);
        ui.set_frame_ratio(s.ratio as i32);
        ui.set_frame_chrome(s.chrome as i32);
        if ui.get_frame_text() != self.frame_text.as_str() { ui.set_frame_text(self.frame_text.as_str().into()); }
    }
    fn push_check(&self, ui: &AppWindow) {
        let review = self.ctl.review.as_ref();
        ui.set_check_state(if self.check_running { "busy" } else { match review { None => "", Some(r) if r.items.is_empty() => "empty", _ => "list" } }.into());
        let items = review.map_or(&[][..], |r| &r.items[..]);
        ui.set_check_replace(review.is_some_and(|r| r.replace));
        ui.set_check_recopy(review.is_some_and(|r| r.delivered == Some(Delivery::Copy)));
        ui.set_check_resave(review.is_some_and(|r| r.delivered == Some(Delivery::Save)));
        ui.set_check_history(review.is_some_and(|r| r.delivered.is_some()) && self.settings.history_limit > 0);
        let rows: Vec<CheckRow> = items.iter().map(|i| CheckRow { label: check::label(i).into(), hint: check::hint(&i.text).into(), on: i.on }).collect();
        ui.set_check_count(items.iter().filter(|i| i.on).count() as i32);
        ui.set_check_rows(ModelRc::new(VecModel::from(rows)));
        let boxes: Vec<MaskBox> = self.ctl.view().map_or(Vec::new(), |v| items.iter().map(|i| {
            let (x, y, w, h) = v.rect(i.region);
            MaskBox { x: x as f32, y: y as f32, w: w as f32, h: h as f32, selected: i.on }
        }).collect());
        ui.set_check_boxes(ModelRc::new(VecModel::from(boxes)));
    }

    // ---- text recognition (Windows' own OCR, offline) ----
    /// Recognise the text in `img` on a worker thread and show it in the OCR window.
    fn start_ocr(&mut self, img: RgbaImage) { self.run_ocr(move || recognize_text(img)); }
    /// Editor: recognise the image exactly as it would be exported (redactions applied).
    fn ocr_document(&mut self) {
        let Some((source, scene)) = self.ctl.ocr_snapshot() else { return };
        self.run_ocr(move || {
            let png = anshot_render::render_png(&source, &scene).map_err(|_| t("无法生成图片，请重试。").to_string())?;
            let img = anshot_core::decode_source(&png).map_err(|_| t("无法生成图片，请重试。").to_string())?;
            recognize_text(img)
        });
    }
    /// Show the OCR window as busy, run `job` on a worker thread, then show its text or error.
    fn run_ocr(&mut self, job: impl FnOnce() -> Result<String, String> + Send + 'static) {
        // One recognition at a time: repeated requests while it runs are ignored.
        if self.ocr_running { if let Some(w) = self.ocr_win.as_ref() { let _ = w.show(); } return; }
        self.ocr_running = true;
        self.ocr_seq += 1;
        let seq = self.ocr_seq;
        let win = match self.ocr_win.as_ref() {
            Some(w) => w.clone_strong(),
            None => {
                let Ok(w) = OcrWindow::new() else { return };
                w.on_copy_all(|text| { with(|a| match platform::write_text(text.as_str()) {
                    Ok(()) => a.system_toast("done", t("已复制文字"), &tf("{} 个字符", &[&text.chars().count()])),
                    Err(e) => a.system_toast("error", t("复制失败"), &e),
                }); });
                w.on_dismiss(|| { with(|a| if let Some(w) = a.ocr_win.as_ref() { let _ = w.hide(); }); });
                self.ocr_win = Some(w.clone_strong());
                w
            }
        };
        win.set_busy(true);
        win.set_error("".into());
        win.set_text("".into());
        let _ = win.show();
        std::thread::spawn(move || {
            let result = job();
            let _ = slint::invoke_from_event_loop(move || {
                with(|a| {
                    a.ocr_running = false;
                    if a.ocr_seq != seq { return; }
                    let Some(w) = a.ocr_win.as_ref() else { return };
                    w.set_busy(false);
                    match result {
                        Ok(text) if text.trim().is_empty() => w.set_error(t("没有识别到文字。").into()),
                        Ok(text) => w.set_text(text.into()),
                        Err(e) => w.set_error(e.into()),
                    }
                });
            });
        });
    }

    // ---- delayed capture ----
    fn start_countdown(&mut self) {
        if self.countdown.is_some() || self.session.is_some() || self.long.is_some() { return; }
        let Ok(win) = CountdownWindow::new() else { return };
        win.on_cancel(|| { with(|a| a.stop_countdown()); });
        let secs = self.settings.delay_secs;
        win.set_remaining(secs as i32);
        let focus = anshot_platform_windows::foreground_window();
        let _ = win.show();
        if let Some(m) = anshot_platform_windows::monitors().ok().and_then(|ms| ms.into_iter().find(|m| m.primary)) {
            let s = win.window().scale_factor();
            let (w, h) = ((220.0 * s) as i32, (56.0 * s) as i32);
            win.window().set_position(slint::PhysicalPosition::new(m.work.x + m.work.w - w - (16.0 * s) as i32, m.work.y + m.work.h - h - (16.0 * s) as i32));
        }
        // Never in the picture; never steals focus from what the user is setting up.
        if let Some(h) = hwnd_of(win.window()) { anshot_platform_windows::exclude_from_capture(h); }
        slint::Timer::single_shot(Duration::from_millis(60), move || anshot_platform_windows::restore_foreground(focus));
        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated, Duration::from_secs(1), || { with(|a| a.tick_countdown()); });
        self.countdown = Some((win, timer, secs));
    }
    fn tick_countdown(&mut self) {
        let Some((win, _, left)) = self.countdown.as_mut() else { return };
        *left = left.saturating_sub(1);
        win.set_remaining(*left as i32);
        if *left == 0 { self.stop_countdown(); self.begin_capture("delayed"); }
    }
    fn stop_countdown(&mut self) {
        if let Some((win, timer, _)) = self.countdown.take() { timer.stop(); let _ = win.hide(); }
    }

    // ---- capture (02 S1) ----
    fn begin_capture(&mut self, source: &'static str) {
        if self.session.is_some() || self.capture_pending || self.long.is_some() { return; }
        diag::start(source);
        // A capture started by hand replaces a delayed one that is still counting down.
        self.stop_countdown();
        if let Some(t) = self.toast_win.as_ref() { let _ = t.hide(); }
        // Never freeze our own editor into the shot: hide it, let the event loop apply the
        // hide, then wait for the compositor to present the desktop without it.
        let visible = self.ui().is_some_and(|ui| ui.window().is_visible());
        self.restore_editor = visible;
        // Pins and the OCR window stay visible to other programs (screen sharing), so they are
        // not excluded from capture; they step aside while Anshot freezes the screen instead.
        self.hidden_for_capture = self.pins.iter().filter(|p| p.win.window().is_visible()).map(|p| p.id).collect();
        for p in &self.pins { if self.hidden_for_capture.contains(&p.id) { let _ = p.win.hide(); } }
        self.ocr_hidden = self.ocr_win.as_ref().is_some_and(|w| w.window().is_visible());
        if let (true, Some(w)) = (self.ocr_hidden, self.ocr_win.as_ref()) { let _ = w.hide(); }
        if !visible && self.hidden_for_capture.is_empty() && !self.ocr_hidden { self.freeze_screen(); return; }
        if let Some(ui) = self.ui() { let _ = ui.hide(); }
        self.capture_pending = true;
        slint::Timer::single_shot(Duration::from_millis(16), || {
            anshot_platform_windows::wait_for_compositor();
            with(|a| { a.capture_pending = false; a.freeze_screen(); });
        });
    }
    fn freeze_screen(&mut self) {
        diag::mark("wait", None);
        let monitors = match anshot_platform_windows::monitors() {
            Ok(m) => m,
            Err(_) => { diag::outcome("error"); self.end_capture(); self.restore_editor = false; self.show_editor(); self.toast("error", t("无法截图"), t("无法读取显示器信息，请重试"), None); return; }
        };
        let Some(area) = capture::union(&monitors) else { diag::outcome("error"); self.cancel_capture(); return };
        let frozen = match anshot_platform_windows::capture_ex(area, self.settings.include_cursor) {
            Ok(c) => c,
            Err(_) => { diag::outcome("error"); self.end_capture(); self.restore_editor = false; self.show_editor(); self.toast("error", t("无法截图"), t("系统拒绝读取屏幕（锁屏或安全桌面时无法截图）"), None); return; }
        };
        diag::mark("grab", Some(format!("{}x{}", area.w, area.h)));
        let session = capture::Session::new(frozen, monitors.clone(), anshot_platform_windows::visible_windows());
        let mut overlays = Vec::new();
        // The overlay under the pointer takes the keyboard (again once it is revealed).
        let focus_index = anshot_platform_windows::cursor().and_then(|(x, y)| monitors.iter().position(|m| m.bounds.contains(x, y))).unwrap_or(0);
        let pad = u32::from(diag::switch("ANSHOT_DIAG_OVERLAY_PAD"));
        for (i, m) in monitors.iter().copied().enumerate() {
            // Overlay i must serve monitor i; on any failure abort the capture cleanly.
            let Some(pixels) = session.crop(m.bounds) else { diag::outcome("error"); self.end_capture(); self.restore_editor = false; self.show_editor(); self.toast("error", t("无法截图"), t("显示器区域无效"), None); return; };
            if self.overlay_pool.len() <= i {
                let Some(win) = new_overlay(i) else { diag::outcome("error"); self.end_capture(); self.restore_editor = false; self.show_editor(); self.toast("error", t("无法截图"), t("无法创建截图窗口"), None); return; };
                self.overlay_pool.push(win);
            }
            let win = self.overlay_pool[i].clone_strong();
            win.set_scrim(!diag::switch("ANSHOT_DIAG_NO_SCRIM"));
            win.set_burst(self.burst);
            win.set_shot(to_image(&pixels));
            diag::mark("prepared", Some(format!("monitor{i}")));
            win.set_size_open(false);
            let place = |w: &CaptureOverlay| {
                w.window().set_position(slint::PhysicalPosition::new(m.bounds.x, m.bounds.y));
                w.window().set_size(slint::PhysicalSize::new(m.bounds.w as u32, m.bounds.h as u32 + pad));
            };
            place(&win);
            // Shown cloaked where possible: the screen never shows the overlay before its picture.
            let cloaked = reveal::hide_until_drawn(i, &win, hwnd_of(win.window()), i == focus_index);
            let _ = win.show();
            place(&win); // re-apply after a possible per-monitor DPI change on show
            if !cloaked { reveal::reveal_now(&win); }
            diag::mark("shown", Some(format!("monitor{i} {}x{} scale={:.2}", m.bounds.w, m.bounds.h, win.window().scale_factor())));
            overlays.push((win, m));
        }
        if overlays.is_empty() { self.cancel_capture(); return; }
        reveal::arm_fallback();
        // Keyboard input (Esc, Enter, R, Shift+C) must reach the overlay before any click:
        // bring the one under the pointer to the front (allowed right after our hotkey).
        let under = anshot_platform_windows::cursor().and_then(|(x, y)| overlays.iter().find(|(_, m)| m.bounds.contains(x, y)));
        if let Some(h) = under.or(overlays.first()).and_then(|(w, _)| hwnd_of(w.window())) { anshot_platform_windows::restore_foreground(h); }
        self.session = Some(session);
        self.overlays = overlays;
        self.last_cursor = anshot_platform_windows::cursor();
        if let Some((x, y)) = self.last_cursor {
            if let Some(s) = self.session.as_mut() { s.pointer_move(x, y); }
        }
        self.refresh_overlays();
        self.cursor_timer.start(slint::TimerMode::Repeated, Duration::from_millis(30), || { with(|a| a.poll_cursor()); });
    }
    fn poll_cursor(&mut self) {
        let now = anshot_platform_windows::cursor();
        if now == self.last_cursor { return; }
        self.last_cursor = now;
        if let (Some((x, y)), Some(s)) = (now, self.session.as_mut()) { if !s.pressed() { s.pointer_move(x, y); } }
        self.refresh_overlays();
    }
    /// Overlay toolbar / shortcuts: copy, save, edit, cancel.
    fn overlay_action(&mut self, action: &str) {
        match action {
            "pin" => self.pin_selection(),
            "step" => self.step_selection(),
            "ocr" => {
                let Some(s) = self.session.as_ref() else { return };
                diag::outcome("ocr");
                let area = s.confirm_area();
                let pixels = area.and_then(|r| s.crop(r));
                if area.is_some() { self.last_region = area; }
                self.end_capture();
                if std::mem::take(&mut self.restore_editor) { self.show_editor(); }
                if let Some(img) = pixels { self.start_ocr(img); }
            }
            "long" => self.start_long_shot(),
            "copy" => self.finish_capture(Some(Delivery::Copy)),
            "save" => self.finish_capture(Some(Delivery::Save)),
            "edit" => self.finish_capture(None),
            "cancel" => self.cancel_capture(),
            "repeat" => {
                if let (Some(r), Some(s)) = (self.last_region, self.session.as_mut()) { s.restore(r); }
                self.refresh_overlays();
            }
            "color" => {
                let px = self.last_cursor.and_then(|(x, y)| self.session.as_ref().and_then(|s| s.pixel(x, y)));
                if let Some([r, g, b, _]) = px {
                    let hex = format!("#{r:02X}{g:02X}{b:02X}");
                    let msg = match platform::write_text(&hex) { Ok(()) => tf("已复制颜色 {}", &[&hex]), Err(e) => e };
                    self.overlay_notice(&msg);
                }
            }
            other => {
                // "ratio:W:H" or "size:WxH" from the selection toolbar's size panel.
                let Some(s) = self.session.as_mut() else { return };
                let num = |t: &str| t.trim().parse::<i32>().ok();
                let ok = if let Some(r) = other.strip_prefix("ratio:") {
                    let mut it = r.split(':');
                    matches!((it.next().and_then(num), it.next().and_then(num)), (Some(w), Some(h)) if s.set_ratio(w, h))
                } else if let Some(r) = other.strip_prefix("size:") {
                    let mut it = r.split(['x', '×', 'X', '*']);
                    matches!((it.next().and_then(num), it.next().and_then(num)), (Some(w), Some(h)) if s.resize_to(w, h))
                } else { return };
                if !ok { self.overlay_notice(t("尺寸无效：请填写 1 以上的整数宽高")); }
                self.refresh_overlays();
            }
        }
    }
    /// A short message in the overlay's hint bar (cleared after 1.5 s).
    fn overlay_notice(&mut self, msg: &str) {
        self.notice_seq += 1;
        let seq = self.notice_seq;
        for (win, _) in &self.overlays { win.set_notice(msg.into()); }
        slint::Timer::single_shot(Duration::from_millis(1500), move || {
            with(|a| if a.notice_seq == seq { for (w, _) in &a.overlays { w.set_notice("".into()); } });
        });
    }
    /// Create and show/hide each monitor's overlay once, off-screen, so the first
    /// hotkey press does not pay window + GL context creation.
    fn prewarm_overlays(&mut self) {
        let Ok(monitors) = anshot_platform_windows::monitors() else { return };
        for (i, _) in monitors.iter().enumerate() {
            if self.overlay_pool.len() > i { continue; }
            let Some(win) = new_overlay(i) else { return };
            win.set_show_hint(false);
            win.window().set_position(slint::PhysicalPosition::new(-32000, -32000));
            win.window().set_size(slint::PhysicalSize::new(8, 8));
            let _ = win.show();
            let w = win.clone_strong();
            // Never hide an overlay that a capture started using in the meantime.
            slint::Timer::single_shot(Duration::from_millis(300), move || { with(|a| if a.session.is_none() { let _ = w.hide(); }); });
            self.overlay_pool.push(win);
        }
    }
    fn overlay_pointer(&mut self, i: usize, lx: f32, ly: f32, phase: Phase) {
        let Some((win, m)) = self.overlays.get(i) else { return };
        let k = win.window().scale_factor();
        let (x, y) = (m.bounds.x + (lx * k).round() as i32, m.bounds.y + (ly * k).round() as i32);
        let Some(s) = self.session.as_mut() else { return };
        s.lock = anshot_platform_windows::shift_down();
        match phase { Phase::Down => s.pointer_down(x, y), Phase::Move => s.pointer_move(x, y), Phase::Up => s.pointer_up(x, y) }
        self.refresh_overlays();
    }
    fn refresh_overlays(&self) {
        let Some(s) = self.session.as_ref() else { return };
        let cursor = anshot_platform_windows::cursor();
        for (win, m) in &self.overlays {
            let k = win.window().scale_factor();
            let local = |r: ScreenRect| r.intersect(m.bounds).map(|r| Box {
                x: (r.x - m.bounds.x) as f32 / k, y: (r.y - m.bounds.y) as f32 / k, w: r.w as f32 / k, h: r.h as f32 / k,
            });
            let sel = s.selection.and_then(local);
            let has_sel = sel.is_some();
            win.set_has_selection(sel.is_some());
            if let Some(b) = sel { win.set_selection(b); }
            win.set_size_text(s.selection.map(|r| format!("{} × {}", r.w, r.h)).unwrap_or_default().into());
            let hover = if s.selection.is_none() { s.hover.and_then(local) } else { None };
            win.set_has_hover(hover.is_some());
            if let Some(b) = hover { win.set_hover(b); }
            match cursor.filter(|&(x, y)| m.bounds.contains(x, y)) {
                Some((x, y)) => {
                    let (px, py) = (x - m.bounds.x, y - m.bounds.y);
                    win.set_cursor_x(px as f32 / k);
                    win.set_cursor_y(py as f32 / k);
                    win.set_mag_x((px - 7).clamp(0, (m.bounds.w - 15).max(0)));
                    win.set_mag_y((py - 7).clamp(0, (m.bounds.h - 15).max(0)));
                    let hex = s.pixel(x, y).map(|[r, g, b, _]| format!("  #{r:02X}{g:02X}{b:02X}")).unwrap_or_default();
                    win.set_mag_text(format!("{x}, {y}{hex}").into());
                    win.set_show_magnifier(true);
                }
                None => win.set_show_magnifier(false),
            }
            win.set_show_hint(m.bounds.contains(cursor.map_or(0, |c| c.0), cursor.map_or(0, |c| c.1)) || m.primary);
            win.set_cursor_kind(cursor_kind(s, cursor).into());
            win.set_show_toolbar(has_sel && !s.pressed());
            win.set_has_last(self.last_region.is_some());
        }
    }
    fn end_capture(&mut self) {
        self.cursor_timer.stop();
        let hidden = (std::mem::take(&mut self.hidden_for_capture), std::mem::take(&mut self.ocr_hidden));
        self.show_hidden(hidden);
        let mut still_visible = 0;
        for (win, _) in self.overlays.drain(..) {
            let _ = win.hide();
            // The OS window, not Slint's own flag: a window that stays on screen must show here.
            if diag::enabled() && hwnd_of(win.window()).is_some_and(anshot_platform_windows::window_visible) { still_visible += 1; }
            // Do not keep frozen screen pixels around between captures.
            win.set_shot(Image::default());
            win.set_has_selection(false);
            win.set_has_hover(false);
        }
        self.session = None;
        // After hiding: uncloaking a hidden overlay shows nothing.
        reveal::finish();
        diag::end(still_visible);
        self.hdr_notice();
    }
    /// Once per run, after a capture: a display in HDR is where captures went black or left
    /// the screen dim (Windows 11 24H2 with an AMD 780M; MEASURED by the user). Asked of the
    /// system only until it has been said.
    fn hdr_notice(&mut self) {
        if !self.settings.hdr_notice || self.hdr_noticed { return; }
        if !anshot_platform_windows::display_facts().iter().any(|d| d.hdr == Some(true)) { return; }
        self.hdr_noticed = true;
        slint::Timer::single_shot(Duration::from_millis(400), || { with(|a| {
            a.toast("warn", t("显示器开着 HDR"), t("截图时可能黑屏或之后偏暗 · Win+Alt+B 可关闭 HDR"), Some((t("不再提示"), ToastAction::MuteHdr)));
        }); });
    }
    /// Esc / right click: put back whatever was on screen before the hotkey.
    fn cancel_capture(&mut self) {
        diag::outcome("cancel");
        self.end_capture();
        if std::mem::take(&mut self.restore_editor) { self.show_editor(); }
    }
    fn confirm_capture(&mut self) { if self.burst { self.step_selection(); } else { self.finish_capture(None); } }
    /// Overlay "pin": the selected screen pixels become a pin at the same place.
    fn pin_selection(&mut self) {
        let Some(s) = self.session.as_ref() else { return };
        diag::outcome("pin");
        let area = s.confirm_area();
        let pixels = area.and_then(|r| s.crop(r));
        if area.is_some() { self.last_region = area; }
        self.end_capture();
        if std::mem::take(&mut self.restore_editor) { self.show_editor(); }
        if let (Some(img), Some(r)) = (pixels, area) { self.open_pin(img, Some((r.x, r.y))); }
    }

    // ---- long screenshot ----
    /// Leave the frozen overlay, keep the selected region, and stitch live captures of it
    /// while the user scrolls the real window underneath.
    fn start_long_shot(&mut self) {
        diag::outcome("long");
        if self.long.is_some() { return; }
        let Some(s) = self.session.as_ref() else { return };
        let Some(region) = s.selection else { return };
        self.last_region = Some(region);
        let Some(mon) = s.monitor_at(region.x + region.w / 2, region.y + region.h / 2) else { return };
        // The panel is sized with the scale of the monitor it will appear on.
        let k = self.overlays.iter().find(|(_, m)| m.bounds == mon.bounds).or(self.overlays.first()).map_or(1.0, |(w, _)| w.window().scale_factor());
        if u64::from(region.w.unsigned_abs()) * u64::from(region.h.unsigned_abs()) > anshot_core::MAX_PIXELS / 2 {
            self.end_capture();
            if std::mem::take(&mut self.restore_editor) { self.show_editor(); }
            self.system_toast("warn", t("选区太大，无法长截图"), t("请框选要滚动的那一部分"));
            return;
        }
        let (pw, ph) = ((300.0 * k) as i32, (96.0 * k) as i32);
        let gap = (12.0 * k) as i32;
        let fits = |x: i32, y: i32| x >= mon.bounds.x && y >= mon.bounds.y && x + pw <= mon.bounds.x + mon.bounds.w && y + ph <= mon.bounds.y + mon.bounds.h;
        let spot = [
            (region.x + region.w + gap, region.y),
            (region.x - gap - pw, region.y),
            (region.x + region.w - pw, region.y + region.h + gap),
            (region.x + region.w - pw, region.y - gap - ph),
        ].into_iter().find(|&(x, y)| fits(x, y));
        // Live frames follow: windows hidden for the freeze stay hidden until the long shot ends.
        let hidden = (std::mem::take(&mut self.hidden_for_capture), std::mem::take(&mut self.ocr_hidden));
        self.end_capture();
        let Some((px, py)) = spot else {
            self.show_hidden(hidden);
            if std::mem::take(&mut self.restore_editor) { self.show_editor(); }
            self.system_toast("warn", t("无法开始长截图"), t("请把选区缩小一些，给控制条留出位置"));
            return;
        };
        let Ok(panel) = LongShotPanel::new() else { return };
        panel.on_finish(|| { with(|a| a.finish_long_shot()); });
        panel.on_cancel(|| { with(|a| a.cancel_long_shot()); });
        panel.window().set_position(slint::PhysicalPosition::new(px, py));
        let _ = panel.show();
        panel.window().set_position(slint::PhysicalPosition::new(px, py));
        if let Some(h) = hwnd_of(panel.window()) { anshot_platform_windows::exclude_from_capture(h); }
        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated, Duration::from_millis(120), || { with(|a| a.poll_long_shot()); });
        self.long = Some(LongShot { hidden, region, stitcher: None, panel, timer });
    }
    fn poll_long_shot(&mut self) {
        let Some(l) = self.long.as_mut() else { return };
        let Ok(cap) = anshot_platform_windows::capture(l.region) else { return };
        let Some(frame) = RgbaImage::from_raw(cap.area.w as u32, cap.area.h as u32, cap.rgba) else { return };
        let state = match l.stitcher.as_mut() {
            None => { l.stitcher = Some(stitch::Stitcher::new(frame, anshot_core::MAX_PIXELS)); "" }
            Some(s) => match s.push(frame) { stitch::Push::Lost => "lost", stitch::Push::Full => "full", _ => "" },
        };
        let h = l.stitcher.as_ref().map_or(0, |s| s.height());
        l.panel.set_stitched_height(h as i32);
        l.panel.set_narrowed(l.stitcher.as_ref().is_some_and(|s| s.narrowed().is_some()));
        l.panel.set_state(state.into());
        if state == "full" { l.timer.stop(); }
    }
    fn close_long_shot(&mut self) -> Option<LongShot> {
        let mut l = self.long.take()?;
        l.timer.stop();
        let _ = l.panel.hide();
        self.show_hidden(std::mem::take(&mut l.hidden));
        Some(l)
    }
    fn show_hidden(&mut self, (pins, ocr): (Vec<u64>, bool)) {
        for id in pins { if let Some(p) = self.pins.iter().find(|p| p.id == id) { let _ = p.win.show(); } }
        if ocr { if let Some(w) = self.ocr_win.as_ref() { let _ = w.show(); } }
    }
    fn cancel_long_shot(&mut self) {
        self.close_long_shot();
        if std::mem::take(&mut self.restore_editor) { self.show_editor(); }
    }
    fn finish_long_shot(&mut self) {
        let Some(l) = self.close_long_shot() else { return };
        let Some(img) = l.stitcher.and_then(stitch::Stitcher::finish) else { self.cancel_long_shot(); return };
        self.restore_editor = false;
        self.show_editor();
        // Encode off the UI thread; very tall pages are trimmed to the 25 MB intake limit.
        std::thread::spawn(move || {
            let mut img = img;
            let full_h = img.height();
            let mut png = capture::encode_png(img.clone());
            for _ in 0..4 {
                match &png {
                    Ok(b) if b.len() > anshot_core::MAX_INPUT_BYTES => {
                        let keep = (u64::from(img.height()) * anshot_core::MAX_INPUT_BYTES as u64 * 9 / 10 / b.len() as u64).max(1) as u32;
                        img = image::imageops::crop_imm(&img, 0, 0, img.width(), keep).to_image();
                        png = capture::encode_png(img.clone());
                    }
                    _ => break,
                }
            }
            let (w, h) = img.dimensions();
            let _ = slint::invoke_from_event_loop(move || {
                if h < full_h { with(|a| a.toast("warn", t("长截图已截短"), &tf("超过 25 MB 上限，保留前 {} 像素高", &[&h]), None)); }
                spawn_open(png, Some(tf("长截图 {}×{}.png", &[&w, &h])));
            });
        });
    }

    // ---- pins ----
    fn open_pin(&mut self, img: RgbaImage, at: Option<(i32, i32)>) {
        let Ok(win) = PinWindow::new() else { self.toast("error", t("无法钉图"), t("无法创建窗口"), None); return };
        let id = self.next_pin;
        self.next_pin += 1;
        win.set_img(to_image(&img));
        win.on_drag_start(move || { with(|a| a.pin_drag_start(id)); });
        win.on_drag_move(move || { with(|a| a.pin_drag_move(id)); });
        win.on_zoom(move |dy| { with(|a| a.pin_zoom(id, dy)); });
        win.on_act(move |k| { let k = k.to_string(); slint::Timer::single_shot(Duration::ZERO, move || pin_action(id, &k)); });
        let (x, y) = at.unwrap_or_else(|| {
            let c = anshot_platform_windows::cursor().unwrap_or((200, 200));
            (c.0 - img.width() as i32 / 2, c.1 - img.height() as i32 / 2)
        });
        win.window().set_position(slint::PhysicalPosition::new(x, y));
        win.window().set_size(slint::PhysicalSize::new(img.width().max(24), img.height().max(24)));
        let _ = win.show();
        win.window().set_size(slint::PhysicalSize::new(img.width().max(24), img.height().max(24)));
        self.pins.push(Pin { id, win, img, zoom: 1.0 });
    }
    fn pin(&self, id: u64) -> Option<&Pin> { self.pins.iter().find(|p| p.id == id) }
    fn pin_drag_start(&mut self, id: u64) {
        let (Some(p), Some(c)) = (self.pin(id), anshot_platform_windows::cursor()) else { return };
        let pos = p.win.window().position();
        self.pin_drag = Some((id, c, (pos.x, pos.y)));
    }
    fn pin_drag_move(&mut self, id: u64) {
        let (Some((pid, c0, w0)), Some(c)) = (self.pin_drag, anshot_platform_windows::cursor()) else { return };
        if pid != id { return; }
        if let Some(p) = self.pin(id) { p.win.window().set_position(slint::PhysicalPosition::new(w0.0 + c.0 - c0.0, w0.1 + c.1 - c0.1)); }
    }
    fn pin_zoom(&mut self, id: u64, dy: f32) {
        let Some(p) = self.pins.iter_mut().find(|p| p.id == id) else { return };
        p.zoom = (p.zoom * if dy < 0.0 { 1.1 } else { 1.0 / 1.1 }).clamp(0.1, 4.0);
        let (w, h) = ((p.img.width() as f32 * p.zoom).round().max(24.0) as u32, (p.img.height() as f32 * p.zoom).round().max(24.0) as u32);
        p.win.window().set_size(slint::PhysicalSize::new(w, h));
        p.win.set_zoom_text(format!("{}%", (p.zoom * 100.0).round()).into());
    }
    fn close_pin(&mut self, id: u64) {
        if let Some(i) = self.pins.iter().position(|p| p.id == id) { let p = self.pins.remove(i); let _ = p.win.hide(); }
    }
    /// Editor "pin": render the current document (redactions, annotations, crop) in Rust off-thread.
    fn pin_document(&mut self) {
        let Some(e) = self.ctl.editor.as_ref() else { return };
        let Some(src) = e.source_bytes().map(<[u8]>::to_vec) else { return };
        let scene = anshot_render::Scene { frame: None, ..e.scene() };
        std::thread::spawn(move || {
            let img = anshot_render::render_png(&src, &scene).ok().and_then(|png| anshot_core::decode_source(&png).ok());
            let _ = slint::invoke_from_event_loop(move || { with(|a| match img {
                Some(img) => a.open_pin(img, None),
                None => a.toast("error", t("无法钉图"), t("生成图片失败"), None),
            }); });
        });
    }

    // ---- step image (ADR 0.6-STEPS): memory only, never written to disk ----
    /// Overlay "add step": the selected screen pixels, unedited, go to the step panel.
    fn step_selection(&mut self) {
        let Some(s) = self.session.as_ref() else { return };
        diag::outcome("step");
        let area = s.confirm_area();
        let pixels = area.and_then(|r| s.crop(r));
        if area.is_some() { self.last_region = area; }
        self.end_capture();
        if std::mem::take(&mut self.restore_editor) { self.show_editor(); }
        if let Some(img) = pixels { self.add_step(img, true, None); }
    }
    /// Editor Ctrl+J: the document as it would be exported (no frame) becomes a step, or
    /// replaces the step being edited.
    fn step_document(&mut self) {
        if self.ctl.editor.is_none() { return; }
        if self.ctl.text_edit.is_some() { self.toast("warn", t("请先完成文字编辑"), t("按 Enter 完成或 Esc 取消"), None); return; }
        let replace = self.editing_step.filter(|id| self.steps.position(*id).is_some());
        if replace.is_none() && !self.step_room() { return; }
        let Some(e) = self.ctl.editor.as_ref() else { return };
        let Some(src) = e.source_bytes().map(<[u8]>::to_vec) else { return };
        let scene = anshot_render::Scene { frame: None, ..e.scene() };
        std::thread::spawn(move || {
            let img = anshot_render::render_png(&src, &scene).ok().and_then(|png| anshot_core::decode_source(&png).ok());
            let _ = slint::invoke_from_event_loop(move || { with(|a| match img {
                Some(img) => a.add_step(img, false, replace),
                None => a.toast("error", t("无法加入步骤"), t("生成图片失败"), None),
            }); });
        });
    }
    /// Room for one more step (counting those still being prepared); says so when full.
    fn step_room(&mut self) -> bool {
        if self.steps.len() + self.steps_adding < anshot_render::MAX_STEPS { return true; }
        self.toast("warn", t("最多 20 步"), t("先拼合或删除一些步骤"), None);
        false
    }
    fn add_step(&mut self, img: RgbaImage, raw: bool, replace: Option<u64>) {
        if replace.is_none() && !self.step_room() { return; }
        self.steps_adding += 1;
        std::thread::spawn(move || {
            let data = steps::prepare(img, raw);
            let _ = slint::invoke_from_event_loop(move || { with(|a| a.step_ready(data, replace)); });
        });
    }
    fn step_ready(&mut self, data: Result<steps::StepData, String>, replace: Option<u64>) {
        self.steps_adding = self.steps_adding.saturating_sub(1);
        let data = match data { Ok(d) => d, Err(e) => { self.toast("error", t("无法加入步骤"), &e, None); return; } };
        match replace.filter(|id| self.steps.position(*id).is_some()) {
            Some(id) => {
                let n = self.steps.replace(id, data).unwrap_or(0);
                self.toast("done", &tf("已更新第 {} 步", &[&n]), &tf("共 {} 步", &[&self.steps.len()]), None);
            }
            None => match self.steps.add(data) {
                Ok(n) => {
                    let action = (!self.steps_open).then(|| (t("打开步骤栏"), ToastAction::OpenSteps));
                    self.toast("done", &tf("已加入第 {} 步", &[&n]), &tf("共 {} 步", &[&self.steps.len()]), action);
                }
                Err(steps::Full) => self.toast("warn", t("最多 20 步"), t("先拼合或删除一些步骤"), None),
            },
        }
        self.push_steps();
        self.request_steps_preview();
        self.refresh();
    }
    fn open_steps(&mut self, open: bool) {
        if open {
            if self.ctl.text_edit.is_some() { self.toast("warn", t("请先完成文字编辑"), t("按 Enter 完成或 Esc 取消"), None); return; }
            self.frame_open = false;
            if self.ctl.review.is_some() { self.ctl.dismiss_review(); }
            self.ctl.cancel_drag();
        }
        self.steps_open = open;
        self.push_steps();
        self.request_steps_preview();
        if let Some(ui) = self.ui() { ui.invoke_focus_keys(); }
    }
    fn set_burst(&mut self, on: bool) {
        if self.burst == on { return; }
        self.burst = on;
        if on {
            self.toast("done", t("连拍已打开"), t("截图选好区域后按 Enter 直接加入步骤"), None);
        } else if !self.steps.is_empty() {
            self.show_editor();
            self.open_steps(true);
        } else {
            self.toast("done", t("连拍已关闭"), "", None);
        }
        self.push_steps_state();
    }
    /// Open one step as a document; Ctrl+J then puts the result back in its place.
    fn edit_step(&mut self, id: u64) {
        let (Some(i), Some(s)) = (self.steps.position(id), self.steps.get(id)) else { return };
        let png = s.data.png.to_vec();
        self.pending_step_edit = Some(id);
        let n = i + 1;
        // Leave the state borrow before starting the shared intake pipeline.
        slint::Timer::single_shot(Duration::ZERO, move || spawn_open(Ok(png), Some(tf("第 {} 步.png", &[&n]))));
    }
    fn compose_steps(&mut self) {
        if self.steps.is_empty() || self.steps_busy { return; }
        self.steps_busy = true;
        self.push_steps_state();
        let (input, style, n) = (self.steps.compose_input(), self.steps.style, self.steps.len());
        std::thread::spawn(move || {
            let png = steps::compose(&input, &style).and_then(capture::encode_png);
            let _ = slint::invoke_from_event_loop(move || {
                let ok = with(|a| {
                    a.steps_busy = false;
                    a.push_steps_state();
                    match &png {
                        Ok(_) => { a.pending_step_edit = None; true }
                        Err(e) if e.contains("too large") => { a.toast("warn", t("图片太大，无法拼合"), t("拼合后会超过 2400 万像素；可减少步骤或改用网格"), None); false }
                        Err(e) => { a.toast("error", t("无法拼合"), e, None); false }
                    }
                }).unwrap_or(false);
                if let (true, Ok(png)) = (ok, png) { spawn_open(Ok(png), Some(tf("步骤图 {} 步.png", &[&n]))); }
            });
        });
    }
    fn steps_command(&mut self, c: &str) {
        let id = |p: &str| c.strip_prefix(p).and_then(|v| v.parse::<u64>().ok());
        let index = |p: &str| c.strip_prefix(p).and_then(|v| v.parse::<usize>().ok());
        let mut style = self.steps.style;
        let mut rows = false;
        match c {
            "steps" => { let open = !self.steps_open; self.open_steps(open); return; }
            "step-add" => { self.step_document(); return; }
            "steps-compose" => { self.compose_steps(); return; }
            "steps-burst-on" => { self.set_burst(true); return; }
            "steps-burst-off" => { self.set_burst(false); return; }
            "steps-undo" => { rows = self.steps.undo_remove().is_some(); }
            "steps-clear" => { self.steps.clear(); self.editing_step = None; rows = true; }
            "steps-badges-on" => style.badges = true,
            "steps-badges-off" => style.badges = false,
            "steps-captions-on" => style.captions = true,
            "steps-captions-off" => style.captions = false,
            "steps-uniform-on" => style.uniform = true,
            "steps-uniform-off" => style.uniform = false,
            _ => {
                if let Some(i) = id("steps-up-") { rows = self.steps.move_by(i, -1); }
                else if let Some(i) = id("steps-down-") { rows = self.steps.move_by(i, 1); }
                else if let Some(i) = id("steps-edit-") { self.edit_step(i); return; }
                else if let Some(i) = id("steps-del-") {
                    if let Some(n) = self.steps.remove(i) {
                        if self.editing_step == Some(i) { self.editing_step = None; }
                        self.toast("done", &tf("已删除第 {} 步", &[&n]), "", Some((t("撤销"), ToastAction::UndoStep)));
                        rows = true;
                    }
                }
                else if let Some(i) = index("steps-arr-") { style.arrangement = anshot_render::Arrangement::ALL[i.min(2)]; }
                else if let Some(i) = index("steps-gap-") { style.gap = anshot_render::Gap::ALL[i.min(2)]; }
                else if let Some(i) = index("steps-bg-") { style.backdrop = anshot_render::Backdrop::ALL[i.min(2)]; }
                else { return; }
            }
        }
        if style != self.steps.style {
            self.steps.style = style;
            self.settings.steps_style = style;
            self.persist();
        }
        if rows { self.push_steps(); } else { self.push_steps_state(); }
        self.request_steps_preview();
        if let Some(ui) = self.ui() { ui.invoke_focus_keys(); }
    }
    fn step_caption_edited(&mut self, id: u64, text: &str) {
        if self.steps.set_caption(id, text) { self.request_steps_preview(); }
    }
    /// Panel rows (thumbnails, captions). Rebuilt only when the steps change: rebuilding
    /// while a caption is typed would reset its text box.
    fn push_steps(&self) {
        let Some(ui) = self.ui() else { return };
        let rows: Vec<StepRow> = self.steps.items().iter().map(|s| StepRow {
            id: s.id as i32, thumb: to_image(&s.data.thumb), caption: s.caption.as_str().into(), raw: s.data.raw,
        }).collect();
        ui.set_step_rows(ModelRc::new(VecModel::from(rows)));
        self.push_steps_state();
    }
    fn push_steps_state(&self) {
        if let Some(t) = self.tray.as_ref() { t.set_steps_count(self.steps.len() as i32); t.set_burst(self.burst); }
        let Some(ui) = self.ui() else { return };
        let s = self.steps.style;
        ui.set_steps_open(self.steps_open);
        ui.set_burst(self.burst);
        ui.set_steps_arr(anshot_render::Arrangement::ALL.iter().position(|x| *x == s.arrangement).unwrap_or(0) as i32);
        ui.set_steps_gap(anshot_render::Gap::ALL.iter().position(|x| *x == s.gap).unwrap_or(1) as i32);
        ui.set_steps_bg(anshot_render::Backdrop::ALL.iter().position(|x| *x == s.backdrop).unwrap_or(0) as i32);
        ui.set_steps_badges(s.badges);
        ui.set_steps_captions(s.captions);
        ui.set_steps_uniform(s.uniform);
        ui.set_steps_busy(self.steps_busy);
        ui.set_steps_can_undo(self.steps.can_undo());
        ui.set_steps_any_raw(self.steps.any_raw());
        ui.set_editing_step(self.editing_step.and_then(|id| self.steps.position(id)).map_or(0, |i| i as i32 + 1));
    }
    /// Canvas room left of the panel.
    fn steps_room(&self) -> (f64, f64) {
        let (aw, ah) = self.ctl.area();
        (if aw > 700.0 { aw - 324.0 } else { aw }, ah)
    }
    /// Draw the joined image at the size it is shown, off the UI thread; only the newest
    /// request is shown.
    fn request_steps_preview(&mut self) {
        self.steps_preview_gen += 1;
        let hide = |a: &Self| if let Some(ui) = a.ui() { ui.set_steps_has_preview(false); };
        if !self.steps_open || self.steps.is_empty() { hide(self); return; }
        let generation = self.steps_preview_gen;
        let style = self.steps.style;
        let (sizes, captions) = self.steps.layout_input();
        let captions: Vec<&str> = captions.iter().map(String::as_str).collect();
        let Ok(layout) = anshot_render::steps_layout(&sizes, &captions, &style) else { hide(self); return };
        let (aw, ah) = self.steps_room();
        let (cw, ch) = (f64::from(layout.canvas.0), f64::from(layout.canvas.1));
        let k = ((aw - 48.0) / cw).min((ah - 48.0) / ch).clamp(0.01, 1.0) as f32;
        let input = self.steps.preview_input();
        std::thread::spawn(move || {
            let img = steps::preview(&input, &style, k).ok();
            let _ = slint::invoke_from_event_loop(move || { with(|a| a.steps_preview_ready(generation, img)); });
        });
    }
    fn steps_preview_ready(&mut self, generation: u64, img: Option<RgbaImage>) {
        if generation != self.steps_preview_gen { return; }
        let Some(ui) = self.ui() else { return };
        let Some(img) = img else { ui.set_steps_has_preview(false); return };
        let (aw, ah) = self.steps_room();
        let (pw, ph) = (f64::from(img.width()), f64::from(img.height()));
        ui.set_steps_x(((aw - pw) / 2.0).max(0.0) as f32);
        ui.set_steps_y(((ah - ph) / 2.0).max(0.0) as f32);
        ui.set_steps_w(pw as f32);
        ui.set_steps_h(ph as f32);
        ui.set_steps_preview(to_image(&img));
        ui.set_steps_has_preview(true);
    }

    // ---- history ----
    fn history(&self) -> Option<history::History> { history::History::default_dir().map(history::History::at) }
    fn record_history(&mut self, png: &[u8]) {
        let limit = self.settings.history_limit;
        if limit == 0 { return; }
        if let Some(h) = self.history() {
            let _ = h.add(png, limit);
            // The tray app can run for weeks: apply the age limit here too, not only at start.
            h.prune_age(self.settings.history_days, history::now_millis());
        }
    }
    fn open_history(&mut self) {
        self.show_editor();
        if self.ui().is_some_and(|ui| !ui.get_dialog().is_empty() && ui.get_dialog() != "history") { return; }
        let Some(ui) = self.ui() else { return };
        ui.set_history_items(ModelRc::default());
        ui.set_history_loading(true);
        ui.set_dialog("history".into());
        let entries = self.history().map(|h| h.list()).unwrap_or_default();
        self.history_shown = Vec::new(); // nothing is actionable until the matching thumbnails arrive
        self.history_gen += 1;
        let generation = self.history_gen;
        let now = history::now_millis();
        // Thumbnails decode through Core off the UI thread; files are untrusted input.
        std::thread::spawn(move || {
            let thumbs: Vec<(Option<RgbaImage>, String)> = entries.iter().map(|e| {
                let img = platform::read_image_file(&e.path).ok().and_then(|b| anshot_core::decode_source(&b).ok())
                    .map(|i| image::imageops::thumbnail(&i, 168.min(i.width()), (168 * i.height() / i.width().max(1)).clamp(1, 112)));
                (img, history::label(e.millis, now))
            }).collect();
            let _ = slint::invoke_from_event_loop(move || { with(|a| {
                if a.history_gen != generation { return; }
                a.history_shown = entries;
                let Some(ui) = a.ui() else { return };
                let items: Vec<HistoryItem> = thumbs.iter().map(|(img, label)| HistoryItem {
                    thumb: img.as_ref().map(to_image).unwrap_or_default(), label: label.clone().into(), broken: img.is_none(),
                }).collect();
                ui.set_history_items(ModelRc::new(VecModel::from(items)));
                ui.set_history_loading(false);
            }); });
        });
    }
    fn history_entry(&self, i: usize) -> Option<history::Entry> { self.history_shown.get(i).cloned() }
    /// Take the selection into the editor; `then` copies or saves it straight away
    /// through the same Rust render and delivery path (the editor stays hidden for a copy).
    fn finish_capture(&mut self, then: Option<Delivery>) {
        let Some(s) = self.session.as_ref() else { return };
        diag::outcome("done");
        let area = s.confirm_area();
        let pixels = area.and_then(|r| s.crop(r));
        if area.is_some() { self.last_region = area; }
        self.end_capture();
        let Some(pixels) = pixels else { if std::mem::take(&mut self.restore_editor) { self.show_editor(); } return };
        let editor_was_open = std::mem::take(&mut self.restore_editor);
        if then != Some(Delivery::Copy) || editor_was_open { self.show_editor(); }
        // Leave the state borrow before starting the shared intake pipeline.
        slint::Timer::single_shot(Duration::ZERO, move || {
            let (w, h) = pixels.dimensions();
            spawn_open_then(capture::encode_png(pixels), Some(tf("截图 {}×{}.png", &[&w, &h])), then);
        });
    }

    // ---- settings ----
    fn register_hotkey(&mut self) -> bool {
        if let Some(h) = self.hotkey.take() { h.stop(); }
        let key = self.settings.hotkey;
        match anshot_platform_windows::spawn_hotkey(key, || {
            let _ = slint::invoke_from_event_loop(|| { with(|a| a.begin_capture("hotkey")); });
        }) {
            Ok(h) => { self.hotkey = Some(h); self.push_settings(); true }
            Err(_) => {
                self.toast("warn", &tf("快捷键 {} 被其他程序占用", &[&key.label()]), t("可在设置中换一个，或从托盘图标截图"), None);
                self.push_settings();
                false
            }
        }
    }
    fn push_settings(&self) {
        let Some(ui) = self.ui() else { return };
        let s = &self.settings;
        ui.set_hotkey_label(s.hotkey.label().into());
        ui.set_hotkey_index(anshot_platform_windows::Hotkey::ALL.iter().position(|h| *h == s.hotkey).unwrap_or(0) as i32);
        ui.set_hotkey_active(self.hotkey.is_some());
        ui.set_hide_after_copy(s.hide_after_copy);
        ui.set_save_dir(s.save_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_default().into());
        ui.set_history_limit_index(settings::HISTORY_LIMITS.iter().position(|n| *n == s.history_limit).unwrap_or(2) as i32);
        ui.set_autostart(matches!(anshot_platform_windows::autostart(), Ok(Some(_))));
        ui.set_delay_index(settings::DELAYS.iter().position(|n| *n == s.delay_secs).unwrap_or(0) as i32);
        ui.set_include_cursor(s.include_cursor);
        ui.set_history_days_index(settings::HISTORY_DAYS.iter().position(|n| *n == s.history_days).unwrap_or(0) as i32);
        ui.set_clear_on_exit(s.clear_on_exit);
        ui.set_pad_redactions(s.pad_redactions);
        ui.set_warn_unredacted(s.warn_unredacted);
        ui.set_check_on_share(s.check_on_share);
        ui.set_frame_default(s.frame_default);
        ui.set_hdr_notice(s.hdr_notice);
        ui.set_clipboard_private(s.clipboard_private);
        ui.set_language_index(settings::LANGUAGES.iter().position(|l| *l == s.language).unwrap_or(0) as i32);
        if let Some(t) = self.tray.as_ref() { t.set_hotkey_label(s.hotkey.label().into()); t.set_delay_secs(s.delay_secs as i32); }
    }
    /// Settings that act outside the UI: redaction padding, the clipboard flags, the
    /// history's age limit.
    fn apply_privacy(&mut self) {
        self.ctl.pad = if self.settings.pad_redactions { settings::REDACT_PAD } else { 0 };
        platform::set_clipboard_private(self.settings.clipboard_private);
        if let Some(h) = self.history() { h.prune_age(self.settings.history_days, history::now_millis()); }
    }
    fn open_settings(&mut self) {
        // Never replace a pending close/replace prompt (the tray bypasses command gating).
        if self.ui().is_some_and(|ui| !ui.get_dialog().is_empty()) { self.show_editor(); return; }
        self.show_editor();
        self.push_settings();
        if let Some(ui) = self.ui() { ui.set_dialog("settings".into()); }
    }
    fn persist(&mut self) {
        if let Err(e) = self.settings.save() { self.toast("error", t("设置未能保存"), &e, None); }
        self.push_settings();
    }
    fn set_hotkey(&mut self, i: usize) {
        let Some(&key) = anshot_platform_windows::Hotkey::ALL.get(i) else { return };
        let old = self.settings.hotkey;
        self.settings.hotkey = key;
        if self.resident && !self.register_hotkey() {
            self.settings.hotkey = old;
            self.register_hotkey();
        }
        self.persist();
    }

    /// Start at login (per-user Run entry). The tray process starts hidden.
    fn set_autostart(&mut self, on: bool) {
        let cmd = on.then(autostart_command).flatten();
        if on && cmd.is_none() { self.toast("error", t("无法设置开机启动"), t("找不到程序路径"), None); self.push_settings(); return; }
        if anshot_platform_windows::set_autostart(cmd.as_deref()).is_err() {
            self.toast("error", t("无法设置开机启动"), t("系统拒绝写入当前用户的启动项"), None);
        }
        self.push_settings();
    }

    // ---- custom colour picker ----
    fn open_picker(&mut self) {
        let (h, s, v) = color::rgb_to_hsv(self.ctl.custom);
        self.set_picker(h, s, v);
        if let Some(ui) = self.ui() {
            let recent: Vec<slint::Color> = self.settings.recent_colors.iter().map(|c| slint::Color::from_rgb_u8(c[0], c[1], c[2])).collect();
            ui.set_recent_colors(ModelRc::new(VecModel::from(recent)));
            ui.set_dialog("color".into());
        }
    }
    fn set_picker(&mut self, h: f32, s: f32, v: f32) {
        let Some(ui) = self.ui() else { return };
        let (c, pure) = (color::hsv_to_rgb(h, s, v), color::hsv_to_rgb(h, 1.0, 1.0));
        ui.set_picker_hue(h); ui.set_picker_sat(s); ui.set_picker_val(v);
        ui.set_picker_color(slint::Color::from_rgb_u8(c[0], c[1], c[2]));
        ui.set_picker_hue_color(slint::Color::from_rgb_u8(pure[0], pure[1], pure[2]));
        ui.set_picker_hex(color::to_hex(c).into());
    }
    fn apply_picker(&mut self) {
        let Some(ui) = self.ui() else { return };
        let c = color::hsv_to_rgb(ui.get_picker_hue(), ui.get_picker_sat(), ui.get_picker_val());
        ui.set_dialog(SharedString::new());
        self.ctl.set_custom_color(c);
        self.settings.remember_color(c);
        self.persist();
        self.refresh();
    }

    fn dialog_choice(&mut self, choice: &str) {
        if let Some(ui) = self.ui() { ui.set_dialog(SharedString::new()); }
        self.ctl.cancel_drag();
        match (self.pending.take(), choice) {
            (Some(Pending::Replace(c, p, n, then)), "discard") => self.apply_source(c, p, n, then),
            // Not opened after all: the step stays unlinked.
            (Some(Pending::Replace(..)), _) => { self.pending_step_edit = None; }
            (Some(Pending::Close), "discard") => self.close_editor(true),
            (Some(Pending::Quit), "discard") => { let _ = slint::quit_event_loop(); }
            // Set only once the copy really started; any refusal leaves the app open.
            (Some(Pending::Close), "copy-close") => { if self.start(Delivery::Copy) { self.after_copy = AfterCopy::Close; } }
            (Some(Pending::Quit), "copy-close") => { if self.start(Delivery::Copy) { self.after_copy = AfterCopy::Quit; } }
            _ => {}
        }
        self.refresh();
    }
}

fn spawn_open(bytes: Result<Vec<u8>, String>, name: Option<String>) { spawn_open_then(bytes, name, None) }

/// Decode off the UI thread; `then` (copy / save) runs only once *this* image is open.
fn spawn_open_then(bytes: Result<Vec<u8>, String>, name: Option<String>, then: Option<Delivery>) {
    let started = with(|a| {
        if a.ctl.opening {
            if then.is_some() { a.show_editor(); }
            a.toast("warn", t("正在打开上一张图片"), t("请稍后再试"), None);
            return false;
        }
        a.ctl.opening = true;
        a.refresh();
        true
    }).unwrap_or(false);
    if !started { return; }
    std::thread::spawn(move || {
        let decoded: Decoded = bytes.and_then(|b| {
            let candidate = SourceCandidate::decode(b).map_err(|e| messages::intake(&e))?;
            let pixels = anshot_core::decode_source(candidate.bytes()).map_err(|m| messages::decode(&m))?;
            Ok((candidate, display::preview(pixels)))
        });
        let _ = slint::invoke_from_event_loop(move || { with(|a| a.source_ready(decoded, name, then)); });
    });
}

/// Where new versions are published (opened in the browser by "检查更新").
const RELEASES_URL: &str = "https://github.com/2023violet/Anshot/releases";

/// What the Run entry launches: this exe, tray only.
fn autostart_command() -> Option<String> {
    std::env::current_exe().ok().map(|p| format!("\"{}\" --background", p.display()))
}

fn open_path(path: PathBuf) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    spawn_open(platform::read_image_file(&path), name);
}

fn suggested_name() -> String {
    with(|a| a.ctl.source_name.clone()).flatten()
        .and_then(|n| n.rsplit_once('.').map(|(stem, _)| stem.to_string()))
        .map(|stem| tf("{}_已遮挡.png", &[&stem]))
        .unwrap_or_else(|| t("安截_已遮挡.png").into())
}

thread_local! { static PICKING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

/// Run a blocking OS picker at most once at a time (rfd dialogs are not modal to our window).
fn picker<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
    if PICKING.with(|p| p.replace(true)) { return None; }
    let r = f();
    PICKING.with(|p| p.set(false));
    r
}

fn command(cmd: &str) {
    let in_settings = with(|a| a.ui().is_some_and(|ui| ui.get_dialog() == "settings")).unwrap_or(false);
    let in_about = with(|a| a.ui().is_some_and(|ui| ui.get_dialog() == "about")).unwrap_or(false);
    let settings_cmd = cmd.starts_with("autocheck-") || cmd.starts_with("hotkey-") || cmd.starts_with("copy-") || cmd.starts_with("save-dir") || cmd.starts_with("autostart-") || cmd.starts_with("keep-") || cmd.starts_with("delay-") || cmd.starts_with("cursor-")
        || cmd.starts_with("days-") || cmd.starts_with("exit-") || cmd.starts_with("pad-") || cmd.starts_with("warn-") || cmd.starts_with("clip-") || cmd.starts_with("lang-") || cmd.starts_with("framedef-") || cmd.starts_with("hdrnote-");
    // The panel's file picker blocks: run it outside the state borrow.
    if cmd == "steps-file" && !with(|a| a.ui().is_some_and(|ui| ui.get_dialog() != "")).unwrap_or(true) {
        if with(|a| a.step_room()).unwrap_or(false) {
            if let Some(p) = picker(platform::pick_image) {
                let img = platform::read_image_file(&p).and_then(|b| anshot_core::decode_source(&b).map_err(|m| messages::decode(&m)));
                // A file may show anything: it is marked unedited, like a raw capture.
                with(|a| match img { Ok(img) => a.add_step(img, true, None), Err(e) => a.toast("error", t("无法加入步骤"), &e, None) });
            }
        }
        return;
    }
    let in_history = with(|a| a.ui().is_some_and(|ui| ui.get_dialog() == "history")).unwrap_or(false);
    if in_history && cmd.starts_with("history-") { history_command(cmd); return; }
    let in_picker = with(|a| a.ui().is_some_and(|ui| ui.get_dialog() == "color")).unwrap_or(false);
    if in_picker && cmd.starts_with("picker-") {
        with(|a| match cmd {
            "picker-apply" => a.apply_picker(),
            "picker-cancel" => { if let Some(ui) = a.ui() { ui.set_dialog(SharedString::new()); } }
            c if c.starts_with("picker-recent-") => {
                if let Some(col) = c[14..].parse::<usize>().ok().and_then(|i| a.settings.recent_colors.get(i).copied()) {
                    let (h, s, v) = color::rgb_to_hsv(col); a.set_picker(h, s, v);
                }
            }
            _ => {}
        });
        return;
    }
    if !(in_settings && settings_cmd) && !(in_about && cmd == "check-update") && with(|a| a.ui().is_some_and(|ui| ui.get_dialog() != "")).unwrap_or(false) { return; }
    match cmd {
        // Blocking dialogs and clipboard reads run outside the state borrow.
        "open" => { if let Some(p) = picker(platform::pick_image) { open_path(p); } }
        "paste" => spawn_open(platform::read_clipboard(), Some(t("剪贴板图片.png").into())),
        // The app never connects anywhere: the default browser opens the release page.
        "check-update" => { let _ = std::process::Command::new("explorer").arg(RELEASES_URL).spawn(); }
        "capture" => { with(|a| a.begin_capture("editor")); }
        "save-dir" => {
            if let Some(dir) = picker(platform::pick_folder) { with(|a| { a.settings.save_dir = Some(dir); a.persist(); }); }
        }
        "save" => {
            if !with(|a| a.ctl.can_deliver()).unwrap_or(false) {
                with(|a| a.start(Delivery::Save));
                return;
            }
            let dir = with(|a| a.settings.save_dir.clone()).flatten();
            if let Some(path) = picker(|| platform::pick_save_path(&suggested_name(), dir.as_deref())) {
                with(|a| { a.save_path = Some(path); a.start(Delivery::Save); });
            }
        }
        _ => {
            with(|a| {
                match cmd {
                    "copy" => { a.start(Delivery::Copy); }
                    "undo" => { a.ctl.undo(); }
                    "redo" => { a.ctl.redo(); }
                    "delete" => { a.ctl.delete_selected(); }
                    "tool-select" => a.ctl.tool = Tool::Select,
                    "tool-arrow" => { a.ctl.set_shape_tool(Tool::Arrow); }
                    "tool-rect" => { a.ctl.set_shape_tool(Tool::Rect); }
                    "tool-ellipse" => { a.ctl.set_shape_tool(Tool::Ellipse); }
                    "tool-line" => { a.ctl.set_shape_tool(Tool::Line); }
                    "tool-pen" => a.ctl.tool = Tool::Pen,
                    "tool-spotlight" => a.ctl.tool = Tool::Spotlight,
                    "tool-magnify" => a.ctl.tool = Tool::Magnify,
                    "color-picker" => a.open_picker(),
                    "tool-text" => a.ctl.tool = Tool::Text,
                    "tool-number" => a.ctl.tool = Tool::Number,
                    "tool-highlight" => a.ctl.tool = Tool::Highlight,
                    "tool-redact" | "style-solid" => { a.ctl.set_redaction_kind(false); }
                    "tool-mosaic" | "style-mosaic" => { a.ctl.set_redaction_kind(true); }
                    "tool-crop" => a.ctl.enter_crop(),
                    "bg-on" => { a.ctl.apply_style(|s| s.background = true); }
                    "bg-off" => { a.ctl.apply_style(|s| s.background = false); }
                    c if c.starts_with("color-") => { if let Ok(i) = c[6..].parse::<usize>() { a.ctl.apply_style(|s| s.color = i.min(CUSTOM_INK)); } }
                    c if c.starts_with("width-") => { if let Ok(i) = c[6..].parse::<usize>() { a.ctl.apply_style(|s| s.width = i.min(2)); } }
                    c if c.starts_with("size-") => { if let Ok(i) = c[5..].parse::<usize>() { a.ctl.apply_style(|s| s.size = i.min(4)); } }
                    c if c.starts_with("solid-") => { if let Ok(i) = c[6..].parse::<usize>() { a.ctl.apply_style(|s| s.solid = i.min(1)); } }
                    "zoom-fit" => a.ctl.zoom_to(Zoom::Fit),
                    "zoom-actual" => a.ctl.zoom_to(Zoom::Scale(1.0)),
                    "zoom-in" => { let c = canvas_centre(&a.ctl); a.ctl.zoom_by(1.25, c); }
                    "zoom-out" => { let c = canvas_centre(&a.ctl); a.ctl.zoom_by(0.8, c); }
                    "settings" => a.open_settings(),
                    "history" => a.open_history(),
                    "ocr" => a.ocr_document(),
                    "check" => a.run_check(),
                    c if c == "frame" || c.starts_with("frame-") => a.frame_command(c),
                    c if c == "steps" || c == "step-add" || c.starts_with("steps-") => a.steps_command(c),
                    "framedef-on" => { a.settings.frame_default = true; a.persist(); a.push_settings(); }
                    "framedef-off" => { a.settings.frame_default = false; a.persist(); a.push_settings(); }
                    "hdrnote-on" => { a.settings.hdr_notice = true; a.persist(); a.push_settings(); }
                    "hdrnote-off" => { a.settings.hdr_notice = false; a.persist(); a.push_settings(); }
                    "same-text" => a.run_same_text(),
                    c if c.starts_with("check-") => a.check_command(c),
                    "pin" => a.pin_document(),
                    c if c.starts_with("keep-") => { if let Some(&n) = c[5..].parse::<usize>().ok().and_then(|i| settings::HISTORY_LIMITS.get(i)) {
                        a.settings.history_limit = n; a.persist();
                        if let Some(h) = a.history() { h.prune(n); }
                    } }
                    c if c.starts_with("hotkey-") => { if let Ok(i) = c[7..].parse::<usize>() { a.set_hotkey(i); } }
                    c if c.starts_with("delay-") => { if let Some(&n) = c[6..].parse::<usize>().ok().and_then(|i| settings::DELAYS.get(i)) { a.settings.delay_secs = n; a.persist(); a.push_settings(); } }
                    c if c.starts_with("days-") => { if let Some(&n) = c[5..].parse::<usize>().ok().and_then(|i| settings::HISTORY_DAYS.get(i)) { a.settings.history_days = n; a.persist(); a.apply_privacy(); a.push_settings(); } }
                    c if c.starts_with("lang-") => { if let Some(l) = c[5..].parse::<usize>().ok().and_then(|i| settings::LANGUAGES.get(i)) { a.settings.language = (*l).into(); a.persist(); apply_language(&a.settings.language); a.push_settings(); a.refresh(); } }
                    "exit-clear" => { a.settings.clear_on_exit = true; a.persist(); a.push_settings(); }
                    "exit-keep" => { a.settings.clear_on_exit = false; a.persist(); a.push_settings(); }
                    "pad-on" => { a.settings.pad_redactions = true; a.persist(); a.apply_privacy(); a.push_settings(); }
                    "pad-off" => { a.settings.pad_redactions = false; a.persist(); a.apply_privacy(); a.push_settings(); }
                    "autocheck-on" => { a.settings.check_on_share = true; a.persist(); a.push_settings(); }
                    "autocheck-off" => { a.settings.check_on_share = false; a.persist(); a.push_settings(); }
                    "warn-on" => { a.settings.warn_unredacted = true; a.persist(); a.push_settings(); }
                    "warn-off" => { a.settings.warn_unredacted = false; a.persist(); a.push_settings(); }
                    "clip-private" => { a.settings.clipboard_private = true; a.persist(); a.apply_privacy(); a.push_settings(); }
                    "clip-open" => { a.settings.clipboard_private = false; a.persist(); a.apply_privacy(); a.push_settings(); }
                    "cursor-on" => { a.settings.include_cursor = true; a.persist(); a.push_settings(); }
                    "cursor-off" => { a.settings.include_cursor = false; a.persist(); a.push_settings(); }
                    "copy-hide" => { a.settings.hide_after_copy = true; a.persist(); }
                    "copy-stay" => { a.settings.hide_after_copy = false; a.persist(); }
                    "save-dir-clear" => { a.settings.save_dir = None; a.persist(); }
                    "autostart-on" => a.set_autostart(true),
                    "autostart-off" => a.set_autostart(false),
                    "toast-action" => match a.toast_action { Some(ToastAction::Retry(k)) => { a.start(k); } Some(ToastAction::Review) => a.open_pending_review(), Some(ToastAction::MuteHdr) => { a.settings.hdr_notice = false; a.persist(); a.clear_toast(); }
                        Some(ToastAction::OpenSteps) => { a.clear_toast(); a.show_editor(); a.open_steps(true); }
                        Some(ToastAction::UndoStep) => { a.clear_toast(); a.steps_command("steps-undo"); }
                        None => {} },
                    _ => {}
                }
                a.refresh();
            });
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // One embedded copy of Noto Sans CJK SC serves both the UI and the exporter.
    {
        use slint::fontique_011::fontique;
        let blob = fontique::Blob::new(std::sync::Arc::new(anshot_render::font_bytes()));
        let _ = slint::fontique_011::shared_collection().register_fonts(blob, None);
    }
    let ui = AppWindow::new()?;
    APP.with(|cell| *cell.borrow_mut() = Some(App {
        ctl: Controller::default(), ui: ui.as_weak(), pending: None, save_path: None,
        after_copy: AfterCopy::Stay, toast_seq: 0, toast_action: None,
        resident: false, tray: None, session: None, overlays: Vec::new(), overlay_pool: Vec::new(), toast_win: None, base: None,
        settings: settings::Settings::load(), hotkey: None, restore_editor: false, capture_pending: false,
        cursor_timer: slint::Timer::default(), last_cursor: None, last_region: None, notice_seq: 0,
        pins: Vec::new(), next_pin: 1, pin_drag: None, history_shown: Vec::new(), history_gen: 0, long: None, countdown: None, ocr_win: None, ocr_seq: 0, hidden_for_capture: Vec::new(), ocr_hidden: false, ocr_running: false, check_running: false, auto_check_running: false, auto_check_next: None, system_toast_seq: 0, replace_target: Cell::new(None),
        frame_open: false, frame_text: String::new(), framed_cache: RefCell::new(None), hdr_noticed: false,
        steps: steps::Steps::default(), steps_open: false, burst: false, steps_adding: 0, steps_busy: false, steps_preview_gen: 0, editing_step: None, pending_step_edit: None,
    }));
    with(|a| a.steps.style = a.settings.steps_style);
    // The first instance owns the tray and the global hotkey.
    if anshot_platform_windows::claim_single_instance() {
        let tray = Tray::new()?;
        tray.on_capture(|| { with(|a| a.begin_capture("tray")); });
        tray.on_delayed_capture(|| { with(|a| a.start_countdown()); });
        tray.on_open_image(|| { with(|a| a.show_editor()); command("open"); });
        tray.on_show_editor(|| { with(|a| a.show_editor()); });
        tray.on_about(|| { with(|a| { a.show_editor(); if let Some(ui) = a.ui() { if ui.get_dialog().is_empty() { ui.set_dialog("about".into()); } } }); });
        tray.on_quit(|| { with(|a| a.request_quit()); });
        tray.on_settings(|| { with(|a| a.open_settings()); });
        tray.on_history(|| { with(|a| a.open_history()); });
        tray.on_steps(|| { with(|a| { a.show_editor(); a.open_steps(true); }); });
        tray.on_toggle_burst(|| { with(|a| { let on = !a.burst; a.set_burst(on); }); });
        with(|a| {
            a.resident = true;
            a.tray = Some(tray);
            a.register_hotkey();
            // Keep an existing start-at-login entry pointing at this exe if it was moved.
            if let Some(new) = autostart_command() {
                if let Ok(Some(old)) = anshot_platform_windows::autostart() {
                    // Another installed copy keeps its entry; only a dangling one is taken over.
                    let target = if old.starts_with('"') { old[1..].split('"').next().unwrap_or("") } else { old.split(" --").next().unwrap_or("").trim() };
                    if old != new && !std::path::Path::new(target).is_file() { let _ = anshot_platform_windows::set_autostart(Some(&new)); }
                }
            }
        });
        slint::Timer::single_shot(Duration::from_millis(800), || { with(|a| if a.session.is_none() { a.prewarm_overlays(); }); });
    }

    ui.on_pointer_down(|x, y| { with(|a| { a.ctl.pointer_down(Point { x: x.into(), y: y.into() }); a.refresh(); }); });
    ui.on_pointer_move(|x, y| { with(|a| if a.ctl.dragging() {
        a.ctl.shift = anshot_platform_windows::shift_down();
        a.ctl.pointer_move(Point { x: x.into(), y: y.into() });
        a.refresh();
    }); });
    ui.on_pointer_up(|| { with(|a| { a.ctl.pointer_up(); a.refresh(); }); });
    ui.on_canvas_size(|w, h| { with(|a| { a.ctl.set_area(w.into(), h.into()); a.refresh(); if a.steps_open { a.request_steps_preview(); } }); });
    ui.on_command(|c| command(c.as_str()));
    ui.on_wheel(|dx, dy, x, y, ctrl, shift, alt| {
        with(|a| {
            if alt { if dy != 0.0 { a.ctl.wheel_adjust(dy > 0.0); } }
            else if ctrl { a.ctl.zoom_by(if dy > 0.0 { 1.1 } else { 1.0 / 1.1 }, Point { x: x.into(), y: y.into() }); }
            else if shift { a.ctl.scroll(f64::from(dy), 0.0); }
            else { a.ctl.scroll(f64::from(dx), f64::from(dy)); }
            a.refresh();
        });
    });
    ui.on_middle_down(|x, y| { with(|a| { a.ctl.begin_pan(Point { x: x.into(), y: y.into() }); a.refresh(); }); });
    ui.on_key_released(|t| { if t.as_str() == " " { with(|a| a.ctl.space_pan = false); } });
    // OS file drops (02 S3/S2): Slint 1.18 does not forward them, so read winit's event.
    {
        use slint::winit_030::{winit::event::WindowEvent, EventResult, WinitWindowAccessor};
        ui.window().on_winit_window_event(|_, ev| {
            match ev {
                WindowEvent::HoveredFile(_) => { with(|a| if let Some(ui) = a.ui() { ui.set_drop_hover(true); }); }
                WindowEvent::HoveredFileCancelled => { with(|a| if let Some(ui) = a.ui() { ui.set_drop_hover(false); }); }
                WindowEvent::DroppedFile(path) => {
                    let path = path.clone();
                    with(|a| if let Some(ui) = a.ui() { ui.set_drop_hover(false); });
                    slint::Timer::single_shot(Duration::ZERO, move || {
                        let busy = with(|a| a.ui().is_some_and(|ui| ui.get_dialog() != "")).unwrap_or(true);
                        if !busy { open_path(path); }
                    });
                }
                _ => {}
            }
            EventResult::Propagate
        });
    }
    ui.on_double_click(|x, y| { with(|a| { a.ctl.edit_text_at(Point { x: x.into(), y: y.into() }); a.refresh(); }); });
    ui.on_text_commit(|t| { with(|a| { a.ctl.commit_text(t.as_str()); a.refresh(); if let Some(ui) = a.ui() { ui.invoke_focus_keys(); } }); });
    ui.on_text_cancel(|| { with(|a| { a.ctl.cancel_text(); a.refresh(); if let Some(ui) = a.ui() { ui.invoke_focus_keys(); } }); });
    ui.on_replace_commit(|text| { with(|a| a.replace_selected(text.as_str())); });
    ui.on_frame_text_edited(|text| { with(|a| a.frame_text_edited(text.as_str())); });
    ui.on_step_caption_edited(|id, text| { with(|a| a.step_caption_edited(id as u64, text.as_str())); });
    ui.on_dialog_choice(|c| { with(|a| a.dialog_choice(c.as_str())); });
    ui.on_picker_hsv(|h, s, v| { with(|a| a.set_picker(h, s, v)); });
    ui.on_picker_hex_edited(|t| { with(|a| if let Some(c) = color::parse_hex(t.as_str()) { let (h, s, v) = color::rgb_to_hsv(c); a.set_picker(h, s, v); }); });
    ui.on_key(|text, ctrl, shift, other| {
        let dialog_open = with(|a| a.ui().is_some_and(|ui| ui.get_dialog() != "")).unwrap_or(true);
        if ctrl && !other && !dialog_open {
            let c = text.chars().next().map(|c| if (c as u32) < 27 && c as u32 > 0 { (b'a' + c as u8 - 1) as char } else { c.to_ascii_lowercase() });
            match c {
                Some('o') if shift => { command("ocr"); return true; }
                Some('k') if shift => { command("check"); return true; }
                Some('b') if shift => { command("frame"); return true; }
                Some('o') => { command("open"); return true; }
                Some('v') => { command("paste"); return true; }
                Some('s') => { command("save"); return true; }
                Some(',') => { command("settings"); return true; }
                Some('h') => { command("history"); return true; }
                Some('t') => { command("pin"); return true; }
                Some('j') if shift => { command("steps"); return true; }
                Some('j') => { command("step-add"); return true; }
                _ => {}
            }
        }
        with(|a| a.key(text.as_str(), ctrl, shift, other)).unwrap_or(false)
    });
    ui.window().on_close_requested(|| {
        with(|a| {
            if a.ctl.has_unsaved_edits() {
                a.ctl.cancel_drag();
                a.pending = Some(Pending::Close);
                a.refresh();
                if let Some(ui) = a.ui() { ui.set_dialog("close".into()); }
                CloseRequestResponse::KeepWindowShown
            } else if !a.resident && !a.steps.is_empty() {
                // A second instance ends with its window: say that the steps go with it.
                a.pending = Some(Pending::Quit);
                if let Some(ui) = a.ui() { ui.set_dialog("steps-quit".into()); }
                CloseRequestResponse::KeepWindowShown
            } else {
                if !a.resident { let _ = slint::quit_event_loop(); }
                CloseRequestResponse::HideWindow
            }
        }).unwrap_or(CloseRequestResponse::HideWindow)
    });

    // "Open with" / command-line path; `--background` (start at login) stays in the tray;
    // `--delayed-capture` starts a delayed capture (with --background: tray only).
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let background = args.iter().any(|a| a == "--background");
    let delayed = args.iter().any(|a| a == "--delayed-capture");
    if let Some(path) = args.into_iter().find(|a| a != "--background" && a != "--delayed-capture").map(PathBuf::from) { open_path(path); }
    ui.set_app_version(env!("CARGO_PKG_VERSION").into());
    with(|a| {
        apply_language(&a.settings.language);
        diag::header();
        // "Clear on exit" also runs at start: a crash, a kill or a log-off may skip the exit path.
        if a.settings.clear_on_exit && a.resident { if let Some(h) = a.history() { h.clear(); } }
        a.apply_privacy(); a.push_settings(); a.refresh();
    });
    let resident = with(|a| a.resident).unwrap_or(false);
    if !(background && resident) { ui.show()?; }
    if delayed && resident { slint::Timer::single_shot(Duration::from_millis(300), || { with(|a| a.start_countdown()); }); }
    let result = slint::run_event_loop_until_quit();
    // Privacy setting: nothing from this run stays in the history.
    with(|a| if a.settings.clear_on_exit { if let Some(h) = a.history() { h.clear(); } });
    result
}

fn tool_name(t: Tool) -> &'static str {
    match t {
        Tool::Select => "select", Tool::Arrow => "arrow", Tool::Rect => "rect", Tool::Text => "text", Tool::Number => "number",
        Tool::Highlight => "highlight", Tool::Redact => "redact", Tool::Mosaic => "mosaic", Tool::Crop => "crop",
        Tool::Ellipse => "ellipse", Tool::Line => "line", Tool::Pen => "pen", Tool::Spotlight => "spotlight", Tool::Magnify => "magnify",
    }
}
/// 02 §5.2 single-key tools.
fn tool_for_key(c: char) -> Option<Tool> {
    Some(match c {
        'v' => Tool::Select, 'a' => Tool::Arrow, 'r' => Tool::Rect, 't' => Tool::Text, 'n' => Tool::Number,
        'h' => Tool::Highlight, 'b' => Tool::Redact, 'm' => Tool::Mosaic, 'c' => Tool::Crop,
        'e' => Tool::Ellipse, 'l' => Tool::Line, 'p' => Tool::Pen, 's' => Tool::Spotlight, 'z' => Tool::Magnify, _ => return None,
    })
}

/// Pointer shape for what a press would grab on the capture overlay.
fn cursor_kind(s: &capture::Session, cursor: Option<(i32, i32)>) -> &'static str {
    let Some((x, y)) = cursor else { return "new" };
    match s.grab_at(x, y) {
        capture::Grab::New => "new",
        capture::Grab::Move(_) => "move",
        capture::Grab::Edge { left, right, top, bottom, .. } => match (left || right, top || bottom) {
            (true, true) if (left && top) || (right && bottom) => "nwse",
            (true, true) => "nesw",
            (true, false) => "ew",
            _ => "ns",
        },
    }
}

/// Pin menu / shortcuts. Runs outside the state borrow (the save dialog blocks).
fn pin_action(id: u64, act: &str) {
    let Some(img) = with(|a| a.pin(id).map(|p| p.img.clone())).flatten() else { return };
    match act {
        "close" => { with(|a| a.close_pin(id)); }
        "ocr" => { with(|a| a.start_ocr(img)); }
        "edit" => { with(|a| a.show_editor()); spawn_open(capture::encode_png(img), Some(t("钉图.png").into())); }
        "copy" => {
            let dims = img.dimensions();
            // Pins are a viewing aid: their copies are not editor deliveries and stay out of the history.
            let result = capture::encode_png(img).and_then(|png| platform::write_clipboard(&png, dims));
            with(|a| match result {
                Ok(()) => a.system_toast("done", t("已复制到剪贴板"), &format!("{} × {}", dims.0, dims.1)),
                Err(e) => a.system_toast("error", t("复制失败"), &e),
            });
        }
        "save" => {
            let dir = with(|a| a.settings.save_dir.clone()).flatten();
            let Some(path) = picker(|| platform::pick_save_path(t("钉图.png"), dir.as_deref())) else { return };
            let path = platform::free_path(&path);
            let dims = img.dimensions();
            // Same check as every export: the bytes must reopen as a clean generated PNG first.
            let result = capture::encode_png(img)
                .and_then(|png| anshot_core::decode_generated_png(&png, dims).map(|_| png))
                .and_then(|png| platform::write_file(&path, &png));
            with(|a| match result {
                Ok(()) => a.system_toast("done", t("已保存"), &path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
                Err(e) => a.system_toast("error", t("保存失败"), &e),
            });
        }
        o if o.starts_with("opacity-") => {
            let level = o[8..].parse::<f32>().unwrap_or(100.0) / 100.0;
            with(|a| if let Some(p) = a.pin(id) { p.win.set_level(level); });
        }
        _ => {}
    }
}

/// History dialog actions on the listed entry `i`.
fn history_command(cmd: &str) {
    let index = |prefix: &str| cmd.strip_prefix(prefix).and_then(|n| n.parse::<usize>().ok());
    if cmd == "history-close" { with(|a| if let Some(ui) = a.ui() { ui.set_dialog(SharedString::new()); }); return; }
    if cmd == "history-clear" {
        with(|a| { if let Some(h) = a.history() { h.clear(); } a.open_history(); });
        return;
    }
    let (entry, h) = match ["history-open-", "history-copy-", "history-pin-", "history-step-", "history-delete-"].iter().find_map(|p| index(p).map(|i| (*p, i))) {
        Some((p, i)) => (with(|a| a.history_entry(i)).flatten().map(|e| (p, e)), with(|a| a.history()).flatten()),
        None => return,
    };
    let (Some((kind, e)), Some(h)) = (entry, h) else { return };
    match kind {
        "history-delete-" => { h.remove(&e); with(|a| a.open_history()); }
        _ => {
            let bytes = match h.read(&e) { Ok(b) => b, Err(err) => { with(|a| a.toast("error", t("无法读取历史"), &err, None)); return; } };
            match kind {
                "history-open-" => { with(|a| if let Some(ui) = a.ui() { ui.set_dialog(SharedString::new()); }); spawn_open(Ok(bytes), Some(t("历史截图.png").into())); }
                "history-copy-" => {
                    let result = anshot_core::decode_source(&bytes).and_then(|img| { let d = img.dimensions(); capture::encode_png(img).and_then(|png| platform::write_clipboard(&png, d)) });
                    with(|a| match result { Ok(()) => a.toast("done", t("已复制到剪贴板"), t("来自截图历史"), None), Err(err) => a.toast("error", t("复制失败"), &err, None) });
                }
                // History holds delivered images: their redactions are already in the pixels.
                "history-step-" => {
                    let img = anshot_core::decode_source(&bytes);
                    with(|a| match img { Ok(img) => a.add_step(img, false, None), Err(_) => a.toast("error", t("无法加入步骤"), t("历史文件已损坏"), None) });
                }
                _ => {
                    let img = anshot_core::decode_source(&bytes);
                    with(|a| match img { Ok(img) => a.open_pin(img, None), Err(_) => a.toast("error", t("无法钉图"), t("历史文件已损坏"), None) });
                }
            }
        }
    }
}

/// Switch Slint's bundled translation and the Rust-side strings together.
fn apply_language(setting: &str) {
    let en = i18n::wants_english(setting, anshot_platform_windows::ui_language_is_chinese());
    i18n::set_english(en);
    let _ = slint::select_bundled_translation(if en { i18n::SLINT_EN } else { "" });
}

/// Small UI text is recognised far better slightly enlarged (spike/ocr: 92 % -> 99 % at
/// 100 % scale), so regions are upscaled 1.5x in Rust first when that stays affordable.
fn recognize_text(img: RgbaImage) -> Result<String, String> {
    let (w, h) = img.dimensions();
    let img = check::upscaled(&img, if check::can_upscale(w, h, 1.5) { 1.5 } else { 1.0 });
    let (w, h) = img.dimensions();
    match anshot_platform_windows::ocr_recognize(img.as_raw(), w, h) {
        Ok(lines) => Ok(lines.into_iter().map(|l| l.text).collect::<Vec<_>>().join("\n")),
        Err(e) => Err(check::ocr_message(&e).into()),
    }
}

/// The native window handle behind a Slint window (once it is shown).
fn hwnd_of(w: &slint::Window) -> Option<isize> {
    use slint::winit_030::{winit::raw_window_handle::{HasWindowHandle, RawWindowHandle}, WinitWindowAccessor};
    w.with_winit_window(|ww| match ww.window_handle().ok()?.as_raw() { RawWindowHandle::Win32(h) => Some(h.hwnd.get()), _ => None }).flatten()
}

/// One pooled overlay window for monitor `i`, with its callbacks wired.
fn new_overlay(i: usize) -> Option<CaptureOverlay> {
    let win = CaptureOverlay::new().ok()?;
    reveal::watch(i, &win);
    win.on_pointer_down(move |x, y| { with(|a| a.overlay_pointer(i, x, y, Phase::Down)); });
    win.on_pointer_move(move |x, y| { with(|a| a.overlay_pointer(i, x, y, Phase::Move)); });
    win.on_pointer_up(move |x, y| { with(|a| a.overlay_pointer(i, x, y, Phase::Up)); });
    win.on_confirm(|| { with(|a| a.confirm_capture()); });
    win.on_cancel(|| { with(|a| a.cancel_capture()); });
    win.on_action(|k| { let k = k.to_string(); with(|a| a.overlay_action(&k)); });
    Some(win)
}

fn canvas_centre(c: &Controller) -> Point { let (w, h) = c.area(); Point { x: w / 2.0, y: h / 2.0 } }
