#Requires -Version 5.1
param(
  [Parameter(Mandatory = $true)][int]$Pr,
  [Parameter(Mandatory = $true)][string]$Body,
  [switch]$Approved,
  [int]$Issue = 0,
  [string]$Repo = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }

$prefix = if ($Approved) { "QA: APPROVED" } else { "QA: CHANGES REQUESTED" }
$full = @"
## $prefix

$Body
"@

Invoke-Gh pr comment $Pr --repo $Repo --body $full

if ($Issue -gt 0) {
  gh issue edit $Issue --repo $Repo --add-label "agent:qa" 2>$null | Out-Null
  if (-not $Approved) {
    Set-StatusLabel -Number $Issue -Status "status:in-progress" -Repo $Repo
    Invoke-Gh issue comment $Issue --repo $Repo --body $full
  }
}

Write-Host "$prefix on PR #$Pr"
