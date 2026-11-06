# Starts and stops the screen recording of the backdrop area, and keeps a
# caption log (seconds from the start of the recording).
$script:FFmpeg = if ($env:FFMPEG) { $env:FFMPEG } else { "ffmpeg" }
$script:Captions = New-Object System.Collections.ArrayList

function Start-Backdrop([int]$X, [int]$Y, [int]$W, [int]$H) {
    $p = Start-Process -FilePath "powershell.exe" -WindowStyle Hidden -PassThru -ArgumentList @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Join-Path $PSScriptRoot "backdrop.ps1"),
        "-X", $X, "-Y", $Y, "-W", $W, "-H", $H)
    Start-Sleep -Milliseconds 1500
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
