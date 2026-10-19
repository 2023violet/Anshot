# Control experiment: same pinyin keystrokes into classic Notepad (Win32 Edit, IMM32/TSF aware).
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class K {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, IntPtr e);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr p, IntPtr a, string c, string w);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, System.Text.StringBuilder l);
}
'@
$p = Start-Process notepad.exe -PassThru
try {
  Start-Sleep 2; $p.Refresh(); [void][K]::SetForegroundWindow($p.MainWindowHandle); Start-Sleep -Milliseconds 600
  [K]::keybd_event(0x10, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60; [K]::keybd_event(0x10, 0, 2, [IntPtr]::Zero); Start-Sleep -Milliseconds 300
  foreach ($k in 0x4E, 0x49, 0x48, 0x41, 0x4F, 0x20) { [K]::keybd_event([byte]$k, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 60; [K]::keybd_event([byte]$k, 0, 2, [IntPtr]::Zero); Start-Sleep -Milliseconds 90 }
  Start-Sleep -Milliseconds 800
  $e = [K]::FindWindowEx($p.MainWindowHandle, [IntPtr]::Zero, "Edit", $null)
  $sb = New-Object System.Text.StringBuilder 256; [void][K]::SendMessage($e, 0x000D, [IntPtr]256, $sb)
  $text = $sb.ToString()
  "NOTEPAD text=[" + $text + "]"
} finally { if (-not $p.HasExited) { $p.Kill() } }
