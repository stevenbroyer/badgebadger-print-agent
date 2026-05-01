# Downloads SumatraPDF.exe into src-tauri/resources/ so `pnpm tauri build`
# bundles it into the MSI. Run once on the build machine before the first
# release; CI invokes it on every job. The version is pinned to a known-good
# release of SumatraPDF — bump intentionally.
#
# Usage:
#   pwsh scripts/fetch-sumatra.ps1
#
# The agent locates the bundled binary via printer::find_sumatra() and
# falls back to PATH / system installs when it isn't bundled.

$ErrorActionPreference = "Stop"
$Version = "3.5.2"
$Url = "https://www.sumatrapdfreader.org/dl/rel/$Version/SumatraPDF-$Version-64.exe"
$Dest = Join-Path $PSScriptRoot "..\src-tauri\resources\SumatraPDF.exe"

if (Test-Path $Dest) {
    Write-Host "SumatraPDF already present at $Dest — skipping."
    exit 0
}

Write-Host "Fetching SumatraPDF $Version → $Dest"
New-Item -ItemType Directory -Path (Split-Path $Dest) -Force | Out-Null
Invoke-WebRequest -Uri $Url -OutFile $Dest
Write-Host "Done."
