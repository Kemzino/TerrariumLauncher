# Знімок вікна лаунчера за заголовком (PrintWindow — не потребує фокусу).
# Якщо вікно згорнуте — на мить розгортає і згортає назад.
param(
	[string]$Title = 'Terrarium',
	[string]$Out = "$env:TEMP\terrarium-window.png"
)

Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System; using System.Text; using System.Runtime.InteropServices; using System.Drawing; using System.Drawing.Imaging;
public static class TerrariumShot {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h, int cmd);
  public static string Capture(string title, string path) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => { var sb = new StringBuilder(256); GetWindowText(h, sb, 256); if (sb.ToString() == title) { found = h; return false; } return true; }, IntPtr.Zero);
    if (found == IntPtr.Zero) return "not found";
    bool wasMin = IsIconic(found);
    if (wasMin) { ShowWindow(found, 9); System.Threading.Thread.Sleep(2500); }
    RECT r; GetWindowRect(found, out r); int w = r.R - r.L, hh = r.B - r.T;
    using (var bmp = new Bitmap(w, hh)) using (var g = Graphics.FromImage(bmp)) {
      IntPtr hdc = g.GetHdc(); PrintWindow(found, hdc, 2); g.ReleaseHdc(hdc); bmp.Save(path, ImageFormat.Png);
    }
    if (wasMin) ShowWindow(found, 6);
    return w + "x" + hh;
  }
}
'@
$size = [TerrariumShot]::Capture($Title, $Out)
"$size $Out"
