use std::io;
use windows_sys::Win32::Foundation::HGLOBAL;
use windows_sys::Win32::System::{DataExchange::{CloseClipboard,GetClipboardData,IsClipboardFormatAvailable,OpenClipboard,RegisterClipboardFormatW},Memory::{GlobalLock,GlobalSize,GlobalUnlock}};
use crate::{ClipboardImage,ClipboardImageFormat};

struct OpenedClipboard;
impl Drop for OpenedClipboard{fn drop(&mut self){
    // SAFETY: created only after this thread successfully opens the clipboard.
    unsafe{CloseClipboard();}
}}
struct LockedGlobal(HGLOBAL);
impl Drop for LockedGlobal{fn drop(&mut self){
    // SAFETY: construction follows successful GlobalLock; borrowed allocation
    // remains alive until this guard drops, before its clipboard/owner closes.
    unsafe{GlobalUnlock(self.0);}
}}

// SAFETY: caller keeps this HGLOBAL allocation alive and stable for the call,
// either by owning it or by holding the clipboard open. Ownership is not taken.
unsafe fn copy_global(handle:HGLOBAL,limit:usize)->io::Result<Vec<u8>>{
    if handle.is_null(){return Err(io::Error::other("Clipboard image handle missing"));}
    // SAFETY: valid borrowed global-memory handle under the caller's contract.
    let size=unsafe{GlobalSize(handle)};
    if size==0 || size>limit || size>isize::MAX as usize{
        return Err(io::Error::new(io::ErrorKind::InvalidData,"Clipboard image exceeds the input byte limit or is empty"));
    }
    // SAFETY: same live allocation; successful lock pins its storage.
    let ptr=unsafe{GlobalLock(handle)};
    if ptr.is_null(){return Err(io::Error::last_os_error());}
    let _lock=LockedGlobal(handle);
    let mut bytes=Vec::new();
    bytes.try_reserve_exact(size).map_err(|_|io::Error::other("Cannot allocate bounded clipboard image"))?;
    // SAFETY: GlobalSize bounded size, GlobalLock returned a valid pinned pointer,
    // clipboard/owner and lock guards outlive this synchronous byte copy.
    bytes.extend_from_slice(unsafe{std::slice::from_raw_parts(ptr.cast::<u8>(),size)});
    Ok(bytes)
}

pub fn read_clipboard_image(png_limit:usize,dib_limit:usize)->io::Result<ClipboardImage>{
    // SAFETY: fixed, NUL-terminated UTF-16 registration name; no clipboard data.
    let png=unsafe{RegisterClipboardFormatW([80u16,78,71,0].as_ptr())};
    if png==0{return Err(io::Error::last_os_error());}
    // SAFETY: read-only open on this thread, closed by the guard on every path.
    if unsafe{OpenClipboard(std::ptr::null_mut())}==0{return Err(io::Error::last_os_error());}
    let _clipboard=OpenedClipboard;
    let choices=[(png,ClipboardImageFormat::Png,png_limit),(17,ClipboardImageFormat::Dib,dib_limit),(8,ClipboardImageFormat::Dib,dib_limit)];
    for (id,format,limit) in choices {
        // SAFETY: fixed/registered image formats while clipboard is open.
        if unsafe{IsClipboardFormatAvailable(id)}==0{continue;}
        // SAFETY: clipboard remains open for handle retrieval, size check, lock
        // and copy. The OS retains ownership. A bad preferred format is an error,
        // never silently replaced with another representation.
        let bytes=unsafe{copy_global(GetClipboardData(id),limit)}?;
        return Ok(ClipboardImage{format,bytes});
    }
    Err(io::Error::new(io::ErrorKind::NotFound,"No supported image on clipboard; copy an image or open a PNG/JPEG file"))
}

/// Copy `bytes` into a new movable global block and hand it to the open clipboard.
/// SAFETY: the clipboard must be open (and emptied) by this thread.
unsafe fn set_global(format:u32,bytes:&[u8])->io::Result<()>{
    use windows_sys::Win32::{Foundation::{GlobalFree,HANDLE},System::{DataExchange::SetClipboardData,Memory::{GlobalAlloc,GMEM_MOVEABLE}}};
    // SAFETY: plain allocation; freed below unless the clipboard takes ownership.
    let h=unsafe{GlobalAlloc(GMEM_MOVEABLE,bytes.len().max(1))};
    if h.is_null(){return Err(io::Error::last_os_error());}
    // SAFETY: fresh allocation of at least bytes.len(); unlocked before handing over.
    let p=unsafe{GlobalLock(h)};
    if p.is_null(){unsafe{GlobalFree(h);}return Err(io::Error::last_os_error());}
    unsafe{std::ptr::copy_nonoverlapping(bytes.as_ptr(),p.cast::<u8>(),bytes.len());GlobalUnlock(h);}
    // SAFETY: on success the system owns the block; on failure it is still ours to free.
    if unsafe{SetClipboardData(format,h as HANDLE)}.is_null(){unsafe{GlobalFree(h);}return Err(io::Error::last_os_error());}
    Ok(())
}

