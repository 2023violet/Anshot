# Capture-start probe: timeline of what the screen shows after the capture hotkey.
# Samples the composed desktop (GDI, 96x54 downscale) every few ms; keeps only statistics.
# Usage: pwsh -STA -File probe.ps1 -Exe <Anshot.exe> -Mode tray|editor [-Env "K=V;K2=V2"] [-Trials 5] [-Image <png>] [-Tag name]
param([string] $Exe, [string] $Mode = 'tray', [string] $Env = '', [int] $Trials = 5, [string] $Image = '', [string] $Tag = 'run', [switch] $RightClick)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices; using System.Text; using System.Threading; using System.Collections.Generic; using System.Diagnostics;
public static class P {
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
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("shell32.dll")] public static extern int SHQueryUserNotificationState(out int state);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  public struct Sample { public double T; public double Mean; public double Black; }
  public static List<Sample> Samples = new List<Sample>();
  static volatile bool run;
  static Thread th;
  public static Stopwatch Clock = new Stopwatch();
  public static volatile int DisplayChanges = 0;
  public static double FgAt = -1;
  public static uint AppPid = 0;
  public static void Start(int sw, int sh) {
    Samples = new List<Sample>(); run = true; Clock.Restart(); FgAt = -1;
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
        double sum = 0; int black = 0;
        for (int i = 0; i < px.Length; i += 4) { double l = 0.0722 * px[i] + 0.7152 * px[i + 1] + 0.2126 * px[i + 2]; sum += l; if (l < 10) black++; }
        if (FgAt < 0) { IntPtr fh = GetForegroundWindow(); uint fp; GetWindowThreadProcessId(fh, out fp); var tb = new StringBuilder(64); GetWindowText(fh, tb, 64); if (fp == AppPid && tb.ToString().Contains("截图")) FgAt = Clock.Elapsed.TotalMilliseconds; }
        lock (Samples) Samples.Add(new Sample { T = Clock.Elapsed.TotalMilliseconds, Mean = sum / (W * H), Black = black / (double)(W * H) });
        Thread.Sleep(2);
      }
      SelectObject(mem, old); DeleteObject(bmp); DeleteDC(mem); ReleaseDC(IntPtr.Zero, scr);
    });
    th.IsBackground = true; th.Start();
  }
  public static void Stop() { run = false; th.Join(); }
  public static string Fg() { IntPtr h = GetForegroundWindow(); uint pid; GetWindowThreadProcessId(h, out pid); var sb = new StringBuilder(256); GetWindowText(h, sb, 256); return pid + "|" + sb.ToString(); }
  public static void Key(byte vk, bool ctrl, bool shift) {
    if (ctrl) keybd_event(0x11, 0, 0, IntPtr.Zero); if (shift) keybd_event(0x10, 0, 0, IntPtr.Zero);
    keybd_event(vk, 0, 0, IntPtr.Zero); Thread.Sleep(40); keybd_event(vk, 0, 2, IntPtr.Zero);
    if (shift) keybd_event(0x10, 0, 2, IntPtr.Zero); if (ctrl) keybd_event(0x11, 0, 2, IntPtr.Zero);
  }
}
'@
[void][P]::SetProcessDPIAware()
$sw = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Width; $sh = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Height

# A synthetic grey backdrop (not top-most) so every trial starts from the same picture.
$form = New-Object System.Windows.Forms.Form
$form.FormBorderStyle = 'None'; $form.StartPosition = 'Manual'; $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $form.Bounds = New-Object System.Drawing.Rectangle 0, 0, $b.Width, ($b.Height - 1)   # not full-screen itself
$form.BackColor = [System.Drawing.Color]::FromArgb(200, 200, 200); $form.ShowInTaskbar = $false; $form.Text = ''   # untitled: never snapped, so the overlay dims it
$form.Show(); [System.Windows.Forms.Application]::DoEvents(); [Microsoft.Win32.SystemEvents]::add_DisplaySettingsChanged({ [P]::DisplayChanges++ }); [Microsoft.Win32.SystemEvents]::add_DisplaySettingsChanging({ [P]::DisplayChanges++ })

