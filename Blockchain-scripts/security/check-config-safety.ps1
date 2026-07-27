$ErrorActionPreference = "Stop"

if ($args -contains "--help" -or $args -contains "-h" -or $args -contains "-Help") {
  Write-Host "Usage: Blockchain-scripts/security/check-config-safety.ps1"
  Write-Host "Fails when repository config files expose unsafe RPC settings, devnet data paths in mainnet-candidate configs, reset flags, secrets, or local wallet material."
  exit 0
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$allowedPlaceholderPattern = '(?i)(CHANGE_ME|example|localhost|127\.0\.0\.1)'
$secretPatterns = @(
  "PRIVATE_KEY=",
  "WALLET_SEED=",
  "MNEMONIC=",
  "API_TOKEN=",
  "GITHUB_TOKEN=",
  "SECRET=",
  "PASSWORD=",
  "RPC_PASSWORD=",
  "ADMIN_TOKEN="
)
$issues = New-Object System.Collections.Generic.List[string]

$configFiles = @()
$configRoots = @(
  (Join-Path $repoRoot "configs"),
  (Join-Path $RepoRoot "Blockchain-prototype\alvenqis-rpc-gateway\config"),
  (Join-Path $RepoRoot "Blockchain-prototype\alvenqis-devnet\config")
)
foreach ($root in $configRoots) {
  if (Test-Path $root) {
    $configFiles += Get-ChildItem -Path $root -Recurse -Force -File -Include *.toml
  }
}

foreach ($file in $configFiles | Sort-Object FullName -Unique) {
  $content = Get-Content $file.FullName -Raw
  $relativePath = $file.FullName.Substring($repoRoot.Path.Length + 1)

  if ($content -match '(?m)^\s*bind_host\s*=\s*"0\.0\.0\.0"' -and $content -notmatch '(?m)^\s*public_rpc_allowed\s*=\s*true') {
    $issues.Add("Unsafe RPC bind without public opt-in: $relativePath")
  }
  if ($file.Name -like "*mainnet-candidate*.toml" -and $content -match '\.alvenqis-dev') {
    $issues.Add("Mainnet-candidate config uses devnet data path: $relativePath")
  }
  if ($file.Name -like "*mainnet-candidate*.toml" -and $content -match '(?im)^\s*(allow_reset|reset)\s*=\s*true\s*$') {
    $issues.Add("Mainnet-candidate config enables reset-like behavior: $relativePath")
  }

  foreach ($pattern in $secretPatterns) {
    $matches = Select-String -Path $file.FullName -SimpleMatch $pattern -ErrorAction SilentlyContinue
    foreach ($match in $matches) {
      if ($match.Line -match $allowedPlaceholderPattern) {
        continue
      }
      $issues.Add("Secret pattern '$pattern' found in config ${relativePath}:$($match.LineNumber)")
    }
  }
}

$walletMaterial = Get-ChildItem -Path $repoRoot -Recurse -Force -File -ErrorAction SilentlyContinue | Where-Object {
  ($_.FullName -notmatch '\\alvenqis-wallet\\') -and
  ($_.FullName -match '\\wallets\\' -or $_.Extension -in @('.wallet', '.seed', '.key', '.pem'))
}
foreach ($file in $walletMaterial) {
  $issues.Add("Wallet material inside repository tree: $($file.FullName)")
}

if ($issues.Count -gt 0) {
  Write-Error ("Config safety check failed:`n- " + ($issues -join "`n- "))
  exit 1
}

Write-Host "Config safety check passed."
