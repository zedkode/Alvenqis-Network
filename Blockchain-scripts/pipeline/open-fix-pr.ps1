#Requires -Version 5.1
<#
.SYNOPSIS
  Push current branch from a fix worktree and open a PR that closes the issue.
#>
param(
  [Parameter(Mandatory = $true)][int]$Number,
  [Parameter(Mandatory = $true)][string]$Branch,
  [Parameter(Mandatory = $true)][string]$Worktree,
  [string]$Title = "",
  [string]$BodyFile = "",
  [string]$Repo = "",
  [switch]$Draft
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }
if (-not (Test-Path $Worktree)) { throw "Worktree not found: $Worktree" }

$issue = gh issue view $Number --repo $Repo --json title | ConvertFrom-Json
if (-not $Title) {
  $Title = "fix(#$Number): $($issue.title -replace '^\[review\]\s*','')"
}

Push-Location $Worktree
try {
  $current = (git rev-parse --abbrev-ref HEAD).Trim()
  if ($current -ne $Branch) {
    git checkout -B $Branch
  }
  git push -u origin $Branch
  if ($LASTEXITCODE -ne 0) { throw "git push failed" }
} finally {
  Pop-Location
}

if (-not $BodyFile) {
  $tmp = Join-Path $env:TEMP "alvenqis-pr-$Number.md"
  @"
## Summary
Implements fix for #$Number.

Closes #$Number

## Agent trail
- Fixer: branch ``$Branch`` from ``origin/main``
- QA: pending

## Test plan
- [ ] Targeted checks via ``scripts/pipeline/run-checks.ps1``
- [ ] CI green on PR
"@ | Set-Content -Path $tmp -Encoding UTF8
  $BodyFile = $tmp
}

$draftArgs = @()
if ($Draft) { $draftArgs = @("--draft") }

$url = gh pr create --repo $Repo --base main --head $Branch --title $Title --body-file $BodyFile @draftArgs
if ($LASTEXITCODE -ne 0) { throw "gh pr create failed" }

Set-StatusLabel -Number $Number -Status "status:in-qa" -Repo $Repo
Invoke-Gh issue comment $Number --repo $Repo --body "Fix PR opened: $url"
Write-Host $url
return $url
