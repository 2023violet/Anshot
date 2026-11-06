# Step image on the real app with real input (ADR 0.6-STEPS): an edited document, a capture
# toolbar selection and a burst capture become steps; the joined image keeps each step pixel
# for pixel in the chosen order; nothing of the steps is written to disk; 20 steps at most.
# Moves the mouse and types: leave the PC alone while it runs (about 2 minutes).
# Usage: pwsh -STA -File tests/desktop/steps_e2e.ps1 -Exe <Anshot.exe> -Out <dir>
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

function Set-Text([IntPtr] $h, [string] $name, [string] $text) {
    $cond = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $name
    $el = (Root $h).FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
    $pat = $null
    if ($el -and $el.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pat)) { $pat.SetValue($text); Start-Sleep -Milliseconds 600; return $true }
    $false
}
function Frame([string] $a, [string] $b, [string] $ratio, [string] $inner) { python (Join-Path $PSScriptRoot 'check_frame.py') inner (Join-Path $Out $a) (Join-Path $Out $b) $ratio (Join-Path $Out $inner) }
function Start-App([string] $file) {
    $script:p = Start-Process $Exe -ArgumentList "`"$file`"" -PassThru
    $d = (Get-Date).AddSeconds(20); $script:h = [IntPtr]::Zero
    while ((Get-Date) -lt $d -and $script:h -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 200; $script:h = [K]::Editor([uint32]$script:p.Id) }
    Start-Sleep -Milliseconds 1500
    [void][K]::SetForegroundWindow($script:h); Start-Sleep -Milliseconds 400
}

function Title([IntPtr] $h) { $sb = New-Object System.Text.StringBuilder 256; [void][K]::GetWindowText($h, $sb, 256); $sb.ToString() }
function Py { python (Join-Path $PSScriptRoot 'check_steps.py') @args }
# A selection on the frozen screen from (x0, y0) to (x1, y1), screen pixels.
function Select-Area([int] $x0, [int] $y0, [int] $x1, [int] $y1) {
    [void][K]::SetCursorPos($x0, $y0); Start-Sleep -Milliseconds 150
    [K]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero)
    foreach ($i in 1..10) { [void][K]::SetCursorPos($x0 + ($x1 - $x0) * $i / 10, $y0 + ($y1 - $y0) * $i / 10); Start-Sleep -Milliseconds 25 }
    [K]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 400
}
function Capture-Hotkey { [void][K]::SetCursorPos(5, 5); Start-Sleep -Milliseconds 200; Key 0x41 -Ctrl -Shift; Start-Sleep -Milliseconds 1500 }

Log "EXE sha256=$((Get-FileHash $Exe -Algorithm SHA256).Hash.ToLower()) path=$(Split-Path $Exe -Leaf)"
$tmp = Join-Path ([IO.Path]::GetTempPath()) "anshot-steps-$PID"
$env:ANSHOT_SETTINGS_DIR = "$tmp\settings"; $env:ANSHOT_HISTORY_DIR = "$tmp\history"
New-Item -ItemType Directory -Force $env:ANSHOT_SETTINGS_DIR | Out-Null
$settingsFile = Join-Path $env:ANSHOT_SETTINGS_DIR 'settings.txt'
'hide_after_copy=false' | Set-Content $settingsFile
$doc = Join-Path $Out 'fake-doc.png'; $truth = Join-Path $Out 'fake-doc.json'
python (Join-Path $PSScriptRoot 'make_fake_doc.py') $doc $truth
$repo = Split-Path (Split-Path $PSScriptRoot)
$shot = Join-Path $repo 'spike\assets\shot.png'
$fx = 1100; $fy = 120
$fixture = Start-Process powershell -ArgumentList @('-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'synthetic_window.ps1'), '-Image', "`"$shot`"", '-X', $fx, '-Y', $fy) -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3
$caption = 'SECRET-CAPTION-7731'
try {
Start-App $doc

# ---- 1. an edited document becomes a step: its covers are in the step's pixels ----
Focus-Editor
Key 0x4B -Ctrl -Shift
[void](Wait-Name $h '^发现 \d+ 处' 20)
$apply = Wait-Name $h '^遮挡所选' 3
Log "COVER $apply=$(Invoke-Named $h $apply)"
Start-Sleep -Milliseconds 800
Log "COPY size=$(CopyTo 'step-edited.png')"
$d = (Get-Date).AddSeconds(15); while ((Get-Date) -lt $d -and (Wait-Name $h '^已核对' 1)) { Start-Sleep -Milliseconds 300 }
$plainDiff = python (Join-Path $PSScriptRoot 'check_png.py') same $doc (Join-Path $Out 'step-edited.png')
Focus-Editor
Key 0x4A -Ctrl
$added1 = Wait-Name $h '^已加入第 1 步' 8
Log "STEP1 notice=$added1 covered=$($plainDiff -notmatch 'SAME diff=0')"
Verdict 'editor_adds_a_step' ($added1 -ne '' -and $plainDiff -notmatch 'SAME diff=0')

# ---- 2. the capture toolbar adds the selection, unedited, without opening it ----
$titleBefore = Title $h
Capture-Hotkey
Select-Area ($fx + 40) ($fy + 40) ($fx + 340) ($fy + 240)
Key 0x4A -Ctrl
Start-Sleep -Milliseconds 1500
$added2 = Wait-Name $h '^已加入第 2 步' 8
Log "STEP2 notice=$added2 title_kept=$((Title $h) -eq $titleBefore)"
Verdict 'overlay_adds_a_raw_step' ($added2 -ne '' -and (Title $h) -eq $titleBefore)

# ---- 3. burst: Enter adds the selection as a step ----
Focus-Editor
Key 0x4A -Ctrl -Shift
[void](Wait-Name $h '^步骤图$' 5)
$on = @((Root $h).FindAll([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), '开')))
$pat = $null; if ($on.Count -and $on[0].TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pat)) { $pat.Invoke() }
Start-Sleep -Milliseconds 600
Log "BURST on=$(Wait-Name $h '^连拍已打开' 3)"
Capture-Hotkey
Select-Area ($fx + 300) ($fy + 150) ($fx + 580) ($fy + 400)
Key 0x0D
Start-Sleep -Milliseconds 1500
$added3 = Wait-Name $h '^已加入第 3 步' 8
Log "STEP3 notice=$added3 title_kept=$((Title $h) -eq $titleBefore)"
Verdict 'burst_enter_adds_a_step' ($added3 -ne '' -and (Title $h) -eq $titleBefore)

