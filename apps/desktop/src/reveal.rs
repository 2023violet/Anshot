//! Capture overlays reach the screen only once they have drawn their picture: each is shown
//! cloaked (the compositor renders it, nobody sees it) and uncloaked after its first frame,
//! or after `FALLBACK` at the latest. Without a frame callback (some renderers) an overlay is
//! shown directly, as before.
#![forbid(unsafe_code)]
use crate::{diag, CaptureOverlay};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::HashSet;
use std::time::Duration;

/// Never keep an overlay invisible longer than this, frame or not.
pub const FALLBACK: Duration = Duration::from_millis(300);

struct Pending { index: usize, hwnd: isize, overlay: slint::Weak<CaptureOverlay>, focus: bool }

thread_local! {
    static PENDING: RefCell<Vec<Pending>> = const { RefCell::new(Vec::new()) };
    /// Overlays whose renderer reports finished frames.
    static NOTIFIED: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    static GENERATION: RefCell<u64> = const { RefCell::new(0) };
}

/// Hook overlay `index`'s frame callback (once, when the overlay is created).
pub fn watch(index: usize, win: &CaptureOverlay) {
    let ok = win.window().set_rendering_notifier(move |state, _| {
        if matches!(state, slint::RenderingState::AfterRendering) { diag::frame_rendered(); drawn(index); }
    }).is_ok();
    if ok { NOTIFIED.with(|n| n.borrow_mut().insert(index)); }
}

/// Cloak overlay `index` before it is shown, if it can be revealed by its first frame.
/// Returns false when it was not cloaked (the caller then reveals it right after showing).
pub fn hide_until_drawn(index: usize, win: &CaptureOverlay, hwnd: Option<isize>, focus: bool) -> bool {
    win.set_revealed(false);
    if diag::switch("ANSHOT_DIAG_NO_CLOAK") || !NOTIFIED.with(|n| n.borrow().contains(&index)) { return false; }
    let Some(hwnd) = hwnd else { return false };
    if !anshot_platform_windows::set_cloaked(hwnd, true) { return false; }
    PENDING.with(|p| p.borrow_mut().push(Pending { index, hwnd, overlay: win.as_weak(), focus }));
    true
}

/// After all overlays of a capture were shown: the safety net.
pub fn arm_fallback() {
    let generation = GENERATION.with(|g| *g.borrow());
    slint::Timer::single_shot(FALLBACK, move || {
        if GENERATION.with(|g| *g.borrow()) == generation { reveal_all("fallback", true); }
    });
}

/// Show an overlay that was not cloaked (or whose cloak failed).
pub fn reveal_now(win: &CaptureOverlay) { win.set_revealed(true); }

fn drawn(index: usize) {
    let hit = PENDING.with(|p| p.borrow().iter().any(|q| q.index == index));
    // Not inside the renderer's callback: reveal on the next turn of the event loop.
    if hit { slint::Timer::single_shot(Duration::ZERO, move || reveal(index, "frame", true)); }
}

fn reveal(index: usize, why: &str, focus: bool) {
    let found = PENDING.with(|p| { let mut p = p.borrow_mut(); p.iter().position(|q| q.index == index).map(|i| p.remove(i)) });
    let Some(q) = found else { return };
    anshot_platform_windows::set_cloaked(q.hwnd, false);
    if let Some(o) = q.overlay.upgrade() { o.set_revealed(true); }
    // Keyboard focus could not be relied on while cloaked: give it again now.
    if focus && q.focus { anshot_platform_windows::restore_foreground(q.hwnd); }
    diag::mark("revealed", Some(format!("monitor{} {why}", q.index)));
}

/// Uncloak everything still waiting (fallback, or the capture ended before a frame came).
fn reveal_all(why: &str, focus: bool) {
    let indexes: Vec<usize> = PENDING.with(|p| p.borrow().iter().map(|q| q.index).collect());
    for i in indexes { reveal(i, why, focus); }
}

/// A capture ended and its overlays are hidden: nothing may stay cloaked, focus goes
/// nowhere, and a pending fallback no longer applies.
pub fn finish() {
    reveal_all("end", false);
    GENERATION.with(|g| *g.borrow_mut() += 1);
}
