# Starts and stops the screen recording of the backdrop area, and keeps a
# caption log (seconds from the start of the recording).
$script:FFmpeg = if ($env:FFMPEG) { $env:FFMPEG } else { "ffmpeg" }
$script:Captions = New-Object System.Collections.ArrayList

Add-Type @"
using System; using System.Runtime.InteropServices;
public static class Cover {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  public static IntPtr Window(uint pid, int x, int y, int w, int h) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((hw, l) => { uint p; GetWindowThreadProcessId(hw, out p); RECT r; GetWindowRect(hw, out r);
      if (p == pid && IsWindowVisible(hw) && r.L == x && r.T == y && r.R - r.L == w && r.B - r.T == h) { found = hw; return false; } return true; }, IntPtr.Zero);
    return found;
  }
  public static IntPtr TopAt(int x, int y) { POINT p; p.X = x; p.Y = y; return GetAncestor(WindowFromPoint(p), 2); }
}
"@

# The backdrop covers the recording area so nothing else on the screen is
# filmed. It is forced on top, and the recording refuses to start unless the
# backdrop is what shows at nine points across the area.
function Start-Backdrop([int]$X, [int]$Y, [int]$W, [int]$H) {
    $p = Start-Process -FilePath "powershell.exe" -WindowStyle Hidden -PassThru -ArgumentList @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Join-Path $PSScriptRoot "backdrop.ps1"),
        "-X", $X, "-Y", $Y, "-W", $W, "-H", $H)
    $hw = [IntPtr]::Zero
    $deadline = (Get-Date).AddSeconds(15)
    while ($hw -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 200
        $hw = [Cover]::Window([uint32]$p.Id, $X, $Y, $W, $H)
    }
    if ($hw -eq [IntPtr]::Zero) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue; throw "the backdrop never appeared; not recording" }
    [void][Cover]::SetWindowPos($hw, [IntPtr](-1), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0010 -bor 0x0040)
    Start-Sleep -Milliseconds 500
    foreach ($fx in 0.02, 0.5, 0.98) {
        foreach ($fy in 0.02, 0.5, 0.98) {
            $top = [Cover]::TopAt([int]($X + $W * $fx), [int]($Y + $H * $fy))
            if ($top -ne $hw) {
                Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
                throw "something else is on top of the recording area; not recording"
            }
        }
    }
    return $p
}

# The cursor is drawn: the video shows where each click lands.
function Start-Recording([string]$Out, [int]$X, [int]$Y, [int]$W, [int]$H) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $script:FFmpeg
    $psi.Arguments = "-y -loglevel error -f gdigrab -framerate 30 -draw_mouse 1 -offset_x $X -offset_y $Y -video_size ${W}x${H} -i desktop -c:v libx264 -preset veryfast -crf 16 -pix_fmt yuv420p `"$Out`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.CreateNoWindow = $true
    $p = [System.Diagnostics.Process]::Start($psi)
    Start-Sleep -Milliseconds 900
    $script:Clock = [System.Diagnostics.Stopwatch]::StartNew()
    return $p
}

function Stop-Recording($Process) {
    $Process.StandardInput.Write("q")
    $Process.StandardInput.Flush()
    if (-not $Process.WaitForExit(20000)) { $Process.Kill() }
}

# Starts a caption now; the previous one ends here. Chapter names group the
# captions for the title cards in the edit.
function Caption([string]$Text, [string]$Chapter = "") {
    $t = [math]::Round($script:Clock.Elapsed.TotalSeconds, 2)
    if ($script:Captions.Count -gt 0) { $script:Captions[$script:Captions.Count - 1].end = $t }
    [void]$script:Captions.Add([pscustomobject]@{ start = $t; end = $null; text = $Text; chapter = $Chapter })
    Write-Host ("{0,6:N1}s  {1}" -f $t, $Text)
}

function Save-Captions([string]$Path) {
    $t = [math]::Round($script:Clock.Elapsed.TotalSeconds, 2)
    if ($script:Captions.Count -gt 0 -and $null -eq $script:Captions[$script:Captions.Count - 1].end) {
        $script:Captions[$script:Captions.Count - 1].end = $t
    }
    ConvertTo-Json -InputObject @($script:Captions) | Set-Content -Path $Path -Encoding UTF8
}
