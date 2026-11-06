# The SmartScreen chapter, with the cursor drawn to match the cloud footage.
# Double-clicks happen through File Explorer on a copy marked as downloaded.
# The cursor clicks "More info", then only rests on "Run anyway"; the
# recording stops there, and the warning is closed with "Don't run" after.
# Never run it without the backdrop: recorder.ps1 refuses to record unless
# the backdrop covers the whole area.
param([string]$Setup, [string]$OutDir, [string]$Kit = $PSScriptRoot, [int]$X = 1840, [int]$Y = 270, [int]$W = 1440, [int]$H = 900)
$ErrorActionPreference = "Stop"
. (Join-Path $Kit "ui.ps1")
. (Join-Path $Kit "recorder.ps1")
$script:FFmpeg = if ($env:FFMPEG) { $env:FFMPEG } else { (Get-Command ffmpeg).Source }
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Text; using System.Collections.Generic;
public static class Frames {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  public static List<long> All() {
    var o = new List<long>();
    EnumWindows((h, l) => { if (!IsWindowVisible(h)) return true; var c = new StringBuilder(64); GetClassName(h, c, 64); if (c.ToString() == "ApplicationFrameWindow") o.Add(h.ToInt64()); return true; }, IntPtr.Zero);
    return o;
  }
  public static RECT Rect(long h) { RECT r; GetWindowRect(new IntPtr(h), out r); return r; }
}
"@

$dl = Join-Path $OutDir "download"
New-Item -ItemType Directory -Force -Path $dl | Out-Null
$name = Split-Path $Setup -Leaf
$file = Join-Path $dl $name
Copy-Item $Setup $file -Force
Set-Content -Path $file -Stream Zone.Identifier -Value "[ZoneTransfer]`r`nZoneId=3`r`nReferrerUrl=https://badgebadger.app/`r`nHostUrl=https://github.com/stevenbroyer/badgebadger-print-agent/releases/latest/download/$name`r`n"

$before = [Frames]::All()
$backdrop = Start-Backdrop $X $Y $W $H
$saved = New-Object Mouse+POINT
[void][Mouse]::GetCursorPos([ref]$saved)
[void][Mouse]::SetCursorPos($X + $W - 220, $Y + $H - 160)
$rec = Start-Recording (Join-Path $OutDir "smartscreen2-raw.mp4") $X $Y $W $H
$frame = 0
try {
    Start-Sleep -Milliseconds 1200
    Caption "Double-click the installer you downloaded" "Download"
    Start-Process -FilePath "explorer.exe" -ArgumentList "`"$file`""
    $deadline = (Get-Date).AddSeconds(20)
    while (-not $frame -and (Get-Date) -lt $deadline) {
        foreach ($h in [Frames]::All()) {
            if ($before -contains $h) { continue }
            $r = [Frames]::Rect($h)
            if (($r.R - $r.L) -gt 400 -and ($r.R - $r.L) -lt 800) { $frame = $h }
        }
        Start-Sleep -Milliseconds 150
    }
    if (-not $frame) { throw "SmartScreen didn't appear" }
    [Win]::Topmost([IntPtr]$frame, $true)
    Start-Sleep -Milliseconds 700
    Caption "Windows may warn you while the app is new" "Download"
    Start-Sleep -Milliseconds 3200
    $r = [Frames]::Rect($frame)
    Click-At ($r.L + 60) ($r.T + 130) 700
    Start-Sleep -Milliseconds 600
    Caption "Click More info: it names the verified publisher, Steven Broyer" "Download"
    Start-Sleep -Milliseconds 4600
    Caption "Then click Run anyway" "Download"
    $r = [Frames]::Rect($frame)
    [Mouse]::Glide($r.L + 343, $r.T + 455, 650)
    Start-Sleep -Milliseconds 2400
} finally {
    Save-Captions (Join-Path $OutDir "smartscreen2-captions.json")
    Stop-Recording $rec
    if ($frame) {
        # Off the recording: close the warning with "Don't run".
        $r = [Frames]::Rect($frame)
        [Mouse]::SetCursorPos($r.L + 465, $r.T + 456) | Out-Null
        Start-Sleep -Milliseconds 200
        [Mouse]::Press()
        Start-Sleep -Milliseconds 800
    }
    [void][Mouse]::SetCursorPos($saved.X, $saved.Y)
    Stop-Process -Id $backdrop.Id -Force -ErrorAction SilentlyContinue
}
