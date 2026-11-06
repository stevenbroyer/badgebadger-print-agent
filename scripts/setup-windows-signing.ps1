# Prepares Windows code signing for a CI build through Azure Artifact
# Signing (Microsoft's signing service, formerly Trusted Signing). Run it
# before `pnpm tauri build`. Without the settings below it does nothing
# and the build comes out unsigned, as before.
#
# Repo secrets, from an Entra app registration that holds only the
# "Artifact Signing Certificate Profile Signer" role:
#   AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET
# Repo variables:
#   ARTIFACT_SIGNING_ENDPOINT  the account's region endpoint, e.g. https://eus.codesigning.azure.net/
#   ARTIFACT_SIGNING_ACCOUNT   the Artifact Signing account name
#   ARTIFACT_SIGNING_PROFILE   its Public Trust certificate profile name
#
# When signing is on it appends to $GITHUB_ENV:
#   WINDOWS_SIGNED=1, SIGNTOOL, ARTIFACT_SIGNING_DLIB,
#   ARTIFACT_SIGNING_METADATA, and TAURI_SIGN_CONFIG, a `--config` overlay
#   that makes Tauri call scripts/sign-windows.ps1 for every file it signs.

$ErrorActionPreference = "Stop"

# Microsoft's signing client: the dlib that signtool loads. Pinned like
# SumatraPDF, so bump it on purpose and update the hash. 1.0.146 is the last
# release built on .NET 8, which the runner image already has; later ones
# need .NET 10.
$ClientVersion = "1.0.146"
$ClientSha = "7761e5d8f31e54b764da21d33cd8b0a2940e1b05526c7dfcb5e94b0431426f70"

$required = @(
    "AZURE_TENANT_ID", "AZURE_CLIENT_ID", "AZURE_CLIENT_SECRET",
    "ARTIFACT_SIGNING_ENDPOINT", "ARTIFACT_SIGNING_ACCOUNT", "ARTIFACT_SIGNING_PROFILE"
)
$missing = @($required | Where-Object { -not [Environment]::GetEnvironmentVariable($_) })
if ($missing.Count -eq $required.Count) {
    Write-Host "No Windows signing settings, so the build will be unsigned."
    exit 0
}
if ($missing.Count -gt 0) {
    throw "Windows signing is half set up. Missing: $($missing -join ', ')"
}

$root = Join-Path $env:RUNNER_TEMP "artifact-signing"
New-Item -ItemType Directory -Force -Path $root | Out-Null

# Expand-Archive only takes a .zip name; a .nupkg is a zip.
$package = Join-Path $root "client.zip"
$url = "https://api.nuget.org/v3-flatcontainer/microsoft.artifactsigning.client/$ClientVersion/microsoft.artifactsigning.client.$ClientVersion.nupkg"
Invoke-WebRequest -Uri $url -OutFile $package
$actual = (Get-FileHash -Algorithm SHA256 -Path $package).Hash.ToLower()
if ($actual -ne $ClientSha) {
    throw "Artifact Signing client SHA-256 mismatch: expected $ClientSha, got $actual"
}
$client = Join-Path $root "client"
Expand-Archive -Path $package -DestinationPath $client -Force
$dlib = Join-Path $client "bin\x64\Azure.CodeSigning.Dlib.dll"
if (-not (Test-Path $dlib)) { throw "The client package has no bin\x64\Azure.CodeSigning.Dlib.dll" }

# The newest x64 signtool from the Windows SDK. Tauri's docs recommend
# 10.0.26100 or later for signing through the dlib.
$kits = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
$signtool = Get-ChildItem -Path $kits -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.Directory.Name -eq "x64" -and $_.Directory.Parent.Name -match '^\d+(\.\d+){3}$' } |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending |
    Select-Object -First 1
if (-not $signtool) { throw "No x64 signtool.exe under $kits" }
Write-Host "signtool: $($signtool.FullName)"

$metadata = Join-Path $root "metadata.json"
@{
    Endpoint               = $env:ARTIFACT_SIGNING_ENDPOINT
    CodeSigningAccountName = $env:ARTIFACT_SIGNING_ACCOUNT
    CertificateProfileName = $env:ARTIFACT_SIGNING_PROFILE
} | ConvertTo-Json | Set-Content -Path $metadata -Encoding utf8

# Object form, so a path with spaces stays one argument. Tauri swaps %1 for
# each file: the app, the NSIS uninstaller and installer, and the MSI.
$signScript = Join-Path $PSScriptRoot "sign-windows.ps1"
$overlay = Join-Path $root "tauri.sign.json"
@{
    bundle = @{
        windows = @{
            signCommand = @{
                cmd  = "pwsh"
                args = @("-NoProfile", "-NonInteractive", "-File", $signScript, "%1")
            }
        }
    }
} | ConvertTo-Json -Depth 6 | Set-Content -Path $overlay -Encoding utf8

@(
    "WINDOWS_SIGNED=1"
    "SIGNTOOL=$($signtool.FullName)"
    "ARTIFACT_SIGNING_DLIB=$dlib"
    "ARTIFACT_SIGNING_METADATA=$metadata"
    "TAURI_SIGN_CONFIG=$overlay"
) | Add-Content -Path $env:GITHUB_ENV
Write-Host "Windows signing is on: account $env:ARTIFACT_SIGNING_ACCOUNT, profile $env:ARTIFACT_SIGNING_PROFILE."
