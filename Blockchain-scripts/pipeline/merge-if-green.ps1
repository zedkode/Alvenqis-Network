#Requires -Version 5.1
<#
.SYNOPSIS
  Wait for PR checks (or accept local QA), then squash-merge to main.
#>
param(
  [Parameter(Mandatory = $true)][int]$Pr,
  [int]$Issue = 0,
  [string]$Repo = "",
  [switch]$SkipChecks,
  [switch]$LocalChecksOk,
  [switch]$AllowCritical,
  [int]$WatchSeconds = 1800
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }

$prView = gh pr view $Pr --repo $Repo --json title,labels,commits,baseRefName,headRefName,state,url | ConvertFrom-Json
if ($prView.state -ne "OPEN") {
  throw "PR #$Pr is not OPEN (state=$($prView.state))"
}
if ($prView.baseRefName -ne "main") {
  throw "PR #$Pr base is $($prView.baseRefName), expected main"
}

# Critical gate via linked issue labels
if ($Issue -gt 0) {
  $labs = Get-IssueLabels -Number $Issue -Repo $Repo
  if ($labs -contains "sev:critical" -and -not $AllowCritical) {
    throw "Issue #$Issue is sev:critical. Re-run with -AllowCritical after human approval."
  }
}

if (-not $SkipChecks) {
  Write-Host "Waiting for PR checks on #$Pr ..."
  gh pr checks $Pr --repo $Repo --watch --fail-fast 2>&1 | Write-Host
  $checkExit = $LASTEXITCODE
  if ($checkExit -ne 0) {
    # No checks configured often returns non-zero — allow local override
    if ($LocalChecksOk) {
      Write-Warning "PR checks failed or missing; proceeding because -LocalChecksOk was set."
      Invoke-Gh pr comment $Pr --repo $Repo --body "Integrator: CI checks unavailable/failed; merging based on local ``run-checks.ps1`` (LocalChecksOk)."
    } else {
      throw "PR checks not green for #$Pr (exit $checkExit). Re-run with -LocalChecksOk only after local run-checks passes."
    }
  }
} elseif (-not $LocalChecksOk) {
  throw "Refusing blind merge: pass -LocalChecksOk when using -SkipChecks."
}

Invoke-Gh pr merge $Pr --repo $Repo --squash --delete-branch
Write-Host "Merged PR #$Pr"

if ($Issue -gt 0) {
  Set-StatusLabel -Number $Issue -Status "status:merged" -Repo $Repo
  gh issue edit $Issue --repo $Repo --add-label "agent:integrator" 2>$null | Out-Null
  # Closes # should close; force close if still open
  $st = gh issue view $Issue --repo $Repo --json state | ConvertFrom-Json
  if ($st.state -eq "OPEN") {
    Invoke-Gh issue close $Issue --repo $Repo --reason completed
  }
  Invoke-Gh issue comment $Issue --repo $Repo --body "Integrator: squash-merged PR #$Pr to ``main``."
}

Write-Host "Done. Fetch origin/main on your machine when ready."
