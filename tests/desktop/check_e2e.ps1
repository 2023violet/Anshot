# Privacy check on the real app with real input: finds invented data on a synthetic page, a
# unticked suggestion stays uncovered, and the export is read back pixel by pixel.
# Usage: pwsh -STA -File tests/desktop/check_e2e.ps1 -Exe <Anshot.exe> -Out <dir>
param(
    [Parameter(Mandatory)] [string] $Exe,
    [Parameter(Mandatory)] [string] $Out
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms, UIAutomationClient, UIAutomationTypes
Add-Type @'
using System; using System.Runtime.InteropServices; using System.Text;
public static class K {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, IntPtr e);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  public static IntPtr Editor(uint pid) {
    IntPtr best = IntPtr.Zero; long area = 0;
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); var sb = new StringBuilder(256); GetWindowText(h, sb, 256);
      RECT r; if (p == pid && IsWindowVisible(h) && sb.ToString().Contains("Anshot") && GetWindowRect(h, out r)) {
        long a = (long)(r.R - r.L) * (r.B - r.T); if (a > area && a > 300000) { area = a; best = h; } } return true; }, IntPtr.Zero);
    return best;
  }
}
'@
[void][K]::SetProcessDPIAware()
New-Item -ItemType Directory -Force $Out | Out-Null
$log = [System.Collections.Generic.List[string]]::new()
function Log($s) { $log.Add($s); $s }
function Verdict([string] $name, [bool] $ok) { Log "VERDICT $name=$(if ($ok) { 'PASS' } else { 'FAIL' })" }
function Key([byte] $vk, [switch] $Ctrl, [switch] $Shift) {
    if ($Ctrl) { [K]::keybd_event(0x11, 0, 0, [IntPtr]::Zero) }
    if ($Shift) { [K]::keybd_event(0x10, 0, 0, [IntPtr]::Zero) }
    [K]::keybd_event($vk, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 50; [K]::keybd_event($vk, 0, 2, [IntPtr]::Zero)
    if ($Shift) { [K]::keybd_event(0x10, 0, 2, [IntPtr]::Zero) }
    if ($Ctrl) { [K]::keybd_event(0x11, 0, 2, [IntPtr]::Zero) }
    Start-Sleep -Milliseconds 250
}
function Shot([IntPtr] $h, [string] $name) {
    $r = New-Object K+RECT; [void][K]::GetWindowRect($h, [ref]$r)
    $bmp = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T); $g = [System.Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc(); [void][K]::PrintWindow($h, $dc, 2); $g.ReleaseHdc($dc); $g.Dispose()
    $bmp.Save((Join-Path $Out $name)); $bmp.Dispose()
}
function Root([IntPtr] $h) { [System.Windows.Automation.AutomationElement]::FromHandle($h) }
function Names([IntPtr] $h) {
    @((Root $h).FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object { $_.Current.Name } | Where-Object { $_ })
}
function Invoke-Named([IntPtr] $h, [string] $name) {
    $cond = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $name
    $el = (Root $h).FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
    $pat = $null
    if ($el -and $el.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pat)) { $pat.Invoke(); Start-Sleep -Milliseconds 400; return $true }
    if ($el -and $el.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pat)) { $pat.Toggle(); Start-Sleep -Milliseconds 400; return $true }
    $false
}
function Wait-Name([IntPtr] $h, [string] $pattern, [int] $seconds) {
    $d = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $d) { $n = @(Names $h | Where-Object { $_ -match $pattern }); if ($n.Count) { return $n[0] }; Start-Sleep -Milliseconds 200 }
    ''
}
# Keys go to the foreground window: refuse to type anywhere but the editor.
function Focus-Editor {
    if (-not [K]::IsWindowVisible($h)) { throw 'the editor is hidden' }
    [void][K]::SetForegroundWindow($h); Start-Sleep -Milliseconds 300
    if ([K]::GetForegroundWindow() -ne $h) { throw 'the editor is not in front' }
}
function CopyTo([string] $name) {
    Focus-Editor
    [System.Windows.Forms.Clipboard]::Clear(); Key 0x43 -Ctrl; Start-Sleep -Milliseconds 1500
    $c = [System.Windows.Forms.Clipboard]::GetImage()
    if (-not $c) { return 'none' }
    $c.Save((Join-Path $Out $name), [System.Drawing.Imaging.ImageFormat]::Png); $s = "$($c.Width)x$($c.Height)"; $c.Dispose(); $s
}