$tmp = Join-Path ([IO.Path]::GetTempPath()) "anshot-probe-$PID"
New-Item -ItemType Directory -Force "$tmp\settings" | Out-Null
'hide_after_copy=false' | Set-Content "$tmp\settings\settings.txt"
$psi = New-Object System.Diagnostics.ProcessStartInfo $Exe
$psi.UseShellExecute = $false
$psi.Environment['ANSHOT_SETTINGS_DIR'] = "$tmp\settings"; $psi.Environment['ANSHOT_HISTORY_DIR'] = "$tmp\history"
foreach ($kv in ($Env -split ';' | Where-Object { $_ })) { $k, $v = $kv -split '=', 2; $psi.Environment[$k] = $v }
if ($Mode -eq 'tray') { $psi.Arguments = '--background' } else { $psi.Arguments = "`"$Image`"" }
$app = [System.Diagnostics.Process]::Start($psi)
[P]::AppPid = [uint32]$app.Id; Start-Sleep -Seconds 4
if ($Mode -eq 'tray') { $form.Activate(); [System.Windows.Forms.Application]::DoEvents() }

$rows = @()
for ($t = 1; $t -le $Trials; $t++) {
    if ($Mode -eq 'tray') { $form.Activate() }
    [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 600
    [void][P]::SetCursorPos([int]($sw/2), [int]($sh/2)); [P]::Start($sw, $sh); Start-Sleep -Milliseconds 300
    $t0 = [P]::Clock.Elapsed.TotalMilliseconds
    $dc0 = [P]::DisplayChanges
    [P]::Key(0x41, $true, $true)  # Ctrl+Shift+A
    $until = (Get-Date).AddMilliseconds(2500); while ((Get-Date) -lt $until) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 10 }
    $fg = [P]::Fg(); $q = 0; [void][P]::SHQueryUserNotificationState([ref]$q)
    if ($RightClick) { [void][P]::SetCursorPos(200, 200); [P]::mouse_event(0x8, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 40; [P]::mouse_event(0x10, 0, 0, 0, [IntPtr]::Zero) }
    else { [P]::Key(0x1B, $false, $false) }
    $until = (Get-Date).AddMilliseconds(700); while ((Get-Date) -lt $until) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 10 }
    [P]::Stop()
    $s = [P]::Samples.ToArray()
    $base = ($s | Where-Object { $_.T -lt $t0 } | Measure-Object Mean -Average).Average
    $after = $s | Where-Object { $_.T -ge $t0 -and $_.T -lt $t0 + 2500 }
    $first = $after | Where-Object { [math]::Abs($_.Mean - $base) -gt 8 } | Select-Object -First 1
    $black = @($after | Where-Object { $_.Black -gt 0.9 })
    $blackMs = if ($black.Count) { ($black[-1].T - $black[0].T) } else { 0 }
    $settled = ($after | Where-Object { $_.T -gt $t0 + 2000 } | Measure-Object Mean -Average).Average
    $end = ($s | Where-Object { $_.T -gt $t0 + 2500 + 500 } | Measure-Object Mean -Average).Average
    $fgPid, $fgTitle = $fg -split '\|', 2
    $rows += [pscustomobject]@{
        tag = $Tag; trial = $t; base = [math]::Round($base); first_change_ms = if ($first) { [math]::Round($first.T - $t0) } else { -1 }
        black_frames = $black.Count; black_span_ms = [math]::Round($blackMs); settled = [math]::Round($settled)
        overlay_focus_ms = if ([P]::FgAt -ge 0) { [math]::Round([P]::FgAt - $t0) } else { -1 }; display_changes = [P]::DisplayChanges - $dc0; fg_is_overlay = ($fgPid -eq "$($app.Id)" -and $fgTitle -match '截图'); notif_state = $q; restored = [math]::Round($end); samples = $s.Count
    }
}
if (-not $app.HasExited) { $app.Kill() }
$form.Close()
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
$rows | ForEach-Object { "$($_.tag) t$($_.trial) focus=$($_.overlay_focus_ms) black=$($_.black_frames) notif=$($_.notif_state) restored=$($_.restored) base=$($_.base)" }
