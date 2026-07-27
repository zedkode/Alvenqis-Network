# Manual rotation for alvenqis-mining-pool admin_token_file.
# The pool fails closed when the token file mtime exceeds admin_token_max_age_seconds
# (default 90 days). Operators must rotate deliberately — there is no auto-renewal.
#
# Usage:
#   .\Blockchain-scripts\operator\rotate-pool-admin-token.ps1 -TokenFile path\to\admin.token

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$TokenFile
)

$ErrorActionPreference = "Stop"
$dir = Split-Path -Parent $TokenFile
if ($dir -and -not (Test-Path $dir)) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
}

# 32 random bytes as hex (64 chars)
$bytes = New-Object byte[] 32
[System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
$token = ($bytes | ForEach-Object { $_.ToString("x2") }) -join ""
Set-Content -Path $TokenFile -Value $token -NoNewline -Encoding ascii
Write-Host "Wrote new admin token to $TokenFile"
Write-Host "Restart alvenqis-mining-pool so require_admin loads the new token."
Write-Host "Store the token only in your secrets manager; do not commit it."
