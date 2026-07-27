#Requires -Version 5.1
<#
.SYNOPSIS
  Run validation checks inside a clean worktree (PR head or main mirror).
#>
param(
  [Parameter(Mandatory = $true)][string]$Worktree,
  [switch]$SkipCargo,
  [switch]$SkipSecurity,
  [switch]$Quick,
  [string[]]$CargoPackages = @()
)

$ErrorActionPreference = "Continue"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not (Test-Path $Worktree)) { throw "Worktree not found: $Worktree" }

$cargo = Get-CargoCommand
$results = @()

function Run-One {
  param([string]$Name, [scriptblock]$Block)
  Write-Host "`n=== $Name ===" -ForegroundColor Cyan
  Push-Location $Worktree
  try {
    $out = & $Block 2>&1 | Out-String
    $ok = ($LASTEXITCODE -eq 0 -or $null -eq $LASTEXITCODE)
    # PowerShell native success
    if (-not $?) { $ok = $false }
    [pscustomobject]@{ name = $Name; ok = $ok; output = $out.Trim() }
  } catch {
    [pscustomobject]@{ name = $Name; ok = $false; output = "$_" }
  } finally {
    Pop-Location
  }
}

if (-not $SkipSecurity) {
  $sec = Join-Path $Worktree "scripts\security"
  if (Test-Path (Join-Path $sec "check-secrets.ps1")) {
    $results += Run-One "security/check-secrets" { powershell -NoProfile -File ".\scripts\security\check-secrets.ps1" }
  }
  if (Test-Path (Join-Path $sec "check-config-safety.ps1")) {
    $results += Run-One "security/check-config-safety" { powershell -NoProfile -File ".\scripts\security\check-config-safety.ps1" }
  }
  if (Test-Path (Join-Path $sec "check-repo-hygiene.ps1")) {
    $results += Run-One "security/check-repo-hygiene" { powershell -NoProfile -File ".\scripts\security\check-repo-hygiene.ps1" }
  }
}

if (-not $SkipCargo) {
  $exe = $cargo.Exe
  $pre = $cargo.Prefix
  if ($Quick) {
    $results += Run-One "cargo check" {
      if ($CargoPackages.Count -gt 0) {
        foreach ($p in $CargoPackages) { & $exe @pre check -p $p; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE } }
      } else {
        & $exe @pre check --workspace
      }
    }
  } else {
    $results += Run-One "cargo fmt --check" { & $exe @pre fmt --all --check }
    if ($CargoPackages.Count -gt 0) {
      $results += Run-One "cargo test (packages)" {
        foreach ($p in $CargoPackages) {
          & $exe @pre test -p $p
          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        }
      }
      $results += Run-One "cargo clippy (packages)" {
        foreach ($p in $CargoPackages) {
          & $exe @pre clippy -p $p --all-targets -- -D warnings
          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        }
      }
    } else {
      $results += Run-One "cargo test --workspace" { & $exe @pre test --workspace --tests }
      $results += Run-One "cargo clippy" { & $exe @pre clippy --workspace --all-targets -- -D warnings }
    }
  }
}

$failed = @($results | Where-Object { -not $_.ok })
$passed = @($results | Where-Object { $_.ok })
Write-Host "`nPassed: $($passed.Count)  Failed: $($failed.Count)"
$results | ForEach-Object {
  $mark = if ($_.ok) { "PASS" } else { "FAIL" }
  Write-Host ("  [{0}] {1}" -f $mark, $_.name)
}

$outPath = Join-Path $Worktree ".pipeline-checks.json"
# Prefer writing next to worktree parent if worktree is clean-only
try {
  $results | Select-Object name, ok | ConvertTo-Json | Set-Content -Path $outPath -Encoding UTF8
} catch {}

if ($failed.Count -gt 0) { exit 1 }
exit 0
