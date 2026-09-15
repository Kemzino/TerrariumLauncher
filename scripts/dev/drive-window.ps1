# Керування вікном лаунчера для скріншотів: фокус, клік у точку, колесо миші, клавіші.
# Приклад: drive-window.ps1 -ClickX 0.5 -ClickY 0.3 -Wheel -15 -Keys "{END}" -Out shot.png
param(
	[string]$Title = 'Terrarium',
	[double]$ClickX = -1,   # частка ширини вікна (0..1); -1 = без кліку
	[double]$ClickY = -1,   # частка висоти вікна (0..1)
	[int]$Wheel = 0,        # кроки колеса: від'ємні = вниз
	[string]$Keys = '',     # SendKeys, напр. "{END}"
	[string]$Out = ''       # якщо задано — зробити знімок після дій
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -TypeDefinition @'
using System; using System.Text; using System.Runtime.InteropServices;
public static class TerrariumDrive {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int x, int y, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  public static IntPtr Find(string title) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => { var sb = new StringBuilder(256); GetWindowText(h, sb, 256); if (sb.ToString() == title) { found = h; return false; } return true; }, IntPtr.Zero);
    return found;
  }
}
'@

[TerrariumDrive]::SetProcessDPIAware() | Out-Null
$h = [TerrariumDrive]::Find($Title)
if ($h -eq [IntPtr]::Zero) { "not found"; exit 1 }
if ([TerrariumDrive]::IsIconic($h)) { [TerrariumDrive]::ShowWindow($h, 9) | Out-Null; Start-Sleep -Milliseconds 800 }
[TerrariumDrive]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 300
# Захист: якщо вікно не стало активним (користувач працює в іншому) — не клікати наосліп
if ([TerrariumDrive]::GetForegroundWindow() -ne $h) { "not foreground - aborting"; exit 2 }
$r = New-Object TerrariumDrive+RECT; [TerrariumDrive]::GetWindowRect($h, [ref]$r) | Out-Null
$w = $r.R - $r.L; $hh = $r.B - $r.T

if ($ClickX -ge 0 -and $ClickY -ge 0) {
	$x = [int]($r.L + $w * $ClickX); $y = [int]($r.T + $hh * $ClickY)
	[TerrariumDrive]::SetCursorPos($x, $y) | Out-Null; Start-Sleep -Milliseconds 150
	[TerrariumDrive]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); [TerrariumDrive]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
	Start-Sleep -Milliseconds 400
}
if ($Wheel -ne 0) {
	1..[math]::Abs($Wheel) | ForEach-Object {
		$d = if ($Wheel -lt 0) { [uint32]4294967176 } else { [uint32]120 }
		[TerrariumDrive]::mouse_event(0x0800, 0, 0, $d, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40
	}
	Start-Sleep -Milliseconds 500
}
if ($Keys) { [System.Windows.Forms.SendKeys]::SendWait($Keys); Start-Sleep -Milliseconds 700 }
if ($Out) { & "$PSScriptRoot\capture-window.ps1" -Title $Title -Out $Out }
"window ${w}x${hh} at $($r.L),$($r.T)"
