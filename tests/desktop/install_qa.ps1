# Install / upgrade / uninstall QA for the Anshot NSIS installer on the current user.
# Usage: pwsh -STA -File tests/desktop/install_qa.ps1 -Setup <Anshot-x-setup.exe> -Exe <portable Anshot.exe> -Out <dir>
# Refuses to run if Anshot is already installed; removes everything it created at the end.
param([Parameter(Mandatory)] [string] $Setup, [Parameter(Mandatory)] [string] $Exe, [Parameter(Mandatory)] [string] $Out)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, UIAutomationClient, UIAutomationTypes
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class Q {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, IntPtr e);
}
'@
[void][Q]::SetProcessDPIAware()
New-Item -ItemType Directory -Force $Out | Out-Null
$log = [System.Collections.Generic.List[string]]::new()
# Never record the account name: profile paths are written as %USERPROFILE%.
function Log($s) { $s = "$s".Replace($env:USERPROFILE, '%USERPROFILE%'); $log.Add($s); $s }

$dir = Join-Path $env:LOCALAPPDATA 'Programs\Anshot'
$uninstKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Anshot'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$lnk = Join-Path ([Environment]::GetFolderPath('Programs')) '安截 Anshot.lnk'
$desk = Join-Path ([Environment]::GetFolderPath('Desktop')) '安截 Anshot.lnk'
$cfg = Join-Path $env:APPDATA 'Anshot'
$data = Join-Path $env:LOCALAPPDATA 'Anshot'
function WaitWin([string] $title, [int] $s = 20) {
    $d = (Get-Date).AddSeconds($s); $h = [IntPtr]::Zero
    # NSIS titles end with a space: match the trimmed title of any process's main window.
    while ($h -eq [IntPtr]::Zero -and (Get-Date) -lt $d) {
        $w = Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle.Trim() -eq $title } | Select-Object -First 1
        if ($w) { $h = $w.MainWindowHandle } else { Start-Sleep -Milliseconds 300 }
    }
    if ($h -eq [IntPtr]::Zero) { throw "window '$title' did not appear" }
    Start-Sleep -Milliseconds 800; $h
}
function RunValue { (Get-ItemProperty -Path $runKey -Name Anshot -ErrorAction SilentlyContinue).Anshot }
function State {
    "dir=$(Test-Path $dir) uninstall_key=$(Test-Path $uninstKey) startmenu=$(Test-Path $lnk) desktop=$(Test-Path $desk) run_value=$([bool](RunValue)) settings_dir=$(Test-Path $cfg)"
}
function AppProcs { @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path -like "$dir\*" }) }
function WaitGone([string] $path, [int] $s) { $d = (Get-Date).AddSeconds($s); while ((Test-Path $path) -and (Get-Date) -lt $d) { Start-Sleep -Milliseconds 250 } }
# NSIS wizard controls are plain Win32 buttons (UIA shows them as panes): click them natively.
function Click-Native($h, [string] $name) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    $el = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $name))
    if (-not $el -or $el.Current.NativeWindowHandle -eq 0) { return $false }
    [void][Q]::SendMessage([IntPtr]$el.Current.NativeWindowHandle, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)  # BM_CLICK
    Start-Sleep -Milliseconds 900; return $true
}
function Invoke-Uia($h, [string] $name) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    $el = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $name))
    if (-not $el) { return $false }
    $pat = $null
    if ($el.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pat)) { $pat.Invoke(); Start-Sleep -Milliseconds 700; return $true }
    if ($el.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pat)) { $pat.Toggle(); Start-Sleep -Milliseconds 300; return $true }
    return $false
}
# The window renders itself (PrintWindow): other windows on top can never end up in the image.
function Shot($h, [string] $file) {
    $r = New-Object Q+RECT; [void][Q]::GetWindowRect($h, [ref]$r)
    $bmp = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T); $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc(); [void][Q]::PrintWindow($h, $hdc, 2); $g.ReleaseHdc($hdc); $g.Dispose()
    $bmp.Save((Join-Path $Out $file)); $bmp.Dispose()
}

