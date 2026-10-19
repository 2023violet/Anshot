//! Test-only OS fixture. Never runs as part of the normal product or unit tests.
//! No user clipboard contents, input injection, display switch or ACL changes.
#![deny(unsafe_op_in_unsafe_fn)]
#[cfg(windows)]
mod windows {
    use std::{io,ptr};
    use windows_sys::Win32::{Foundation::{HANDLE,GetLastError,SetLastError},System::{StationsAndDesktops::*,Threading::GetCurrentThreadId,
        DataExchange::{CloseClipboard,CountClipboardFormats,GetClipboardSequenceNumber,OpenClipboard}},
        UI::WindowsAndMessaging::{CWF_CREATE_ONLY,WINSTA_ACCESSCLIPBOARD,WINSTA_CREATEDESKTOP,WINSTA_READATTRIBUTES,WSF_VISIBLE}};

    fn check(ok:i32,api:&str)->Result<(),String>{if ok==0{Err(format!("{api}: {}",io::Error::last_os_error()))}else{Ok(())}}
    fn name(object:HANDLE)->Result<Vec<u16>,String>{
        let mut buf=[0u16;512];let mut needed=0;
        // SAFETY: borrowed live object, fixed writable buffer and exact byte size.
        check(unsafe{GetUserObjectInformationW(object,UOI_NAME,buf.as_mut_ptr().cast(),std::mem::size_of_val(&buf) as u32,&mut needed)},"GetUserObjectInformationW(name)")?;
        let end=buf.iter().position(|v|*v==0).ok_or("Object name not terminated")?;
        Ok(buf[..end].to_vec())
    }
    pub struct Isolated {
        original_station:HWINSTA,original_desktop:HDESK,station:HWINSTA,desktop:HDESK,
        original_sequence:u32,ready:bool,
    }
    impl Isolated {
        pub fn new()->Result<Self,String>{
            // SAFETY: metadata queries only; handles returned here are borrowed.
            let (original_station,original_desktop,original_sequence)=unsafe{
                (GetProcessWindowStation(),GetThreadDesktop(GetCurrentThreadId()),GetClipboardSequenceNumber())
            };
            if original_station.is_null() || original_desktop.is_null(){return Err("Original context unavailable".into());}
            let unique_name:Vec<u16>=format!("ShareReadyClipboardProbe_{}_{}",std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos())
                .encode_utf16().chain(Some(0)).collect();
            // SAFETY: fixed-prefix unique NUL name and CREATE_ONLY prohibit
            // opening an existing station. Only the current token is used;
            // access denial is terminal, never an elevation/reuse fallback.
            // NULL SA uses defaults, without modifying existing permissions.
            let station=unsafe{CreateWindowStationW(unique_name.as_ptr(),CWF_CREATE_ONLY,
                (WINSTA_ACCESSCLIPBOARD|WINSTA_CREATEDESKTOP|WINSTA_READATTRIBUTES) as u32,ptr::null())};
            if station.is_null(){return Err(format!("CreateWindowStationW: {}",io::Error::last_os_error()));}
            let mut own=Self{original_station,original_desktop,station,desktop:ptr::null_mut(),original_sequence,ready:false};
            let station_name=name(station)?;
            if station_name==name(original_station)? || String::from_utf16_lossy(&station_name).eq_ignore_ascii_case("WinSta0"){
                return Err("Isolation refused: station identity is not new/noninteractive".into());
            }
            let mut flags=USEROBJECTFLAGS::default();let mut needed=0;
            // SAFETY: exact generated repr(C) structure/size, live owned handle.
            check(unsafe{GetUserObjectInformationW(station,UOI_FLAGS,(&mut flags as *mut USEROBJECTFLAGS).cast(),std::mem::size_of_val(&flags) as u32,&mut needed)},"GetUserObjectInformationW(flags)")?;
            if flags.dwFlags&WSF_VISIBLE as u32!=0{return Err("Isolation refused: station is visible".into());}
            // SAFETY: affects this test process only, never the interactive station.
            check(unsafe{SetProcessWindowStation(station)},"SetProcessWindowStation")?;
            let desktop_name=[b'p' as u16,b'r' as u16,b'o' as u16,b'b' as u16,b'e' as u16,0];
            // SAFETY: fixed NUL name; own new station; no flags/security overrides,
            // no SWITCHDESKTOP access requested and no display switch occurs.
            own.desktop=unsafe{CreateDesktopW(desktop_name.as_ptr(),ptr::null(),ptr::null(),0,
                DESKTOP_CREATEWINDOW|DESKTOP_READOBJECTS|DESKTOP_WRITEOBJECTS,ptr::null())};
            if own.desktop.is_null(){return Err(format!("CreateDesktopW: {}",io::Error::last_os_error()));}
            // SAFETY: this fresh process/thread has no windows/hooks yet.
            check(unsafe{SetThreadDesktop(own.desktop)},"SetThreadDesktop")?;
            if name(unsafe{GetProcessWindowStation()})?!=station_name ||
                name(unsafe{GetThreadDesktop(GetCurrentThreadId())})?!=name(own.desktop)?{
                return Err("Isolation refused: current context mismatch".into());
            }
            // First clipboard access occurs only after all isolation assertions.
            check(unsafe{OpenClipboard(ptr::null_mut())},"OpenClipboard(isolated)")?;
            // A zero count can also mean failure; preserve the immediate error
            // before CloseClipboard can alter this thread's last-error value.
            let (count,error)=unsafe{SetLastError(0);let count=CountClipboardFormats();(count,GetLastError())};
            let closed=unsafe{CloseClipboard()};
            check(closed,"CloseClipboard(isolated)")?;
            if count==0 && error!=0{return Err(format!("CountClipboardFormats: OS error {error}"));}
            if count!=0{return Err("Isolation refused: new clipboard is not empty".into());}
            own.ready=true;println!("SR_CLIPBOARD_ISOLATION_READY noninteractive=true fresh_clipboard=true");
            Ok(own)
        }
        pub fn finish(mut self)->Result<(),String>{
            self.restore()?;
            // Auxiliary metadata only, not a content-integrity proof: zero can
            // indicate unavailable access and delayed rendering affects counts.
            // A difference is reported, never overwritten/restored.
            let equal=unsafe{GetClipboardSequenceNumber()}==self.original_sequence;
            println!("SR_CLIPBOARD_ISOLATION_FINISH context_restored=true original_sequence_equal={equal}");
            if !equal{return Err("Original clipboard sequence changed; no original content was read/restored".into());}
            Ok(())
        }
        fn restore(&mut self)->Result<(),String>{
            self.ready=false;
            // SAFETY: original handles are borrowed and remain open; only this
            // thread/process is rebound. All fixture windows must be dropped first.
            check(unsafe{SetProcessWindowStation(self.original_station)},"Restore process station")?;
            check(unsafe{SetThreadDesktop(self.original_desktop)},"Restore thread desktop")?;
            if !self.desktop.is_null(){check(unsafe{CloseDesktop(self.desktop)},"Close owned desktop")?;self.desktop=ptr::null_mut();}
            if !self.station.is_null(){check(unsafe{CloseWindowStation(self.station)},"Close owned station")?;self.station=ptr::null_mut();}
            Ok(())
        }
    }
    impl Drop for Isolated {fn drop(&mut self){if !self.station.is_null(){let _=self.restore();}}}
    pub fn run()->Result<(),String>{
        if std::env::args().nth(1).as_deref()!=Some("--isolation-only"){return Err("Only --isolation-only is implemented; no clipboard writes performed".into());}
        let own=Isolated::new()?;own.finish()
    }
}
fn main(){
    #[cfg(windows)]
    if let Err(e)=windows::run(){eprintln!("SR_CLIPBOARD_PROBE_FAILED {e}");std::process::exit(2);}
    #[cfg(not(windows))]
    {eprintln!("Windows clipboard probe requires Windows");std::process::exit(2);}
}
