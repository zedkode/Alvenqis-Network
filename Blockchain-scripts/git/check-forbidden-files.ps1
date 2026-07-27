$ErrorActionPreference = "Stop"

$forbiddenNames = @(".env", ".env.local", ".env.production")
$forbiddenExtensions = @(".key", ".pem", ".seed", ".wallet")
$forbiddenTrackedPatterns = @(".alvenqis-dev/", ".alvenqis-testnet/", ".alvenqis-mainnet/", "target/", "node_modules/")
$secretPatterns = @(
  "PRIVATE_KEY=",
  "WALLET_SEED=",
  "API_TOKEN=",
  "GITHUB_TOKEN=",
  "SECRET=",
  "PASSWORD=",
  "MNEMONIC="
)

$candidateFiles = @()
$candidateFiles += git ls-files
$candidateFiles += git ls-files --others --exclude-standard
$candidateFiles = $candidateFiles | Where-Object { $_ } | Sort-Object -Unique
$badEnvFiles = $candidateFiles | Where-Object {
  (Split-Path $_ -Leaf) -like ".env*" -and (Split-Path $_ -Leaf) -ne ".env.example"
}
if ($badEnvFiles) {
  throw "Forbidden tracked or unignored .env files found: $($badEnvFiles -join ', ')"
}

$badKeyFiles = Get-ChildItem -Recurse -Force -File | Where-Object {
  $forbiddenExtensions -contains $_.Extension
}
if ($badKeyFiles) {
  throw "Forbidden key or wallet files found: $($badKeyFiles.FullName -join ', ')"
}

$tracked = git ls-files
foreach ($pattern in $forbiddenTrackedPatterns) {
  if ($tracked | Select-String -SimpleMatch $pattern -Quiet) {
    throw "Tracked files contain forbidden pattern: $pattern"
  }
}

& (Join-Path $PSScriptRoot "..\security\check-secrets.ps1")
if ($LASTEXITCODE -ne 0) {
  throw "Secret scan failed."
}

Write-Host "Forbidden file check passed."