$setupHash = (Get-FileHash $Setup -Algorithm SHA256).Hash.ToLower()
$exeHash = (Get-FileHash $Exe -Algorithm SHA256).Hash.ToLower()
Log "SETUP sha256=$setupHash exe_sha256=$exeHash"
# A. preconditions
Log "A before: $(State)"
if ((Test-Path $dir) -or (Test-Path $uninstKey) -or (RunValue)) { throw "Anshot is already installed or registered on this account; QA refuses to touch it." }
$cfgExisted = Test-Path $cfg; $dataExisted = Test-Path $data

# B. interactive install with the real wizard (Chinese UI), default options
$p = Start-Process $Setup -PassThru
$h = WaitWin '安截 Anshot 0.2.0 安装'
# (no screenshot of the folder page: it shows the account's profile path)
Log "B next=$(Click-Native $h '下一步(N) >')"
Shot $h 'install-2-components.png'
Log "B install=$(Click-Native $h '安装(I)')"
# Wait for the finish page (copying takes a few seconds).
$root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
$cond = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), '现在启动 安截'
$d = (Get-Date).AddSeconds(60); while (-not $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond) -and (Get-Date) -lt $d) { Start-Sleep -Milliseconds 300 }
Shot $h 'install-3-finish.png'
# untick "run now" so the test controls the first start, then Finish (IDOK)
Log "B untick_run=$(Click-Native $h '现在启动 安截')"
[void][Q]::PostMessage($h, 0x0111, [IntPtr]1, [IntPtr]::Zero); Start-Sleep -Seconds 1
Log "B finish=$($p.WaitForExit(15000))"
$installed = Join-Path $dir 'Anshot.exe'
$iv = Get-ItemProperty -Path $uninstKey -ErrorAction SilentlyContinue
Log "B after: $(State) exe_matches=$((Test-Path $installed) -and (Get-FileHash $installed).Hash.ToLower() -eq $exeHash) display=$($iv.DisplayName) version=$($iv.DisplayVersion) uninstall=$($iv.UninstallString)"

if (-not (Test-Path $installed)) { $log | Set-Content -Encoding utf8 (Join-Path $Out 'install_qa.log'); throw "wizard did not install" }
# C. first run from the installed location
$app = Start-Process $installed -PassThru; Start-Sleep -Seconds 3
Log "C running=$(-not $app.HasExited) window=$([bool]((Get-Process -Id $app.Id).MainWindowTitle -like '*Anshot*'))"

# D. start at login through the app's own settings (real per-user Run value)
$main = (Get-Process -Id $app.Id).MainWindowHandle
[void][Q]::SetForegroundWindow($main); Start-Sleep -Milliseconds 400
$sh = New-Object -ComObject WScript.Shell; $sh.SendKeys('^,'); Start-Sleep -Milliseconds 800
Log "D autostart_on=$(Invoke-Uia $main '登录后在托盘待命') run_value=$(RunValue)"

# D2. change a setting through the app, so user data exists to be kept or removed later
Log "D2 keep_200=$(Invoke-Uia $main '200 张') settings_file=$(Test-Path (Join-Path $cfg 'settings.txt'))"

# E. upgrade (silent reinstall) while the app is running: the installer closes it and replaces files
$up = Start-Process $Setup -ArgumentList '/S' -PassThru; $up.WaitForExit(60000) | Out-Null
Start-Sleep -Seconds 1
Log "E upgrade_exit=$($up.ExitCode) old_process_gone=$($app.HasExited) exe_matches=$((Get-FileHash $installed).Hash.ToLower() -eq $exeHash) $(State)"

