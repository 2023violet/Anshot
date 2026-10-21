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
mod autostart;
#[cfg(windows)]
pub use autostart::{autostart,migrate_legacy_autostart,set_autostart};
#[cfg(not(windows))]
pub fn autostart()->std::io::Result<Option<String>>{Ok(None)}
#[cfg(not(windows))]
pub fn set_autostart(_command:Option<&str>)->std::io::Result<()>{Err(std::io::ErrorKind::Unsupported.into())}
#[cfg(not(windows))]
pub fn migrate_legacy_autostart(_command:&str)->std::io::Result<bool>{Ok(false)}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::save_new;

#[cfg(not(windows))]
pub fn save_new(_path:&std::path::Path,_bytes:&[u8])->std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"Windows delivery adapter requires Windows"))
}

/// Global capture shortcut choices offered in settings.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Hotkey { CtrlShiftA, CtrlAltA, CtrlShiftX, PrintScreen }
impl Hotkey {
    pub const ALL:[Hotkey;4]=[Hotkey::CtrlShiftA,Hotkey::CtrlAltA,Hotkey::CtrlShiftX,Hotkey::PrintScreen];
    pub fn label(self)->&'static str{match self{Hotkey::CtrlShiftA=>"Ctrl+Shift+A",Hotkey::CtrlAltA=>"Ctrl+Alt+A",Hotkey::CtrlShiftX=>"Ctrl+Shift+X",Hotkey::PrintScreen=>"PrtSc"}}
}

/// Physical virtual-desktop rectangle.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ScreenRect {pub x:i32,pub y:i32,pub w:i32,pub h:i32}
impl ScreenRect {
    pub fn contains(self,x:i32,y:i32)->bool{x>=self.x && y>=self.y && x<self.x+self.w && y<self.y+self.h}
    pub fn intersect(self,o:ScreenRect)->Option<ScreenRect>{
        let (l,t)=(self.x.max(o.x),self.y.max(o.y));
        let (r,b)=((self.x+self.w).min(o.x+o.w),(self.y+self.h).min(o.y+o.h));
        (l<r && t<b).then_some(ScreenRect{x:l,y:t,w:r-l,h:b-t})
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Monitor {pub bounds:ScreenRect,pub work:ScreenRect,pub primary:bool}
/// Straight RGBA8 pixels of `area`, row-major, opaque.
pub struct Capture {pub area:ScreenRect,pub rgba:Vec<u8>}

#[cfg(windows)]
mod screen;
#[cfg(windows)]
pub use screen::{capture,claim_single_instance,cursor,foreground_window,monitors,restore_foreground,shift_down,spawn_hotkey,visible_windows,wait_for_compositor,HotkeyHandle};

#[cfg(not(windows))]
pub fn monitors()->std::io::Result<Vec<Monitor>>{Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"screen adapter requires Windows"))}
#[cfg(not(windows))]
pub fn capture(_area:ScreenRect)->std::io::Result<Capture>{Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"screen adapter requires Windows"))}
#[cfg(not(windows))]
pub fn visible_windows()->Vec<ScreenRect>{Vec::new()}
#[cfg(not(windows))]
pub fn claim_single_instance()->bool{true}
#[cfg(not(windows))]
pub fn cursor()->Option<(i32,i32)>{None}
#[cfg(not(windows))]
pub fn shift_down()->bool{false}
#[cfg(not(windows))]
pub fn foreground_window()->isize{0}
#[cfg(not(windows))]
pub fn restore_foreground(_hwnd:isize){}
#[cfg(not(windows))]
pub fn wait_for_compositor(){}
#[cfg(not(windows))]
pub struct HotkeyHandle;
#[cfg(not(windows))]
impl HotkeyHandle{pub fn stop(self){}}
#[cfg(not(windows))]
pub fn spawn_hotkey(_hotkey:Hotkey,_on_press:impl Fn()+Send+'static)->std::io::Result<HotkeyHandle>{Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"hotkey requires Windows"))}

#[cfg(test)]
mod rect_tests {
    use super::ScreenRect;
    #[test] fn intersection_and_containment_are_half_open(){
        let a=ScreenRect{x:-1920,y:0,w:1920,h:1080};let b=ScreenRect{x:-100,y:500,w:300,h:300};
        assert_eq!(a.intersect(b),Some(ScreenRect{x:-100,y:500,w:100,h:300}));
        assert!(a.contains(-1,1079) && !a.contains(0,0));
        assert_eq!(a.intersect(ScreenRect{x:0,y:0,w:10,h:10}),None);
    }
}
