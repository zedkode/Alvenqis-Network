#Requires -Version 5.1
<#
.SYNOPSIS
  Apply triage decision to a GitHub issue after verifying against GitHub main.
#>
param(
  [Parameter(Mandatory = $true)][int]$Number,
  [Parameter(Mandatory = $true)]
  [ValidateSet("confirm", "reject", "duplicate")]
  [string]$Decision,
  [ValidateSet("critical", "high", "medium", "low", "info")]
  [string]$Severity = "medium",
  [string[]]$AreaLabels = @(),
  [string]$Comment = "",
  [string]$DuplicateOf = "",
  [string]$MirrorSha = "",
  [string]$Repo = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }

$shaNote = if ($MirrorSha) { "Verified against ``origin/main`` @ ``$MirrorSha``." } else { "Verified against GitHub main (see triage comment)." }

switch ($Decision) {
  "confirm" {
    Set-StatusLabel -Number $Number -Status "status:ready" -Repo $Repo
    gh issue edit $Number --repo $Repo --add-label "agent:triage" 2>$null | Out-Null
    gh issue edit $Number --repo $Repo --add-label "sev:$Severity" 2>$null | Out-Null
    foreach ($a in $AreaLabels) {
      if ($a) { gh issue edit $Number --repo $Repo --add-label $a 2>$null | Out-Null }
    }
    $body = @"
## TRIAGE: confirmed

$shaNote

Severity: ``sev:$Severity``
Areas: $($AreaLabels -join ', ')

$Comment
"@
    Invoke-Gh issue comment $Number --repo $Repo --body $body
    Write-Host "Confirmed #$Number as status:ready sev:$Severity"
  }
  "reject" {
    Set-StatusLabel -Number $Number -Status "status:rejected" -Repo $Repo
    gh issue edit $Number --repo $Repo --add-label "agent:triage" 2>$null | Out-Null
    $body = @"
## TRIAGE: rejected

$shaNote

$Comment
"@
    Invoke-Gh issue comment $Number --repo $Repo --body $body
    Invoke-Gh issue close $Number --repo $Repo --reason "not planned"
    Write-Host "Rejected and closed #$Number"
  }
  "duplicate" {
    if (-not $DuplicateOf) { throw "-DuplicateOf required for duplicate decision" }
    Set-StatusLabel -Number $Number -Status "status:rejected" -Repo $Repo
    gh issue edit $Number --repo $Repo --add-label "agent:triage" 2>$null | Out-Null
    gh issue edit $Number --repo $Repo --add-label "duplicate" 2>$null | Out-Null
    $body = @"
## TRIAGE: duplicate

Duplicate of #$DuplicateOf

$shaNote

$Comment
"@
    Invoke-Gh issue comment $Number --repo $Repo --body $body
    Invoke-Gh issue close $Number --repo $Repo --reason "not planned"
    Write-Host "Closed #$Number as duplicate of #$DuplicateOf"
  }
}
