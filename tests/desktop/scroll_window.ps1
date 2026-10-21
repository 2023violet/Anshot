# A scrollable synthetic page in a borderless top-most window (long-screenshot tests).
# The page image is shown 1:1 inside an auto-scroll panel; the mouse wheel scrolls it.
param([Parameter(Mandatory)] [string] $Image, [int] $X = 200, [int] $Y = 150, [int] $W = 640, [int] $H = 420)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type 'using System.Runtime.InteropServices; public static class D { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }'
[void][D]::SetProcessDPIAware()
$img = [System.Drawing.Image]::FromFile((Resolve-Path $Image))
$f = New-Object System.Windows.Forms.Form
$f.Text = 'Anshot scroll fixture'
$f.FormBorderStyle = 'None'; $f.StartPosition = 'Manual'; $f.TopMost = $true; $f.ShowInTaskbar = $true
$f.Location = New-Object System.Drawing.Point $X, $Y
$f.ClientSize = New-Object System.Drawing.Size $W, $H
$panel = New-Object System.Windows.Forms.Panel
$panel.Dock = 'Fill'; $panel.AutoScroll = $true
$pb = New-Object System.Windows.Forms.PictureBox
$pb.SizeMode = 'AutoSize'; $pb.Image = $img; $pb.Location = New-Object System.Drawing.Point 0, 0
$panel.Controls.Add($pb)
$f.Controls.Add($panel)
$f.Add_Shown({ $panel.Focus() })
[void]$f.ShowDialog()
