//! Stage V2-00 test-only Rust reference: export the V1 native decoder's normalized
//! source preview without modifying the shipping product code or CLI behavior.
use std::{env, fs, io};
use shareready_e1::preview_png;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 { return Err("usage: v1_baseline_preview INPUT OUTPUT".into()); }
    let source = fs::read(&args[0])?;
    let png = preview_png(&source).map_err(io::Error::other)?;
    fs::write(&args[1], png)?;
    Ok(())
}
