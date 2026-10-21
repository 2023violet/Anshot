# Show a synthetic image in a borderless top-most window at a fixed screen position,
# so capture tests only ever select synthetic pixels. Runs until killed.
# -Behind: not top-most, so app windows can sit in front of it.
param([Parameter(Mandatory)] [string] $Image, [int] $X = 200, [int] $Y = 150, [switch] $Behind)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type 'using System.Runtime.InteropServices; public static class D { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }'
[void][D]::SetProcessDPIAware()
$img = [System.Drawing.Image]::FromFile((Resolve-Path $Image))
$f = New-Object System.Windows.Forms.Form
$f.Text = 'Anshot synthetic fixture'
$f.FormBorderStyle = 'None'; $f.StartPosition = 'Manual'; $f.TopMost = -not $Behind; $f.ShowInTaskbar = $true
$f.Location = New-Object System.Drawing.Point $X, $Y
$f.ClientSize = New-Object System.Drawing.Size $img.Width, $img.Height
$pb = New-Object System.Windows.Forms.PictureBox
$pb.Dock = 'Fill'; $pb.SizeMode = 'Normal'; $pb.Image = $img
$f.Controls.Add($pb)
[void]$f.ShowDialog()
