//! Capture the primary monitor with GDI, DXGI Desktop Duplication and Windows.Graphics.Capture,
//! and print only statistics (timings, sizes, pixel differences): no pixels leave memory.
//! Usage: capture-api-spike [runs]
use std::time::{Duration, Instant};
use windows::core::{Interface, Result};
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{HMODULE, POINT, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::WinRT::Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

struct Shot { w: u32, h: u32, bgra: Vec<u8> }

fn primary() -> (HMONITOR, RECT) {
    unsafe {
        let m = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(m, &mut mi);
        (m, mi.rcMonitor)
    }
}

/// As in the product: BitBlt(SRCCOPY | CAPTUREBLT) from the screen DC, then GetDIBits.
fn gdi(r: RECT) -> Result<Shot> {
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());
        BitBlt(mem, 0, 0, w, h, Some(screen), r.left, r.top, SRCCOPY | CAPTUREBLT)?;
        SelectObject(mem, old);
        let mut info = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() }, ..Default::default() };
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        GetDIBits(mem, bmp, 0, h as u32, Some(bgra.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        Ok(Shot { w: w as u32, h: h as u32, bgra })
    }
}

fn device(adapter: Option<&IDXGIAdapter>) -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    let (mut d, mut c) = (None, None);
    let kind = if adapter.is_some() { D3D_DRIVER_TYPE_UNKNOWN } else { D3D_DRIVER_TYPE_HARDWARE };
    unsafe { D3D11CreateDevice(adapter, kind, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut d), None, Some(&mut c))?; }
    Ok((d.unwrap(), c.unwrap()))
}

fn read_texture(d: &ID3D11Device, c: &ID3D11DeviceContext, tex: &ID3D11Texture2D) -> Result<Shot> {
    unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        tex.GetDesc(&mut desc);
        desc.Usage = D3D11_USAGE_STAGING; desc.BindFlags = 0; desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32; desc.MiscFlags = 0;
        let mut staging = None;
        d.CreateTexture2D(&desc, None, Some(&mut staging))?;
        let staging = staging.unwrap();
        c.CopyResource(&staging, tex);
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        c.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut m))?;
        let (w, h) = (desc.Width, desc.Height);
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        for y in 0..h as usize {
            let src = std::slice::from_raw_parts((m.pData as *const u8).add(y * m.RowPitch as usize), w as usize * 4);
            bgra[y * w as usize * 4..(y + 1) * w as usize * 4].copy_from_slice(src);
        }
        c.Unmap(&staging, 0);
        Ok(Shot { w, h, bgra })
    }
}

fn wide(s: &[u16]) -> String { String::from_utf16_lossy(&s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]) }

/// DXGI Desktop Duplication of the output that shows the primary monitor.
fn dxgi(r: RECT) -> Result<(Shot, String, f64)> {
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
        let mut i = 0;
        loop {
            let adapter = factory.EnumAdapters1(i)?;
            let mut j = 0;
            while let Ok(output) = adapter.EnumOutputs(j) {
                let od = output.GetDesc()?;
                if od.DesktopCoordinates == r {
                    // HDR shows as the BT.2020 PQ colour space (G2084); SDR as G22 (sRGB).
                    let hdr = output.cast::<IDXGIOutput6>().and_then(|o| o.GetDesc1()).map(|d| format!("colorspace={} bits={} max_nits={:.0}", d.ColorSpace.0, d.BitsPerColor, d.MaxLuminance)).unwrap_or_else(|e| format!("desc1_failed={:#x}", e.code().0));
                    let name = format!("{}, {hdr}", wide(&adapter.GetDesc1()?.Description));
                    let (d, c) = device(Some(&adapter.cast()?))?;
                    let dup = output.cast::<IDXGIOutput1>()?.DuplicateOutput(&d)?;
                    let t = Instant::now();
                    // The first frame can arrive before any image was presented (LastPresentTime 0):
                    // keep acquiring until one carries a picture, or report how many were empty.
                    let mut empty = 0;
                    for _ in 0..20 {
                        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
                        let mut res = None;
                        match dup.AcquireNextFrame(500, &mut info, &mut res) {
                            Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => { empty += 1; continue; }
                            Err(e) => return Err(e),
                            Ok(()) => {}
                        }
                        if info.LastPresentTime == 0 { empty += 1; let _ = dup.ReleaseFrame(); continue; }
                        let wait = t.elapsed().as_secs_f64() * 1e3;
                        let shot = read_texture(&d, &c, &res.unwrap().cast()?);
                        let _ = dup.ReleaseFrame();
                        return Ok((shot?, format!("{name}, empty_frames_skipped={empty}"), wait));
                    }
                    return Ok((Shot { w: 0, h: 0, bgra: Vec::new() }, format!("{name}, no presented frame in {empty} tries (static screen)"), t.elapsed().as_secs_f64() * 1e3));
                }
                j += 1;
            }
            i += 1;
        }
    }
}

