# Launch a spike executable, capture its client area via PrintWindow, then close it.
# Usage: pwsh -File spike/capture.ps1 -Exe <path> -Out <png> [-Scale 1.5] [-WaitMs 4000]
param(
    [Parameter(Mandatory)] [string] $Exe,
    [Parameter(Mandatory)] [string] $Out,
    [double] $Scale = 0,
    [int] $WaitMs = 4000,
    [switch] $Screen
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class W {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
}
'@
[void][W]::SetProcessDPIAware()
$psi = [System.Diagnostics.ProcessStartInfo]::new($Exe)
$psi.WorkingDirectory = Split-Path $Exe
if ($Scale -gt 0) {
    $psi.Environment['SPIKE_SCALE'] = "$Scale"
    $psi.Environment['SLINT_SCALE_FACTOR'] = "$Scale"
}
$p = [System.Diagnostics.Process]::Start($psi)
try {
    $deadline = (Get-Date).AddMilliseconds($WaitMs + 10000)
    while ($p.MainWindowHandle -eq 0 -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 200; $p.Refresh() }
    if ($p.MainWindowHandle -eq 0) { throw "no window" }
    Start-Sleep -Milliseconds $WaitMs; $p.Refresh()
    $h = $p.MainWindowHandle
    [void][W]::SetWindowPos($h, [IntPtr]::Zero, -8, 0, 0, 0, 0x0015); Start-Sleep -Milliseconds 1500
    $wr = New-Object W+RECT; [void][W]::GetWindowRect($h, [ref]$wr)
    $cr = New-Object W+RECT; [void][W]::GetClientRect($h, [ref]$cr)
    $pt = New-Object W+POINT; [void][W]::ClientToScreen($h, [ref]$pt)
    "WR $($wr.L),$($wr.T),$($wr.R),$($wr.B) CR $($cr.R)x$($cr.B) PT $($pt.X),$($pt.Y)"
    if ($Screen) {
        [void][W]::SetForegroundWindow($h); Start-Sleep -Milliseconds 800
        $client = New-Object System.Drawing.Bitmap $cr.R, $cr.B
        $g = [System.Drawing.Graphics]::FromImage($client)
        $g.CopyFromScreen($pt.X, $pt.Y, 0, 0, (New-Object System.Drawing.Size $cr.R, $cr.B)); $g.Dispose()
    } else {
        $bmp = New-Object System.Drawing.Bitmap ($wr.R - $wr.L), ($wr.B - $wr.T)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $dc = $g.GetHdc(); [void][W]::PrintWindow($h, $dc, 2); $g.ReleaseHdc($dc); $g.Dispose()
        $rect = New-Object System.Drawing.Rectangle ($pt.X - $wr.L), ($pt.Y - $wr.T), $cr.R, $cr.B
        $client = $bmp.Clone($rect, $bmp.PixelFormat)
    }
    New-Item -ItemType Directory -Force (Split-Path $Out) | Out-Null
    $client.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    "CAPTURED $Out $($cr.R)x$($cr.B)"
} finally {
    if (-not $p.HasExited) { $p.Kill() }
}
