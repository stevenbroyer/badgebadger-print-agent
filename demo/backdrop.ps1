# A plain backdrop over the recording area, so nothing else on the screen
# ends up in the video. Runs until its process is stopped.
param([int]$X, [int]$Y, [int]$W, [int]$H)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class Dpi { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }
"@
[Dpi]::SetProcessDPIAware() | Out-Null
$form = New-Object System.Windows.Forms.Form
$form.FormBorderStyle = "None"
$form.StartPosition = "Manual"
$form.ShowInTaskbar = $false
$form.TopMost = $true
$form.Bounds = New-Object System.Drawing.Rectangle $X, $Y, $W, $H
$form.Add_Paint({
    param($sender, $e)
    $rect = $sender.ClientRectangle
    # A gradient brush the exact size of the area leaves a seam where it wraps.
    $brushRect = New-Object System.Drawing.Rectangle 0, -1, $rect.Width, ($rect.Height + 2)
    $top = [System.Drawing.Color]::FromArgb(0xE8, 0xEC, 0xF2)
    $bottom = [System.Drawing.Color]::FromArgb(0xCC, 0xD5, 0xE0)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush $brushRect, $top, $bottom, 90
    $e.Graphics.FillRectangle($brush, $rect)
    $brush.Dispose()
})
[System.Windows.Forms.Application]::Run($form)