Log "EXE sha256=$((Get-FileHash $Exe -Algorithm SHA256).Hash.ToLower()) path=$(Split-Path $Exe -Leaf)"
$tmp = Join-Path ([IO.Path]::GetTempPath()) "anshot-check-$PID"
$env:ANSHOT_SETTINGS_DIR = "$tmp\settings"; $env:ANSHOT_HISTORY_DIR = "$tmp\history"
New-Item -ItemType Directory -Force $env:ANSHOT_SETTINGS_DIR | Out-Null
# A copy would hide the editor; this script copies several times.
'hide_after_copy=false' | Set-Content (Join-Path $env:ANSHOT_SETTINGS_DIR 'settings.txt')
$doc = Join-Path $Out 'fake-doc.png'; $truth = Join-Path $Out 'fake-doc.json'
python (Join-Path $PSScriptRoot 'make_fake_doc.py') $doc $truth

$p = Start-Process $Exe -ArgumentList "`"$doc`"" -PassThru
$d = (Get-Date).AddSeconds(20); $h = [IntPtr]::Zero
while ((Get-Date) -lt $d -and $h -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 200; $h = [K]::Editor([uint32]$p.Id) }
Start-Sleep -Milliseconds 1500
[void][K]::SetForegroundWindow($h); Start-Sleep -Milliseconds 400

# ---- 1. the check finds the values and changes nothing by itself ----
$t0 = Get-Date
Focus-Editor
Key 0x4B -Ctrl -Shift
$head = Wait-Name $h '^发现 \d+ 处' 20
Log ("CHECK head=$head seconds={0:N1}" -f ((Get-Date) - $t0).TotalSeconds)
$names = Names $h
# A misread value is listed as possible ("疑似…"): the place is what counts here.
$want = '手机号$', '身份证号$', '银行卡号$', '邮箱$', 'IP 地址$', '密钥$'
$missing = @($want | Where-Object { $w = $_; -not @($names | Where-Object { $_ -match $w }).Count })
Log "CHECK missing=$($missing -join ',') rows=$(@($names | Where-Object { $_ -match ($want -join '|') }).Count)"
Verdict 'check_finds_all' ($missing.Count -eq 0)
Shot $h 'check-1-list.png'
Log "COPY size=$(CopyTo 'check-before.png')"
Log "SAME_BEFORE $(python (Join-Path $PSScriptRoot 'check_png.py') same $doc (Join-Path $Out 'check-before.png'))"

# ---- 2. untick one, apply, export: pixels read back ----
$ip = @($names | Where-Object { $_ -match 'IP 地址$' })[0]
Log "UNTICK ip=$(Invoke-Named $h $ip)"
$apply = Wait-Name $h '^遮挡所选' 3
Log "APPLY $apply=$(Invoke-Named $h $apply)"
Start-Sleep -Milliseconds 800
Shot $h 'check-2-applied.png'
Log "COPY size=$(CopyTo 'check-after.png')"
$read = python (Join-Path $PSScriptRoot 'check_review.py') $doc (Join-Path $Out 'check-after.png') $truth 'phone,id,card,email,key'
Log $read
Verdict 'check_covers_ticked_only' ("$read" -match 'PASS$')

# ---- 3. one undo removes the whole batch ----
Focus-Editor
Key 0x5A -Ctrl; Start-Sleep -Milliseconds 400
Log "COPY size=$(CopyTo 'check-undone.png')"
$same = python (Join-Path $PSScriptRoot 'check_png.py') same $doc (Join-Path $Out 'check-undone.png')
Log "SAME_UNDONE $same"
Verdict 'check_undo_is_one_step' ("$same" -match 'diff=0$')

if (-not $p.HasExited) { $p.Kill() }
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
$log | Set-Content -Encoding utf8 (Join-Path $Out 'check.log')
