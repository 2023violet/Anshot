//! Opt-in synthetic diagnostic build using the actual NativeApp and renderer.
#![forbid(unsafe_code)]
#[cfg(feature="performance-evidence")] #[path="../src/app.rs"] mod app;
#[cfg(feature="performance-evidence")] #[path="../src/display.rs"] mod display;
#[cfg(feature="performance-evidence")] #[path="../src/trace.rs"] mod trace;
#[cfg(feature="performance-evidence")] #[path="../src/fonts.rs"] mod fonts;
#[cfg(feature="performance-evidence")] #[path="../src/input.rs"] mod input;
#[cfg(feature="performance-evidence")] #[path="../src/visual.rs"] mod visual;
#[cfg(feature="performance-evidence")]
fn main()->eframe::Result{app::performance_driver::run()}
#[cfg(not(feature="performance-evidence"))]
fn main(){eprintln!("This explicit synthetic tool requires --features performance-evidence");std::process::exit(2);}
