# Tauri's signCommand on CI (set up by setup-windows-signing.ps1). Tauri runs
# it once per file it signs. It signs through Azure Artifact Signing with a
# timestamp, which keeps the signature valid after the three-day
# certificate expires, then has signtool check the result.
#
# Needs SIGNTOOL, ARTIFACT_SIGNING_DLIB and ARTIFACT_SIGNING_METADATA, plus
# the AZURE_* credentials the dlib reads from the environment.

param([Parameter(Mandatory = $true)][string]$File)

$ErrorActionPreference = "Stop"

foreach ($name in "SIGNTOOL", "ARTIFACT_SIGNING_DLIB", "ARTIFACT_SIGNING_METADATA") {
    if (-not [Environment]::GetEnvironmentVariable($name)) {
        throw "$name isn't set; run setup-windows-signing.ps1 first"
    }
}

$signArgs = @(
    "sign", "/v", "/fd", "SHA256",
    "/tr", "http://timestamp.acs.microsoft.com", "/td", "SHA256",
    "/d", "BadgeBadger Print Agent", "/du", "https://badgebadger.app",
    "/dlib", $env:ARTIFACT_SIGNING_DLIB, "/dmdf", $env:ARTIFACT_SIGNING_METADATA,
    $File
)

# The signing and timestamp services drop the odd request, so retry.
for ($attempt = 1; ; $attempt++) {
    & $env:SIGNTOOL @signArgs
    if ($LASTEXITCODE -eq 0) { break }
    if ($attempt -ge 3) { throw "signtool couldn't sign $File after $attempt tries" }
    Write-Warning "signtool couldn't sign $File (try $attempt of 3); retrying"
    Start-Sleep -Seconds (15 * $attempt)
}

& $env:SIGNTOOL verify /pa $File
if ($LASTEXITCODE -ne 0) { throw "signtool verify failed on $File" }