/// Windows.Graphics.Capture of the primary monitor: one frame, then close.
fn wgc(m: HMONITOR, border_off: bool) -> Result<(Shot, String)> {
    let (d, c) = device(None)?;
    let rt: IDirect3DDevice = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&d.cast::<IDXGIDevice>()?)? }.cast()?;
    let item: GraphicsCaptureItem = unsafe { windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?.CreateForMonitor(m)? };
    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&rt, DirectXPixelFormat::B8G8R8A8UIntNormalized, 1, item.Size()?)?;
    let session = pool.CreateCaptureSession(&item)?;
    let border = if border_off { match session.SetIsBorderRequired(false) { Ok(()) => "border_off=ok".to_string(), Err(e) => format!("border_off=unsupported({:#x})", e.code().0) } } else { "border_off=not_tried".into() };
    let _ = session.SetIsCursorCaptureEnabled(false);
    session.StartCapture()?;
    let t = Instant::now();
    let frame = loop {
        if let Ok(f) = pool.TryGetNextFrame() { break f; }
        if t.elapsed() > Duration::from_secs(3) { return Err(windows::core::Error::from_hresult(windows::core::HRESULT(0x8000_000Au32 as i32))); }
        std::thread::sleep(Duration::from_millis(1));
    };
    let tex: ID3D11Texture2D = unsafe { frame.Surface()?.cast::<IDirect3DDxgiInterfaceAccess>()?.GetInterface()? };
    let shot = read_texture(&d, &c, &tex)?;
    frame.Close()?; session.Close()?; pool.Close()?;
    Ok((shot, border))
}

fn diff(a: &Shot, b: &Shot) -> String {
    if (a.w, a.h) != (b.w, b.h) { return format!("size {}x{} vs {}x{}", a.w, a.h, b.w, b.h); }
    let mut n = 0usize; let mut max = 0u8;
    for (p, q) in a.bgra.chunks_exact(4).zip(b.bgra.chunks_exact(4)) {
        let d = (0..3).map(|k| p[k].abs_diff(q[k])).max().unwrap();
        if d > 0 { n += 1; max = max.max(d); }
    }
    format!("differing_px={n} max_channel_diff={max}")
}

/// Yellow-ish pixels on the screen's top edge, as GDI sees the composed desktop.
fn yellow_edge(r: RECT) -> usize {
    unsafe {
        let dc = GetDC(None);
        let n = (0..(r.right - r.left)).step_by(4).filter(|&x| {
            let c = GetPixel(dc, r.left + x, r.top + 1).0;
            let (red, green, blue) = (c & 0xff, (c >> 8) & 0xff, (c >> 16) & 0xff);
            red > 200 && green > 150 && blue < 80
        }).count();
        ReleaseDC(None, dc);
        n
    }
}

fn main() -> Result<()> {
    let runs: usize = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(5);
    unsafe { let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2); }
    let (m, r) = primary();
    println!("MONITOR {}x{} at {},{}  WGC_supported={}", r.right - r.left, r.bottom - r.top, r.left, r.top, GraphicsCaptureSession::IsSupported()?);
    let mut reference = None;
    for run in 1..=runs {
        let t = Instant::now(); let g = gdi(r)?; let tg = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now(); let x = dxgi(r); let tx = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now(); let w = wgc(m, true); let tw = t.elapsed().as_secs_f64() * 1e3;
        match &x { Ok((s, name, wait)) => println!("RUN {run} GDI {tg:.1} ms | DXGI {tx:.1} ms (first frame wait {wait:.1} ms, adapter {name}) {} | ", diff(&g, s)), Err(e) => println!("RUN {run} GDI {tg:.1} ms | DXGI failed {:#x} {}", e.code().0, e.message()) }
        match &w { Ok((s, b)) => println!("        WGC {tw:.1} ms {b} {}", diff(&g, s)), Err(e) => println!("        WGC failed {:#x} {}", e.code().0, e.message()) }
        if let Some(prev) = &reference { println!("        GDI run-to-run {}", diff(prev, &g)); }
        reference = Some(g);
    }
    // Border: the top edge with no session, then during a session with and without IsBorderRequired(false).
    let before = yellow_edge(r);
    let probe = |off: bool| -> Result<usize> {
        let (d, _c) = device(None)?;
        let rt: IDirect3DDevice = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&d.cast::<IDXGIDevice>()?)? }.cast()?;
        let item: GraphicsCaptureItem = unsafe { windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?.CreateForMonitor(m)? };
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&rt, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, item.Size()?)?;
        let s = pool.CreateCaptureSession(&item)?;
        if off { let _ = s.SetIsBorderRequired(false); }
        s.StartCapture()?;
        std::thread::sleep(Duration::from_millis(700));
        let n = yellow_edge(r);
        s.Close()?; pool.Close()?;
        Ok(n)
    };
    println!("BORDER yellow_edge_px before={before} during_default={:?} during_border_off={:?}", probe(false).ok(), probe(true).ok());
    Ok(())
}
