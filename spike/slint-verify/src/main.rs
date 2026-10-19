//! Gate 03 Slint verification harness. Emits `VERIFY ...` lines on stdout.
//! Env: VERIFY_UPDATES=<n> runs n full 24MP updates after startup, then idles.
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

slint::include_modules!();

const W: u32 = 6000;
const H: u32 = 4000;

fn base_pixels() -> Vec<u8> {
    let mut v = vec![0u8; (W * H * 4) as usize];
    for y in 0..H {
        for x in 0..W {
            let i = ((y * W + x) * 4) as usize;
            let grid = x % 200 == 0 || y % 200 == 0;
            v[i] = if grid { 0x90 } else { (x * 255 / W) as u8 };
            v[i + 1] = if grid { 0x90 } else { (y * 255 / H) as u8 };
            v[i + 2] = 0xd0;
            v[i + 3] = 0xff;
        }
    }
    v
}

/// Simulates one Rust composite pass: a redaction block moves, then the whole
/// frame is copied into a fresh Slint buffer (worst case: full upload).
fn composite(base: &[u8], step: u32) -> Image {
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(W, H);
    let bytes = buf.make_mut_bytes();
    bytes.copy_from_slice(base);
    let (bx, by) = (200 + step * 150, 600 + step * 60);
    for y in by..(by + 400).min(H) {
        let row = ((y * W + bx) * 4) as usize;
        let end = ((y * W + (bx + 900).min(W)) * 4) as usize;
        for px in bytes[row..end].chunks_exact_mut(4) { px.copy_from_slice(&[0x1a, 0x19, 0x15, 0xff]); }
    }
    Image::from_rgba8(buf)
}

#[derive(Clone, Copy, Debug)]
enum Grab { Body, Handle(f32, f32) }

