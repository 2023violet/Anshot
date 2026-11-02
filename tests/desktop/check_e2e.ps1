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
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(ref POINT p);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, IntPtr e);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  public static System.Collections.Generic.List<IntPtr> Small(uint pid) {
    var found = new System.Collections.Generic.List<IntPtr>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); RECT r;
      if (p == pid && IsWindowVisible(h) && GetWindowRect(h, out r) && (long)(r.R - r.L) * (r.B - r.T) < 300000) found.Add(h); return true; }, IntPtr.Zero);
    return found;
  }
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

# ---- 3b. cover one name by hand, then every copy of it ----
$rows = Get-Content $truth -Raw -Encoding utf8 | ConvertFrom-Json
$b = @($rows | Where-Object { $_.kind -eq 'name' })[0].box
function Draw-Over($b) {
$o = New-Object K+POINT; [void][K]::ClientToScreen($h, [ref]$o)
$cr = New-Object K+RECT; [void][K]::GetClientRect($h, [ref]$cr)
$ix = [math]::Round(($cr.R - 900) / 2) + $o.X; $iy = 92 + [math]::Round(($cr.B - 122 - 560) / 2) + $o.Y
Focus-Editor
Log "DRAW box=$($b -join ',') origin=$ix,$iy client=$($cr.R)x$($cr.B)"
[void][K]::SetCursorPos($ix + $b[0] - 3, $iy + $b[1] - 3); Start-Sleep -Milliseconds 200
[K]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80
foreach ($i in 1..10) { [void][K]::SetCursorPos($ix + $b[0] - 3 + ($b[2] - $b[0] + 6) * $i / 10, $iy + $b[1] - 3 + ($b[3] - $b[1] + 6) * $i / 10); Start-Sleep -Milliseconds 25 }
Start-Sleep -Milliseconds 100
$c = New-Object K+POINT; [void][K]::GetCursorPos([ref]$c)
Log "DRAW pointer_at_release=$($c.X),$($c.Y) expected=$($ix + $b[2] + 3),$($iy + $b[3] + 3)"
[K]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 500
}
Draw-Over $b
Log "SAME button=$(Invoke-Named $h '遮住相同内容')"
$apply = Wait-Name $h '^遮挡所选' 15
Log "SAME list=$apply rows=$(@(Names $h | Where-Object { $_ -eq '相同内容' }).Count)"
Shot $h 'check-2b-same.png'
Log "SAME apply=$(Invoke-Named $h $apply)"
Log "COPY size=$(CopyTo 'check-same.png')"
$read = python (Join-Path $PSScriptRoot 'check_review.py') $doc (Join-Path $Out 'check-same.png') $truth 'name'
Log $read
Verdict 'same_text_covers_every_copy' ("$read" -match 'PASS$')
Focus-Editor
Key 0x5A -Ctrl; Key 0x5A -Ctrl; Start-Sleep -Milliseconds 400

# ---- 3c. replace instead of cover: stand-ins read back, the originals do not ----
Focus-Editor
Key 0x4B -Ctrl -Shift
$apply = Wait-Name $h '^遮挡所选' 20
Log "REPLACE mode=$(Invoke-Named $h '替换成占位文字')"
Start-Sleep -Milliseconds 300
Log "REPLACE apply=$(Invoke-Named $h (Wait-Name $h '^遮挡所选' 3))"
Start-Sleep -Milliseconds 600
Shot $h 'check-2c-replaced.png'
Log "COPY size=$(CopyTo 'check-replaced.png')"
$read = python (Join-Path $PSScriptRoot 'check_review.py') $doc (Join-Path $Out 'check-replaced.png') $truth 'phone,id,card,email,ip,key' replaced
Log $read
Focus-Editor
Key 0x4F -Ctrl -Shift
$ocr = [IntPtr]::Zero; $d = (Get-Date).AddSeconds(15); $text = ''
while ((Get-Date) -lt $d -and $text -eq '') {
    Start-Sleep -Milliseconds 300
    foreach ($w in [K]::Small([uint32]$p.Id)) {
        $edit = (Root $w).FindFirst([System.Windows.Automation.TreeScope]::Descendants, ([System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Edit)))
        $vp = $null
        if ($edit -and $edit.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$vp) -and $vp.Current.Value) { $text = $vp.Current.Value; $ocr = $w }
    }
}
$flat = ($text -replace '\s', '')
$leaks = @('13800138000', '11010519491231002X', '6222020000000000000', 'wangxiaoming', 'ghp_2NiP' | Where-Object { $flat.Contains($_) })
Log "REPLACE ocr_chars=$($flat.Length) stand_in=$($flat.Contains('example.com')) leaks=$($leaks -join ',')"
Verdict 'replace_shows_stand_ins_only' (("$read" -match 'PASS$') -and $flat.Contains('example.com') -and $leaks.Count -eq 0)
if ($ocr -ne [IntPtr]::Zero) { [void](Invoke-Named $ocr '关闭') }
Focus-Editor
Key 0x5A -Ctrl; Start-Sleep -Milliseconds 400

