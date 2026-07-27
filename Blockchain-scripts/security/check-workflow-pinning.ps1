$ErrorActionPreference = "Stop"

if ($args -contains "--help" -or $args -contains "-h" -or $args -contains "-Help") {
  Write-Host "Usage: Blockchain-scripts/security/check-workflow-pinning.ps1"
  Write-Host "Fails when a third-party GitHub Action is not pinned to a full commit SHA."
  exit 0
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$workflowRoot = Join-Path $repoRoot ".github\workflows"
$issues = New-Object System.Collections.Generic.List[string]

Get-ChildItem -LiteralPath $workflowRoot -File | Where-Object { $_.Extension -in @(".yml", ".yaml") } | ForEach-Object {
  $workflow = $_
  $lineNumber = 0
  Get-Content -LiteralPath $workflow.FullName | ForEach-Object {
    $lineNumber++
    if ($_ -match '^\s*-?\s*uses:\s*([^#\s]+)') {
      $reference = $Matches[1]
      if (-not $reference.StartsWith("./") -and $reference -notmatch '@[0-9a-fA-F]{40}$') {
        $issues.Add("$($workflow.Name):${lineNumber}: unpinned action $reference")
      }
    }
  }
}

if ($issues.Count -gt 0) {
  Write-Error ("GitHub Actions pinning check failed:`n- " + ($issues -join "`n- "))
  exit 1
}

Write-Host "GitHub Actions pinning check passed."
