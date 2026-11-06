# Window and UI Automation helpers for the recordings (Windows PowerShell 5.1).
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Text; using System.Diagnostics;
public static class Win {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  // First visible top-level window whose title passes the test, or whose
  // process has the given name when processName isn't empty.
  public static IntPtr Find(string titleContains, string exactTitle, string processName) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => {
      if (!IsWindowVisible(h)) return true;
      var sb = new StringBuilder(512); GetWindowText(h, sb, 512); var t = sb.ToString();
      if (t.Length == 0) return true;
      bool ok = false;
      if (exactTitle != null && t == exactTitle) ok = true;
      if (titleContains != null && t.Contains(titleContains)) ok = true;
      if (processName != null) {
        uint pid; GetWindowThreadProcessId(h, out pid);
        try { if (Process.GetProcessById((int)pid).ProcessName.Equals(processName, StringComparison.OrdinalIgnoreCase)) ok = true; } catch {}
      }
      if (ok) { found = h; return false; }
      return true;
    }, IntPtr.Zero);
    return found;
  }
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  // One left click at a screen point, then the cursor goes back where it was.
  public static void Click(int x, int y) {
    POINT old; GetCursorPos(out old);
    SetCursorPos(x, y); System.Threading.Thread.Sleep(40);
    mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero); mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
    System.Threading.Thread.Sleep(60); SetCursorPos(old.X, old.Y);
  }
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h, int id);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  // NSIS keeps Next/Install/Finish on control 1 and changes its text per page.
  public static string ButtonText(IntPtr dlg, int id) { var b = GetDlgItem(dlg, id); if (b == IntPtr.Zero) return ""; var sb = new StringBuilder(64); GetWindowText(b, sb, 64); return sb.ToString(); }
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  public static bool ButtonEnabled(IntPtr dlg, int id) { var b = GetDlgItem(dlg, id); return b != IntPtr.Zero && IsWindowEnabled(b); }
  // What the button itself tells its dialog when clicked: WM_COMMAND, BN_CLICKED.
  public static void PressButton(IntPtr dlg, int id) { var b = GetDlgItem(dlg, id); if (b != IntPtr.Zero) PostMessage(dlg, 0x0111, new IntPtr(id), b); }
  // The agent's main window: its process and its exact title, so neither the
  // installer nor the agent's hidden helper windows match.
  public static IntPtr FindAgent() {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => {
      if (!IsWindowVisible(h)) return true;
      var sb = new StringBuilder(256); GetWindowText(h, sb, 256);
      if (sb.ToString() != "BadgeBadger Print Agent") return true;
      uint pid; GetWindowThreadProcessId(h, out pid);
      try { if (Process.GetProcessById((int)pid).ProcessName == "badgebadger-print-agent") { found = h; return false; } } catch {}
      return true;
    }, IntPtr.Zero);
    return found;
  }
  // What a dialog's Cancel button sends: WM_COMMAND with IDCANCEL.
  public static void Cancel(IntPtr h) { PostMessage(h, 0x0111, new IntPtr(2), IntPtr.Zero); }
  public static void Topmost(IntPtr h, bool on) { SetWindowPos(h, new IntPtr(on ? -1 : -2), 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0010 | 0x0040); }
}
"@
[Win]::SetProcessDPIAware() | Out-Null

function Wait-Window([string]$Contains = $null, [string]$Exact = $null, [string]$Process = $null, [double]$TimeoutSec = 30) {
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $h = [Win]::Find($Contains, $Exact, $Process)
        if ($h -ne [IntPtr]::Zero) { return $h }
        Start-Sleep -Milliseconds 200
    }
    return [IntPtr]::Zero
}

# $null when the window or the element isn't there (windows come and go
# while dialogs open and close).
function Find-Element([IntPtr]$Hwnd, [string]$Name) {
    try {
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
        $cond = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $Name
        return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
    } catch {
        return $null
    }
}

# Presses a button or link by its visible name, without moving the mouse.
function Invoke-Named([IntPtr]$Hwnd, [string]$Name) {
    $el = Find-Element $Hwnd $Name
    if (-not $el) { return $false }
    if (-not $el.Current.IsEnabled) { return $false }
    $pattern = $null
    if ($el.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        $pattern.Invoke(); return $true
    }
    return $false
}

# Clicks the middle of a named element, for controls UI Automation can find
# but not invoke (SmartScreen's "More info" link).
function Click-Named([IntPtr]$Hwnd, [string]$Name) {
    $el = Find-Element $Hwnd $Name
    if (-not $el) { return $false }
    $r = $el.Current.BoundingRectangle
    if ($r.IsEmpty -or $r.Width -le 0) { return $false }
    [Win]::Click([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2))
    return $true
}

Add-Type @"
using System; using System.Runtime.InteropServices;
public static class Mouse {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int hh, bool repaint);
  // Glides to a point the way a hand would: eased, about 22 steps.
  public static void Glide(int x, int y, int ms) {
    POINT s; GetCursorPos(out s);
    int steps = Math.Max(8, ms / 16);
    for (int i = 1; i <= steps; i++) {
      double t = (double)i / steps; double e = t < 0.5 ? 4 * t * t * t : 1 - Math.Pow(-2 * t + 2, 3) / 2;
      SetCursorPos((int)(s.X + (x - s.X) * e), (int)(s.Y + (y - s.Y) * e));
      System.Threading.Thread.Sleep(16);
    }
  }
  public static void Press() {
    mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero); System.Threading.Thread.Sleep(90);
    mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
  }
  public static POINT ClientOrigin(IntPtr h) { POINT p = new POINT(); ClientToScreen(h, ref p); return p; }
}
"@

# Glide to a screen point, pause like a person would, click.
function Click-At([int]$X, [int]$Y, [int]$GlideMs = 650) {
    [Mouse]::Glide($X, $Y, $GlideMs)
    Start-Sleep -Milliseconds 220
    [Mouse]::Press()
}

# The centre of an installer button (NSIS control id), in screen pixels.
function Button-Center([IntPtr]$Dlg, [int]$Id) {
    $b = [Win]::GetDlgItem($Dlg, $Id)
    $r = New-Object Mouse+RECT
    [void][Mouse]::GetWindowRect($b, [ref]$r)
    return @([int](($r.L + $r.R) / 2), [int](($r.T + $r.B) / 2))
}
