# Runs the capture comparison over a synthetic full-screen test pattern (top-most), so only
# generated pixels are on screen while the three APIs read it. Prints statistics only.
# Usage: pwsh -STA -File spike/capture-api/run.ps1 [-Runs 5]
param([int] $Runs = 5)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type 'using System.Runtime.InteropServices; public static class D { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); }'
[void][D]::SetProcessDPIAware()
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
for ($x = 0; $x -lt $b.Width; $x += 8) {
    $c = [System.Drawing.Color]::FromArgb(($x * 7) % 256, ($x * 3) % 256, 255 - ($x % 256))
    $g.FillRectangle((New-Object System.Drawing.SolidBrush $c), $x, 0, 8, $b.Height / 2)
}
$g.FillRectangle([System.Drawing.Brushes]::White, 0, $b.Height / 2, $b.Width, $b.Height / 2)
$font = New-Object System.Drawing.Font 'Microsoft YaHei', 18
for ($y = [int]($b.Height / 2) + 20; $y -lt $b.Height - 30; $y += 40) { $g.DrawString("合成测试图 Synthetic pattern 0123456789 截图接口比较", $font, [System.Drawing.Brushes]::Black, 20, $y) }
$g.Dispose()
$form = New-Object System.Windows.Forms.Form
$form.FormBorderStyle = 'None'; $form.StartPosition = 'Manual'; $form.Bounds = $b; $form.TopMost = $true
$form.BackgroundImage = $bmp; $form.ShowInTaskbar = $false
$form.Show(); [System.Windows.Forms.Application]::DoEvents()
[void][D]::SetCursorPos($b.Width - 2, $b.Height - 2)
Start-Sleep -Milliseconds 800; [System.Windows.Forms.Application]::DoEvents()
try { & (Join-Path $PSScriptRoot 'target\release\capture-api-spike.exe') $Runs } finally { $form.Close(); $bmp.Dispose() }
