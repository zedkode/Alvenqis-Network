# Regenerates Tauri app icons from the project logo.
# Usage: .\scripts\generate-icons.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
# The root logo is the single canonical brand master.
$Master = Join-Path $Root "logo.png"
if (-not (Test-Path $Master)) {
  throw "Missing canonical logo.png at $Root"
}

Write-Host "==> Generating Tauri icons from master logo (no pixel edits to master)"
Push-Location $Root
try {
  npx tauri icon $Master
  # Restore exact master into public assets (tauri icon may rewrite intermediate sizes only).
  Copy-Item -LiteralPath $Master (Join-Path $Root "public\logo.png") -Force
  Copy-Item -LiteralPath $Master (Join-Path $Root "public\logo-mark.png") -Force
  Copy-Item -LiteralPath $Master (Join-Path $Root "public\icon.png") -Force
  Copy-Item -LiteralPath $Master (Join-Path $Root "logo.png") -Force
  Copy-Item -LiteralPath $Master (Join-Path $Root "src-tauri\icons\logo-source.png") -Force
  Write-Host "Updated public logos from master and src-tauri/icons/*"
}
finally {
  Pop-Location
}
