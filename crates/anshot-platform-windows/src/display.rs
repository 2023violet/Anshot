//! Display facts for opt-in capture diagnostics: which outputs are in HDR, colour depth and
//! the adapter, plus the Windows build. Nothing about windows, content or the user.
use crate::ScreenRect;
use windows::core::Interface;
use windows::Win32::Graphics::Dxgi::Common::DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, IDXGIOutput6};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayFacts {
    pub bounds: ScreenRect,
    /// The output is in HDR (BT.2020 PQ) mode; `None` when the system cannot say.
    pub hdr: Option<bool>,
    pub bits_per_color: u32,
    pub adapter: String,
}

/// One entry per output attached to the desktop, as DXGI reports them.
pub fn display_facts() -> Vec<DisplayFacts> {
    let mut out = Vec::new();
    // SAFETY: plain DXGI enumeration; every interface is reference counted by the crate.
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else { return out };
        let mut i = 0;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            let name = adapter.GetDesc1().map(|d| {
                let n = d.Description.iter().position(|&c| c == 0).unwrap_or(d.Description.len());
                String::from_utf16_lossy(&d.Description[..n])
            }).unwrap_or_default();
            let mut j = 0;
            while let Ok(output) = adapter.EnumOutputs(j) {
                if let Ok(d) = output.GetDesc() {
                    if d.AttachedToDesktop.as_bool() {
                        let r = d.DesktopCoordinates;
                        let bounds = ScreenRect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top };
                        let desc1 = output.cast::<IDXGIOutput6>().and_then(|o| o.GetDesc1()).ok();
                        out.push(DisplayFacts {
                            bounds,
                            hdr: desc1.map(|d| d.ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020),
                            bits_per_color: desc1.map_or(0, |d| d.BitsPerColor),
                            adapter: name.clone(),
                        });
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }
    out
}

/// "Windows <display version> build <build>.<ubr>" from the registry, or "unknown".
pub fn windows_build() -> String {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ};
    let key: Vec<u16> = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\0".encode_utf16().collect();
    let text = |name: &str| -> Option<String> {
        let v: Vec<u16> = format!("{name}\0").encode_utf16().collect();
        let mut buf = [0u16; 64];
        let mut len = (buf.len() * 2) as u32;
        // SAFETY: buffers are owned here and sized in bytes as the API expects.
        let ok = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, key.as_ptr(), v.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut len) } == 0;
        ok.then(|| String::from_utf16_lossy(&buf[..buf.iter().position(|&c| c == 0).unwrap_or(0)]))
    };
    let ubr = {
        let v: Vec<u16> = "UBR\0".encode_utf16().collect();
        let (mut n, mut len) = (0u32, 4u32);
        // SAFETY: a DWORD-sized output owned here.
        let ok = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, key.as_ptr(), v.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), (&mut n as *mut u32).cast(), &mut len) } == 0;
        ok.then_some(n)
    };
    match text("CurrentBuild") {
        Some(b) => format!("Windows {} build {b}.{}", text("DisplayVersion").unwrap_or_default(), ubr.map_or(String::from("?"), |n| n.to_string())),
        None => "unknown".into(),
    }
}
