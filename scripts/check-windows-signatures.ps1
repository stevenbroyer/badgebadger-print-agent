# Fails unless every file given is Authenticode-signed, timestamped and
# chains to a trusted root. When WINDOWS_PUBLISHER is set (the certificate's
# CN, e.g. "Jane Q Doe"), the signer must match it too.
#
# Usage: pwsh scripts/check-windows-signatures.ps1 <file> [<file> ...]

param([Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)][string[]]$Files)

$ErrorActionPreference = "Stop"

$failed = 0
foreach ($file in $Files) {
    $sig = Get-AuthenticodeSignature -FilePath $file
    $subject = if ($sig.SignerCertificate) { $sig.SignerCertificate.Subject } else { "(no signer)" }
    $problems = @()
    if ($sig.Status -ne "Valid") { $problems += "status $($sig.Status): $($sig.StatusMessage)" }
    if (-not $sig.TimeStamperCertificate) { $problems += "not timestamped" }
    if ($env:WINDOWS_PUBLISHER) {
        $cn = "CN=$([regex]::Escape($env:WINDOWS_PUBLISHER))"
        if ($subject -notmatch "(^|, )$cn(,|$)") {
            $problems += "signed by '$subject', expected CN=$env:WINDOWS_PUBLISHER"
        }
    }
    if ($problems.Count -gt 0) {
        $failed++
        Write-Host "FAIL $file"
        $problems | ForEach-Object { Write-Host "     $_" }
    } else {
        Write-Host "ok   $file"
        Write-Host "     $subject"
    }
}
if ($failed -gt 0) {
    throw "$failed of $($Files.Count) files failed the signature check"
}
