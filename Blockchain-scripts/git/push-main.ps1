$ErrorActionPreference = "Stop"

& "$PSScriptRoot\check-forbidden-files.ps1"
& (Join-Path $PSScriptRoot "..\release\check-release.ps1")

git add .
$status = git status --short
if (-not $status) {
  Write-Host "No changes to commit."
  exit 0
}

$timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
git commit -m "main: mainnet-candidate update $timestamp"
git push origin main
