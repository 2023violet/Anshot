//! Fixed synthetic file intake through the ordinary source worker; no OS input.
#![forbid(unsafe_code)]
#[cfg(feature="performance-evidence")] #[path="../src/app.rs"] mod app;
#[cfg(feature="performance-evidence")] #[path="../src/display.rs"] mod display;
#[cfg(feature="performance-evidence")] #[path="../src/trace.rs"] mod trace;
#[cfg(feature="performance-evidence")] #[path="../src/input.rs"] mod input;
#[cfg(feature="performance-evidence")] #[path="../src/fonts.rs"] mod fonts;
#[cfg(feature="performance-evidence")] #[path="../src/visual.rs"] mod visual;
#[cfg(feature="performance-evidence")]
fn main()->eframe::Result{app::source_driver::run()}
#[cfg(not(feature="performance-evidence"))]
fn main(){eprintln!("Requires --features performance-evidence; fixed synthetic sources only");std::process::exit(2);}
