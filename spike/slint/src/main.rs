//! Gate 03 spike: Slint renders the 03_VISUAL_SAMPLE editor skeleton (ui/app.slint).
//! Scale override: SLINT_SCALE_FACTOR env var (read by Slint itself).
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    AppWindow::new()?.run()
}
