//! Window-scoped Ctrl+V bridge before egui-winit consumes the key as text paste.
#![deny(unsafe_op_in_unsafe_fn)]
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::PasteBridge;

#[cfg(not(windows))]
#[derive(Default,Clone)]
pub struct PasteBridge;
#[cfg(not(windows))]
impl PasteBridge {
    pub fn install<T>(&self,_builder:&mut winit::event_loop::EventLoopBuilder<T>){}
    pub fn bind(&self,_window:&impl raw_window_handle::HasWindowHandle,_ctx:egui::Context)->Result<(),String>{Err("Windows paste bridge unavailable".into())}
    pub fn take_pending(&self)->bool{false}
}
