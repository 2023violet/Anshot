# Beautify on the real app with real input: framed copies keep the export pixel for pixel,
# the checks read only the screenshot, and the chrome text is never saved.
# Usage: pwsh -STA -File tests/desktop/beautify_e2e.ps1 -Exe <Anshot.exe> -Out <dir>
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

Log "EXE sha256=$((Get-FileHash $Exe -Algorithm SHA256).Hash.ToLower()) path=$(Split-Path $Exe -Leaf)"
$tmp = Join-Path ([IO.Path]::GetTempPath()) "anshot-frame-$PID"
$env:ANSHOT_SETTINGS_DIR = "$tmp\settings"; $env:ANSHOT_HISTORY_DIR = "$tmp\history"
New-Item -ItemType Directory -Force $env:ANSHOT_SETTINGS_DIR | Out-Null
$settingsFile = Join-Path $env:ANSHOT_SETTINGS_DIR 'settings.txt'
# A copy would hide the editor; this script copies several times.
'hide_after_copy=false' | Set-Content $settingsFile
$doc = Join-Path $Out 'fake-doc.png'; $truth = Join-Path $Out 'fake-doc.json'
python (Join-Path $PSScriptRoot 'make_fake_doc.py') $doc $truth
Start-App $doc

# ---- 1. without beautify the export is the picture as it was ----
Log "COPY size=$(CopyTo 'frame-plain.png')"
$same = python (Join-Path $PSScriptRoot 'check_png.py') same $doc (Join-Path $Out 'frame-plain.png')
Log "PLAIN $same"
Verdict 'frame_off_is_plain' ($same -match 'SAME diff=0')

# ---- 2. cover everything the check finds, then frame: the covers are kept inside ----
Focus-Editor
Key 0x4B -Ctrl -Shift
[void](Wait-Name $h '^发现 \d+ 处' 20)
$apply = Wait-Name $h '^遮挡所选' 3
Log "COVER $apply=$(Invoke-Named $h $apply)"
Start-Sleep -Milliseconds 800
Log "COPY size=$(CopyTo 'frame-covered.png')"
# The check after that copy runs on a worker: wait for its notice, then for it to go.
[void](Wait-Name $h '^已核对 \d+ 处遮挡' 20)
$d = (Get-Date).AddSeconds(15); while ((Get-Date) -lt $d -and (Wait-Name $h '^已核对' 1)) { Start-Sleep -Milliseconds 300 }
Focus-Editor
Key 0x42 -Ctrl -Shift
$panel = Wait-Name $h '^比例' 5
Log "PANEL shown=$($panel -ne '')"
Log "ON $(Invoke-Named $h '开')"
Log "RATIO $(Invoke-Named $h '16:9')"
Start-Sleep -Milliseconds 800
Shot $h 'beautify-1-panel.png'
Log "COPY size=$(CopyTo 'frame-16x9.png')"
$f = Frame 'frame-covered.png' 'frame-16x9.png' '16:9' 'frame-inner.png'
Log $f
$read = python (Join-Path $PSScriptRoot 'check_review.py') $doc (Join-Path $Out 'frame-inner.png') $truth 'phone,id,card,email,ip,key'
Log $read
Verdict 'framed_copy_keeps_the_export' (($f -match 'PASS') -and ($read -match 'PASS'))
# The check after that copy reads only the screenshot: every cover is vouched for, nothing new.
$audit = Wait-Name $h '^已核对 \d+ 处遮挡' 20
$found = Wait-Name $h '发现 \d+ 处' 1
Log "AUTO audit=$audit found=$found"
Verdict 'framed_check_reads_the_screenshot' ($audit -ne '' -and $found -eq '')