# ---- 4. the check after a copy: the warning, its button, and covering plus copying again ----
Focus-Editor
Log "COPY size=$(CopyTo 'check-auto-before.png')"
$warn = Wait-Name $h '^已复制 · 发现' 20
Log "AUTO warning=$warn"
Verdict 'auto_check_warns' ($warn -ne '')
Log "AUTO review=$(Invoke-Named $h '查看')"
$apply = Wait-Name $h '^遮挡并重新复制' 5
Shot $h 'check-3-auto.png'
[System.Windows.Forms.Clipboard]::Clear()
Log "AUTO apply $apply=$(Invoke-Named $h $apply)"
$c = $null; $d = (Get-Date).AddSeconds(5)
while (-not $c -and (Get-Date) -lt $d) { Start-Sleep -Milliseconds 200; $c = [System.Windows.Forms.Clipboard]::GetImage() }
if ($c) { $c.Save((Join-Path $Out 'check-auto-after.png'), [System.Drawing.Imaging.ImageFormat]::Png); $c.Dispose() }
# The IP stays: it was unticked in step 2, so the check does not raise it again.
$read = python (Join-Path $PSScriptRoot 'check_review.py') $doc (Join-Path $Out 'check-auto-after.png') $truth 'phone,id,card,email,key'
Log $read
Verdict 'auto_check_recopies_covered' ("$read" -match 'PASS$')
$done = Wait-Name $h '^已核对 \d+ 处遮挡' 20
Log "AUDIT done=$done"
Verdict 'audit_vouches_for_covers' ($done -match '^已核对 5 处')
Shot $h 'check-3b-audit.png'
Log "AUTO quiet_after=$(-not (Wait-Name $h '^已复制 · 发现' 1))"
if (-not $p.HasExited) { $p.Kill() }
Start-Sleep -Milliseconds 800

# ---- 4b. a name covered in one place but readable in another is reported ----
$p = Start-Process $Exe -ArgumentList "`"$doc`"" -PassThru
$d = (Get-Date).AddSeconds(20); $h = [IntPtr]::Zero
while ((Get-Date) -lt $d -and $h -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 200; $h = [K]::Editor([uint32]$p.Id) }
Start-Sleep -Milliseconds 1500
Draw-Over $b
Log "COPY size=$(CopyTo 'check-leak.png')"
$warn = Wait-Name $h '^已复制 · 发现' 20
Log "LEAK warning=$warn review=$(Invoke-Named $h '查看')"
Start-Sleep -Milliseconds 500
$same = @(Names $h | Where-Object { $_ -eq '相同内容' }).Count
Log "LEAK same_rows=$same"
Verdict 'audit_reports_uncovered_copy' ($same -eq 1)
Shot $h 'check-3c-leak.png'

if (-not $p.HasExited) { $p.Kill() }
Start-Sleep -Milliseconds 800

# ---- 5. default settings, tray app: quick copy from a selection, so no editor is on screen; the warning comes as a system toast and its button opens the list ----
Remove-Item (Join-Path $env:ANSHOT_SETTINGS_DIR 'settings.txt') -ErrorAction SilentlyContinue
$fixture = Start-Process pwsh -ArgumentList @('-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'synthetic_window.ps1'), '-Image', "`"$doc`"", '-X', 200, '-Y', 150) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3
$p = Start-Process $Exe -ArgumentList '--background' -PassThru
Start-Sleep -Seconds 3
[System.Windows.Forms.Clipboard]::Clear()
Key 0x41 -Ctrl -Shift; Start-Sleep -Milliseconds 900
[void][K]::SetCursorPos(220, 170); Start-Sleep -Milliseconds 200
[K]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80
foreach ($i in 1..12) { [void][K]::SetCursorPos(220 + 56 * $i, 170 + 28 * $i); Start-Sleep -Milliseconds 25 }
Start-Sleep -Milliseconds 100; [K]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 400
Key 0x43 -Ctrl
$toast = [IntPtr]::Zero; $d = (Get-Date).AddSeconds(20)
while ($toast -eq [IntPtr]::Zero -and (Get-Date) -lt $d) {
    Start-Sleep -Milliseconds 200
    foreach ($w in [K]::Small([uint32]$p.Id)) { if (@(Names $w | Where-Object { $_ -match '发现 \d+ 处' }).Count) { $toast = $w } }
}
Log "QUICK toast=$($toast -ne [IntPtr]::Zero) editor_on_screen=$([K]::Editor([uint32]$p.Id) -ne [IntPtr]::Zero)"
$opened = ($toast -ne [IntPtr]::Zero) -and (Invoke-Named $toast '查看')
Start-Sleep -Milliseconds 1000
$h = [K]::Editor([uint32]$p.Id)
$list = if ($h -ne [IntPtr]::Zero) { Wait-Name $h '^遮挡并重新复制' 5 } else { '' }
Log "QUICK opened=$opened list=$list"
Verdict 'quick_copy_toast_opens_list' ($list -ne '')
if ($h -ne [IntPtr]::Zero) { Shot $h 'check-4-quick.png' }
if (-not $p.HasExited) { $p.Kill() }
Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
$log | Set-Content -Encoding utf8 (Join-Path $Out 'check.log')
