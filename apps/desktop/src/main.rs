//! ShareReady desktop: capture / open -> redact -> copy / save. Slint UI over the Rust editor;
//! a tray-resident process with a global hotkey (Ctrl+Shift+A).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Slint's generated code needs `unsafe`; our own modules each `forbid` it.
#![deny(unsafe_code)]

mod canvas;
mod capture;
mod color;
mod controller;
mod display;
mod history;
mod messages;
mod platform;
mod settings;

use canvas::Zoom;
use controller::{Controller, Delivery, Notice, Tool, CUSTOM_INK, TEXT_SIZES};
use image::RgbaImage;
use shareready_editor::{EditError, Point, ReviewKey, ReviewResult, SourceCandidate};
use slint::{CloseRequestResponse, ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::cell::RefCell;
use std::path::PathBuf;
use shareready_platform_windows::{Monitor, ScreenRect};
use std::time::Duration;

slint::include_modules!();

type Decoded = Result<(SourceCandidate, RgbaImage), String>;

enum Pending { Replace(SourceCandidate, RgbaImage, Option<String>), Close, Quit }

/// What to do once a confirm-dialog copy succeeds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AfterCopy { Stay, Close, Quit }

#[derive(Clone, Copy)]
enum ToastAction { Retry(Delivery) }

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
    hotkey: Option<shareready_platform_windows::HotkeyHandle>,
    /// The editor was hidden for a capture and comes back if the capture is cancelled.
    restore_editor: bool,
    /// A capture waits for the hidden editor to leave the screen.
    capture_pending: bool,
    /// Follows the pointer while an overlay is open (R7: the magnifier must track the real
    /// cursor even when no mouse-move event reaches the overlay).
    cursor_timer: slint::Timer,
    last_cursor: Option<(i32, i32)>,
    /// What to do as soon as a captured image is open (overlay toolbar: copy / save).
    after_open: Option<Delivery>,
    /// Pinned images (top-most windows); ids stay stable while others close.
    pins: Vec<Pin>,
    next_pin: u64,
    /// Pin being dragged: id, cursor and window position at the press.
    pin_drag: Option<(u64, (i32, i32), (i32, i32))>,
    /// History entries currently listed in the history dialog (same order as the UI).
    history_shown: Vec<history::Entry>,
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
            shareready_render::render_into(&mut img, &ctl.display_scene(), *k);
            ui.set_preview(to_image(&img));
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
        let mode = match kind.unwrap_or(ctl.tool) {
            Tool::Redact => "redact", Tool::Mosaic => "mosaic",
            Tool::Arrow | Tool::Rect | Tool::Ellipse | Tool::Line | Tool::Pen => "stroke",
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
        let title = ctl.source_name.clone().unwrap_or_else(|| "未命名".into());
        ui.set_doc_title(if ctl.has_unsaved_edits() { format!("● {title}") } else { title }.into());
    }

    fn toast(&mut self, kind: &str, title: &str, detail: &str, action: Option<(&str, ToastAction)>) {
        let Some(ui) = self.ui() else { return };
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
    fn source_ready(&mut self, decoded: Decoded, name: Option<String>) {
        self.ctl.opening = false;
        match decoded {
            Err(e) => self.toast("error", "无法打开", &e, None),
            Ok((candidate, preview)) => {
                if self.ctl.has_unsaved_edits() {
                    self.ctl.cancel_drag();
                    self.pending = Some(Pending::Replace(candidate, preview, name));
                    self.show_editor();
                    if let Some(ui) = self.ui() { ui.set_dialog("replace".into()); }
                } else {
                    self.apply_source(candidate, preview, name);
                }
            }
        }
        self.refresh();
    }
    fn apply_source(&mut self, candidate: SourceCandidate, preview: RgbaImage, name: Option<String>) {
        if let Err(e) = self.ctl.accept_source(candidate, name) {
            self.toast("error", "无法打开", &messages::intake(&e), None);
            return;
        }
        self.after_copy = AfterCopy::Stay;
        let k = self.ctl.editor.as_ref().map_or(1.0, |e| preview.width() as f32 / e.dimensions().0 as f32);
        self.base = Some((preview, k));
        self.clear_toast();
        match self.after_open.take() {
            Some(Delivery::Copy) => { self.start(Delivery::Copy); }
            // The save dialog is blocking: run it outside the state borrow.
            Some(Delivery::Save) => { slint::Timer::single_shot(Duration::ZERO, || command("save")); }
            None => {}
        }
    }

    // ---- delivery ----
    fn start(&mut self, kind: Delivery) -> bool {
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
            Err(EditError::NoEffectiveMask) => { self.toast("warn", "还没有遮挡", "先用遮挡工具框住要隐藏的内容", None); false }
            Err(EditError::OperationPending) => { self.toast("warn", "正在处理", "请等上一次复制或保存完成", None); false }
            Err(EditError::GestureInProgress) => { self.toast("warn", "正在拖动", "请先松开鼠标或按 Esc", None); false }
            Err(e) => { self.toast("error", "无法开始", &messages::edit(&e), None); false }
        };
        self.refresh();
        started
    }
    fn review_done(&mut self, key: &ReviewKey, result: Result<ReviewResult, EditError>) {
        match self.ctl.review_finished(key, result) {
            Ok((Delivery::Copy, ticket)) => {
                let mut kept = None;
                let outcome = self.ctl.deliver(ticket, |png, dims| { let r = platform::write_clipboard(png, dims); if r.is_ok() { kept = Some(png.to_vec()); } r });
                if let Some(png) = kept { self.record_history(&png); }
                self.after_delivery(Delivery::Copy, outcome, None);
            }
            Ok((Delivery::Save, ticket)) => {
                let Some(path) = self.save_path.take() else { self.ctl.abandon_delivery(ticket); self.refresh(); return };
                let path = platform::free_path(&path);
                let mut kept = None;
                let outcome = self.ctl.deliver(ticket, |png, _| { let r = platform::write_file(&path, png); if r.is_ok() { kept = Some(png.to_vec()); } r });
                if let Some(png) = kept { self.record_history(&png); }
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
                self.after_delivery(Delivery::Save, outcome, name);
            }
            Err(Notice::Changed) => {
                self.after_copy = AfterCopy::Stay;
                self.toast("warn", "内容已变化", "请重新复制或保存", None);
            }
            Err(Notice::Failed { title, detail }) => {
                self.after_copy = AfterCopy::Stay;
                self.toast("error", &title, &detail, None);
            }
            Err(Notice::Ignored) => {}
        }
        self.refresh();
    }
    fn after_delivery(&mut self, kind: Delivery, outcome: Result<(), String>, name: Option<String>) {
        let n = self.ctl.redaction_count();
        let (w, h) = self.ctl.editor.as_ref().map_or((0, 0), |e| e.output_dimensions());
        match (kind, outcome) {
            (Delivery::Copy, Ok(())) => {
                let detail = if n == 0 { format!("{w} × {h} · 未遮挡任何内容") } else { format!("{w} × {h} · 已遮挡 {n} 处") };
                match std::mem::replace(&mut self.after_copy, AfterCopy::Stay) {
                    AfterCopy::Quit => { let _ = slint::quit_event_loop(); }
                    // 02 U07: a successful copy ends the task; the editor steps aside.
                    _ if self.resident && self.settings.hide_after_copy => { self.close_editor(false); self.system_toast("done", "已复制到剪贴板", &detail); }
                    AfterCopy::Close if self.resident => { self.close_editor(false); self.system_toast("done", "已复制到剪贴板", &detail); }
                    AfterCopy::Close => { let _ = slint::quit_event_loop(); }
                    AfterCopy::Stay if !self.ui().is_some_and(|ui| ui.window().is_visible()) => self.system_toast("done", "已复制到剪贴板", &detail),
                    AfterCopy::Stay => self.toast("done", "已复制到剪贴板", &detail, None),
                }
            }
            (Delivery::Save, Ok(())) => { self.after_copy = AfterCopy::Stay; self.toast("done", "已保存", &name.unwrap_or_default(), None) }
            (Delivery::Copy, Err(e)) => {
                self.after_copy = AfterCopy::Stay;
                self.toast("error", "复制失败", &e, Some(("重试", ToastAction::Retry(Delivery::Copy))));
            }
            (Delivery::Save, Err(e)) => { self.after_copy = AfterCopy::Stay; self.toast("error", "保存失败", &e, None) }
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
        if discard { self.ctl.close_document(); self.base = None; self.refresh(); }
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
        if self.ctl.has_unsaved_edits() {
            self.show_editor();
            self.ctl.cancel_drag();
            self.pending = Some(Pending::Quit);
            if let Some(ui) = self.ui() { ui.set_dialog("close".into()); }
        } else { let _ = slint::quit_event_loop(); }
    }

    /// S4 outside the editor: bottom-right of the primary work area, 3 s.
    fn system_toast(&mut self, kind: &str, heading: &str, detail: &str) {
        if self.toast_win.is_none() { self.toast_win = ToastWindow::new().ok(); }
        let Some(t) = self.toast_win.as_ref() else { return };
        t.set_kind(kind.into());
        t.set_heading(heading.into());
        t.set_detail(detail.into());
        // The toast must not take keyboard focus (e.g. from a pin the user is working with).
        let focus = shareready_platform_windows::foreground_window();
        let _ = t.show();
        // The window system shows it on a later frame: hand focus back after that.
        slint::Timer::single_shot(Duration::from_millis(60), move || shareready_platform_windows::restore_foreground(focus));
        if let Some(m) = shareready_platform_windows::monitors().ok().and_then(|ms| ms.into_iter().find(|m| m.primary)) {
            let s = t.window().scale_factor();
            let (w, h) = ((300.0 * s) as i32, (64.0 * s) as i32);
            t.window().set_position(slint::PhysicalPosition::new(m.work.x + m.work.w - w - (16.0 * s) as i32, m.work.y + m.work.h - h - (16.0 * s) as i32));
        }
        self.toast_seq += 1;
        let seq = self.toast_seq;
        slint::Timer::single_shot(Duration::from_secs(3), move || {
            with(|a| if a.toast_seq == seq { if let Some(t) = a.toast_win.as_ref() { let _ = t.hide(); } });
        });
    }

    // ---- capture (02 S1) ----
    fn begin_capture(&mut self) {
        if self.session.is_some() || self.capture_pending { return; }
        if let Some(t) = self.toast_win.as_ref() { let _ = t.hide(); }
        // Never freeze our own editor into the shot: hide it, let the event loop apply the
        // hide, then wait for the compositor to present the desktop without it.
        let visible = self.ui().is_some_and(|ui| ui.window().is_visible());
        self.restore_editor = visible;
        if !visible { self.freeze_screen(); return; }
        if let Some(ui) = self.ui() { let _ = ui.hide(); }
        self.capture_pending = true;
        slint::Timer::single_shot(Duration::from_millis(16), || {
            shareready_platform_windows::wait_for_compositor();
            with(|a| { a.capture_pending = false; a.freeze_screen(); });
        });
    }
    fn freeze_screen(&mut self) {
        let monitors = match shareready_platform_windows::monitors() {
            Ok(m) => m,
            Err(_) => { self.restore_editor = false; self.show_editor(); self.toast("error", "无法截图", "无法读取显示器信息，请重试", None); return; }
        };
        let Some(area) = capture::union(&monitors) else { self.cancel_capture(); return };
        let frozen = match shareready_platform_windows::capture(area) {
            Ok(c) => c,
            Err(_) => { self.restore_editor = false; self.show_editor(); self.toast("error", "无法截图", "系统拒绝读取屏幕（锁屏或安全桌面时无法截图）", None); return; }
        };
        let session = capture::Session::new(frozen, monitors.clone(), shareready_platform_windows::visible_windows());
        let mut overlays = Vec::new();
        for (i, m) in monitors.iter().copied().enumerate() {
            // Overlay i must serve monitor i; on any failure abort the capture cleanly.
            let Some(pixels) = session.crop(m.bounds) else { self.end_capture(); self.restore_editor = false; self.show_editor(); self.toast("error", "无法截图", "显示器区域无效", None); return; };
            if self.overlay_pool.len() <= i {
                let Some(win) = new_overlay(i) else { self.end_capture(); self.restore_editor = false; self.show_editor(); self.toast("error", "无法截图", "无法创建截图窗口", None); return; };
                self.overlay_pool.push(win);
            }
            let win = self.overlay_pool[i].clone_strong();
            win.set_shot(to_image(&pixels));
            let place = |w: &CaptureOverlay| {
                w.window().set_position(slint::PhysicalPosition::new(m.bounds.x, m.bounds.y));
                w.window().set_size(slint::PhysicalSize::new(m.bounds.w as u32, m.bounds.h as u32));
            };
            place(&win);
            let _ = win.show();
            place(&win); // re-apply after a possible per-monitor DPI change on show
            overlays.push((win, m));
        }
        if overlays.is_empty() { self.cancel_capture(); return; }
        self.session = Some(session);
        self.overlays = overlays;
        self.last_cursor = shareready_platform_windows::cursor();
        if let Some((x, y)) = self.last_cursor {
            if let Some(s) = self.session.as_mut() { s.pointer_move(x, y); }
        }
        self.refresh_overlays();
        self.cursor_timer.start(slint::TimerMode::Repeated, Duration::from_millis(30), || { with(|a| a.poll_cursor()); });
    }
    fn poll_cursor(&mut self) {
        let now = shareready_platform_windows::cursor();
        if now == self.last_cursor { return; }
        self.last_cursor = now;
        if let (Some((x, y)), Some(s)) = (now, self.session.as_mut()) { if !s.pressed() { s.pointer_move(x, y); } }
        self.refresh_overlays();
    }
    /// Overlay toolbar / shortcuts: copy, save, edit, cancel.
    fn overlay_action(&mut self, action: &str) {
        match action {
            "pin" => self.pin_selection(),
            "copy" => self.finish_capture(Some(Delivery::Copy)),
            "save" => self.finish_capture(Some(Delivery::Save)),
            "edit" => self.finish_capture(None),
            "cancel" => self.cancel_capture(),
            _ => {}
        }
    }
    /// Create and show/hide each monitor's overlay once, off-screen, so the first
    /// hotkey press does not pay window + GL context creation.
    fn prewarm_overlays(&mut self) {
        let Ok(monitors) = shareready_platform_windows::monitors() else { return };
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
        match phase { Phase::Down => s.pointer_down(x, y), Phase::Move => s.pointer_move(x, y), Phase::Up => s.pointer_up(x, y) }
        self.refresh_overlays();
    }
    fn refresh_overlays(&self) {
        let Some(s) = self.session.as_ref() else { return };
        let cursor = shareready_platform_windows::cursor();
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
                    win.set_mag_text(format!("x {x}  y {y}").into());
                    win.set_show_magnifier(true);
                }
                None => win.set_show_magnifier(false),
            }
            win.set_show_hint(m.bounds.contains(cursor.map_or(0, |c| c.0), cursor.map_or(0, |c| c.1)) || m.primary);
            win.set_cursor_kind(cursor_kind(s, cursor).into());
            win.set_show_toolbar(has_sel && !s.pressed());
        }
    }
    fn end_capture(&mut self) {
        self.cursor_timer.stop();
        for (win, _) in self.overlays.drain(..) {
            let _ = win.hide();
            // Do not keep frozen screen pixels around between captures.
            win.set_shot(Image::default());
            win.set_has_selection(false);
            win.set_has_hover(false);
        }
        self.session = None;
    }
    /// Esc / right click: put back whatever was on screen before the hotkey.
    fn cancel_capture(&mut self) {
        self.end_capture();
        if std::mem::take(&mut self.restore_editor) { self.show_editor(); }
    }
    fn confirm_capture(&mut self) { self.finish_capture(None); }
    /// Overlay "pin": the selected screen pixels become a pin at the same place.
    fn pin_selection(&mut self) {
        let Some(s) = self.session.as_ref() else { return };
        let area = s.confirm_area();
        let pixels = area.and_then(|r| s.crop(r));
        self.end_capture();
        if std::mem::take(&mut self.restore_editor) { self.show_editor(); }
        if let (Some(img), Some(r)) = (pixels, area) { self.open_pin(img, Some((r.x, r.y))); }
    }

    // ---- pins ----
    fn open_pin(&mut self, img: RgbaImage, at: Option<(i32, i32)>) {
        let Ok(win) = PinWindow::new() else { self.toast("error", "无法钉图", "无法创建窗口", None); return };
        let id = self.next_pin;
        self.next_pin += 1;
        win.set_img(to_image(&img));
        win.on_drag_start(move || { with(|a| a.pin_drag_start(id)); });
        win.on_drag_move(move || { with(|a| a.pin_drag_move(id)); });
        win.on_zoom(move |dy| { with(|a| a.pin_zoom(id, dy)); });
        win.on_act(move |k| { let k = k.to_string(); slint::Timer::single_shot(Duration::ZERO, move || pin_action(id, &k)); });
        let (x, y) = at.unwrap_or_else(|| {
            let c = shareready_platform_windows::cursor().unwrap_or((200, 200));
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
        let (Some(p), Some(c)) = (self.pin(id), shareready_platform_windows::cursor()) else { return };
        let pos = p.win.window().position();
        self.pin_drag = Some((id, c, (pos.x, pos.y)));
    }
    fn pin_drag_move(&mut self, id: u64) {
        let (Some((pid, c0, w0)), Some(c)) = (self.pin_drag, shareready_platform_windows::cursor()) else { return };
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
        let scene = e.scene();
        std::thread::spawn(move || {
            let img = shareready_render::render_png(&src, &scene).ok().and_then(|png| shareready_core::decode_source(&png).ok());
            let _ = slint::invoke_from_event_loop(move || { with(|a| match img {
                Some(img) => a.open_pin(img, None),
                None => a.toast("error", "无法钉图", "生成图片失败", None),
            }); });
        });
    }

    // ---- history ----
    fn history(&self) -> Option<history::History> { history::History::default_dir().map(history::History::at) }
    fn record_history(&mut self, png: &[u8]) {
        let limit = self.settings.history_limit;
        if limit == 0 { return; }
        if let Some(h) = self.history() { let _ = h.add(png, limit); }
    }
    fn open_history(&mut self) {
        self.show_editor();
        let Some(ui) = self.ui() else { return };
        ui.set_history_items(ModelRc::default());
        ui.set_history_loading(true);
        ui.set_dialog("history".into());
        let entries = self.history().map(|h| h.list()).unwrap_or_default();
        self.history_shown = entries.clone();
        let now = history::now_millis();
        // Thumbnails decode through Core off the UI thread; files are untrusted input.
        std::thread::spawn(move || {
            let thumbs: Vec<(Option<RgbaImage>, String)> = entries.iter().map(|e| {
                let img = platform::read_image_file(&e.path).ok().and_then(|b| shareready_core::decode_source(&b).ok())
                    .map(|i| image::imageops::thumbnail(&i, 168.min(i.width()), (168 * i.height() / i.width().max(1)).clamp(1, 112)));
                (img, history::label(e.millis, now))
            }).collect();
            let _ = slint::invoke_from_event_loop(move || { with(|a| {
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
        let pixels = s.confirm_area().and_then(|r| s.crop(r));
        self.end_capture();
        let Some(pixels) = pixels else { if std::mem::take(&mut self.restore_editor) { self.show_editor(); } return };
        let editor_was_open = std::mem::take(&mut self.restore_editor);
        self.after_open = then;
        if then != Some(Delivery::Copy) || editor_was_open { self.show_editor(); }
        // Leave the state borrow before starting the shared intake pipeline.
        slint::Timer::single_shot(Duration::ZERO, move || {
            let (w, h) = pixels.dimensions();
            spawn_open(capture::encode_png(pixels), Some(format!("截图 {w}×{h}.png")));
        });
    }

    // ---- settings ----
    fn register_hotkey(&mut self) -> bool {
        if let Some(h) = self.hotkey.take() { h.stop(); }
        let key = self.settings.hotkey;
        match shareready_platform_windows::spawn_hotkey(key, || {
            let _ = slint::invoke_from_event_loop(|| { with(|a| a.begin_capture()); });
        }) {
            Ok(h) => { self.hotkey = Some(h); self.push_settings(); true }
            Err(_) => {
                self.toast("warn", &format!("快捷键 {} 被其他程序占用", key.label()), "可在设置中换一个，或从托盘图标截图", None);
                self.push_settings();
                false
            }
        }
    }
    fn push_settings(&self) {
        let Some(ui) = self.ui() else { return };
        let s = &self.settings;
        ui.set_hotkey_label(s.hotkey.label().into());
        ui.set_hotkey_index(shareready_platform_windows::Hotkey::ALL.iter().position(|h| *h == s.hotkey).unwrap_or(0) as i32);
        ui.set_hotkey_active(self.hotkey.is_some());
        ui.set_hide_after_copy(s.hide_after_copy);
        ui.set_save_dir(s.save_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_default().into());
        ui.set_history_limit_index(settings::HISTORY_LIMITS.iter().position(|n| *n == s.history_limit).unwrap_or(2) as i32);
        ui.set_autostart(matches!(shareready_platform_windows::autostart(), Ok(Some(_))));
        if let Some(t) = self.tray.as_ref() { t.set_hotkey_label(s.hotkey.label().into()); }
    }
    fn open_settings(&mut self) {
        // Never replace a pending close/replace prompt (the tray bypasses command gating).
        if self.ui().is_some_and(|ui| !ui.get_dialog().is_empty()) { self.show_editor(); return; }
        self.show_editor();
        self.push_settings();
        if let Some(ui) = self.ui() { ui.set_dialog("settings".into()); }
    }
    fn persist(&mut self) {
        if let Err(e) = self.settings.save() { self.toast("error", "设置未能保存", &e, None); }
        self.push_settings();
    }
    fn set_hotkey(&mut self, i: usize) {
        let Some(&key) = shareready_platform_windows::Hotkey::ALL.get(i) else { return };
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
        if on && cmd.is_none() { self.toast("error", "无法设置开机启动", "找不到程序路径", None); self.push_settings(); return; }
        if shareready_platform_windows::set_autostart(cmd.as_deref()).is_err() {
            self.toast("error", "无法设置开机启动", "系统拒绝写入当前用户的启动项", None);
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
        if choice == "cancel" { self.after_open = None; }
        match (self.pending.take(), choice) {
            (Some(Pending::Replace(c, p, n)), "discard") => self.apply_source(c, p, n),
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

fn spawn_open(bytes: Result<Vec<u8>, String>, name: Option<String>) {
    let started = with(|a| {
        if a.ctl.opening { a.toast("warn", "正在打开上一张图片", "请稍后再试", None); return false; }
        a.ctl.opening = true;
        a.refresh();
        true
    }).unwrap_or(false);
    if !started { return; }
    std::thread::spawn(move || {
        let decoded: Decoded = bytes.and_then(|b| {
            let candidate = SourceCandidate::decode(b).map_err(|e| messages::intake(&e))?;
            let pixels = shareready_core::decode_source(candidate.bytes()).map_err(|m| messages::decode(&m))?;
            Ok((candidate, display::preview(pixels)))
        });
        let _ = slint::invoke_from_event_loop(move || { with(|a| a.source_ready(decoded, name)); });
    });
}

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
        .map(|stem| format!("{stem}_已遮挡.png"))
        .unwrap_or_else(|| "ShareReady_已遮挡.png".into())
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
    let settings_cmd = cmd.starts_with("hotkey-") || cmd.starts_with("copy-") || cmd.starts_with("save-dir") || cmd.starts_with("autostart-") || cmd.starts_with("keep-");
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
    if !(in_settings && settings_cmd) && with(|a| a.ui().is_some_and(|ui| ui.get_dialog() != "")).unwrap_or(false) { return; }
    match cmd {
        // Blocking dialogs and clipboard reads run outside the state borrow.
        "open" => { if let Some(p) = picker(platform::pick_image) { open_path(p); } }
        "paste" => spawn_open(platform::read_clipboard(), Some("剪贴板图片.png".into())),
        "capture" => { with(|a| a.begin_capture()); }
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
                    "pin" => a.pin_document(),
                    c if c.starts_with("keep-") => { if let Some(&n) = c[5..].parse::<usize>().ok().and_then(|i| settings::HISTORY_LIMITS.get(i)) {
                        a.settings.history_limit = n; a.persist();
                        if let Some(h) = a.history() { h.prune(n); }
                    } }
                    c if c.starts_with("hotkey-") => { if let Ok(i) = c[7..].parse::<usize>() { a.set_hotkey(i); } }
                    "copy-hide" => { a.settings.hide_after_copy = true; a.persist(); }
                    "copy-stay" => { a.settings.hide_after_copy = false; a.persist(); }
                    "save-dir-clear" => { a.settings.save_dir = None; a.persist(); }
                    "autostart-on" => a.set_autostart(true),
                    "autostart-off" => a.set_autostart(false),
                    "toast-action" => match a.toast_action { Some(ToastAction::Retry(k)) => { a.start(k); } None => {} },
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
        let blob = fontique::Blob::new(std::sync::Arc::new(shareready_render::font_bytes()));
        let _ = slint::fontique_011::shared_collection().register_fonts(blob, None);
    }
    let ui = AppWindow::new()?;
    APP.with(|cell| *cell.borrow_mut() = Some(App {
        ctl: Controller::default(), ui: ui.as_weak(), pending: None, save_path: None,
        after_copy: AfterCopy::Stay, toast_seq: 0, toast_action: None,
        resident: false, tray: None, session: None, overlays: Vec::new(), overlay_pool: Vec::new(), toast_win: None, base: None,
        settings: settings::Settings::load(), hotkey: None, restore_editor: false, capture_pending: false,
        cursor_timer: slint::Timer::default(), last_cursor: None, after_open: None,
        pins: Vec::new(), next_pin: 1, pin_drag: None, history_shown: Vec::new(),
    }));
    // The first instance owns the tray and the global hotkey.
    if shareready_platform_windows::claim_single_instance() {
        let tray = Tray::new()?;
        tray.on_capture(|| { with(|a| a.begin_capture()); });
        tray.on_open_image(|| { with(|a| a.show_editor()); command("open"); });
        tray.on_show_editor(|| { with(|a| a.show_editor()); });
        tray.on_about(|| { with(|a| { a.show_editor(); if let Some(ui) = a.ui() { ui.set_dialog("about".into()); } }); });
        tray.on_quit(|| { with(|a| a.request_quit()); });
        tray.on_settings(|| { with(|a| a.open_settings()); });
        tray.on_history(|| { with(|a| a.open_history()); });
        with(|a| {
            a.resident = true;
            a.tray = Some(tray);
            a.register_hotkey();
            // Keep an existing start-at-login entry pointing at this exe if it was moved.
            if let (Ok(Some(old)), Some(new)) = (shareready_platform_windows::autostart(), autostart_command()) {
                if old != new { let _ = shareready_platform_windows::set_autostart(Some(&new)); }
            }
        });
        slint::Timer::single_shot(Duration::from_millis(800), || { with(|a| if a.session.is_none() { a.prewarm_overlays(); }); });
    }

    ui.on_pointer_down(|x, y| { with(|a| { a.ctl.pointer_down(Point { x: x.into(), y: y.into() }); a.refresh(); }); });
    ui.on_pointer_move(|x, y| { with(|a| if a.ctl.dragging() {
        a.ctl.shift = shareready_platform_windows::shift_down();
        a.ctl.pointer_move(Point { x: x.into(), y: y.into() });
        a.refresh();
    }); });
    ui.on_pointer_up(|| { with(|a| { a.ctl.pointer_up(); a.refresh(); }); });
    ui.on_canvas_size(|w, h| { with(|a| { a.ctl.set_area(w.into(), h.into()); a.refresh(); }); });
    ui.on_command(|c| command(c.as_str()));
    ui.on_wheel(|dx, dy, x, y, ctrl, shift| {
        with(|a| {
            if ctrl { a.ctl.zoom_by(if dy > 0.0 { 1.1 } else { 1.0 / 1.1 }, Point { x: x.into(), y: y.into() }); }
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
    ui.on_dialog_choice(|c| { with(|a| a.dialog_choice(c.as_str())); });
    ui.on_picker_hsv(|h, s, v| { with(|a| a.set_picker(h, s, v)); });
    ui.on_picker_hex_edited(|t| { with(|a| if let Some(c) = color::parse_hex(t.as_str()) { let (h, s, v) = color::rgb_to_hsv(c); a.set_picker(h, s, v); }); });
    ui.on_key(|text, ctrl, shift, other| {
        let dialog_open = with(|a| a.ui().is_some_and(|ui| ui.get_dialog() != "")).unwrap_or(true);
        if ctrl && !other && !dialog_open {
            let c = text.chars().next().map(|c| if (c as u32) < 27 && c as u32 > 0 { (b'a' + c as u8 - 1) as char } else { c.to_ascii_lowercase() });
            match c {
                Some('o') => { command("open"); return true; }
                Some('v') => { command("paste"); return true; }
                Some('s') => { command("save"); return true; }
                Some(',') => { command("settings"); return true; }
                Some('h') => { command("history"); return true; }
                Some('t') => { command("pin"); return true; }
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
            } else {
                if !a.resident { let _ = slint::quit_event_loop(); }
                CloseRequestResponse::HideWindow
            }
        }).unwrap_or(CloseRequestResponse::HideWindow)
    });

    // "Open with" / command-line path; `--background` (start at login) stays in the tray.
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let background = args.iter().any(|a| a == "--background");
    if let Some(path) = args.into_iter().find(|a| a != "--background").map(PathBuf::from) { open_path(path); }
    with(|a| { a.push_settings(); a.refresh(); });
    let resident = with(|a| a.resident).unwrap_or(false);
    if !(background && resident) { ui.show()?; }
    slint::run_event_loop_until_quit()
}

fn tool_name(t: Tool) -> &'static str {
    match t {
        Tool::Select => "select", Tool::Arrow => "arrow", Tool::Rect => "rect", Tool::Text => "text", Tool::Number => "number",
        Tool::Highlight => "highlight", Tool::Redact => "redact", Tool::Mosaic => "mosaic", Tool::Crop => "crop",
        Tool::Ellipse => "ellipse", Tool::Line => "line", Tool::Pen => "pen",
    }
}
/// 02 §5.2 single-key tools.
fn tool_for_key(c: char) -> Option<Tool> {
    Some(match c {
        'v' => Tool::Select, 'a' => Tool::Arrow, 'r' => Tool::Rect, 't' => Tool::Text, 'n' => Tool::Number,
        'h' => Tool::Highlight, 'b' => Tool::Redact, 'm' => Tool::Mosaic, 'c' => Tool::Crop,
        'e' => Tool::Ellipse, 'l' => Tool::Line, 'p' => Tool::Pen, _ => return None,
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
        "edit" => { with(|a| a.show_editor()); spawn_open(capture::encode_png(img), Some("钉图.png".into())); }
        "copy" => {
            let dims = img.dimensions();
            let result = capture::encode_png(img).and_then(|png| platform::write_clipboard(&png, dims).map(|()| png));
            with(|a| match result {
                Ok(png) => { a.record_history(&png); a.system_toast("done", "已复制到剪贴板", &format!("{} × {}", dims.0, dims.1)); }
                Err(e) => a.system_toast("error", "复制失败", &e),
            });
        }
        "save" => {
            let dir = with(|a| a.settings.save_dir.clone()).flatten();
            let Some(path) = picker(|| platform::pick_save_path("钉图.png", dir.as_deref())) else { return };
            let path = platform::free_path(&path);
            let result = capture::encode_png(img).and_then(|png| platform::write_file(&path, &png).map(|()| png));
            with(|a| match result {
                Ok(png) => { a.record_history(&png); a.system_toast("done", "已保存", &path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()); }
                Err(e) => a.system_toast("error", "保存失败", &e),
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
    let (entry, h) = match ["history-open-", "history-copy-", "history-pin-", "history-delete-"].iter().find_map(|p| index(p).map(|i| (*p, i))) {
        Some((p, i)) => (with(|a| a.history_entry(i)).flatten().map(|e| (p, e)), with(|a| a.history()).flatten()),
        None => return,
    };
    let (Some((kind, e)), Some(h)) = (entry, h) else { return };
    match kind {
        "history-delete-" => { h.remove(&e); with(|a| a.open_history()); }
        _ => {
            let bytes = match h.read(&e) { Ok(b) => b, Err(err) => { with(|a| a.toast("error", "无法读取历史", &err, None)); return; } };
            match kind {
                "history-open-" => { with(|a| if let Some(ui) = a.ui() { ui.set_dialog(SharedString::new()); }); spawn_open(Ok(bytes), Some("历史截图.png".into())); }
                "history-copy-" => {
                    let result = shareready_core::decode_source(&bytes).and_then(|img| { let d = img.dimensions(); capture::encode_png(img).and_then(|png| platform::write_clipboard(&png, d)) });
                    with(|a| match result { Ok(()) => a.toast("done", "已复制到剪贴板", "来自截图历史", None), Err(err) => a.toast("error", "复制失败", &err, None) });
                }
                _ => {
                    let img = shareready_core::decode_source(&bytes);
                    with(|a| match img { Ok(img) => a.open_pin(img, None), Err(_) => a.toast("error", "无法钉图", "历史文件已损坏", None) });
                }
            }
        }
    }
}

/// One pooled overlay window for monitor `i`, with its callbacks wired.
fn new_overlay(i: usize) -> Option<CaptureOverlay> {
    let win = CaptureOverlay::new().ok()?;
    win.on_pointer_down(move |x, y| { with(|a| a.overlay_pointer(i, x, y, Phase::Down)); });
    win.on_pointer_move(move |x, y| { with(|a| a.overlay_pointer(i, x, y, Phase::Move)); });
    win.on_pointer_up(move |x, y| { with(|a| a.overlay_pointer(i, x, y, Phase::Up)); });
    win.on_confirm(|| { with(|a| a.confirm_capture()); });
    win.on_cancel(|| { with(|a| a.cancel_capture()); });
    win.on_action(|k| { let k = k.to_string(); with(|a| a.overlay_action(&k)); });
    Some(win)
}

fn canvas_centre(c: &Controller) -> Point { let (w, h) = c.area(); Point { x: w / 2.0, y: h / 2.0 } }
