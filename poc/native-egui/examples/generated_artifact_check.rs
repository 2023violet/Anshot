//! Explicit fixed synthetic release regression; never opens UI or OS clipboard.
#![forbid(unsafe_code)]
#[cfg(feature="performance-evidence")] #[path="../src/app.rs"] mod app;
#[cfg(feature="performance-evidence")] #[path="../src/display.rs"] mod display;
#[cfg(feature="performance-evidence")] #[path="../src/trace.rs"] mod trace;
#[cfg(feature="performance-evidence")] #[path="../src/input.rs"] mod input;
#[cfg(feature="performance-evidence")] #[path="../src/fonts.rs"] mod fonts;
#[cfg(feature="performance-evidence")] #[path="../src/visual.rs"] mod visual;
#[cfg(feature="performance-evidence")]
fn main(){if let Err(error)=app::performance_driver::run_large_artifact(){eprintln!("{error}");std::process::exit(2);}}
#[cfg(not(feature="performance-evidence"))]
fn main(){eprintln!("Requires --features performance-evidence; fixed synthetic source only");std::process::exit(2);}