# F. start again (as after login: --background stays in the tray), then uninstall silently while it runs
$bg = Start-Process $installed -ArgumentList '--background' -PassThru; Start-Sleep -Seconds 3
Log "F background_running=$(-not $bg.HasExited) visible_window=$([bool]((Get-Process -Id $bg.Id).MainWindowTitle))"
# F2. interactive upgrade while it runs: the wizard must warn before closing it
$gui = Start-Process $Setup -PassThru
$gh = WaitWin '安截 Anshot 0.2.0 安装'
[void](Click-Native $gh '下一步(N) >'); [void](Click-Native $gh '安装(I)')
$desktop = [System.Windows.Automation.AutomationElement]::RootElement
$byPid = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ProcessIdProperty), $gui.Id
$warn = $null; $warnText = ''; $d = (Get-Date).AddSeconds(15)
while (-not $warn -and (Get-Date) -lt $d) {
    foreach ($top in $desktop.FindAll([System.Windows.Automation.TreeScope]::Children, $byPid)) {
        $text = (@($top.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) | ForEach-Object { $_.Current.Name }) -join ' '
        if ($text -match '正在运行') { $warn = $top; $warnText = $text }
    }
    if (-not $warn) { Start-Sleep -Milliseconds 300 }
}
Log "F2 warning_shown=$($warnText -match '正在运行') app_still_running_at_prompt=$(-not $bg.HasExited)"
if ($warn) { [void](Click-Native ([IntPtr]$warn.Current.NativeWindowHandle) '确定') }
$root2 = [System.Windows.Automation.AutomationElement]::FromHandle($gh)
$cond2 = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), '现在启动 安截'
$d = (Get-Date).AddSeconds(60); while (-not $root2.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond2) -and (Get-Date) -lt $d) { Start-Sleep -Milliseconds 300 }
[void](Click-Native $gh '现在启动 安截'); [void][Q]::PostMessage($gh, 0x0111, [IntPtr]1, [IntPtr]::Zero)
Log "F2 upgraded=$($gui.WaitForExit(15000)) app_closed=$($bg.HasExited) exe_matches=$((Get-FileHash $installed).Hash.ToLower() -eq $exeHash)"
$un = Start-Process (Join-Path $dir 'uninstall.exe') -ArgumentList '/S' -PassThru; $un.WaitForExit(30000) | Out-Null
WaitGone $installed 30; WaitGone $dir 10
Start-Sleep -Seconds 1
# G. everything installed is gone; user data is kept by default
Log "G after_uninstall: $(State) process_gone=$($bg.HasExited) leftover_procs=$((AppProcs).Count)"

# H. default uninstall kept the settings; now reinstall and uninstall with the wizard,
#    ticking "同时删除设置与截图历史": the user data must go as well.
Log "H settings_kept_by_default=$(Test-Path (Join-Path $cfg 'settings.txt'))"
$re = Start-Process $Setup -ArgumentList '/S' -PassThru; $re.WaitForExit(60000) | Out-Null
$ui = Start-Process (Join-Path $dir 'uninstall.exe') -PassThru
$uh = WaitWin '安截 Anshot 0.2.0 卸载'
# The uninstaller is driven with its own commands (IDOK = Next / Uninstall / Close).
[void][Q]::SetForegroundWindow($uh); Start-Sleep -Milliseconds 300
[void][Q]::PostMessage($uh, 0x0111, [IntPtr]1, [IntPtr]::Zero); Start-Sleep -Seconds 2
# The components page is a check-box tree that UIA does not expose: click the second row's box.
$root = [System.Windows.Automation.AutomationElement]::FromHandle($uh)
$tv = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ClassNameProperty), 'SysTreeView32'))
$tvb = $tv.Current.BoundingRectangle
[void][Q]::SetCursorPos([int]$tvb.Left + 12, [int]$tvb.Top + 26); Start-Sleep -Milliseconds 150
[Q]::mouse_event(2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60; [Q]::mouse_event(4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 500
Shot $uh 'uninstall-components.png'
Log "H data_option_clicked=$($null -ne $tv)"
[void][Q]::PostMessage($uh, 0x0111, [IntPtr]1, [IntPtr]::Zero)
$d = (Get-Date).AddSeconds(30); while ((Test-Path $installed) -and (Get-Date) -lt $d) { Start-Sleep -Milliseconds 300 }
Start-Sleep -Seconds 2
[void][Q]::PostMessage($uh, 0x0111, [IntPtr]1, [IntPtr]::Zero); Start-Sleep -Seconds 1
Log "H after_uninstall_with_data: $(State) appdata_dir=$(Test-Path $cfg) localdata_dir=$(Test-Path $data)"

# Clean up what this QA created (never data that existed before it ran).
if (-not $cfgExisted -and (Test-Path $cfg)) { Remove-Item -Recurse -Force $cfg; Log "cleanup removed $cfg (created by this QA)" }
if (-not $dataExisted -and (Test-Path $data)) { Remove-Item -Recurse -Force $data; Log "cleanup removed $data (created by this QA)" }
Log "Z final: $(State)"
$log | Set-Content -Encoding utf8 (Join-Path $Out 'install_qa.log')
