# Chinese IME composition in a text annotation (Microsoft Pinyin, real OS input).
# Assumes Pinyin starts in English mode: one Shift tap switches to Chinese, a second restores it.
# Expect: Space picks 你好, Enter while composing "ma" inserts the letters and keeps editing, the next Enter commits.
# Usage: pwsh -STA -File tests/desktop/ime.ps1 -Exe <shareready.exe> -Image <png> -Out <dir>
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
function CopyTo([string] $name) {
    [System.Windows.Forms.Clipboard]::Clear(); Key 0x43 -Ctrl; Start-Sleep -Milliseconds 1500
    $c = [System.Windows.Forms.Clipboard]::GetImage()
    if ($c) { $c.Save((Join-Path $Out $name), [System.Drawing.Imaging.ImageFormat]::Png); return "$($c.Width)x$($c.Height)" } else { return 'none' }
}


# ---- Pinyin composition inside a text annotation ----
$w = Launch @("`"$((Resolve-Path $Image).Path)`"")
$io = ImageOrigin $w $iw $ih
Key 0x54
[void][E]::SetCursorPos($w.O.X + $io.X + 40, $w.O.Y + $io.Y + 330); Start-Sleep -Milliseconds 200
[E]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60; [E]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 600
Key 0x10; Start-Sleep -Milliseconds 300                          # Shift tap: Pinyin English -> Chinese mode
foreach ($kk in 0x4E, 0x49, 0x48, 0x41, 0x4F) { Key ([byte]$kk) }      # n i h a o
Shot $w 'ime-1-composing.png'
Key 0x20                                                             # Space: pick first candidate
Shot $w 'ime-2-committed.png'
foreach ($kk in 0x4D, 0x41) { Key ([byte]$kk) }                      # m a (composing)
Key 0x0D                                                             # Enter while composing
Start-Sleep -Milliseconds 300
Shot $w 'ime-3-enter-while-composing.png'
Key 0x10; Start-Sleep -Milliseconds 300                          # Shift tap: restore English mode
Shot $w 'ime-3b-restored.png'
Key 0x0D                                                             # Enter again: commit annotation
Shot $w 'ime-4-final.png'
Log "IME copy=$(CopyTo 'clipboard-ime.png')"
if (-not $w.P.HasExited) { $w.P.Kill() }
$log | Set-Content -Encoding utf8 (Join-Path $Out 'ime.log')
