use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
use windows_sys::Win32::UI::WindowsAndMessaging::{MSG,WM_KEYDOWN};
use raw_window_handle::{HasWindowHandle,RawWindowHandle};

struct Binding {hwnd:isize,ctx:egui::Context}
#[derive(Clone,Default)]
pub struct PasteBridge {binding:Arc<Mutex<Option<Binding>>>,pending:Arc<AtomicBool>}
impl PasteBridge {
    fn accept(&self,msg:&MSG,foreground:isize,ctrl:bool,alt:bool)->bool{
        if msg.message!=WM_KEYDOWN || msg.wParam!=86 || !ctrl || alt{return false;}
        let Ok(binding)=self.binding.lock() else{return false;};
        let Some(binding)=binding.as_ref() else{return false;};
        if msg.hwnd as isize!=binding.hwnd || foreground!=binding.hwnd || binding.ctx.text_edit_focused(){return false;}
        if msg.lParam&(1<<30)==0{
            self.pending.store(true,Ordering::Release);binding.ctx.request_repaint();
        }
        true
    }
    pub fn bind(&self,window:&impl HasWindowHandle,ctx:egui::Context)->Result<(),String>{
        let RawWindowHandle::Win32(handle)=window.window_handle().map_err(|e|e.to_string())?.as_raw()
            else{return Err("Windows paste bridge requires a Win32 window".into());};
        *self.binding.lock().map_err(|_|"Paste bridge unavailable")?=Some(Binding{hwnd:handle.hwnd.get(),ctx});
        Ok(())
    }
    pub fn install<T>(&self,builder:&mut winit::event_loop::EventLoopBuilder<T>){
        use winit::platform::windows::EventLoopBuilderExtWindows;
        use windows_sys::Win32::UI::{Input::KeyboardAndMouse::{GetKeyState,VK_CONTROL,VK_MENU},WindowsAndMessaging::GetForegroundWindow};
        let bridge=self.clone();
        builder.with_msg_hook(move |ptr|{
            if ptr.is_null(){return false;}
            // SAFETY: winit's with_msg_hook supplies a live, aligned MSG pointer
            // for this synchronous callback; no reference escapes the call.
            let msg=unsafe{&*ptr.cast::<MSG>()};
            // SAFETY: these Win32 queries have no pointer arguments or ownership.
            let (ctrl,alt,foreground)=unsafe{(GetKeyState(VK_CONTROL as i32)<0,GetKeyState(VK_MENU as i32)<0,GetForegroundWindow())};
            bridge.accept(msg,foreground as isize,ctrl,alt)
        });
    }
    pub fn take_pending(&self)->bool{self.pending.swap(false,Ordering::AcqRel)}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bridge()->PasteBridge{
        let bridge=PasteBridge::default();
        *bridge.binding.lock().unwrap()=Some(Binding{hwnd:42,ctx:egui::Context::default()});bridge
    }
    fn key()->MSG{MSG{hwnd:42isize as _,message:WM_KEYDOWN,wParam:86,..Default::default()}}
    #[test] fn app_ctrl_v_requests_one_image_paste_before_egui_text_lookup(){
        let b=bridge();assert!(b.accept(&key(),42,true,false));assert!(b.take_pending());assert!(!b.take_pending());
    }
    #[test] fn repeat_is_consumed_without_reading_again(){
        let b=bridge();let mut msg=key();msg.lParam=1<<30;
        assert!(b.accept(&msg,42,true,false));assert!(!b.take_pending());
    }
    #[test] fn other_windows_and_modified_keys_are_left_alone(){
        let b=bridge();let msg=key();
        assert!(!b.accept(&msg,99,true,false));
        assert!(!b.accept(&MSG{hwnd:99isize as _,..msg},99,true,false));
        assert!(!b.accept(&msg,42,false,false));assert!(!b.accept(&msg,42,true,true));
        assert!(!b.accept(&MSG{message:0x101,..msg},42,true,false));
        assert!(!b.accept(&MSG{wParam:67,..msg},42,true,false));assert!(!b.take_pending());
    }
    #[test] fn focused_text_edit_keeps_native_text_paste(){
        let b=bridge();let ctx=b.binding.lock().unwrap().as_ref().unwrap().ctx.clone();
        let mut out=ctx.run_ui(Default::default(),|ui|{ui.add(egui::TextEdit::singleline(&mut String::new())).request_focus();});out.textures_delta.clear();
        assert!(ctx.text_edit_focused());assert!(!b.accept(&key(),42,true,false));assert!(!b.take_pending());
    }
}
