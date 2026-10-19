//! OS boundaries: file dialogs, clipboard image in/out, new-file save.
//! Writers receive only Rust-generated PNG bytes from a live Editor ticket.
#![forbid(unsafe_code)]
use shareready_core::{decode_generated_png, dib_to_png, MAX_DIB_BYTES, MAX_INPUT_BYTES};
use shareready_platform_windows::{read_clipboard_image, save_new, ClipboardImageFormat};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

pub fn pick_image() -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("打开图片").add_filter("图片", &["png", "jpg", "jpeg"]).pick_file()
}

pub fn pick_save_path(suggested: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("另存为新 PNG").add_filter("PNG 图片", &["png"]).set_file_name(suggested).save_file()
}

/// Bounded read of a user-chosen file; Core re-validates size and format.
pub fn read_image_file(path: &Path) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("无法读取文件：{e}"))?;
    if meta.len() > MAX_INPUT_BYTES as u64 { return Err("文件超过 25 MB".into()); }
    std::fs::read(path).map_err(|e| format!("无法读取文件：{e}"))
}

/// Clipboard image as PNG/JPEG-compatible bytes for Core intake.
pub fn read_clipboard() -> Result<Vec<u8>, String> {
    let img = read_clipboard_image(MAX_INPUT_BYTES, MAX_DIB_BYTES).map_err(|e| format!("剪贴板中没有可用图片（{e}）"))?;
    match img.format {
        ClipboardImageFormat::Png => Ok(img.bytes),
        ClipboardImageFormat::Dib => dib_to_png(&img.bytes),
    }
}

pub fn write_clipboard(png: &[u8], dims: (u32, u32)) -> Result<(), String> {
    let pixels = decode_generated_png(png, dims)?;
    let data = arboard::ImageData { width: pixels.width() as usize, height: pixels.height() as usize, bytes: Cow::Owned(pixels.into_raw()) };
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("剪贴板被其他程序占用（{e}）"))?;
    clipboard.set_image(data).map_err(|e| format!("剪贴板被其他程序占用（{e}）"))
}

pub fn write_file(path: &Path, png: &[u8]) -> Result<(), String> {
    save_new(path, png).map_err(|e| match e.kind() {
        std::io::ErrorKind::AlreadyExists => "同名文件已存在，请换个名字（不会覆盖原文件）".to_string(),
        std::io::ErrorKind::PermissionDenied => "没有写入权限".to_string(),
        _ => e.to_string(),
    })
}
