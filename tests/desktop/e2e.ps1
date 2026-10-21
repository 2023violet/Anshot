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
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
  public static IntPtr TitleAny(string title) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => { var sb = new System.Text.StringBuilder(256); GetWindowText(h, sb, 256);
      if (IsWindowVisible(h) && sb.ToString() == title) found = h; return true; }, IntPtr.Zero);
    return found;
  }
  public static System.Collections.Generic.List<IntPtr> Visible(uint pid, string title) {
    var r = new System.Collections.Generic.List<IntPtr>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); var sb = new System.Text.StringBuilder(256); GetWindowText(h, sb, 256);
      if (p == pid && IsWindowVisible(h) && sb.ToString() == title) r.Add(h); return true; }, IntPtr.Zero);
    return r;
  }
  // Largest visible top-level window of the process whose title mentions ShareReady.
  public static IntPtr Editor(uint pid) {
    IntPtr best = IntPtr.Zero; long area = 0;
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); var sb = new System.Text.StringBuilder(256); GetWindowText(h, sb, 256);
      RECT r; if (p == pid && IsWindowVisible(h) && sb.ToString().Contains("ShareReady") && GetWindowRect(h, out r)) {
        long a = (long)(r.R - r.L) * (r.B - r.T); if (a > area && a > 300000) { area = a; best = h; } } return true; }, IntPtr.Zero);
    return best;
  }
}
'@
[void][E]::SetProcessDPIAware()
New-Item -ItemType Directory -Force $Out | Out-Null
# Never touch the user's real settings: every launched instance reads this folder.
$env:SHAREREADY_SETTINGS_DIR = Join-Path $Out 'settings'
if (Test-Path $env:SHAREREADY_SETTINGS_DIR) { Get-ChildItem $env:SHAREREADY_SETTINGS_DIR | ForEach-Object { [System.IO.File]::Delete($_.FullName) } }
$log = [System.Collections.Generic.List[string]]::new()
function Log($s) { $log.Add($s); $s }
Log ("EXE sha256=" + (Get-FileHash $Exe -Algorithm SHA256).Hash.ToLower() + " path=" + (Split-Path $Exe -Leaf))