# ---- 4. reorder, caption, join ----
Focus-Editor
if (-not (Wait-Name $h '^步骤图$' 1)) { Key 0x4A -Ctrl -Shift; [void](Wait-Name $h '^步骤图$' 5) }
$raw = @(Names $h | Where-Object { $_ -eq '未编辑' }).Count
Log "PANEL rows=$(@(Names $h | Where-Object { $_ -match '^第 \d+ 步$' }).Count) unedited=$raw"
Log "MOVE $(Invoke-Named $h '下移 1')"
Log "BACKDROP $(Invoke-Named $h '深')"
Log "CAPTION $(Set-Text $h '第 3 步的说明' $caption)"
Start-Sleep -Milliseconds 1200
Shot $h 'steps-1-panel.png'
Log "JOIN $(Invoke-Named $h '拼合并编辑')"
$d = (Get-Date).AddSeconds(15); while ((Get-Date) -lt $d -and (Title $h) -notmatch '步骤图 3 步') { Start-Sleep -Milliseconds 300 }
Log "TITLE $(Title $h)"
Start-Sleep -Milliseconds 800
Shot $h 'steps-2-joined.png'
Log "COPY size=$(CopyTo 'steps-joined.png')"
$joined = Join-Path $Out 'steps-joined.png'
$p1 = Py find $joined (Join-Path $Out 'step-edited.png')
$p2 = Py find $joined $shot 60 60 240 120
$p3 = Py find $joined $shot 320 170 220 200
$op = Py opaque $joined
Log "FIND edited=$p1 toolbar=$p2 burst=$p3 $op"
$y = @($p1, $p2, $p3 | ForEach-Object { if ($_ -match 'FOUND \d+ (\d+)') { [int]$Matches[1] } else { -1 } })
# Step 1 moved down: the toolbar capture is first, then the edited document, then the burst capture.
$order = ($y -notcontains -1) -and $y[1] -lt $y[0] -and $y[0] -lt $y[2]
Verdict 'joined_keeps_each_step_and_the_order' ($order -and $op -match 'yes' -and $raw -eq 2)
$auto = Wait-Name $h '^(已核对|发现|没有发现)' 20
Log "AUTO $auto"

# ---- 5. at most 20 steps ----
Focus-Editor
$d = (Get-Date).AddSeconds(5); while ((Get-Date) -lt $d -and (Wait-Name $h '^(已核对|发现)' 1)) { Start-Sleep -Milliseconds 300 }
foreach ($i in 4..20) { Focus-Editor; Key 0x4A -Ctrl; Start-Sleep -Milliseconds 700 }
[void](Wait-Name $h '^已加入第 20 步' 10)
Focus-Editor
Key 0x4A -Ctrl
$full = Wait-Name $h '^最多 20 步' 8
Key 0x4A -Ctrl -Shift
Start-Sleep -Milliseconds 800
$count = Wait-Name $h '^20 / 20 步' 3
Log "LIMIT notice=$full count=$count"
Verdict 'twenty_steps_at_most' ($full -ne '' -and $count -ne '')
if (-not $p.HasExited) { $p.Kill() }
Start-Sleep -Milliseconds 800

# ---- 6. nothing of the steps reaches the disk ----
$hist = @(Get-ChildItem $env:ANSHOT_HISTORY_DIR -File -ErrorAction SilentlyContinue).Count
$bytes = Py nobytes $tmp $caption
$style = ((Get-Content $settingsFile -Raw) -split "`n" | Where-Object { $_ -match '^steps_style=' }) -join ''
Log "DISK history=$hist $bytes style=$style"
# Two deliveries (the edited document, the joined image); the steps themselves leave no file.
Verdict 'steps_stay_in_memory' ($hist -eq 2 -and $bytes -match 'ABSENT' -and $style -match '^steps_style=0,1,1,1,2,0')

# ---- 7. a new run starts with no steps ----
Start-App $doc
Focus-Editor
Key 0x4A -Ctrl -Shift
$empty = Wait-Name $h '^还没有步骤' 5
Log "RESTART empty=$empty"
Verdict 'restart_has_no_steps' ($empty -ne '')
} finally {
    if ($p -and -not $p.HasExited) { $p.Kill() }
    Stop-Process -Id $fixture.Id -Force -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    $log | Set-Content -Encoding utf8 (Join-Path $Out 'steps.log')
}
