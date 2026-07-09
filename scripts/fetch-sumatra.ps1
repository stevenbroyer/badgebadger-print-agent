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
# SHA-256 of the official SumatraPDF-3.5.2-64.exe. Pinning defends every
# operator's install against a tampered/corrupted download — this exe ships
# inside our signed installer and spawns a print process, so an unverified
# fetch is a supply-chain hole (and a Defender heuristic trigger). Bump this
# hash whenever $Version changes; cross-check against sumatrapdfreader.org.
$ExpectedSha = "290e4aa7ed64c728138711c011e89aab7aa48dbc1ae430371dc2be4100b92bf0"
$Url = "https://www.sumatrapdfreader.org/dl/rel/$Version/SumatraPDF-$Version-64.exe"
$Dest = Join-Path $PSScriptRoot "..\src-tauri\resources\SumatraPDF.exe"

function Assert-Sha([string]$Path) {
    $actual = (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLower()
    if ($actual -ne $ExpectedSha) {
        Remove-Item $Path -Force -ErrorAction SilentlyContinue
        throw "SumatraPDF SHA-256 mismatch. expected $ExpectedSha got $actual — refusing to bundle an unverified binary."
    }
    Write-Host "SHA-256 verified: $actual"
}

if (Test-Path $Dest) {
    # Verify the cached copy too, so a bad file from a prior run can't linger.
    Assert-Sha $Dest
    Write-Host "SumatraPDF already present and verified at $Dest — skipping."
    exit 0
}

Write-Host "Fetching SumatraPDF $Version → $Dest"
New-Item -ItemType Directory -Path (Split-Path $Dest) -Force | Out-Null
Invoke-WebRequest -Uri $Url -OutFile $Dest
Assert-Sha $Dest
# Belt-and-suspenders: warn if the downloaded exe isn't validly Authenticode-
# signed by SumatraPDF (an unsigned second exe in our installer trips AV).
try {
    $sig = Get-AuthenticodeSignature $Dest
    if ($sig.Status -ne "Valid") {
        Write-Warning "SumatraPDF Authenticode status is '$($sig.Status)' (expected Valid)."
    } else {
        Write-Host "Authenticode: Valid ($($sig.SignerCertificate.Subject))"
    }
} catch {
    Write-Warning "Could not check Authenticode signature: $_"
}
Write-Host "Done."
