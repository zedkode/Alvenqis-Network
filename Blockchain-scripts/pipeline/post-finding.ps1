#Requires -Version 5.1
<#
.SYNOPSIS
  Create a GitHub issue from a finder agent (body file preferred).
#>
param(
  [Parameter(Mandatory = $true)][string]$Title,
  [Parameter(Mandatory = $true)][string]$BodyFile,
  [string]$Repo = "",
  [string]$Commit = "",
  [string[]]$ExtraLabels = @()
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }
if (-not (Test-Path $BodyFile)) { throw "Body file not found: $BodyFile" }

$labels = @("pipeline", "agent:finder", "status:needs-triage", "review") + $ExtraLabels
$labelArgs = @()
foreach ($l in ($labels | Select-Object -Unique)) {
  $labelArgs += @("--label", $l)
}

$url = gh issue create --repo $Repo --title $Title --body-file $BodyFile @labelArgs
if ($LASTEXITCODE -ne 0) { throw "gh issue create failed" }
Write-Host $url
if ($Commit) {
  $num = if ($url -match '/issues/(\d+)') { $Matches[1] } else { $null }
  if ($num) {
    gh issue comment $num --repo $Repo --body "Finder mirror commit: ``$Commit``" | Out-Null
  }
}
return $url
