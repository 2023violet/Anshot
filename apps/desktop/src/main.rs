//! ShareReady desktop (B1): open -> redact -> copy/save, Slint UI over the Rust editor.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Slint's generated code needs `unsafe`; our own modules each `forbid` it.
#![deny(unsafe_code)]

mod canvas;
mod controller;
mod display;
mod platform;

use canvas::Zoom;
use controller::{Controller, Delivery, Notice, Tool};
use image::RgbaImage;
use shareready_editor::{EditError, Point, ReviewKey, ReviewResult, SourceCandidate};
use slint::{CloseRequestResponse, ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

slint::include_modules!();

type Decoded = Result<(SourceCandidate, RgbaImage), String>;

enum Pending { Replace(SourceCandidate, RgbaImage, Option<String>), Close }

#[derive(Clone, Copy)]
enum ToastAction { Retry(Delivery) }

struct App {
    ctl: Controller,
    ui: slint::Weak<AppWindow>,
    pending: Option<Pending>,
    save_path: Option<PathBuf>,
    close_after_copy: bool,
    toast_seq: u64,
    toast_action: Option<ToastAction>,
}

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
        ui.set_tool(match ctl.tool { Tool::Select => "select", Tool::Redact => "redact" }.into());
        ui.set_busy(match ctl.busy() { Some(Delivery::Copy) => "copy", Some(Delivery::Save) => "save", None => "" }.into());
        ui.set_can_deliver(ctl.can_deliver());
        ui.set_narrow(ctl.area().0 < 900.0);
        ui.set_tiny(ctl.area().0 < 720.0);
        let (Some(e), Some(view)) = (ctl.editor.as_ref(), ctl.view()) else {
            ui.set_masks(ModelRc::default());
            ui.set_has_selection(false);
            ui.set_dims_text(SharedString::new());
            ui.set_can_undo(false);
            ui.set_can_redo(false);
            return;
        };
        ui.set_can_undo(e.can_undo() && !ctl.dragging());
        ui.set_can_redo(e.can_redo() && !ctl.dragging());
        ui.set_img_x(view.x as f32);
        ui.set_img_y(view.y as f32);
        ui.set_img_w(view.width() as f32);
        ui.set_img_h(view.height() as f32);
        let (w, h) = e.dimensions();
        ui.set_dims_text(format!("{w} × {h}").into());
        ui.set_zoom_fit(ctl.zoom == Zoom::Fit);
        ui.set_zoom_text(format!("{}%", (view.scale * 100.0).round()).into());
        let boxes: Vec<MaskBox> = ctl.display_masks().into_iter().map(|(_, r, selected)| {
            let (x, y, w, h) = view.rect(r);
            MaskBox { x: x as f32, y: y as f32, w: w as f32, h: h as f32, selected }
        }).collect();
        ui.set_mask_count(e.masks().len() as i32);
        let selected = ctl.display_masks().into_iter().find(|m| m.2).map(|m| m.1);
        ui.set_has_selection(selected.is_some());
        if let Some(r) = selected {
            ui.set_sel_x(r.x0.to_string().into());
            ui.set_sel_y(r.y0.to_string().into());
            ui.set_sel_w((r.x1 - r.x0).to_string().into());
            ui.set_sel_h((r.y1 - r.y0).to_string().into());
        }
        ui.set_masks(ModelRc::new(VecModel::from(boxes)));
        match ctl.draft() {
            Some(r) => {
                let (x, y, w, h) = view.rect(r);
                ui.set_draft(MaskBox { x: x as f32, y: y as f32, w: w as f32, h: h as f32, selected: false });
                ui.set_draft_visible(true);
            }
            None => ui.set_draft_visible(false),
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
            self.toast("error", "无法打开", &format!("{e:?}"), None);
            return;
        }
        let (w, h) = preview.dimensions();
        let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
        buf.make_mut_bytes().copy_from_slice(preview.as_raw());
        if let Some(ui) = self.ui() { ui.set_preview(Image::from_rgba8(buf)); }
        self.clear_toast();
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
            Err(e) => { self.toast("error", "无法开始", &format!("{e:?}"), None); false }
        };
        self.refresh();
        started
    }
    fn review_done(&mut self, key: &ReviewKey, result: Result<ReviewResult, EditError>) {
        match self.ctl.review_finished(key, result) {
            Ok((Delivery::Copy, ticket)) => {
                let outcome = self.ctl.deliver(ticket, platform::write_clipboard);
                self.after_delivery(Delivery::Copy, outcome, None);
            }
            Ok((Delivery::Save, ticket)) => {
                let Some(path) = self.save_path.take() else { self.ctl.abandon_delivery(ticket); self.refresh(); return };
                let outcome = self.ctl.deliver(ticket, |png, _| platform::write_file(&path, png));
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
                self.after_delivery(Delivery::Save, outcome, name);
            }
            Err(Notice::Changed) => {
                self.close_after_copy = false;
                self.toast("warn", "内容已变化", "请重新复制或保存", None);
            }
            Err(Notice::Failed { title, detail }) => {
                self.close_after_copy = false;
                self.toast("error", &title, &detail, None);
            }
            Err(Notice::Ignored) => {}
        }
        self.refresh();
    }
    fn after_delivery(&mut self, kind: Delivery, outcome: Result<(), String>, name: Option<String>) {
        let n = self.ctl.editor.as_ref().map_or(0, |e| e.masks().len());
        let (w, h) = self.ctl.editor.as_ref().map_or((0, 0), |e| e.dimensions());
        match (kind, outcome) {
            (Delivery::Copy, Ok(())) => {
                if self.close_after_copy { let _ = slint::quit_event_loop(); return; }
                self.toast("done", "已复制到剪贴板", &format!("{w} × {h} · 已遮挡 {n} 处"), None);
            }
            (Delivery::Save, Ok(())) => { self.close_after_copy = false; self.toast("done", "已保存", &name.unwrap_or_default(), None) }
            (Delivery::Copy, Err(e)) => {
                self.close_after_copy = false;
                self.toast("error", "复制失败", &e, Some(("重试", ToastAction::Retry(Delivery::Copy))));
            }
            (Delivery::Save, Err(e)) => { self.close_after_copy = false; self.toast("error", "保存失败", &e, None) }
        }
    }

    fn key(&mut self, text: &str, ctrl: bool, shift: bool, other: bool) -> bool {
        use slint::platform::Key;
        let is = |k: Key| text == SharedString::from(k).as_str();
        if let Some(ui) = self.ui() {
            if ui.get_dialog() != "" {
                if is(Key::Escape) { self.dialog_choice("cancel"); return true; }
                if is(Key::Return) && ui.get_dialog() == "close" && self.ctl.can_deliver() { self.dialog_choice("copy-close"); return true; }
                return false;
            }
        }
        if other { return false; } // Alt / Win combinations are not ours.
        if ctrl {
            // Some platforms deliver Ctrl+letter as a control character.
            let c = text.chars().next().map(|c| if (c as u32) < 27 && c as u32 > 0 { (b'a' + c as u8 - 1) as char } else { c.to_ascii_lowercase() });
            match c {
                Some('z') if shift => { self.ctl.redo(); }
                Some('z') => { self.ctl.undo(); }
                Some('y') => { self.ctl.redo(); }
                Some('c') => { self.start(Delivery::Copy); return true; }
                Some('0') => self.ctl.zoom = Zoom::Fit,
                Some('1') => self.ctl.zoom = Zoom::Actual,
                _ => return false, // Ctrl+O / Ctrl+V / Ctrl+S are handled outside the borrow.
            }
            self.refresh();
            return true;
        }
        let step = if shift { 10 } else { 1 };
        let handled = if is(Key::Delete) || is(Key::Backspace) { self.ctl.delete_selected() }
            else if is(Key::LeftArrow) { self.ctl.nudge(-step, 0) }
            else if is(Key::RightArrow) { self.ctl.nudge(step, 0) }
            else if is(Key::UpArrow) { self.ctl.nudge(0, -step) }
            else if is(Key::DownArrow) { self.ctl.nudge(0, step) }
            else if is(Key::Escape) { if self.ctl.dragging() { self.ctl.cancel_drag() } else { self.ctl.deselect() }; true }
            else {
                match text.chars().next().map(|c| c.to_ascii_lowercase()) {
                    Some('v') => { self.ctl.tool = Tool::Select; true }
                    Some('b') => { self.ctl.tool = Tool::Redact; true }
                    _ => false,
                }
            };
        self.refresh();
        handled
    }

    fn dialog_choice(&mut self, choice: &str) {
        if let Some(ui) = self.ui() { ui.set_dialog(SharedString::new()); }
        self.ctl.cancel_drag();
        match (self.pending.take(), choice) {
            (Some(Pending::Replace(c, p, n)), "discard") => self.apply_source(c, p, n),
            (Some(Pending::Close), "discard") => { let _ = slint::quit_event_loop(); }
            // Set only once the copy really started; any refusal leaves the app open.
            (Some(Pending::Close), "copy-close") => { if self.start(Delivery::Copy) { self.close_after_copy = true; } }
            _ => {}
        }
        self.refresh();
    }
}

