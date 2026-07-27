#Requires -Version 5.1
<#
.SYNOPSIS
  Tag open GitHub issues for pipeline triage (idempotent).
  Default: all open issues that already have label 'review' or numbers 3-11.
#>
param(
  [string]$Repo = "",
  [int[]]$Numbers = @(),
  [switch]$AllOpen
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }

if ($Numbers.Count -eq 0 -and -not $AllOpen) {
  $Numbers = 3..11
}

if ($AllOpen) {
  $issues = gh issue list --repo $Repo --state open --limit 100 --json number | ConvertFrom-Json
  $Numbers = @($issues | ForEach-Object { [int]$_.number })
}

Write-Host "Importing backlog on $Repo : $($Numbers -join ', ')"
foreach ($n in $Numbers) {
  $view = gh issue view $n --repo $Repo --json state,labels,title 2>$null | ConvertFrom-Json
  if (-not $view) {
    Write-Warning "Skip #$n (not found)"
    continue
  }
  if ($view.state -ne "OPEN") {
    Write-Host "Skip #$n (not open)"
    continue
  }
  $labels = @($view.labels | ForEach-Object { $_.name })
  if ($labels -contains "status:merged" -or $labels -contains "status:rejected") {
    Write-Host "Skip #$n (terminal status)"
    continue
  }
  if ($labels -notcontains "pipeline") {
    gh issue edit $n --repo $Repo --add-label pipeline | Out-Null
  }
  $hasStatus = $labels | Where-Object { $_ -like "status:*" }
  if (-not $hasStatus) {
    gh issue edit $n --repo $Repo --add-label "status:needs-triage" | Out-Null
    Write-Host "  #$n -> status:needs-triage ($($view.title))"
  } else {
    Write-Host "  #$n keep status ($($hasStatus -join ','))"
  }
  if ($labels -notcontains "agent:finder" -and $labels -contains "review") {
    gh issue edit $n --repo $Repo --add-label "agent:finder" 2>$null | Out-Null
  }
}
Write-Host "Import complete."
