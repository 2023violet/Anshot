//! Screen boundary: monitor layout, one-shot GDI capture, visible window bounds
//! for snapping, and a process-wide global hotkey. Coordinates are physical
//! virtual-desktop pixels (callers must be per-monitor DPI aware).
use crate::{Capture, Hotkey, Monitor, ScreenRect};
use std::io;
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC, GetDIBits,
    GetMonitorInfoW, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HDC,
    HMONITOR, MONITORINFO, SRCCOPY,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetCursorPos, GetMessageW, GetWindowLongW, GetWindowTextLengthW, IsIconic, IsWindowVisible, GWL_EXSTYLE,
    MONITORINFOF_PRIMARY, MSG, WM_HOTKEY, WM_USER, WS_EX_TOOLWINDOW,
};

fn rect(r: RECT) -> ScreenRect { ScreenRect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top } }

unsafe extern "system" fn monitor_cb(m: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
    // SAFETY: `data` is the address of the Vec owned by `monitors()` for the
    // duration of the synchronous EnumDisplayMonitors call.
    let out = unsafe { &mut *(data as *mut Vec<Monitor>) };
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    // SAFETY: valid monitor handle from the enumeration and a sized MONITORINFO.
    if unsafe { GetMonitorInfoW(m, &mut info) } != 0 {
        out.push(Monitor { bounds: rect(info.rcMonitor), work: rect(info.rcWork), primary: info.dwFlags & MONITORINFOF_PRIMARY != 0 });
    }
    1
}

pub fn monitors() -> io::Result<Vec<Monitor>> {
    let mut out: Vec<Monitor> = Vec::new();
    // SAFETY: the callback only dereferences `out`, which outlives this call.
    let ok = unsafe { EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(monitor_cb), &mut out as *mut _ as LPARAM) };
    if ok == 0 || out.is_empty() { return Err(io::Error::last_os_error()); }
    Ok(out)
}

pub fn cursor() -> Option<(i32, i32)> {
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: writes a POINT owned by this frame.
    (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x, p.y))
}

/// Block until the compositor has presented the current window state twice, so a
/// window hidden just before is no longer on screen when the desktop is copied.
pub fn wait_for_compositor() {
    for _ in 0..2 {
        // SAFETY: no arguments; returns an HRESULT we can ignore (no compositor = nothing to wait for).
        unsafe { windows_sys::Win32::Graphics::Dwm::DwmFlush() };
    }
}

/// The window that has keyboard focus now (opaque handle), to hand it back after
/// showing a notification window.
pub fn foreground_window() -> isize {
    // SAFETY: plain query; returns a handle value or null.
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() as isize }
}
/// Give focus back to a window returned by [`foreground_window`] (no-op for null).
pub fn restore_foreground(hwnd: isize) {
    if hwnd == 0 { return; }
    // SAFETY: the handle came from GetForegroundWindow; a stale handle just fails.
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd as _) };
}

/// Whether a Shift key is held right now (for Shift-constrained drawing).
pub fn shift_down() -> bool {
    // SAFETY: plain query of the calling thread's keyboard state; no pointers.
    let state = unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState(i32::from(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT)) };
    (state as u16) & 0x8000 != 0
}

struct ScreenDc(HDC);
impl Drop for ScreenDc { fn drop(&mut self) {
    // SAFETY: obtained from GetDC(NULL) and released exactly once.
    unsafe { ReleaseDC(std::ptr::null_mut(), self.0) };
} }
struct MemDc(HDC);
impl Drop for MemDc { fn drop(&mut self) {
    // SAFETY: created by CreateCompatibleDC and deleted exactly once.
    unsafe { DeleteDC(self.0) };
} }
struct Bitmap(*mut core::ffi::c_void);
impl Drop for Bitmap { fn drop(&mut self) {
    // SAFETY: created by CreateCompatibleBitmap, deselected before drop order ends.
    unsafe { DeleteObject(self.0) };
} }

/// Copy a physical-pixel rectangle of the desktop into straight RGBA8.
pub fn capture(area: ScreenRect) -> io::Result<Capture> {
    if area.w <= 0 || area.h <= 0 || i64::from(area.w) * i64::from(area.h) > 100_000_000 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "capture area out of range"));
    }
    // SAFETY: each handle is checked and owned by a drop guard; the bitmap is
    // selected out of the memory DC (old object restored) before guards drop.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        if screen.is_null() { return Err(io::Error::last_os_error()); }
        let screen = ScreenDc(screen);
        let mem = CreateCompatibleDC(screen.0);
        if mem.is_null() { return Err(io::Error::last_os_error()); }
        let mem = MemDc(mem);
        let bmp = CreateCompatibleBitmap(screen.0, area.w, area.h);
        if bmp.is_null() { return Err(io::Error::last_os_error()); }
        let bmp = Bitmap(bmp);
        let old = SelectObject(mem.0, bmp.0);
        let blit = BitBlt(mem.0, 0, 0, area.w, area.h, screen.0, area.x, area.y, SRCCOPY | CAPTUREBLT);
        SelectObject(mem.0, old);
        if blit == 0 { return Err(io::Error::last_os_error()); }
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: area.w, biHeight: -area.h, biPlanes: 1,
            biBitCount: 32, biCompression: BI_RGB, ..std::mem::zeroed()
        };
        let mut pixels = vec![0u8; area.w as usize * area.h as usize * 4];
        let lines = GetDIBits(mem.0, bmp.0, 0, area.h as u32, pixels.as_mut_ptr().cast(), &mut info, DIB_RGB_COLORS);
        if lines != area.h { return Err(io::Error::last_os_error()); }
        for px in pixels.chunks_exact_mut(4) { px.swap(0, 2); px[3] = 255; } // BGRX -> RGBA, opaque
        Ok(Capture { area, rgba: pixels })
    }
}

