# 07 performance and offline checks on the real app (agent-run, wall-clock, single machine).
# Usage: pwsh -STA -File tests/desktop/perf.ps1 -Exe <shareready.exe> -Big <6000x4000.png> -Out <dir>
param([Parameter(Mandatory)] [string] $Exe, [Parameter(Mandatory)] [string] $Big, [Parameter(Mandatory)] [string] $Out)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, UIAutomationClient, UIAutomationTypes
Add-Type @'
using System; using System.Runtime.InteropServices; using System.Text;
public static class P {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, IntPtr e);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  public static IntPtr Find(uint pid, string contains, long minArea) {
    IntPtr best = IntPtr.Zero;
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); var sb = new StringBuilder(256); GetWindowText(h, sb, 256); RECT r;
      if (p == pid && IsWindowVisible(h) && sb.ToString().Contains(contains) && GetWindowRect(h, out r) && (long)(r.R-r.L)*(r.B-r.T) >= minArea) best = h; return true; }, IntPtr.Zero);
    return best;
  }
}
'@
[void][P]::SetProcessDPIAware()
New-Item -ItemType Directory -Force $Out | Out-Null
$env:SHAREREADY_SETTINGS_DIR = Join-Path $Out "settings"
$env:SHAREREADY_HISTORY_DIR = Join-Path $Out "history-perf"
$log = [System.Collections.Generic.List[string]]::new()
function Log($s) { $log.Add($s); $s }
Log ("EXE sha256=" + (Get-FileHash $Exe -Algorithm SHA256).Hash.ToLower() + " path=" + (Split-Path $Exe -Leaf))
function Key([byte] $vk, [switch] $Ctrl, [switch] $Shift) {
    if ($Ctrl) { [P]::keybd_event(0x11, 0, 0, [IntPtr]::Zero) }; if ($Shift) { [P]::keybd_event(0x10, 0, 0, [IntPtr]::Zero) }
    [P]::keybd_event($vk, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 30; [P]::keybd_event($vk, 0, 2, [IntPtr]::Zero)
    if ($Shift) { [P]::keybd_event(0x10, 0, 2, [IntPtr]::Zero) }; if ($Ctrl) { [P]::keybd_event(0x11, 0, 2, [IntPtr]::Zero) }
}
function UiaHas($h, [string] $pattern) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($e in $all) { if ($e.Current.Name -match $pattern) { return $true } }
    return $false
}
function WaitUntil([scriptblock] $cond, [int] $ms) { $sw = [Diagnostics.Stopwatch]::StartNew(); while ($sw.ElapsedMilliseconds -lt $ms) { if (& $cond) { return $sw.ElapsedMilliseconds }; Start-Sleep -Milliseconds 20 }; return -1 }
$netSeen = [System.Collections.Generic.HashSet[string]]::new()
function NetProbe($p) {
    Get-NetTCPConnection -OwningProcess $p.Id -ErrorAction SilentlyContinue | ForEach-Object { [void]$netSeen.Add("TCP $($_.LocalAddress):$($_.LocalPort)->$($_.RemoteAddress):$($_.RemotePort) $($_.State)") }
    Get-NetUDPEndpoint -OwningProcess $p.Id -ErrorAction SilentlyContinue | ForEach-Object { [void]$netSeen.Add("UDP $($_.LocalAddress):$($_.LocalPort)") }
}

Get-Process shareready -ErrorAction SilentlyContinue | Stop-Process -Force
# 1. cold start to visible empty editor (3 runs)
$starts = @()
foreach ($i in 1..3) {
    $sw = [Diagnostics.Stopwatch]::StartNew(); $p = Start-Process $Exe -PassThru
    $t = WaitUntil { [P]::Find([uint32]$p.Id, 'ShareReady', 300000) -ne [IntPtr]::Zero } 15000
    $starts += $t; NetProbe $p; $p.Kill(); $p.WaitForExit(); Start-Sleep -Milliseconds 400
}
Log "STARTUP_MS runs=$($starts -join ',')"

# 2. hotkey -> overlay visible (3 runs each), Esc between: tray only (usual), then with the editor open
#    (0.1.1: the editor is hidden and the compositor flushed before the freeze).
foreach ($mode in @('tray', 'editor_open')) {
    $p = if ($mode -eq 'tray') { Start-Process $Exe -ArgumentList '--background' -PassThru } else { Start-Process $Exe -PassThru }
    Start-Sleep -Milliseconds 2500
    $caps = @()
    foreach ($i in 1..3) {
        $sw = [Diagnostics.Stopwatch]::StartNew(); Key 0x41 -Ctrl -Shift
        $t = WaitUntil { [P]::Find([uint32]$p.Id, 'ShareReady 截图', 1) -ne [IntPtr]::Zero } 10000
        $caps += $t; NetProbe $p; Start-Sleep -Milliseconds 300; Key 0x1B; Start-Sleep -Milliseconds 600
    }
    Log "HOTKEY_TO_OVERLAY_MS mode=$mode runs=$($caps -join ',')"
    if ($mode -eq 'tray') { $p.Kill(); $p.WaitForExit(); Start-Sleep -Milliseconds 400 }
}
$p.Kill(); $p.WaitForExit(); Start-Sleep -Milliseconds 400

# 3. open 24MP via argument -> dimensions shown; then redact + copy -> clipboard holds the image
[System.Windows.Forms.Clipboard]::Clear()
$sw = [Diagnostics.Stopwatch]::StartNew(); $p = Start-Process $Exe -ArgumentList "`"$((Resolve-Path $Big).Path)`"" -PassThru
[void](WaitUntil { [P]::Find([uint32]$p.Id, 'ShareReady', 300000) -ne [IntPtr]::Zero } 15000)
$h = [P]::Find([uint32]$p.Id, 'ShareReady', 300000)
$t = WaitUntil { UiaHas $h '6000 × 4000' } 30000
Log "OPEN_24MP_MS from_launch=$t"
[void][P]::SetWindowPos($h, [IntPtr]::Zero, -8, 0, 0, 0, 0x0015); Start-Sleep -Milliseconds 500; [void][P]::SetForegroundWindow($h); Start-Sleep -Milliseconds 300
$r = New-Object P+RECT; [void][P]::GetWindowRect($h, [ref]$r)
[void][P]::SetCursorPos($r.L + 400, $r.T + 300); Start-Sleep -Milliseconds 100; [P]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero)
foreach ($i in 1..10) { [void][P]::SetCursorPos($r.L + 400 + 20 * $i, $r.T + 300 + 8 * $i); Start-Sleep -Milliseconds 20 }
[P]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 500
$sw = [Diagnostics.Stopwatch]::StartNew(); Key 0x43 -Ctrl
$t = WaitUntil { [System.Windows.Forms.Clipboard]::ContainsImage() } 60000
Log "COPY_24MP_MS ctrl_c_to_clipboard=$t"
$img = [System.Windows.Forms.Clipboard]::GetImage(); if ($img) { Log "COPY_24MP size=$($img.Width)x$($img.Height)" }
NetProbe $p
$p.Refresh(); Log ("MEMORY_24MP peak_working_set_mb={0:N0}" -f ($p.PeakWorkingSet64 / 1MB))
$p.Kill()
Log "NETWORK endpoints_seen=$($netSeen.Count) $(@($netSeen) -join '; ')"
if (Test-Path $env:SHAREREADY_HISTORY_DIR) { Remove-Item -Recurse -Force $env:SHAREREADY_HISTORY_DIR }
$log | Set-Content -Encoding utf8 (Join-Path $Out 'perf.log')

