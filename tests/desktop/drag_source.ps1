# A small top-most window that starts an OLE file drag of -File when pressed,
# so tests can exercise real OS drag-and-drop into Anshot. Runs until killed.
param([Parameter(Mandatory)] [string] $File, [int] $X = 40, [int] $Y = 40)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type 'using System.Runtime.InteropServices; public static class D { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }'
[void][D]::SetProcessDPIAware()
$f = New-Object System.Windows.Forms.Form
$f.Text = 'Anshot drag source'; $f.FormBorderStyle = 'None'; $f.StartPosition = 'Manual'; $f.TopMost = $true
$f.Location = New-Object System.Drawing.Point $X, $Y
$f.ClientSize = New-Object System.Drawing.Size 120, 60
$f.BackColor = [System.Drawing.Color]::FromArgb(245, 78, 0)
$path = (Resolve-Path $File).Path
$f.Add_MouseDown({
    $data = New-Object System.Windows.Forms.DataObject
    $list = New-Object System.Collections.Specialized.StringCollection
    [void]$list.Add($path)
    $data.SetFileDropList($list)
    [void]$f.DoDragDrop($data, [System.Windows.Forms.DragDropEffects]::Copy)
})
[void]$f.ShowDialog()