# ---- 3. browser chrome with typed text: drawn, never checked, never saved ----
$secret = 'tel 13912345678 example.com/frame-e2e'
Log "CHROME $(Invoke-Named $h '浏览器')"
Log "TEXT $(Set-Text $h '地址栏文字' $secret)"
Key 0x0D # Enter hands the keyboard back to the canvas, so Ctrl+C copies the image
Start-Sleep -Milliseconds 800
Shot $h 'beautify-2-browser.png'
$d = (Get-Date).AddSeconds(15); while ((Get-Date) -lt $d -and (Wait-Name $h '^已核对' 1)) { Start-Sleep -Milliseconds 300 }
Log "COPY size=$(CopyTo 'frame-browser.png')"
$f = Frame 'frame-covered.png' 'frame-browser.png' '16:9' 'frame-inner-browser.png'
Log $f
$wide = python (Join-Path $PSScriptRoot 'check_png.py') same (Join-Path $Out 'frame-16x9.png') (Join-Path $Out 'frame-browser.png')
$audit = Wait-Name $h '^已核对 \d+ 处遮挡' 20
$found = Wait-Name $h '发现 \d+ 处' 1
Log "AUTO audit=$audit found=$found chrome_differs=$($wide -notmatch 'SAME diff=0')"
Verdict 'chrome_text_is_not_checked' (($f -match 'PASS') -and $audit -ne '' -and $found -eq '' -and $wide -notmatch 'SAME diff=0')

# ---- 4. undo takes back the covers, not the frame; switching off restores the plain export ----
Focus-Editor
Key 0x1B # leave the panel: keys edit again
Key 0x5A -Ctrl
Log "COPY size=$(CopyTo 'frame-undone.png')"
$f = Frame 'frame-plain.png' 'frame-undone.png' '16:9' 'frame-inner-undone.png'
Log $f
Verdict 'undo_keeps_the_frame' ($f -match 'PASS')
Focus-Editor
Key 0x42 -Ctrl -Shift
Log "OFF $(Invoke-Named $h '关')"
Shot $h 'beautify-4-off.png'
Log "FOCUS $([System.Windows.Automation.AutomationElement]::FocusedElement.Current.Name)|$([System.Windows.Automation.AutomationElement]::FocusedElement.Current.ControlType.ProgrammaticName)"
Log "COPY size=$(CopyTo 'frame-off.png')"
$same = python (Join-Path $PSScriptRoot 'check_png.py') same $doc (Join-Path $Out 'frame-off.png')
Log "OFF $same"
Verdict 'frame_off_restores_the_plain_export' ($same -match 'SAME diff=0')
if (-not $p.HasExited) { $p.Kill() }
Start-Sleep -Milliseconds 800

# ---- 5. the style is remembered, the chrome text is not ----
$file = Get-Content $settingsFile -Raw
$style = ($file -split "`n" | Where-Object { $_ -match '^frame_style=' }) -join ''
$files = @(Get-ChildItem $tmp -Recurse -File -ErrorAction SilentlyContinue)
$hits = foreach ($x in $files) {
    $b = [IO.File]::ReadAllBytes($x.FullName)
    if (([Text.Encoding]::UTF8.GetString($b), [Text.Encoding]::Unicode.GetString($b) | Where-Object { $_.Contains('13912345678') -or $_.Contains('frame-e2e') })) { $x.Name }
}
Log "SAVED style=$style files=$($files.Count) text_hits=$(@($hits).Count)"
Verdict 'style_saved_text_not' ($style -match '^frame_style=0,2,2,2,3,3' -and @($hits).Count -eq 0)

# ---- 6. too large: refused with a notice; the copy still works, unframed ----
$big = Join-Path $tmp 'big.png'
@('hide_after_copy=false', 'frame_style=0,2,2,2,3,0') | Set-Content $settingsFile
python (Join-Path $PSScriptRoot 'check_frame.py') big $big | Out-Null
Start-App $big
Focus-Editor
Key 0x42 -Ctrl -Shift
[void](Wait-Name $h '^比例' 5)
Log "BIG on=$(Invoke-Named $h '开')"
$notice = Wait-Name $h '^图片太大，无法美化' 5
Shot $h 'beautify-3-too-large.png'
Key 0x1B
Log "BIG notice=$notice copy=$(CopyTo 'frame-big.png')"
$kept = (Get-Content $settingsFile -Raw) -match 'frame_style=0,2,2,2,3,0'
Log "BIG style_kept=$kept"
Verdict 'too_large_is_refused' ($notice -ne '' -and $kept -and (Test-Path (Join-Path $Out 'frame-big.png')) -and ((python -c "from PIL import Image; print(Image.open(r'$(Join-Path $Out 'frame-big.png')').size)") -match '6000, 4000'))
Remove-Item (Join-Path $Out 'frame-big.png') -ErrorAction SilentlyContinue
if (-not $p.HasExited) { $p.Kill() }
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
$log | Set-Content -Encoding utf8 (Join-Path $Out 'beautify.log')