fn main() -> Result<(), slint::PlatformError> {
    let ui = VerifyWindow::new()?;
    let updates: u32 = std::env::var("VERIFY_UPDATES").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    println!("VERIFY start backend={} scale_env={}", std::env::var("SLINT_BACKEND").unwrap_or_else(|_| "default".into()),
        std::env::var("SLINT_SCALE_FACTOR").unwrap_or_else(|_| "none".into()));

    let t = Instant::now();
    let base = Rc::new(base_pixels());
    println!("VERIFY base_generated_ms={:.1}", t.elapsed().as_secs_f64() * 1e3);
    let t = Instant::now();
    ui.set_canvas_image(composite(&base, 0));
    println!("VERIFY first_image_set_ms={:.1}", t.elapsed().as_secs_f64() * 1e3);

    // --- 24MP update loop timed to the end of the following rendered frame ---
    let pending: Rc<Cell<Option<(u32, Instant, f64)>>> = Rc::new(Cell::new(None));
    let frames = Rc::new(Cell::new(0u32));
    {
        let (pending, frames, weak, base0) = (pending.clone(), frames.clone(), ui.as_weak(), base.clone());
        let base_r = base0;
        let res = ui.window().set_rendering_notifier(move |state, _| {
            let base = &base_r;
            if let slint::RenderingState::AfterRendering = state {
                frames.set(frames.get() + 1);
                if frames.get() == 1 { println!("VERIFY first_frame_rendered"); }
                if let Some((step, start, set_ms)) = pending.take() {
                    println!("VERIFY update step={step} set_ms={set_ms:.1} to_frame_ms={:.1}", start.elapsed().as_secs_f64() * 1e3);
                    if step < updates {
                        let (pending, weak, base) = (pending.clone(), weak.clone(), base.clone());
                        slint::Timer::single_shot(std::time::Duration::ZERO, move || schedule(&weak, &base, &pending, step + 1));
                    } else { println!("VERIFY updates_done"); }
                } else if frames.get() == 2 && updates > 0 {
                    let (pending, weak, base) = (pending.clone(), weak.clone(), base.clone());
                    slint::Timer::single_shot(std::time::Duration::from_millis(500), move || schedule(&weak, &base, &pending, 1));
                }
            }
        });
        println!("VERIFY rendering_notifier={}", if res.is_ok() { "supported" } else { "unsupported" });
        if res.is_err() && updates > 0 {
            // Software renderer: time set + an event-loop turn (lower bound, no frame confirmation).
            let (weak, base) = (ui.as_weak(), base.clone());
            slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                for step in 1..=updates {
                    let s = Instant::now();
                    if let Some(ui) = weak.upgrade() { ui.set_canvas_image(composite(&base, step)); ui.window().request_redraw(); }
                    println!("VERIFY update step={step} set_ms={:.1} to_frame_ms=NA", s.elapsed().as_secs_f64() * 1e3);
                }
                println!("VERIFY updates_done");
            });
        }
    }

    // --- Rust-side hit test + drag state machine (logical px) ---
    let drag: Rc<RefCell<Option<(Grab, f32, f32, [f32; 4])>>> = Rc::new(RefCell::new(None));
    {
        let (drag, weak) = (drag.clone(), ui.as_weak());
        ui.on_pointer_down(move |x, y| {
            let ui = weak.upgrade().unwrap();
            let r = [ui.get_sel_x(), ui.get_sel_y(), ui.get_sel_w(), ui.get_sel_h()];
            let mut grab = None;
            for fx in [0.0f32, 0.5, 1.0] { for fy in [0.0f32, 0.5, 1.0] {
                if fx == 0.5 && fy == 0.5 { continue; }
                let (hx, hy) = (r[0] + r[2] * fx, r[1] + r[3] * fy);
                if (x - hx).abs() <= 8.0 && (y - hy).abs() <= 8.0 { grab = Some(Grab::Handle(fx, fy)); }
            }}
            if grab.is_none() && x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3] { grab = Some(Grab::Body); }
            println!("VERIFY down x={x:.1} y={y:.1} grab={grab:?}");
            *drag.borrow_mut() = grab.map(|g| (g, x, y, r));
        });
    }
    {
        let (drag, weak) = (drag.clone(), ui.as_weak());
        ui.on_pointer_move(move |x, y| {
            let Some((g, sx, sy, r)) = *drag.borrow() else { return };
            let ui = weak.upgrade().unwrap();
            let (dx, dy) = ((x - sx).round(), (y - sy).round());
            match g {
                Grab::Body => { ui.set_sel_x(r[0] + dx); ui.set_sel_y(r[1] + dy); }
                Grab::Handle(fx, fy) => {
                    let (mut nx, mut ny, mut nw, mut nh) = (r[0], r[1], r[2], r[3]);
                    if fx == 0.0 { nx = r[0] + dx; nw = r[2] - dx; } else if fx == 1.0 { nw = r[2] + dx; }
                    if fy == 0.0 { ny = r[1] + dy; nh = r[3] - dy; } else if fy == 1.0 { nh = r[3] + dy; }
                    ui.set_sel_x(nx); ui.set_sel_y(ny); ui.set_sel_w(nw.max(1.0)); ui.set_sel_h(nh.max(1.0));
                }
            }
        });
    }
    {
        let (drag, weak) = (drag.clone(), ui.as_weak());
        ui.on_pointer_up(move || {
            let ui = weak.upgrade().unwrap();
            let had = drag.borrow_mut().take().is_some();
            println!("VERIFY up grabbed={had} sel={} {} {} {}", ui.get_sel_x(), ui.get_sel_y(), ui.get_sel_w(), ui.get_sel_h());
        });
    }
    ui.on_text_edited(|s| println!("VERIFY text={s}"));
    ui.on_tool_clicked(|s| println!("VERIFY tool={s}"));
    ui.run()
}

fn schedule(weak: &slint::Weak<VerifyWindow>, base: &Rc<Vec<u8>>, pending: &Rc<Cell<Option<(u32, Instant, f64)>>>, step: u32) {
    let Some(ui) = weak.upgrade() else { return };
    let start = Instant::now();
    let img = composite(base, step);
    ui.set_canvas_image(img);
    let set_ms = start.elapsed().as_secs_f64() * 1e3;
    pending.set(Some((step, start, set_ms)));
    ui.window().request_redraw();
}
