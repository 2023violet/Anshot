# Anshot capture diagnostics (Windows PowerShell 5.1 or PowerShell 7).
#
# What it does, on the machine where captures misbehave:
#  1. Starts Anshot with a separate, temporary settings and history folder (your own settings
#     and history are not read or changed) and with diagnostics on.
#  2. Runs automatic capture starts with the hotkey (default renderer, then the software
#     renderer), pressing Esc after each, and with the editor open.
#  3. Asks you to start a few captures from the tray icon yourself.
#  4. Writes text results to a new folder on your Desktop.
#
# What it records: timings, whether the screen went black (a brightness statistic of a
# downscaled sample, computed in memory), the number of displays, size, scaling, HDR state,
# colour depth, graphics adapter name and Windows build. It never saves screen pixels,
# window titles or file contents.
#
# Usage: right-click > Run with PowerShell, or:
#   powershell -ExecutionPolicy Bypass -File anshot-diagnose.ps1 [-Exe <Anshot.exe>] [-Rounds 30] [-Groups default,software]
# Groups (one variable each; the switches work only together with diagnostics):
#   default   as shipped                       software  SLINT_BACKEND=winit-software
#   nocloak   overlay shown before its frame    noscrim   overlay without the dimming
#   pad       overlay 1 px taller than the display
# Do not touch the mouse or keyboard while it runs, except when it asks you to.
param([string] $Exe = "$env:LOCALAPPDATA\Programs\Anshot\Anshot.exe", [int] $Rounds = 30, [string[]] $Groups = @('default', 'software'), [switch] $SkipTray, [string] $OutDir = [Environment]::GetFolderPath('Desktop'))
$ErrorActionPreference = 'Stop'
if (-not (Test-Path $Exe)) { Write-Host "Anshot.exe not found at $Exe. Pass -Exe <path>." -ForegroundColor Red; exit 1 }
if (Get-Process -Name Anshot -ErrorAction SilentlyContinue) {
    Write-Host 'Anshot is running. Right-click its tray icon and choose Exit (退出), then run this again.' -ForegroundColor Yellow; exit 1
}
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices; using System.Text; using System.Threading; using System.Collections.Generic; using System.Diagnostics;
public static class AD {
  [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr h);
  [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr h, IntPtr dc);
  [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleDC(IntPtr dc);
  [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleBitmap(IntPtr dc, int w, int h);
  [DllImport("gdi32.dll")] public static extern IntPtr SelectObject(IntPtr dc, IntPtr o);
  [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr o);
  [DllImport("gdi32.dll")] public static extern bool DeleteDC(IntPtr dc);
  [DllImport("gdi32.dll")] public static extern bool StretchBlt(IntPtr d, int x, int y, int w, int h, IntPtr s, int sx, int sy, int sw, int sh, int rop);
  [DllImport("gdi32.dll")] public static extern int SetStretchBltMode(IntPtr dc, int m);
  [DllImport("gdi32.dll")] public static extern int GetDIBits(IntPtr dc, IntPtr bmp, uint start, uint lines, byte[] bits, byte[] bi, uint usage);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  public static List<double[]> Samples = new List<double[]>();
  public static Stopwatch Clock = new Stopwatch();
  public static uint AppPid = 0;
  public static double FocusAt = -1;
  static IntPtr startFg;
  static volatile bool run; static Thread th;
  public static void Start(int sw, int sh) {
    Samples = new List<double[]>(); run = true; FocusAt = -1; startFg = GetForegroundWindow(); Clock.Restart();
    th = new Thread(() => {
      const int W = 96, H = 54;
      IntPtr scr = GetDC(IntPtr.Zero); IntPtr mem = CreateCompatibleDC(scr); IntPtr bmp = CreateCompatibleBitmap(scr, W, H);
      IntPtr old = SelectObject(mem, bmp); SetStretchBltMode(mem, 3);
      byte[] bi = new byte[40]; BitConverter.GetBytes(40).CopyTo(bi, 0); BitConverter.GetBytes(W).CopyTo(bi, 4); BitConverter.GetBytes(-H).CopyTo(bi, 8);
      BitConverter.GetBytes((short)1).CopyTo(bi, 12); BitConverter.GetBytes((short)32).CopyTo(bi, 14);
      byte[] px = new byte[W * H * 4];
      while (run) {
        StretchBlt(mem, 0, 0, W, H, scr, 0, 0, sw, sh, 0x00CC0020);
        SelectObject(mem, old); GetDIBits(mem, bmp, 0, H, px, bi, 0); SelectObject(mem, bmp);
        int black = 0; double sum = 0;
        for (int i = 0; i < px.Length; i += 4) { double l = 0.0722 * px[i] + 0.7152 * px[i + 1] + 0.2126 * px[i + 2]; sum += l; if (l < 10) black++; }
        if (FocusAt < 0) { IntPtr fg = GetForegroundWindow(); uint p; GetWindowThreadProcessId(fg, out p); if (p == AppPid && fg != startFg) FocusAt = Clock.Elapsed.TotalMilliseconds; }
        lock (Samples) Samples.Add(new double[] { Clock.Elapsed.TotalMilliseconds, sum / (W * H), black / (double)(W * H) });
        Thread.Sleep(2);
      }
      SelectObject(mem, old); DeleteObject(bmp); DeleteDC(mem); ReleaseDC(IntPtr.Zero, scr);
    });
    th.IsBackground = true; th.Start();
  }
  public static void Stop() { run = false; th.Join(); }
  public static void Key(byte vk, bool ctrl, bool shift) {
    if (ctrl) keybd_event(0x11, 0, 0, IntPtr.Zero); if (shift) keybd_event(0x10, 0, 0, IntPtr.Zero);
    keybd_event(vk, 0, 0, IntPtr.Zero); Thread.Sleep(40); keybd_event(vk, 0, 2, IntPtr.Zero);
    if (shift) keybd_event(0x10, 0, 2, IntPtr.Zero); if (ctrl) keybd_event(0x11, 0, 2, IntPtr.Zero);
  }
}
'@
[void][AD]::SetProcessDPIAware()
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$out = Join-Path $OutDir "anshot-diagnose-$stamp"
$work = Join-Path ([IO.Path]::GetTempPath()) "anshot-diagnose-$PID"
New-Item -ItemType Directory -Force $out, "$work\settings", "$work\history" | Out-Null
'hide_after_copy=false' | Set-Content "$work\settings\settings.txt"
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$results = New-Object System.Collections.Generic.List[string]
$script:app = $null

$GroupEnv = @{
    'default' = @{}; 'software' = @{ SLINT_BACKEND = 'winit-software' }
    'nocloak' = @{ ANSHOT_DIAG_NO_CLOAK = '1' }; 'noscrim' = @{ ANSHOT_DIAG_NO_SCRIM = '1' }; 'pad' = @{ ANSHOT_DIAG_OVERLAY_PAD = '1' }
}
$Groups = @($Groups | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
foreach ($g in $Groups) { if (-not $GroupEnv.ContainsKey($g)) { Write-Host "Unknown group '$g'. Use: $($GroupEnv.Keys -join ', ')" -ForegroundColor Red; exit 1 } }
function Start-Anshot([string] $group, [string] $file) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo $Exe
    $psi.UseShellExecute = $false
    $psi.EnvironmentVariables['ANSHOT_SETTINGS_DIR'] = "$work\settings"
    $psi.EnvironmentVariables['ANSHOT_HISTORY_DIR'] = "$work\history"
    $psi.EnvironmentVariables['ANSHOT_DIAGNOSE_DIR'] = $out
    foreach ($k in $GroupEnv[$group].Keys) { $psi.EnvironmentVariables[$k] = $GroupEnv[$group][$k] }
    $psi.Arguments = if ($file) { "`"$file`"" } else { '--background' }
    $p = [System.Diagnostics.Process]::Start($psi)
    $script:app = $p
    [AD]::AppPid = [uint32]$p.Id
    Start-Sleep -Seconds 4
    if ($p.HasExited) { throw 'Anshot closed right after starting (exit code ' + $p.ExitCode + '). Nothing more was sent.' }
}
function Stop-Anshot { if ($script:app -and -not $script:app.HasExited) { $script:app.Kill(); $script:app.WaitForExit(5000) | Out-Null }; Start-Sleep -Seconds 1 }
# Never type into other windows: after the first round the capture must have reached Anshot.
function Check-First([int] $focus) {
    $diag = Join-Path $out 'anshot-diag.txt'
    $captures = if (Test-Path $diag) { @(Get-Content $diag | Where-Object { $_ -like 'capture *' }).Count } else { 0 }
    # Only Anshot's own record counts: any window of the started process could take focus.
    if ($captures -le $script:seen) { throw 'The capture hotkey did not reach Anshot (another program may use Ctrl+Shift+A). Stopped; no more keys were sent.' }
    $script:seen = $captures
}
$script:seen = 0
function One-Round([string] $tag, [int] $n) {
    [void][AD]::SetCursorPos([int]($b.Width / 2), [int]($b.Height / 2))
    Start-Sleep -Milliseconds 500
    [AD]::Start($b.Width, $b.Height); Start-Sleep -Milliseconds 300
    $t0 = [AD]::Clock.Elapsed.TotalMilliseconds
    [AD]::Key(0x41, $true, $true)  # Ctrl+Shift+A, the default capture hotkey
    Start-Sleep -Milliseconds 2500
    [AD]::Key(0x1B, $false, $false)  # Esc
    Start-Sleep -Milliseconds 1500
    [AD]::Stop()
    $s = [AD]::Samples.ToArray()
    $base = ($s | Where-Object { $_[0] -lt $t0 } | ForEach-Object { $_[1] } | Measure-Object -Average).Average
    $black = @($s | Where-Object { $_[0] -ge $t0 -and $_[2] -gt 0.9 })
    $span = if ($black.Count) { [math]::Round($black[-1][0] - $black[0][0]) } else { 0 }
    $afterEsc = ($s | Where-Object { $_[0] -gt $t0 + 3500 } | ForEach-Object { $_[1] } | Measure-Object -Average).Average
    $focus = if ([AD]::FocusAt -ge 0) { [math]::Round([AD]::FocusAt - $t0) } else { -1 }
    $line = "round $tag #$n focus_ms=$focus black_samples=$($black.Count) black_span_ms=$span brightness_before=$([math]::Round($base)) brightness_after_esc=$([math]::Round($afterEsc))"
    $results.Add($line); Write-Host $line
    $focus
}

Write-Host "Results go to $out" -ForegroundColor Cyan
Write-Host 'Please do not touch the mouse or keyboard until asked.' -ForegroundColor Cyan
Start-Sleep -Seconds 3
try {
    foreach ($group in $Groups) {
        Start-Anshot $group ''
        for ($i = 1; $i -le $Rounds; $i++) { $f = One-Round "hotkey-$group" $i; if ($i -eq 1) { Check-First $f } }
        Stop-Anshot
    }
    # Editor open: a small generated picture, not a screenshot.
    $img = Join-Path $work 'pattern.png'
    $bmp = New-Object System.Drawing.Bitmap 640, 400
    $g = [System.Drawing.Graphics]::FromImage($bmp); $g.Clear([System.Drawing.Color]::FromArgb(230, 230, 220)); $g.Dispose()
    $bmp.Save($img, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    Start-Anshot 'default' $img
    for ($i = 1; $i -le [math]::Min($Rounds, 10); $i++) { $f = One-Round 'editor-default' $i; if ($i -eq 1) { Check-First $f } }
    Stop-Anshot
    # Tray: by hand.
    if ($SkipTray) { throw 'skip-tray' }
    Start-Anshot 'default' ''
    Write-Host ''
    Write-Host 'Now please start 5 captures from the Anshot tray icon yourself:' -ForegroundColor Yellow
    Write-Host '  click the icon (in the hidden-icons area if needed), wait until the screen settles, press Esc.' -ForegroundColor Yellow
    Write-Host '  If the screen stays black, wait 10 s, then press Esc; if that does nothing, right-click.' -ForegroundColor Yellow
    [void](Read-Host 'Press Enter here when you have done the 5 tray captures')
    Write-Host 'The diagnostics file now has one capture line per tray capture.'
    $results.Add("tray-manual answered=" + (Read-Host 'Did the screen go black or change colour? (y/n)'))
    $results.Add("tray-manual recovered_by=" + (Read-Host 'If so, what brought it back? (esc / rightclick / taskmanager / reboot / none)'))
    Stop-Anshot
} catch {
    if ($_.Exception.Message -ne 'skip-tray') { $results.Add('stopped: ' + $_.Exception.Message); Write-Host $_.Exception.Message -ForegroundColor Red }
} finally {
    $results | Set-Content -Encoding UTF8 (Join-Path $out 'rounds.txt')
    # Only the Anshot this script started, and only if it is still running.
    Stop-Anshot
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
Write-Host ''
Write-Host "Done. Please send the two text files in $out (anshot-diag.txt, rounds.txt)." -ForegroundColor Green
Write-Host 'They contain timings and display facts only; you can open them in Notepad first.' -ForegroundColor Green
