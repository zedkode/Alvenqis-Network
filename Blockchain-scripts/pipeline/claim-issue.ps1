#Requires -Version 5.1
<#
.SYNOPSIS
  Atomically claim a ready issue for a fixer (ready -> in-progress).
#>
param(
  [Parameter(Mandatory = $true)][int]$Number,
  [string]$Repo = "",
  [string]$Agent = "fixer"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }

$labels = Get-IssueLabels -Number $Number -Repo $Repo
if ($labels -notcontains "status:ready") {
  throw "Issue #$Number is not status:ready (have: $($labels -join ', ')). Cannot claim."
}

Set-StatusLabel -Number $Number -Status "status:in-progress" -Repo $Repo
gh issue edit $Number --repo $Repo --add-label "agent:fixer" 2>$null | Out-Null
Invoke-Gh issue comment $Number --repo $Repo --body "Claimed by agent ``$Agent`` for implementation from ``origin/main``."
Write-Host "Claimed #$Number -> status:in-progress"
