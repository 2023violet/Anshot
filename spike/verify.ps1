# Drive spike-slint-verify with real OS input and record results.
# Usage: pwsh -File spike/verify.ps1 -Exe <exe> -Log <txt> [-Scale 1.5] [-Backend winit-software] [-Updates 20] [-Interact]
param(
    [Parameter(Mandatory)] [string] $Exe,
    [Parameter(Mandatory)] [string] $Log,
    [double] $Scale = 0,
    [string] $Backend = '',
    [int] $Updates = 0,
    [switch] $Interact,
    [string] $Shot = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class V {
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
  [DllImport("user32.dll")] public static extern IntPtr GetKeyboardLayout(uint thread);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
}
public static class Sink {
  public static System.Collections.Concurrent.ConcurrentQueue<string> Q = new System.Collections.Concurrent.ConcurrentQueue<string>();
  public static void Attach(System.Diagnostics.Process p) { p.OutputDataReceived += (s, e) => { if (e.Data != null) Q.Enqueue(e.Data); }; p.BeginOutputReadLine(); }
}
'@
[void][V]::SetProcessDPIAware()
$psi = [System.Diagnostics.ProcessStartInfo]::new($Exe)
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.WorkingDirectory = Split-Path $Exe
if ($Scale -gt 0) { $psi.Environment['SLINT_SCALE_FACTOR'] = "$Scale" }
if ($Backend) { $psi.Environment['SLINT_BACKEND'] = $Backend }
if ($Updates -gt 0) { $psi.Environment['VERIFY_UPDATES'] = "$Updates" }
$p = [System.Diagnostics.Process]::Start($psi)
[Sink]::Attach($p)
$out = [System.Collections.Generic.List[string]]::new()
function Drain { $x = $null; while ([Sink]::Q.TryDequeue([ref]$x)) { $out.Add($x) } }
$peakPrivate = 0; $peakWs = 0
function Sample { Drain; $p.Refresh(); if ($p.PrivateMemorySize64 -gt $script:peakPrivate) { $script:peakPrivate = $p.PrivateMemorySize64 }; if ($p.PeakWorkingSet64 -gt $script:peakWs) { $script:peakWs = $p.PeakWorkingSet64 } }
function Wait-For([string] $pattern, [int] $ms) { $d = (Get-Date).AddMilliseconds($ms); while ((Get-Date) -lt $d) { Sample; if ($out | Where-Object { $_ -match $pattern }) { return $true }; Start-Sleep -Milliseconds 100 }; return $false }
try {
    $d = (Get-Date).AddSeconds(20)
    while ((Get-Date) -lt $d) { Drain; $p.Refresh(); if ($p.MainWindowHandle -ne 0 -and ($out -match 'first_frame_rendered')) { break }; Start-Sleep -Milliseconds 150 }
    Start-Sleep -Milliseconds 800; $p.Refresh(); $h = $p.MainWindowHandle
    [void][V]::SetWindowPos($h, [IntPtr]::Zero, -8, 0, 0, 0, 0x0015); Start-Sleep -Milliseconds 600
    [void][V]::SetForegroundWindow($h); Start-Sleep -Milliseconds 400
    $o = New-Object V+POINT; [void][V]::ClientToScreen($h, [ref]$o)
    $s = if ($Scale -gt 0) { $Scale } else { 1.0 }
    function P([double] $lx, [double] $ly) { [int]($o.X + $lx * $s), [int]($o.Y + $ly * $s) }
    function Drag([double] $x0, [double] $y0, [double] $dx, [double] $dy) {
        $a = P $x0 $y0; [void][V]::SetCursorPos($a[0], $a[1]); Start-Sleep -Milliseconds 120
        [V]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80
        foreach ($i in 1..10) { $b = P ($x0 + $dx * $i / 10) ($y0 + $dy * $i / 10); [void][V]::SetCursorPos($b[0], $b[1]); Start-Sleep -Milliseconds 25 }
        Start-Sleep -Milliseconds 80; [V]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 250
    }
    if ($Updates -gt 0) { $ok = Wait-For 'updates_done' 180000; $out.Add("HARNESS updates_done_seen=$ok") }
    if ($Interact) {
        Drain; $out.Add("HARNESS interact_begin")
        # stage origin (20,60); selection starts at 120,100 160x60 (logical px)
        Drag (20 + 200) (60 + 130) 30 20                 # body move      -> 150,120,160,60
        Drag (20 + 310 + 6) (60 + 180 + 6) 25 15         # BR, 6px outside visual handle, inside 16px hit -> 150,120,185,75
        Drag (20 + 150) (60 + 120) 10 10                 # TL handle      -> 160,130,175,65
        Drag (20 + 335 + 14) (60 + 195 + 14) 20 20       # 14px off BR: miss, no change
        # IME: click text field, type pinyin through the active keyboard layout
        $tid = 0; $tid = [V]::GetWindowThreadProcessId($h, [ref]$null)
        $out.Add(("HARNESS keyboard_layout=0x{0:x}" -f [int64][V]::GetKeyboardLayout($tid)))
        $c = P 270 420; [void][V]::SetCursorPos($c[0], $c[1]); Start-Sleep -Milliseconds 200; [V]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); [V]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 400
        [V]::keybd_event(0x10, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60; [V]::keybd_event(0x10, 0, 2, [IntPtr]::Zero); Start-Sleep -Milliseconds 300
        foreach ($k in 0x4E, 0x49, 0x48, 0x41, 0x4F, 0x20) { [V]::keybd_event([byte]$k, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60; [V]::keybd_event([byte]$k, 0, 2, [IntPtr]::Zero); Start-Sleep -Milliseconds 90 }
        Start-Sleep -Milliseconds 800
        # UIA tree (what Narrator reads)
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        foreach ($e in $all) { $n = $e.Current.Name; if ($n) { $out.Add("UIA $($e.Current.ControlType.ProgrammaticName) name=$n focusable=$($e.Current.IsKeyboardFocusable)") } }
        $out.Add("UIA total_elements=$($all.Count)")
    }
    if ($Shot) {
        New-Item -ItemType Directory -Force (Split-Path $Shot) | Out-Null
        $cr = New-Object V+RECT; [void][V]::GetClientRect($h, [ref]$cr)
        $bmp = New-Object System.Drawing.Bitmap $cr.R, $cr.B; $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($o.X, $o.Y, 0, 0, (New-Object System.Drawing.Size $cr.R, $cr.B)); $g.Dispose(); $bmp.Save($Shot)
        $out.Add("HARNESS shot=$Shot $($cr.R)x$($cr.B)")
    }
    Sample
    $out.Add(("HARNESS peak_working_set_mb={0:N1} peak_private_mb={1:N1}" -f ($peakWs / 1MB), ($peakPrivate / 1MB)))
} finally {
    Drain
    if (-not $p.HasExited) { $p.Kill(); $p.WaitForExit(3000) | Out-Null }
    Drain
    New-Item -ItemType Directory -Force (Split-Path $Log) | Out-Null
    $out | Set-Content -Encoding utf8 $Log
    $out | Where-Object { $_ -notmatch '^VERIFY update step' } | ForEach-Object { $_ }
}