function Launch([string[]] $argv) {
    $p = Start-Process $Exe -ArgumentList $argv -PassThru
    $d = (Get-Date).AddSeconds(20)
    $h = [IntPtr]::Zero
    while ((Get-Date) -lt $d) { Start-Sleep -Milliseconds 200; $h = [E]::Editor([uint32]$p.Id); if ($h -ne [IntPtr]::Zero) { break } }
    Start-Sleep -Milliseconds 1500
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
function Key([byte] $vk, [switch] $Ctrl, [switch] $Shift) {
    if ($Ctrl) { [E]::keybd_event(0x11, 0, 0, [IntPtr]::Zero) }
    if ($Shift) { [E]::keybd_event(0x10, 0, 0, [IntPtr]::Zero) }
    [E]::keybd_event($vk, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 50; [E]::keybd_event($vk, 0, 2, [IntPtr]::Zero)
    if ($Shift) { [E]::keybd_event(0x10, 0, 2, [IntPtr]::Zero) }
    if ($Ctrl) { [E]::keybd_event(0x11, 0, 2, [IntPtr]::Zero) }
    Start-Sleep -Milliseconds 250
}
function ScreenDrag($x0, $y0, $x1, $y1) {
    [void][E]::SetCursorPos($x0, $y0); Start-Sleep -Milliseconds 200
    [E]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80
    foreach ($i in 1..15) { [void][E]::SetCursorPos($x0 + ($x1 - $x0) * $i / 15, $y0 + ($y1 - $y0) * $i / 15); Start-Sleep -Milliseconds 25 }
    Start-Sleep -Milliseconds 100; [E]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 300
}
function ScreenCrop($x, $y, $w, $h, [string] $name) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h; $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size $w, $h)); $g.Dispose(); $bmp.Save((Join-Path $Out $name)); $bmp.Dispose()
}
function WaitEditor($p) {
    $d = (Get-Date).AddSeconds(10)
    while ((Get-Date) -lt $d) { $p.Refresh(); $hs = [E]::Visible([uint32]$p.Id, '') ; Start-Sleep -Milliseconds 200; if ($p.MainWindowHandle -ne 0 -and [E]::IsWindowVisible($p.MainWindowHandle)) { return } }
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
# The editor hides after copy (U07): never screenshot its area now, it would show the real desktop.
$names = UiaNames $w
Log ("UIA toast=" + (($names | Where-Object { $_ -match '已复制|复制失败|内容已变化' }) -join ' | '))
$clip = [System.Windows.Forms.Clipboard]::GetImage()
if ($clip) { $clip.Save((Join-Path $Out 'clipboard.png'), [System.Drawing.Imaging.ImageFormat]::Png); Log "CLIPBOARD image=$($clip.Width)x$($clip.Height)" } else { Log "CLIPBOARD image=none" }
# 02 U07: after copy the editor hides, the process stays in the tray, a system toast shows.
$toast = [E]::Visible([uint32]$w.P.Id, 'ShareReady')
Log "AFTER_COPY editor_visible=$([E]::IsWindowVisible($w.H)) running=$(-not $w.P.HasExited) toast_windows=$($toast.Count)"
# Inset by 14 px so rounded transparent corners never include desktop pixels.
if ($toast.Count -gt 0) { $tr = New-Object E+RECT; [void][E]::GetWindowRect($toast[0], [ref]$tr); ScreenCrop ($tr.L + 14) ($tr.T + 14) ($tr.R - $tr.L - 28) ($tr.B - $tr.T - 28) '03-system-toast-inner.png' }
$w.P.Kill(); Start-Sleep -Milliseconds 500

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

# ---- 4. global hotkey capture of a synthetic window, drag-select, edit, copy ----
$fx = 200; $fy = 150
$fixture = Start-Process pwsh -ArgumentList @('-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'synthetic_window.ps1'), '-Image', "`"$((Resolve-Path $Image).Path)`"", '-X', $fx, '-Y', $fy) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3
[System.Windows.Forms.Clipboard]::Clear()
$app = Start-Process $Exe -PassThru
Start-Sleep -Seconds 2
Key 0x41 -Ctrl -Shift                   # Ctrl+Shift+A (global hotkey)
Start-Sleep -Milliseconds 1500
$ov = [E]::Visible([uint32]$app.Id, 'ShareReady 截图')
Log "CAPTURE overlays=$($ov.Count)"
ScreenDrag $fx $fy ($fx + $iw) ($fy + $ih)
# Crop strictly inside the synthetic window: only synthetic pixels are ever saved.
ScreenCrop $fx $fy $iw $ih '08-overlay-selected-inner.png'
Key 0x0D                                # Enter
Start-Sleep -Milliseconds 1200
Stop-Process -Id $fixture.Id -Force     # the top-most fixture must not cover the editor
Start-Sleep -Milliseconds 1300
$w = [pscustomobject]@{ P = $app; H = [E]::Editor([uint32]$app.Id); O = $null; W = 0; Hh = 0 }
[void][E]::SetWindowPos($w.H, [IntPtr]::Zero, -8, 0, 0, 0, 0x0015); Start-Sleep -Milliseconds 500; [void][E]::SetForegroundWindow($w.H); Start-Sleep -Milliseconds 300
$o = New-Object E+POINT; [void][E]::ClientToScreen($w.H, [ref]$o); $cr = New-Object E+RECT; [void][E]::GetClientRect($w.H, [ref]$cr)
$w.O = $o; $w.W = $cr.R; $w.Hh = $cr.B
Log ("CAPTURE editor_status=" + (((UiaNames $w) | Where-Object { $_ -match ' × ' }) -join ' | '))
$io = ImageOrigin $w $iw $ih
Drag $w ($io.X + 518) ($io.Y + 143) ($io.X + 596) ($io.Y + 163)
Shot $w '09-captured-in-editor.png'
Key 0x43 -Ctrl
Start-Sleep -Milliseconds 1500
$clip = [System.Windows.Forms.Clipboard]::GetImage()
if ($clip) { $clip.Save((Join-Path $Out 'clipboard-capture.png'), [System.Drawing.Imaging.ImageFormat]::Png); Log "CAPTURE clipboard=$($clip.Width)x$($clip.Height)" } else { Log "CAPTURE clipboard=none" }

# ---- 5. click-to-snap on the synthetic window ----
$fixture = Start-Process pwsh -ArgumentList @('-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'synthetic_window.ps1'), '-Image', "`"$((Resolve-Path $Image).Path)`"", '-X', $fx, '-Y', $fy) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3
Key 0x41 -Ctrl -Shift
Start-Sleep -Milliseconds 1500
[void][E]::SetCursorPos($fx + 300, $fy + 200); Start-Sleep -Milliseconds 300
[E]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60; [E]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 400
ScreenCrop $fx $fy $iw $ih '10-overlay-snapped-inner.png'
Key 0x0D
Start-Sleep -Milliseconds 1200
Stop-Process -Id $fixture.Id -Force
Start-Sleep -Milliseconds 1300
Log ("SNAP editor_status=" + (((UiaNames $w) | Where-Object { $_ -match ' × ' }) -join ' | '))

# ---- 6. Esc cancels the overlay ----
Key 0x41 -Ctrl -Shift
Start-Sleep -Milliseconds 1200
$before = ([E]::Visible([uint32]$app.Id, 'ShareReady 截图')).Count
Key 0x1B
Start-Sleep -Milliseconds 600
Log "ESC overlays_before=$before after=$(([E]::Visible([uint32]$app.Id, 'ShareReady 截图')).Count)"

if (-not $app.HasExited) { $app.Kill() }
if (-not $fixture.HasExited) { Stop-Process -Id $fixture.Id -Force }
Start-Sleep -Milliseconds 500

# ---- 7. B3: single-key tools, safe mosaic, arrow, text, copy ----
[System.Windows.Forms.Clipboard]::Clear()
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
$io = ImageOrigin $w $iw $ih
Key 0x4D                                # M: safe mosaic
Drag $w ($io.X + 374) ($io.Y + 143) ($io.X + 476) ($io.Y + 163)
Key 0x41                                # A: arrow
Drag $w ($io.X + 450) ($io.Y + 360) ($io.X + 360) ($io.Y + 300)
Key 0x54                                # T: text
[void][E]::SetCursorPos($w.O.X + $io.X + 400, $w.O.Y + $io.Y + 380); Start-Sleep -Milliseconds 200
[E]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60; [E]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 500
foreach ($k in 0x32, 0x30, 0x32, 0x36) { Key ([byte]$k) }   # "2026" (digits pass through any IME mode)
Key 0x0D                                # Enter commits the text
Shot $w '11-b3-annotated.png'
Key 0x43 -Ctrl
Start-Sleep -Milliseconds 1500
$clip = [System.Windows.Forms.Clipboard]::GetImage()
if ($clip) { $clip.Save((Join-Path $Out 'clipboard-b3.png'), [System.Drawing.Imaging.ImageFormat]::Png); Log "B3 clipboard=$($clip.Width)x$($clip.Height)" } else { Log "B3 clipboard=none" }
if (-not $w.P.HasExited) { $w.P.Kill() }
Start-Sleep -Milliseconds 500

# ---- 8. B4: OS drag-and-drop into an empty editor, then Ctrl+wheel zoom ----
$w = Launch @()
$src = Start-Process pwsh -ArgumentList @('-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'drag_source.ps1'), '-File', "`"$((Resolve-Path $Image).Path)`"", '-X', 1300, '-Y', 200) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3
[void][E]::SetCursorPos(1360, 230); Start-Sleep -Milliseconds 300
[E]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 300
foreach ($i in 1..25) { [void][E]::SetCursorPos(1360 - (1360 - ($w.O.X + 590)) * $i / 25, 230 + (($w.O.Y + 380) - 230) * $i / 25); Start-Sleep -Milliseconds 40 }
Start-Sleep -Milliseconds 400; [E]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 2500
Log ("DROP status=" + (((UiaNames $w) | Where-Object { $_ -match ' × ' }) -join ' | '))
if (-not $src.HasExited) { Stop-Process -Id $src.Id -Force }
[void][E]::SetForegroundWindow($w.H); Start-Sleep -Milliseconds 300
[void][E]::SetCursorPos($w.O.X + 590, $w.O.Y + 400); Start-Sleep -Milliseconds 200
[E]::keybd_event(0x11, 0, 0, [IntPtr]::Zero)
foreach ($i in 1..3) { [E]::mouse_event(0x0800, 0, 0, 120, [IntPtr]::Zero); Start-Sleep -Milliseconds 150 }
[E]::keybd_event(0x11, 0, 2, [IntPtr]::Zero); Start-Sleep -Milliseconds 500
Log ("ZOOM after_ctrl_wheel=" + (((UiaNames $w) | Where-Object { $_ -match '当前缩放' }) -join ' | '))
Shot $w '12-zoomed.png'
if (-not $w.P.HasExited) { $w.P.Kill() }
Start-Sleep -Milliseconds 500

# ---- 9. keyboard: Ctrl+Right x5 resizes the selected redaction, then Ctrl+C ----
[System.Windows.Forms.Clipboard]::Clear()
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
$io = ImageOrigin $w $iw $ih
Drag $w ($io.X + 518) ($io.Y + 143) ($io.X + 596) ($io.Y + 163)
foreach ($i in 1..5) { Key 0x27 -Ctrl }
Key 0x43 -Ctrl
Start-Sleep -Milliseconds 1500
$clip = [System.Windows.Forms.Clipboard]::GetImage()
if ($clip) { $clip.Save((Join-Path $Out 'clipboard-keyboard.png'), [System.Drawing.Imaging.ImageFormat]::Png); Log "KEYBOARD clipboard=$($clip.Width)x$($clip.Height)" } else { Log 'KEYBOARD clipboard=none' }
if (-not $w.P.HasExited) { $w.P.Kill() }


function WaitTitle([string] $t, [int] $ms) { $d = (Get-Date).AddMilliseconds($ms); while ((Get-Date) -lt $d) { $h = [E]::TitleAny($t); if ($h -ne [IntPtr]::Zero) { return $h }; Start-Sleep -Milliseconds 150 }; return [IntPtr]::Zero }
function Invoke-Uia($w, [string] $name) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($w.H)
    $cond = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $name
    $el = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
    if (-not $el) { return $false }
    $pat = $null
    if ($el.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pat)) { $pat.Invoke(); Start-Sleep -Milliseconds 400; return $true }
    return $false
}
function CopyTo([string] $name) {
    [System.Windows.Forms.Clipboard]::Clear(); Key 0x43 -Ctrl; Start-Sleep -Milliseconds 1500
    $c = [System.Windows.Forms.Clipboard]::GetImage()
    if ($c) { $c.Save((Join-Path $Out $name), [System.Drawing.Imaging.ImageFormat]::Png); return "$($c.Width)x$($c.Height)" } else { return 'none' }
}

# ---- 10. real Save As dialog, then a second save to the same name (never overwrites) ----
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
$io = ImageOrigin $w $iw $ih
Drag $w ($io.X + 518) ($io.Y + 143) ($io.X + 596) ($io.Y + 163)
$saved = Join-Path $Out 'saved.png'
foreach ($f in @($saved, (Join-Path $Out 'saved (2).png'))) { if (Test-Path $f) { [System.IO.File]::Delete($f) } }
foreach ($round in 1..2) {
    Key 0x53 -Ctrl
    $dlg = WaitTitle '另存为新 PNG' 8000
    Log "SAVE round=$round dialog=$($dlg -ne [IntPtr]::Zero)"
    if ($dlg -eq [IntPtr]::Zero) { break }
    [void][E]::SetForegroundWindow($dlg); Start-Sleep -Milliseconds 500
    [System.Windows.Forms.SendKeys]::SendWait('^a'); Start-Sleep -Milliseconds 100
    [System.Windows.Forms.SendKeys]::SendWait($saved); Start-Sleep -Milliseconds 200
    [System.Windows.Forms.SendKeys]::SendWait('{ENTER}'); Start-Sleep -Milliseconds 1200
    if ($round -eq 2) {
        # Windows asks "replace?"; answering yes must still not overwrite.
        [System.Windows.Forms.SendKeys]::SendWait('%y'); Start-Sleep -Milliseconds 1500
    }
    Start-Sleep -Milliseconds 1500
}
$h1 = if (Test-Path $saved) { (Get-FileHash $saved).Hash } else { 'missing' }
Log "SAVE first=$(Test-Path $saved) second=$(Test-Path (Join-Path $Out 'saved (2).png'))"
Start-Sleep -Milliseconds 300
$h1b = if (Test-Path $saved) { (Get-FileHash $saved).Hash } else { 'missing' }
Log "SAVE original_unchanged=$($h1 -eq $h1b)"
if (-not $w.P.HasExited) { $w.P.Kill() }
Start-Sleep -Milliseconds 500

# ---- 11. keyboard routing: R, colour 4, Esc, Tab, Delete, Ctrl+Z, Ctrl+Y ----
# Keep the editor open after copy so the following keys reach it.
New-Item -ItemType Directory -Force $env:SHAREREADY_SETTINGS_DIR | Out-Null
Set-Content -Encoding utf8 (Join-Path $env:SHAREREADY_SETTINGS_DIR 'settings.txt') 'hide_after_copy=false'
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
$io = ImageOrigin $w $iw $ih
Key 0x52                                  # R: frame
Drag $w ($io.X + 300) ($io.Y + 380) ($io.X + 420) ($io.Y + 420)
Key 0x34                                  # 4: blue (applies to the selection)
Log "KEYS blue=$(CopyTo 'keys-1-blue.png')"
Key 0x1B; Key 0x09; Key 0x2E              # Esc, Tab (select), Delete
Log "KEYS deleted=$(CopyTo 'keys-2-deleted.png')"
Key 0x5A -Ctrl                            # Ctrl+Z
Log "KEYS undone=$(CopyTo 'keys-3-undo.png')"
Key 0x59 -Ctrl                            # Ctrl+Y
Log "KEYS redone=$(CopyTo 'keys-4-redo.png')"
if (-not $w.P.HasExited) { $w.P.Kill() }
[System.IO.File]::Delete((Join-Path $env:SHAREREADY_SETTINGS_DIR 'settings.txt'))
Start-Sleep -Milliseconds 500

# ---- 12. settings: Ctrl+, -> choose Ctrl+Alt+A, persisted and active; copy keeps editor ----
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
Key 0xBC -Ctrl                            # Ctrl+,
Start-Sleep -Milliseconds 500
Log "SETTINGS dialog=$(((UiaNames $w) -contains '设置'))"
# Ctrl+Alt+A is often owned by another app (e.g. QQ); then ShareReady must keep the old key and say so.
[void](Invoke-Uia $w 'Ctrl+Alt+A')
Log "SETTINGS taken_key_warning=$(((UiaNames $w) -match '被其他程序占用').Count -gt 0)"
Log "SETTINGS pick_hotkey=$(Invoke-Uia $w 'Ctrl+Shift+X') pick_stay=$(Invoke-Uia $w '保留编辑窗')"
Shot $w '13-settings.png'
Key 0x1B
$sf = Join-Path $env:SHAREREADY_SETTINGS_DIR 'settings.txt'
Log ("SETTINGS file=" + ((Get-Content $sf -ErrorAction SilentlyContinue) -join ' | '))
Key 0x58 -Ctrl -Shift                     # Ctrl+Shift+X, the newly chosen hotkey
Start-Sleep -Milliseconds 1200
Log "SETTINGS new_hotkey_overlays=$(([E]::Visible([uint32]$w.P.Id, 'ShareReady 截图')).Count)"
Key 0x1B; Start-Sleep -Milliseconds 500
[void][E]::SetForegroundWindow($w.H); Start-Sleep -Milliseconds 300
$io = ImageOrigin $w $iw $ih
Drag $w ($io.X + 518) ($io.Y + 143) ($io.X + 596) ($io.Y + 163)
Key 0x43 -Ctrl; Start-Sleep -Milliseconds 1500
Log "SETTINGS editor_visible_after_copy=$([E]::IsWindowVisible($w.H))"
if (-not $w.P.HasExited) { $w.P.Kill() }
if (Test-Path $sf) { [System.IO.File]::Delete($sf) }
Start-Sleep -Milliseconds 500

# ---- 13. simulated 125% scale (SLINT_SCALE_FACTOR): physical-pixel drag maps to exact source pixels ----
$env:SLINT_SCALE_FACTOR = '1.25'
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
Remove-Item Env:SLINT_SCALE_FACTOR
$k = 1.25; $lw = $w.W / $k; $lh = $w.Hh / $k
$cw = $lw; $ch = $lh - 52 - 40 - 30
$sc = [math]::Min(1.0, [math]::Min(($cw - 48) / $iw, ($ch - 48) / $ih))
$ox = [math]::Round(($cw - $iw * $sc) / 2); $oy = 92 + [math]::Round(($ch - $ih * $sc) / 2)
Log "DPI125 client=$($w.W)x$($w.Hh) logical_origin=$ox,$oy scale=$sc"
Drag $w (($ox + 520) * $k) (($oy + 144) * $k) (($ox + 596) * $k) (($oy + 164) * $k)
Log "DPI125 copy=$(CopyTo 'clipboard-dpi125.png')"
if (-not $w.P.HasExited) { $w.P.Kill() }
Start-Sleep -Milliseconds 500

# ---- 14. multi-line text: Shift+Enter inserts a line, Enter commits ----
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
$io = ImageOrigin $w $iw $ih
Key 0x54
[void][E]::SetCursorPos($w.O.X + $io.X + 40, $w.O.Y + $io.Y + 330); Start-Sleep -Milliseconds 200
[E]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60; [E]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 500
foreach ($kk in 0x31, 0x32) { Key ([byte]$kk) }
Key 0x0D -Shift
foreach ($kk in 0x33, 0x34) { Key ([byte]$kk) }
Key 0x0D
Shot $w '14-multiline.png'
Log "MULTILINE copy=$(CopyTo 'clipboard-multiline.png')"
if (-not $w.P.HasExited) { $w.P.Kill() }

Start-Sleep -Milliseconds 500

# ---- 15. (0.1.1 R1) the editor leaves the screen before the freeze and returns on Esc ----
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
Key 0x41 -Ctrl -Shift; Start-Sleep -Milliseconds 1200
$ov = ([E]::Visible([uint32]$w.P.Id, 'ShareReady 截图')).Count
Log "HIDE overlays=$ov editor_visible_during_capture=$([E]::IsWindowVisible($w.H))"
Key 0x1B; Start-Sleep -Milliseconds 800
Log "HIDE editor_visible_after_esc=$([E]::IsWindowVisible($w.H))"

# ---- 16. (R3) hovering a tool shows its tooltip ----
[void][E]::SetForegroundWindow($w.H); Start-Sleep -Milliseconds 300
[void][E]::SetCursorPos($w.O.X + 160, $w.O.Y + 26); Start-Sleep -Milliseconds 100
[void][E]::SetCursorPos($w.O.X + 169, $w.O.Y + 26); Start-Sleep -Milliseconds 1200
Shot $w '15-tooltip.png'
[void][E]::SetCursorPos($w.O.X + 600, $w.O.Y + 700); Start-Sleep -Milliseconds 300

# ---- 17. (R4) Shift while drawing a redaction gives a square ----
$io = ImageOrigin $w $iw $ih
[E]::keybd_event(0x10, 0, 0, [IntPtr]::Zero)
Drag $w ($io.X + 300) ($io.Y + 300) ($io.X + 360) ($io.Y + 320)
[E]::keybd_event(0x10, 0, 2, [IntPtr]::Zero); Start-Sleep -Milliseconds 200
Log "SHIFT copy=$(CopyTo 'clipboard-shift-square.png')"
if (-not $w.P.HasExited) { $w.P.Kill() }
Start-Sleep -Milliseconds 500

# ---- 18. (R2) unreadable files get plain wording ----
$broken = Join-Path $Out 'broken.png'
[System.IO.File]::WriteAllBytes($broken, [byte[]](0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A) + [System.Text.Encoding]::ASCII.GetBytes('not really a png'))
foreach ($case in @(@('broken', $broken), @('missing', (Join-Path $Out 'no-such-file.png')))) {
    $w = Launch @("`"$($case[1])`"")
    Start-Sleep -Milliseconds 800
    $names = (UiaNames $w) -join ' | '
    $raw = $names -match 'CoreFailure|os error|ChunkType|\{'
    $msg = (UiaNames $w) | Where-Object { $_ -match '不是有效|找不到' } | Select-Object -First 1
    Log "ERRORS case=$($case[0]) message=$msg raw_text=$raw"
    if (-not $w.P.HasExited) { $w.P.Kill() }
    Start-Sleep -Milliseconds 500
}
[System.IO.File]::Delete($broken)

# ---- 19. (R5) start at login: written to a scratch key only, --background stays in the tray ----
$env:SHAREREADY_RUN_KEY = 'Software\ShareReady-e2e\Run'
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
Key 0xBC -Ctrl; Start-Sleep -Milliseconds 500
Log "AUTOSTART on=$(Invoke-Uia $w '登录后在托盘待命')"
$val = (Get-ItemProperty -Path 'HKCU:\Software\ShareReady-e2e\Run' -Name ShareReady -ErrorAction SilentlyContinue).ShareReady
Log "AUTOSTART value_has_exe=$($val -like "*$(Split-Path $Exe -Leaf)*") background=$($val -like '* --background')"
Shot $w '16-settings-autostart.png'
Log "AUTOSTART off=$(Invoke-Uia $w '不启动')"
$gone = $null -eq (Get-ItemProperty -Path 'HKCU:\Software\ShareReady-e2e\Run' -Name ShareReady -ErrorAction SilentlyContinue)
Log "AUTOSTART removed=$gone"
if (-not $w.P.HasExited) { $w.P.Kill() }
Start-Sleep -Milliseconds 500
$p = Start-Process $Exe -ArgumentList '--background' -PassThru; Start-Sleep -Seconds 3
Log "BACKGROUND running=$(-not $p.HasExited) editor_windows=$(if ([E]::Editor([uint32]$p.Id) -eq [IntPtr]::Zero) { 0 } else { 1 })"
if (-not $p.HasExited) { $p.Kill() }
Remove-Item -Path 'HKCU:\Software\ShareReady-e2e' -Recurse -ErrorAction SilentlyContinue
Remove-Item Env:SHAREREADY_RUN_KEY
Start-Sleep -Milliseconds 500

# ---- 20. (R6) capture selection: drag the corner to resize, drag inside to move ----
$fixture = Start-Process pwsh -ArgumentList @('-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'synthetic_window.ps1'), '-Image', "`"$((Resolve-Path $Image).Path)`"", '-X', $fx, '-Y', $fy) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3
$app = Start-Process $Exe -ArgumentList '--background' -PassThru
Start-Sleep -Seconds 2
Key 0x41 -Ctrl -Shift; Start-Sleep -Milliseconds 1500
ScreenDrag ($fx + 100) ($fy + 100) ($fx + 300) ($fy + 200)
ScreenDrag ($fx + 300) ($fy + 200) ($fx + 340) ($fy + 230)
ScreenDrag ($fx + 200) ($fy + 150) ($fx + 220) ($fy + 160)
ScreenCrop $fx $fy $iw $ih '17-overlay-adjusted-inner.png'
Key 0x0D; Start-Sleep -Milliseconds 1200
Stop-Process -Id $fixture.Id -Force; Start-Sleep -Milliseconds 1300
$w = [pscustomobject]@{ P = $app; H = [E]::Editor([uint32]$app.Id); O = $null; W = 0; Hh = 0 }
[void][E]::SetForegroundWindow($w.H); Start-Sleep -Milliseconds 300
Log ("ADJUST editor_status=" + (((UiaNames $w) | Where-Object { $_ -match ' × ' }) -join ' | '))
Log "ADJUST copy=$(CopyTo 'clipboard-adjusted.png')"
if (-not $app.HasExited) { $app.Kill() }

Start-Sleep -Milliseconds 500

# ---- 21. (R1, pixels) a full-screen capture taken while the editor is open never contains the editor ----
# A synthetic backdrop fills the primary screen behind the editor. Captured pixels are compared in
# memory only and never written to disk (other apps' top-most windows could be in them).
$sb = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$backdrop = Join-Path $Out 'backdrop-synthetic.png'
$bmp = New-Object System.Drawing.Bitmap $sb.Width, $sb.Height; $g = [System.Drawing.Graphics]::FromImage($bmp)
$g.Clear([System.Drawing.Color]::FromArgb(52, 58, 70)); $tile = [System.Drawing.Image]::FromFile((Resolve-Path $Image))
for ($ty = 0; $ty -lt $sb.Height; $ty += $tile.Height + 37) { for ($tx = 0; $tx -lt $sb.Width; $tx += $tile.Width + 53) { $g.DrawImageUnscaled($tile, $tx, $ty) } }
$tile.Dispose(); $g.Dispose(); $bmp.Save($backdrop, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
$fixture = Start-Process pwsh -ArgumentList @('-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'synthetic_window.ps1'), '-Image', "`"$backdrop`"", '-X', 0, '-Y', 0, '-Behind') -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
Key 0x41 -Ctrl -Shift; Start-Sleep -Milliseconds 1200
[void][E]::SetCursorPos($w.O.X + 400, $w.O.Y + 300); Start-Sleep -Milliseconds 200
Key 0x0D; Start-Sleep -Milliseconds 2000            # Enter: the whole screen
Stop-Process -Id $fixture.Id -Force; Start-Sleep -Milliseconds 500
[System.Windows.Forms.Clipboard]::Clear(); Key 0x43 -Ctrl; Start-Sleep -Milliseconds 2000
$clip = [System.Windows.Forms.Clipboard]::GetImage()
if ($clip) {
    $cb = New-Object System.Drawing.Bitmap $clip; $ref = New-Object System.Drawing.Bitmap $backdrop
    $n = 0; $bad = 0
    for ($y = $w.O.Y + 2; $y -lt [math]::Min($w.O.Y + $w.Hh - 2, $cb.Height); $y += 7) {
        for ($x = [math]::Max(0, $w.O.X + 2); $x -lt [math]::Min($w.O.X + $w.W - 2, $cb.Width); $x += 7) {
            $n++; if ($cb.GetPixel($x, $y).ToArgb() -ne $ref.GetPixel($x, $y).ToArgb()) { $bad++ }
        }
    }
    Log "FREEZE capture=$($cb.Width)x$($cb.Height) editor_area_samples=$n mismatches=$bad"
    $cb.Dispose(); $ref.Dispose(); $clip.Dispose()
} else { Log "FREEZE capture=none" }
[System.Windows.Forms.Clipboard]::Clear()
if (-not $w.P.HasExited) { $w.P.Kill() }
[System.IO.File]::Delete($backdrop)

Start-Sleep -Milliseconds 500

# ---- 22. (R7) the magnifier follows the real pointer without any mouse-move event ----
$fixture = Start-Process pwsh -ArgumentList @('-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'synthetic_window.ps1'), '-Image', "`"$((Resolve-Path $Image).Path)`"", '-X', $fx, '-Y', $fy) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3
$app = Start-Process $Exe -ArgumentList '--background' -PassThru; Start-Sleep -Seconds 2
Key 0x41 -Ctrl -Shift; Start-Sleep -Milliseconds 1200
$ovh = ([E]::Visible([uint32]$app.Id, 'ShareReady 截图'))[0]
[void][E]::SetCursorPos($fx + 333, $fy + 222); Start-Sleep -Milliseconds 400
$mag = (UiaNames ([pscustomobject]@{ H = $ovh })) | Where-Object { $_ -like '放大镜*' } | Select-Object -First 1
Log "MAGNIFIER expected=x $($fx + 333)  y $($fy + 222) got=$mag"

# ---- 23. overlay toolbar: Ctrl+C copies the selection without opening the editor ----
[System.Windows.Forms.Clipboard]::Clear()
ScreenDrag ($fx + 100) ($fy + 100) ($fx + 300) ($fy + 200)
Start-Sleep -Milliseconds 300
$tb = ((UiaNames ([pscustomobject]@{ H = $ovh })) -contains '复制')
Key 0x43 -Ctrl; Start-Sleep -Milliseconds 2000
$clip = [System.Windows.Forms.Clipboard]::GetImage()
if ($clip) { $clip.Save((Join-Path $Out 'clipboard-quick-copy.png'), [System.Drawing.Imaging.ImageFormat]::Png); $cs = "$($clip.Width)x$($clip.Height)"; $clip.Dispose() } else { $cs = 'none' }
$ed = [E]::Editor([uint32]$app.Id)
Log "QUICKCOPY toolbar=$tb clipboard=$cs editor_visible=$(($ed -ne [IntPtr]::Zero) -and [E]::IsWindowVisible($ed)) toasts=$(([E]::Visible([uint32]$app.Id, 'ShareReady')).Count)"

# ---- 24. overlay toolbar: the Edit button opens the selection in the editor ----
Key 0x41 -Ctrl -Shift; Start-Sleep -Milliseconds 1200
$ovh = ([E]::Visible([uint32]$app.Id, 'ShareReady 截图'))[0]
ScreenDrag ($fx + 50) ($fy + 60) ($fx + 250) ($fy + 160)
Start-Sleep -Milliseconds 300
Log "TOOLBAR edit=$(Invoke-Uia ([pscustomobject]@{ H = $ovh }) '编辑')"
Start-Sleep -Milliseconds 1500
Stop-Process -Id $fixture.Id -Force; Start-Sleep -Milliseconds 800
$w = [pscustomobject]@{ P = $app; H = [E]::Editor([uint32]$app.Id); O = $null; W = 0; Hh = 0 }
Log ("TOOLBAR editor_status=" + (((UiaNames $w) | Where-Object { $_ -match ' × ' }) -join ' | '))
if (-not $app.HasExited) { $app.Kill() }
Start-Sleep -Milliseconds 500

# ---- 25. ellipse (E), Shift line (L), pen (P), custom colour ----
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
$io = ImageOrigin $w $iw $ih
Key 0x45                                   # E: ellipse
Drag $w ($io.X + 40) ($io.Y + 330) ($io.X + 200) ($io.Y + 410)
Key 0x1B
Key 0x4C                                   # L: line, Shift keeps it level
[E]::keybd_event(0x10, 0, 0, [IntPtr]::Zero)
Drag $w ($io.X + 230) ($io.Y + 380) ($io.X + 400) ($io.Y + 386)
[E]::keybd_event(0x10, 0, 2, [IntPtr]::Zero); Start-Sleep -Milliseconds 200
Key 0x1B
Key 0x50                                   # P: pen
[void][E]::SetCursorPos($w.O.X + $io.X + 420, $w.O.Y + $io.Y + 330); Start-Sleep -Milliseconds 150
[E]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero)
foreach ($i in 1..30) { [void][E]::SetCursorPos($w.O.X + $io.X + 420 + $i * 5, $w.O.Y + $io.Y + 330 + [int](20 * [math]::Sin($i / 3.0))); Start-Sleep -Milliseconds 15 }
[E]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 300
Key 0x1B
Key 0x52                                   # R: frame, then pick the custom colour
Log "PICKER open=$(Invoke-Uia $w '自定义…')"
Start-Sleep -Milliseconds 400
Shot $w '18-color-picker.png'
Key 0x0D; Start-Sleep -Milliseconds 300    # Enter: use the shown colour (#9B30D9 default)
Drag $w ($io.X + 300) ($io.Y + 250) ($io.X + 360) ($io.Y + 300)
Shot $w '19-new-shapes.png'
Log "SHAPES copy=$(CopyTo 'clipboard-shapes.png')"
Log ("PICKER settings=" + ((Get-Content (Join-Path $env:SHAREREADY_SETTINGS_DIR 'settings.txt') -ErrorAction SilentlyContinue) -match 'recent_colors' -join ''))
if (-not $w.P.HasExited) { $w.P.Kill() }

# Leave no test settings behind.
if (Test-Path $env:SHAREREADY_SETTINGS_DIR) { Remove-Item -Recurse -Force $env:SHAREREADY_SETTINGS_DIR }
$log | Set-Content -Encoding utf8 (Join-Path $Out 'e2e.log')
