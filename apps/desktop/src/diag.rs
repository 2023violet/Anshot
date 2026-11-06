//! Opt-in capture diagnostics. Only when `ANSHOT_DIAGNOSE_DIR` names a folder does Anshot
//! append to `anshot-diag.txt` there: timings of each capture start and end, and display
//! facts (size, scaling, HDR, colour depth, adapter, Windows build). Never pixels, window
//! titles, file paths or recognised text. Off by default: Anshot writes no log otherwise.
#![forbid(unsafe_code)]
use std::cell::RefCell;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

struct Run { source: &'static str, start: Instant, marks: Vec<String>, first_frame: Option<f64>, outcome: &'static str }

thread_local! { static RUN: RefCell<Option<Run>> = const { RefCell::new(None) }; }

fn dir() -> Option<&'static PathBuf> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| std::env::var_os("ANSHOT_DIAGNOSE_DIR").map(PathBuf::from).filter(|p| p.is_dir())).as_ref()
}

pub fn enabled() -> bool { dir().is_some() }

/// An experiment switch (`NAME=1`), honoured only while diagnostics are on.
pub fn switch(name: &str) -> bool { enabled() && std::env::var(name).is_ok_and(|v| v == "1") }

const SWITCHES: [&str; 3] = ["ANSHOT_DIAG_NO_CLOAK", "ANSHOT_DIAG_NO_SCRIM", "ANSHOT_DIAG_OVERLAY_PAD"];

fn write(line: &str) {
    let Some(d) = dir() else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(d.join("anshot-diag.txt")) {
        let _ = writeln!(f, "{line}");
    }
}

/// Once per start: what this run is on.
pub fn header() {
    if !enabled() { return; }
    let displays: Vec<String> = anshot_platform_windows::display_facts().iter().map(|d| format!(
        "{}x{}@{},{} hdr={} bits={} adapter=\"{}\"", d.bounds.w, d.bounds.h, d.bounds.x, d.bounds.y,
        d.hdr.map_or("unknown", |h| if h { "on" } else { "off" }), d.bits_per_color, d.adapter.replace('"', "'"))).collect();
    let on: Vec<&str> = SWITCHES.iter().copied().filter(|s| switch(s)).collect();
    write(&format!("start version={} os=\"{}\" renderer={} switches=[{}] displays=[{}]", env!("CARGO_PKG_VERSION"), anshot_platform_windows::windows_build(),
        std::env::var("SLINT_BACKEND").unwrap_or_else(|_| "default".into()), on.join(","), displays.join("; ")));
}

/// A capture starts: `source` is tray, hotkey, editor or delayed.
pub fn start(source: &'static str) {
    if !enabled() { return; }
    RUN.with(|r| *r.borrow_mut() = Some(Run { source, start: Instant::now(), marks: Vec::new(), first_frame: None, outcome: "other" }));
}

/// Time since the start, with an optional detail (`name=12.3` or `name=12.3(detail)`).
pub fn mark(name: &str, detail: Option<String>) {
    RUN.with(|r| if let Some(run) = r.borrow_mut().as_mut() {
        let ms = run.start.elapsed().as_secs_f64() * 1e3;
        run.marks.push(match detail { Some(d) => format!("{name}={ms:.1}({d})"), None => format!("{name}={ms:.1}") });
    });
}

/// An overlay finished rendering a frame; only the first after the start counts.
pub fn frame_rendered() {
    RUN.with(|r| if let Some(run) = r.borrow_mut().as_mut() {
        if run.first_frame.is_none() { run.first_frame = Some(run.start.elapsed().as_secs_f64() * 1e3); }
    });
}

/// How the capture ended (cancel, done, pin, ocr, long, error). The first one set stands, so
/// a failure that then cancels is still recorded as an error.
pub fn outcome(o: &'static str) { RUN.with(|r| if let Some(run) = r.borrow_mut().as_mut() { if run.outcome == "other" { run.outcome = o; } }); }

/// The capture is over: one line with everything recorded.
pub fn end(still_visible: usize) {
    let Some(run) = RUN.with(|r| r.borrow_mut().take()) else { return };
    let ms = run.start.elapsed().as_secs_f64() * 1e3;
    write(&format!("capture source={} {} first_frame={} end={ms:.1} outcome={} overlays_still_visible={still_visible}",
        run.source, run.marks.join(" "), run.first_frame.map_or("none".into(), |f| format!("{f:.1}")), run.outcome));
}