fn spawn_open(bytes: Result<Vec<u8>, String>, name: Option<String>) {
    let started = with(|a| {
        if a.ctl.opening { return false; }
        a.ctl.opening = true;
        a.refresh();
        true
    }).unwrap_or(false);
    if !started { return; }
    std::thread::spawn(move || {
        let decoded: Decoded = bytes.and_then(|b| {
            let candidate = SourceCandidate::decode(b).map_err(|e| match e {
                EditError::CoreFailure(m) if m.contains("magic") => "格式不支持（仅 PNG / JPEG）".to_string(),
                EditError::CoreFailure(m) if m.contains("oversized") => "文件超过 25 MB".to_string(),
                EditError::CoreFailure(m) if m.contains("bounds") => "图片超过 2400 万像素".to_string(),
                other => format!("文件已损坏（{other:?}）"),
            })?;
            let pixels = shareready_core::decode_source(candidate.bytes())?;
            Ok((candidate, display::preview(pixels)))
        });
        let _ = slint::invoke_from_event_loop(move || { with(|a| a.source_ready(decoded, name)); });
    });
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
    if with(|a| a.ui().is_some_and(|ui| ui.get_dialog() != "")).unwrap_or(false) { return; }
    match cmd {
        // Blocking dialogs and clipboard reads run outside the state borrow.
        "open" => { if let Some(p) = picker(platform::pick_image) { open_path(p); } }
        "paste" => spawn_open(platform::read_clipboard(), Some("剪贴板图片.png".into())),
        "save" => {
            if !with(|a| a.ctl.can_deliver()).unwrap_or(false) {
                with(|a| a.start(Delivery::Save));
                return;
            }
            if let Some(path) = picker(|| platform::pick_save_path(&suggested_name())) {
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
                    "tool-redact" => a.ctl.tool = Tool::Redact,
                    "zoom-fit" => a.ctl.zoom = Zoom::Fit,
                    "zoom-actual" => a.ctl.zoom = Zoom::Actual,
                    "toast-action" => match a.toast_action { Some(ToastAction::Retry(k)) => { a.start(k); } None => {} },
                    _ => {}
                }
                a.refresh();
            });
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    APP.with(|cell| *cell.borrow_mut() = Some(App {
        ctl: Controller::default(), ui: ui.as_weak(), pending: None, save_path: None,
        close_after_copy: false, toast_seq: 0, toast_action: None,
    }));

    ui.on_pointer_down(|x, y| { with(|a| { a.ctl.pointer_down(Point { x: x.into(), y: y.into() }); a.refresh(); }); });
    ui.on_pointer_move(|x, y| { with(|a| if a.ctl.dragging() { a.ctl.pointer_move(Point { x: x.into(), y: y.into() }); a.refresh(); }); });
    ui.on_pointer_up(|| { with(|a| { a.ctl.pointer_up(); a.refresh(); }); });
    ui.on_canvas_size(|w, h| { with(|a| { a.ctl.set_area(w.into(), h.into()); a.refresh(); }); });
    ui.on_command(|c| command(c.as_str()));
    ui.on_dialog_choice(|c| { with(|a| a.dialog_choice(c.as_str())); });
    ui.on_key(|text, ctrl, shift, other| {
        let dialog_open = with(|a| a.ui().is_some_and(|ui| ui.get_dialog() != "")).unwrap_or(true);
        if ctrl && !other && !dialog_open {
            let c = text.chars().next().map(|c| if (c as u32) < 27 && c as u32 > 0 { (b'a' + c as u8 - 1) as char } else { c.to_ascii_lowercase() });
            match c {
                Some('o') => { command("open"); return true; }
                Some('v') => { command("paste"); return true; }
                Some('s') => { command("save"); return true; }
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
            } else { CloseRequestResponse::HideWindow }
        }).unwrap_or(CloseRequestResponse::HideWindow)
    });

    // "Open with" / command-line path.
    if let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) { open_path(path); }
    with(|a| a.refresh());
    ui.run()
}
