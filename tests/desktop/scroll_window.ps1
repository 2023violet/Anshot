# A scrollable synthetic page in a borderless top-most window (long-screenshot tests).
# The page image is shown 1:1 inside an auto-scroll panel; the mouse wheel scrolls it.
# Optional hard cases: -Side N adds a still sidebar (its own content) on the left; -Header/-Footer N add
# sticky bars above/below the scrolling page inside the right-hand pane.
param([Parameter(Mandatory)] [string] $Image, [int] $X = 200, [int] $Y = 150, [int] $W = 640, [int] $H = 420,
      [int] $Side = 0, [int] $Header = 0, [int] $Footer = 0)
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
$pane = New-Object System.Windows.Forms.Panel
$pane.Dock = 'Fill'
$pane.Controls.Add($panel)
# Docked bars are added after the Fill panel so they claim their edge first.
if ($Header -gt 0) { $hb = New-Object System.Windows.Forms.Panel; $hb.Dock = 'Top'; $hb.Height = $Header; $hb.BackColor = [System.Drawing.Color]::FromArgb(40, 90, 200); $pane.Controls.Add($hb) }
if ($Footer -gt 0) { $fb = New-Object System.Windows.Forms.Panel; $fb.Dock = 'Bottom'; $fb.Height = $Footer; $fb.BackColor = [System.Drawing.Color]::FromArgb(30, 140, 80); $pane.Controls.Add($fb) }
$f.Controls.Add($pane)
if ($Side -gt 0) {
    # A still sidebar with content of its own: horizontal stripes of varying grey.
    $sb = New-Object System.Drawing.Bitmap $Side, $H
    for ($y = 0; $y -lt $H; $y++) { $c = [System.Drawing.Color]::FromArgb(255, 200 + ($y * 7) % 50, 205, 210 + ($y * 3) % 40); for ($x = 0; $x -lt $Side; $x++) { $sb.SetPixel($x, $y, $c) } }
    $sp = New-Object System.Windows.Forms.PictureBox
    $sp.Dock = 'Left'; $sp.Width = $Side; $sp.Image = $sb
    $f.Controls.Add($sp)
}
$f.Add_Shown({ $panel.Focus() })
[void]$f.ShowDialog()
