# Stability soak on the real app (agent-run, single machine): every few seconds the global
# hotkey opens the overlay, a region of the synthetic window is selected and Ctrl+C copies it.
# Logs crashes, failed copies, and memory / handles once a minute. Only synthetic pixels are
# selected; the clipboard image is checked for size in memory and never written to disk.
# Takes over the mouse and keyboard for the whole run: run it on an idle machine. Rounds where the
# pointer was not where the script put it (someone else moved or clicked the mouse) or the screen was
# locked are counted as INTERFERED, apart from real failures.
# Usage: pwsh -STA -File tests/desktop/soak.ps1 -Exe <Anshot.exe> -Image <png> -Out <dir> [-Minutes 60] [-Every 5]
param(
    [Parameter(Mandatory)] [string] $Exe,
    [Parameter(Mandatory)] [string] $Image,
    [Parameter(Mandatory)] [string] $Out,
    [int] $Minutes = 60,
    [int] $Every = 5
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class K {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, IntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct P { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out P p);
  public static bool At(int x, int y) { P p; return GetCursorPos(out p) && p.X == x && p.Y == y; }
}
'@
[void][K]::SetProcessDPIAware()
New-Item -ItemType Directory -Force $Out | Out-Null
$log = [System.Collections.Generic.List[string]]::new()
function Log($s) { $log.Add($s); $s }
function Key([byte] $vk, [switch] $Ctrl, [switch] $Shift) {
    if ($Ctrl) { [K]::keybd_event(0x11, 0, 0, [IntPtr]::Zero) }
    if ($Shift) { [K]::keybd_event(0x10, 0, 0, [IntPtr]::Zero) }
    [K]::keybd_event($vk, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 50; [K]::keybd_event($vk, 0, 2, [IntPtr]::Zero)
    if ($Shift) { [K]::keybd_event(0x10, 0, 2, [IntPtr]::Zero) }
    if ($Ctrl) { [K]::keybd_event(0x11, 0, 2, [IntPtr]::Zero) }
}
# Returns $false when the pointer was not where the script put it right before the press or release.
function ScreenDrag($x0, $y0, $x1, $y1) {
    [void][K]::SetCursorPos($x0, $y0); Start-Sleep -Milliseconds 150
    $ok = [K]::At($x0, $y0)
    [K]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60
    foreach ($i in 1..10) { [void][K]::SetCursorPos($x0 + ($x1 - $x0) * $i / 10, $y0 + ($y1 - $y0) * $i / 10); Start-Sleep -Milliseconds 20 }
    Start-Sleep -Milliseconds 80
    $ok = $ok -and [K]::At($x1, $y1)
    [K]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 250
    $ok
}
# The lock screen (LogonUI) is up: no capture is possible, and none is expected.
function Locked { @(Get-Process LogonUI -ErrorAction SilentlyContinue).Count -gt 0 }

Log "EXE sha256=$((Get-FileHash $Exe -Algorithm SHA256).Hash.ToLower()) path=$(Split-Path $Exe -Leaf)"
# Test-only settings and history, removed at the end.
$tmp = Join-Path ([IO.Path]::GetTempPath()) "anshot-soak-$PID"
$env:ANSHOT_SETTINGS_DIR = "$tmp\settings"; $env:ANSHOT_HISTORY_DIR = "$tmp\history"
New-Item -ItemType Directory -Force $env:ANSHOT_SETTINGS_DIR | Out-Null

$fx = 200; $fy = 150
$fixture = Start-Process pwsh -ArgumentList @('-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'synthetic_window.ps1'), '-Image', "`"$((Resolve-Path $Image).Path)`"", '-X', $fx, '-Y', $fy) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3
$app = Start-Process $Exe -ArgumentList '--background' -PassThru
Start-Sleep -Seconds 3

$ok = 0; $failed = 0; $interfered = 0; $crashes = 0; $n = 0
$end = (Get-Date).AddMinutes($Minutes); $nextSample = Get-Date
while ((Get-Date) -lt $end) {
    $t0 = Get-Date
    if ($app.HasExited) {
        $crashes++; Log "CRASH at=$($t0.ToString('HH:mm:ss')) exit=$($app.ExitCode)"
        $app = Start-Process $Exe -ArgumentList '--background' -PassThru; Start-Sleep -Seconds 3
    }
    if ($t0 -ge $nextSample) {
        $app.Refresh()
        Log ("SAMPLE min={0} iter={1} ws_mb={2:N1} private_mb={3:N1} handles={4}" -f [math]::Round(($t0 - $end.AddMinutes(-$Minutes)).TotalMinutes), $n, ($app.WorkingSet64 / 1MB), ($app.PrivateMemorySize64 / 1MB), $app.HandleCount)
        $nextSample = $t0.AddMinutes(1)
    }
    $n++
    [System.Windows.Forms.Clipboard]::Clear()
    Key 0x41 -Ctrl -Shift; Start-Sleep -Milliseconds 700
    # Vary the region a little so every copy is a fresh render.
    $dx = ($n % 7) * 10
    $clean = ScreenDrag ($fx + 40 + $dx) ($fy + 40) ($fx + 240 + $dx) ($fy + 140)
    Key 0x43 -Ctrl; Start-Sleep -Milliseconds 1200
    $clip = [System.Windows.Forms.Clipboard]::GetImage()
    $got = if ($clip) { "$($clip.Width)x$($clip.Height)" } else { 'none' }
    if ($got -eq '200x100') { $ok++ }
    elseif (-not $clean -or (Locked)) { $interfered++; Log "INTERFERED iter=$n clipboard=$got pointer_moved_by_someone_else=$(-not $clean) locked=$(Locked)"; Key 0x1B }
    else { $failed++; Log "FAILED iter=$n clipboard=$got" }
    if ($clip) { $clip.Dispose() }
    $rest = $Every - ((Get-Date) - $t0).TotalSeconds
    if ($rest -gt 0) { Start-Sleep -Milliseconds ([int]($rest * 1000)) }
}
$app.Refresh()
Log ("SAMPLE end iter={0} ws_mb={1:N1} private_mb={2:N1} handles={3}" -f $n, ($app.WorkingSet64 / 1MB), ($app.PrivateMemorySize64 / 1MB), $app.HandleCount)
Log "SOAK minutes=$Minutes iterations=$n copies_ok=$ok copies_failed=$failed interfered=$interfered crashes=$crashes"
Log "VERDICT soak=$(if ($crashes -eq 0 -and $failed -eq 0) { 'PASS' } else { 'FAIL' })"
if (-not $app.HasExited) { $app.Kill() }
Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
$log | Set-Content -Encoding utf8 (Join-Path $Out 'soak.log')
