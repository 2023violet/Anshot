# B1 end-to-end on the real desktop app with real OS input.
# Usage: pwsh -File tests/desktop/e2e.ps1 -Exe <shareready.exe> -Image <png> -Out <dir>
param(
    [Parameter(Mandatory)] [string] $Exe,
    [Parameter(Mandatory)] [string] $Image,
    [Parameter(Mandatory)] [string] $Out
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms, UIAutomationClient, UIAutomationTypes
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class E {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, IntPtr e);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
}
'@
[void][E]::SetProcessDPIAware()
New-Item -ItemType Directory -Force $Out | Out-Null
$log = [System.Collections.Generic.List[string]]::new()
function Log($s) { $log.Add($s); $s }

function Launch([string[]] $argv) {
    $p = Start-Process $Exe -ArgumentList $argv -PassThru
    $d = (Get-Date).AddSeconds(20)
    while ((Get-Date) -lt $d) { Start-Sleep -Milliseconds 200; $p.Refresh(); if ($p.MainWindowHandle -ne 0) { break } }
    Start-Sleep -Milliseconds 1500; $p.Refresh()
    $h = $p.MainWindowHandle
    [void][E]::SetWindowPos($h, [IntPtr]::Zero, -8, 0, 0, 0, 0x0015); Start-Sleep -Milliseconds 500
    [void][E]::SetForegroundWindow($h); Start-Sleep -Milliseconds 400
    $o = New-Object E+POINT; [void][E]::ClientToScreen($h, [ref]$o)
    $cr = New-Object E+RECT; [void][E]::GetClientRect($h, [ref]$cr)
    [pscustomobject]@{ P = $p; H = $h; O = $o; W = $cr.R; Hh = $cr.B }
}
function Shot($w, [string] $name) {
    $bmp = New-Object System.Drawing.Bitmap $w.W, $w.Hh; $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($w.O.X, $w.O.Y, 0, 0, (New-Object System.Drawing.Size $w.W, $w.Hh)); $g.Dispose()
    $bmp.Save((Join-Path $Out $name)); $bmp.Dispose()
}
# Layout per 04_DESIGN: toolbar 52 + propbar 40, status 30; image fit-centred, never upscaled.
function ImageOrigin($w, [int] $iw, [int] $ih) {
    $cw = $w.W; $ch = $w.Hh - 52 - 40 - 30
    $k = [math]::Min(1.0, [math]::Min(($cw - 48) / $iw, ($ch - 48) / $ih))
    [pscustomobject]@{ X = [math]::Round(($cw - $iw * $k) / 2); Y = 92 + [math]::Round(($ch - $ih * $k) / 2); K = $k }
}
function Drag($w, $x0, $y0, $x1, $y1) {
    [void][E]::SetCursorPos($w.O.X + $x0, $w.O.Y + $y0); Start-Sleep -Milliseconds 150
    [E]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80
    foreach ($i in 1..12) { [void][E]::SetCursorPos($w.O.X + $x0 + ($x1 - $x0) * $i / 12, $w.O.Y + $y0 + ($y1 - $y0) * $i / 12); Start-Sleep -Milliseconds 25 }
    Start-Sleep -Milliseconds 80; [E]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 300
}
function Key([byte] $vk, [switch] $Ctrl) {
    if ($Ctrl) { [E]::keybd_event(0x11, 0, 0, [IntPtr]::Zero) }
    [E]::keybd_event($vk, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 50; [E]::keybd_event($vk, 0, 2, [IntPtr]::Zero)
    if ($Ctrl) { [E]::keybd_event(0x11, 0, 2, [IntPtr]::Zero) }
    Start-Sleep -Milliseconds 250
}
function UiaNames($w) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($w.H)
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    @($all | ForEach-Object { $_.Current.Name } | Where-Object { $_ })
}

$src = [System.Drawing.Image]::FromFile((Resolve-Path $Image)); $iw = $src.Width; $ih = $src.Height; $src.Dispose()

# ---- 1. open via argument, draw, nudge, copy, read clipboard back ----
[System.Windows.Forms.Clipboard]::Clear()
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
Shot $w '01-opened.png'
$io = ImageOrigin $w $iw $ih
Log "LAYOUT client=$($w.W)x$($w.Hh) image_origin=$($io.X),$($io.Y) scale=$($io.K)"
# Phone number in the synthetic shot: source px (518,143)-(596,163) at scale 1.
Drag $w ($io.X + 518) ($io.Y + 143) ($io.X + 596) ($io.Y + 163)
Shot $w '02-drawn-selected.png'
Key 0x27; Key 0x27; Key 0x27            # Right arrow x3 -> region (521,143)-(599,163)
Key 0x43 -Ctrl                          # Ctrl+C
Start-Sleep -Milliseconds 1500
Shot $w '03-copied-toast.png'
$names = UiaNames $w
Log ("UIA toast=" + (($names | Where-Object { $_ -match '已复制|复制失败|内容已变化' }) -join ' | '))
$clip = [System.Windows.Forms.Clipboard]::GetImage()
if ($clip) { $clip.Save((Join-Path $Out 'clipboard.png'), [System.Drawing.Imaging.ImageFormat]::Png); Log "CLIPBOARD image=$($clip.Width)x$($clip.Height)" } else { Log "CLIPBOARD image=none" }
# Close: delivered, so no dialog expected.
[void][E]::PostMessage($w.H, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero); Start-Sleep -Milliseconds 1200
Log "CLOSE_AFTER_COPY exited=$($w.P.HasExited)"
if (-not $w.P.HasExited) { $w.P.Kill() }

# ---- 2. close guard with unsaved edits ----
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
$io = ImageOrigin $w $iw $ih
Drag $w ($io.X + 40) ($io.Y + 70) ($io.X + 200) ($io.Y + 100)
[void][E]::PostMessage($w.H, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero); Start-Sleep -Milliseconds 800
Shot $w '04-close-guard.png'
Log "CLOSE_GUARD exited=$($w.P.HasExited) dialog=$(((UiaNames $w) -match '有未复制或保存的编辑').Count -gt 0)"
Key 0x1B                                # Esc -> cancel
Start-Sleep -Milliseconds 400
Log "CLOSE_GUARD_CANCEL still_running=$(-not $w.P.HasExited)"
Key 0x5A -Ctrl                          # Ctrl+Z
Shot $w '05-after-undo.png'
if (-not $w.P.HasExited) { $w.P.Kill() }

# ---- 3. paste an image from the clipboard into an empty app ----
$img = [System.Drawing.Image]::FromFile((Resolve-Path $Image)); [System.Windows.Forms.Clipboard]::SetImage($img); $img.Dispose()
$w = Launch @()
Shot $w '06-empty.png'
Key 0x56 -Ctrl                          # Ctrl+V
Start-Sleep -Milliseconds 1500
Shot $w '07-pasted.png'
Log ("PASTE status=" + (((UiaNames $w) | Where-Object { $_ -match '×|已遮挡' }) -join ' | '))
if (-not $w.P.HasExited) { $w.P.Kill() }

$log | Set-Content -Encoding utf8 (Join-Path $Out 'e2e.log')
