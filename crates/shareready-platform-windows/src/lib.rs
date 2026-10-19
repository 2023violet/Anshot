//! Small OS boundary for identity-bound Windows delivery; no pixel processing.
#![deny(unsafe_op_in_unsafe_fn)]

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ClipboardImageFormat { Png,Dib }
#[derive(Debug)]
pub struct ClipboardImage {pub format:ClipboardImageFormat,pub bytes:Vec<u8>}

#[cfg(windows)]
mod clipboard;
#[cfg(windows)]
pub use clipboard::read_clipboard_image;

#[cfg(not(windows))]
pub fn read_clipboard_image(_png_limit:usize,_dib_limit:usize)->std::io::Result<ClipboardImage>{
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"Windows clipboard adapter requires Windows"))
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::save_new;

#[cfg(not(windows))]
pub fn save_new(_path:&std::path::Path,_bytes:&[u8])->std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"Windows delivery adapter requires Windows"))
}
