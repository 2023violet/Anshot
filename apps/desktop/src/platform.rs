//! OS boundaries: file dialogs, clipboard image in/out, new-file save.
//! Writers receive only Rust-generated PNG bytes from a live Editor ticket.
#![forbid(unsafe_code)]
use anshot_core::{decode_generated_png, dib_to_png, MAX_DIB_BYTES, MAX_INPUT_BYTES};
use anshot_platform_windows::{read_clipboard_image, save_new, ClipboardImageFormat};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

pub fn pick_image() -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("打开图片").add_filter("图片", &["png", "jpg", "jpeg"]).pick_file()
}

pub fn pick_save_path(suggested: &str, dir: Option<&Path>) -> Option<PathBuf> {
    let mut d = rfd::FileDialog::new().set_title("另存为新 PNG").add_filter("PNG 图片", &["png"]).set_file_name(suggested);
    if let Some(dir) = dir.filter(|d| d.is_dir()) { d = d.set_directory(dir); }
    d.save_file()
}

pub fn pick_folder() -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("选择默认保存文件夹").pick_folder()
}

/// Bounded read of a user-chosen file; Core re-validates size and format.
pub fn read_image_file(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let err = |e: std::io::Error| crate::messages::read_file(e.kind());
    if std::fs::symlink_metadata(path).map_err(err)?.file_type().is_symlink() { return Err("不支持打开快捷链接，请选择图片文件本身".into()); }
    let file = std::fs::File::open(path).map_err(err)?;
    if !file.metadata().map_err(err)?.is_file() { return Err("这不是一个文件".into()); }
    // Read at most one byte past the limit from the opened handle: a file that grows
    // after the check is still bounded.
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(err)?;
    if bytes.len() > MAX_INPUT_BYTES { return Err("文件超过 25 MB".into()); }
    Ok(bytes)
}

/// Clipboard image as PNG/JPEG-compatible bytes for Core intake.
pub fn read_clipboard() -> Result<Vec<u8>, String> {
    let img = read_clipboard_image(MAX_INPUT_BYTES, MAX_DIB_BYTES).map_err(|_| "剪贴板里没有图片，先复制一张图片再粘贴".to_string())?;
    match img.format {
        ClipboardImageFormat::Png => Ok(img.bytes),
        ClipboardImageFormat::Dib => dib_to_png(&img.bytes).map_err(|_| "剪贴板里的图片无法读取".to_string()),
    }
}

pub fn write_clipboard(png: &[u8], dims: (u32, u32)) -> Result<(), String> {
    let pixels = decode_generated_png(png, dims).map_err(|_| "生成的图片未通过校验，未写入剪贴板".to_string())?;
    let data = arboard::ImageData { width: pixels.width() as usize, height: pixels.height() as usize, bytes: Cow::Owned(pixels.into_raw()) };
    let busy = |_| "剪贴板正被其他程序占用，请稍后重试".to_string();
    let mut clipboard = arboard::Clipboard::new().map_err(busy)?;
    clipboard.set_image(data).map_err(busy)
}

/// The dialog may already have asked "replace?"; Anshot never overwrites, so an
/// existing name becomes "name (2).png", "name (3).png", ...
pub fn free_path(path: &Path) -> PathBuf {
    if !path.exists() { return path.to_path_buf(); }
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Anshot".into());
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "png".into());
    (2..10_000).map(|n| path.with_file_name(format!("{stem} ({n}).{ext}"))).find(|p| !p.exists()).unwrap_or_else(|| path.to_path_buf())
}

pub fn write_file(path: &Path, png: &[u8]) -> Result<(), String> {
    save_new(path, png).map_err(|e| crate::messages::write_file(e.kind()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_reads_are_bounded_and_refuse_folders() {
        let dir = std::env::temp_dir().join(format!("sr-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(read_image_file(&dir).is_err(), "a folder is not an image");
        let big = dir.join("big.bin");
        std::fs::File::create(&big).unwrap().set_len(MAX_INPUT_BYTES as u64 + 1).unwrap();
        assert_eq!(read_image_file(&big).unwrap_err(), "文件超过 25 MB");
        let ok = dir.join("ok.bin");
        std::fs::write(&ok, b"abc").unwrap();
        assert_eq!(read_image_file(&ok).unwrap(), b"abc");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn free_path_never_returns_an_existing_file() {
        let dir = std::env::temp_dir().join(format!("sr-free-path-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("shot.png");
        assert_eq!(free_path(&a), a);
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(dir.join("shot (2).png"), b"x").unwrap();
        assert_eq!(free_path(&a), dir.join("shot (3).png"));
        for f in ["shot.png", "shot (2).png"] { std::fs::remove_file(dir.join(f)).unwrap(); }
        std::fs::remove_dir(&dir).unwrap();
    }
}