/// UTF-16, NUL-terminated registration of a named clipboard format.
fn register(name:&str)->u32{
    let w:Vec<u16>=name.encode_utf16().chain(Some(0)).collect();
    // SAFETY: NUL-terminated UTF-16 name owned for the call.
    unsafe{RegisterClipboardFormatW(w.as_ptr())}
}

/// Put an exported image on the clipboard as "PNG" and CF_DIBV5 in one session. With
/// `private`, the same session marks it with the documented zero-DWORD formats that keep
/// it out of Windows clipboard history and the cloud clipboard (they must be set before
/// the clipboard is closed, or history has already taken the item).
pub fn write_clipboard_image(png:&[u8],dibv5:&[u8],private:bool)->io::Result<()>{
    use windows_sys::Win32::System::DataExchange::EmptyClipboard;
    let png_format=register("PNG");
    if png_format==0{return Err(io::Error::last_os_error());}
    // Another program may hold the clipboard for a moment.
    let mut opened=false;
    for _ in 0..10 {
        // SAFETY: open on this thread, closed by the guard below on every path.
        if unsafe{OpenClipboard(std::ptr::null_mut())}!=0{opened=true;break;}
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    if !opened{return Err(io::Error::last_os_error());}
    let _clipboard=OpenedClipboard;
    // SAFETY: clipboard is open on this thread.
    if unsafe{EmptyClipboard()}==0{return Err(io::Error::last_os_error());}
    // SAFETY: clipboard open and emptied by this thread for all three calls.
    unsafe{set_global(png_format,png)}?;
    unsafe{set_global(17,dibv5)}?;
    if private {
        for name in ["CanIncludeInClipboardHistory","CanUploadToCloudClipboard"] {
            let f=register(name);
            if f!=0 { unsafe{set_global(f,&0u32.to_ne_bytes())}?; }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::{Foundation::GlobalFree,System::Memory::{GlobalAlloc,GlobalLock,GlobalUnlock,GlobalSize,GMEM_MOVEABLE,GMEM_ZEROINIT}};
    struct Block(HGLOBAL);
    impl Drop for Block{fn drop(&mut self){unsafe{GlobalFree(self.0);}}}
    fn block()->Block{
        // Tests own this allocation; never use GetClipboardData/EmptyClipboard.
        let h=unsafe{GlobalAlloc(GMEM_MOVEABLE|GMEM_ZEROINIT,32)};assert!(!h.is_null());
        let p=unsafe{GlobalLock(h)};assert!(!p.is_null());
        unsafe{std::ptr::copy_nonoverlapping(b"synthetic".as_ptr(),p.cast(),9);GlobalUnlock(h);}
        Block(h)
    }
    #[test] fn owned_global_block_is_copied_exactly_and_remains_owned_by_caller(){
        let block=block();let size=unsafe{GlobalSize(block.0)};
        let got=unsafe{copy_global(block.0,size)}.unwrap();
        assert_eq!(&got[..9],b"synthetic");assert_eq!(got.len(),size);assert!(got[9..].iter().all(|b|*b==0));
        assert_eq!(unsafe{copy_global(block.0,size)}.unwrap(),got);
    }
    #[test] fn size_bound_is_checked_before_copy_and_can_retry_with_a_valid_bound(){
        let block=block();let size=unsafe{GlobalSize(block.0)};
        assert_eq!(unsafe{copy_global(block.0,size-1)}.unwrap_err().kind(),io::ErrorKind::InvalidData);
        assert_eq!(unsafe{copy_global(block.0,0)}.unwrap_err().kind(),io::ErrorKind::InvalidData);
        assert_eq!(unsafe{copy_global(block.0,size)}.unwrap().len(),size);
    }
    #[test] fn null_handle_is_rejected_without_dereference(){
        assert!(unsafe{copy_global(std::ptr::null_mut(),32)}.is_err());
    }
}