struct Windows(Vec<ScreenRect>);
unsafe extern "system" fn window_cb(h: HWND, data: LPARAM) -> i32 {
    // SAFETY: `data` points at the Windows collector owned by `visible_windows()`.
    let out = unsafe { &mut *(data as *mut Windows) };
    // SAFETY: plain queries on a window handle supplied by EnumWindows.
    unsafe {
        if IsWindowVisible(h) == 0 || IsIconic(h) != 0 || GetWindowTextLengthW(h) == 0 { return 1; }
        if (GetWindowLongW(h, GWL_EXSTYLE) as u32) & WS_EX_TOOLWINDOW != 0 { return 1; }
        let mut cloaked = 0u32;
        DwmGetWindowAttribute(h, DWMWA_CLOAKED as u32, (&mut cloaked as *mut u32).cast(), 4);
        if cloaked != 0 { return 1; }
        let mut r: RECT = std::mem::zeroed();
        if DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS as u32, (&mut r as *mut RECT).cast(), std::mem::size_of::<RECT>() as u32) == 0 {
            let s = rect(r);
            if s.w > 40 && s.h > 40 { out.0.push(s); }
        }
    }
    1
}

/// Visible top-level window frames in Z order (topmost first), for click-to-snap.
pub fn visible_windows() -> Vec<ScreenRect> {
    let mut out = Windows(Vec::new());
    // SAFETY: the callback only touches `out`, which outlives this call.
    unsafe { EnumWindows(Some(window_cb), &mut out as *mut _ as LPARAM) };
    out.0
}

/// A running global hotkey; dropping it does not stop it, call [`HotkeyHandle::stop`].
pub struct HotkeyHandle { thread_id: u32, join: Option<std::thread::JoinHandle<()>> }
impl HotkeyHandle {
    /// End the message thread; the OS releases its thread-bound hotkey.
    pub fn stop(mut self) {
        // SAFETY: posts WM_QUIT to a thread id this handle owns; no pointers are passed.
        let posted = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(self.thread_id, windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT, 0, 0) };
        // Never block the UI thread on a thread that did not get the message.
        if posted != 0 { if let Some(j) = self.join.take() { let _ = j.join(); } }
    }
}

/// Register `hotkey` on a dedicated message thread; `on_press` runs on that thread.
pub fn spawn_hotkey(hotkey: Hotkey, on_press: impl Fn() + Send + 'static) -> io::Result<HotkeyHandle> {
    let (tx, rx) = std::sync::mpsc::channel();
    let join = std::thread::Builder::new().name("anshot-hotkey".into()).spawn(move || {
        // SAFETY: plain query of the calling thread id.
        let tid = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
        let (mods, vk) = hotkey.keys();
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        // SAFETY: creates this thread's message queue before its id is published, so stop()'s
        // PostThreadMessageW cannot fail with ERROR_INVALID_THREAD_ID.
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::PeekMessageW(&mut msg, std::ptr::null_mut(), WM_USER, WM_USER, windows_sys::Win32::UI::WindowsAndMessaging::PM_NOREMOVE) };
        // SAFETY: thread-bound hotkey (NULL hwnd) delivered to this thread's queue.
        let ok = unsafe { RegisterHotKey(std::ptr::null_mut(), 1, mods | MOD_NOREPEAT, vk) };
        let _ = tx.send(if ok != 0 { Ok(tid) } else { Err(io::Error::last_os_error()) });
        if ok == 0 { return; }
        // SAFETY: standard blocking message loop on this thread; MSG is owned here.
        while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
            if msg.message == WM_HOTKEY { on_press(); }
        }
        // SAFETY: release this thread's registration before it exits.
        unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey(std::ptr::null_mut(), 1) };
    })?;
    match rx.recv().map_err(|_| io::Error::other("hotkey thread ended"))? {
        Ok(thread_id) => Ok(HotkeyHandle { thread_id, join: Some(join) }),
        Err(e) => { let _ = join.join(); Err(e) }
    }
}

impl Hotkey {
    fn keys(self) -> (u32, u32) {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, VK_SNAPSHOT};
        match self {
            Hotkey::CtrlShiftA => (MOD_CONTROL | MOD_SHIFT, u32::from(b'A')),
            Hotkey::CtrlAltA => (MOD_CONTROL | MOD_ALT, u32::from(b'A')),
            Hotkey::CtrlShiftX => (MOD_CONTROL | MOD_SHIFT, u32::from(b'X')),
            Hotkey::PrintScreen => (0, u32::from(VK_SNAPSHOT)),
        }
    }
}

/// True for the first Anshot process in this session (named mutex). The
/// handle is intentionally kept for the process lifetime.
pub fn claim_single_instance() -> bool {
    let name: Vec<u16> = "Local\\Anshot.Desktop.Instance\0".encode_utf16().collect();
    // SAFETY: NUL-terminated UTF-16 name; default security; handle never closed.
    let handle = unsafe { windows_sys::Win32::System::Threading::CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    // SAFETY: reads this thread's last-error value right after the call.
    let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
    !handle.is_null() && err != windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS
}
