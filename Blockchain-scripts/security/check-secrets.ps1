$ErrorActionPreference = "Stop"

if ($args -contains "--help" -or $args -contains "-h" -or $args -contains "-Help") {
  Write-Host "Usage: Blockchain-scripts/security/check-secrets.ps1"
  Write-Host "Fails when forbidden secret files or non-placeholder secret patterns are found in the repository."
  exit 0
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$allowedPlaceholderPattern = '(?i)(CHANGE_ME|example|replace_|generated_|youshallnotpass|localhost|127\.0\.0\.1|^\$|^\$\(|^<.+>$)'
$secretAssignmentPattern = '^\s*(?:export\s+)?(?:[A-Z0-9_]*(?:PRIVATE_KEY|WALLET_SEED|MNEMONIC|API_TOKEN|GITHUB_TOKEN|SECRET|PASSWORD|RPC_PASSWORD|ADMIN_TOKEN))\s*=\s*(?<value>[^\s#]*)'
$credentialPatterns = @(
  '(?i)\bgh[pousr]_[A-Za-z0-9]{20,}\b',
  '\bAKIA[0-9A-Z]{16}\b',
  '(?i)-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'
)
$forbiddenExtensions = @(".key", ".pem", ".seed", ".wallet", ".mnemonic")
$selfRuleFiles = @(
  "Blockchain-scripts/git/check-forbidden-files.ps1",
  "Blockchain-scripts/git/check-forbidden-files.sh",
  "Blockchain-scripts/security/check-secrets.ps1",
  "Blockchain-scripts/security/check-secrets.sh",
  "Blockchain-scripts/security/check-config-safety.ps1",
  "Blockchain-scripts/security/check-config-safety.sh"
)
$issues = New-Object System.Collections.Generic.List[string]
$candidateFiles = @()
$candidateFiles += git ls-files
$candidateFiles += git ls-files --others --exclude-standard
$candidateFiles = $candidateFiles | Where-Object { $_ } | Sort-Object -Unique

$badEnvFiles = $candidateFiles | Where-Object {
  (Split-Path $_ -Leaf) -like ".env*" -and (Split-Path $_ -Leaf) -ne ".env.example"
}
foreach ($file in $badEnvFiles) {
  $issues.Add("Forbidden tracked or unignored environment file: $file")
}

foreach ($relativePath in $candidateFiles) {
  if ($forbiddenExtensions -contains [System.IO.Path]::GetExtension($relativePath)) {
    $issues.Add("Forbidden tracked or unignored secret or wallet file: $relativePath")
  }
}

$candidateFiles = $candidateFiles |
  Where-Object { $_ -and (Split-Path $_ -Leaf) -ne ".env.example" -and ($selfRuleFiles -notcontains ($_ -replace '\\', '/')) } |
  Sort-Object -Unique

foreach ($relativePath in $candidateFiles) {
  $fullPath = Join-Path $repoRoot $relativePath
  if (-not (Test-Path $fullPath -PathType Leaf)) {
    continue
  }

  $lineNumber = 0
  foreach ($line in Get-Content -LiteralPath $fullPath -ErrorAction SilentlyContinue) {
    $lineNumber++
    if ($line -cmatch $secretAssignmentPattern) {
      $value = $Matches['value'].Trim('"', "'")
      if (-not $value -or $value -match $allowedPlaceholderPattern) {
        continue
      }
      $issues.Add("Non-placeholder secret assignment found in ${relativePath}:${lineNumber}")
    }
    foreach ($pattern in $credentialPatterns) {
      if ($line -match $pattern) {
        $issues.Add("Credential-like value found in ${relativePath}:${lineNumber}")
      }
    }
  }
}

if ($issues.Count -gt 0) {
  Write-Error ("Secret scan failed:`n- " + ($issues -join "`n- "))
  exit 1
}

Write-Host "Secret scan passed."
