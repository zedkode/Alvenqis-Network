#Requires -Version 5.1
<#
.SYNOPSIS
  Create pipeline labels on the GitHub repo (idempotent).
#>
param(
  [string]$Repo = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }

$labels = @(
  @{ n = "pipeline"; c = "0E8A16"; d = "Multi-agent pipeline tracked" }
  @{ n = "agent:finder"; c = "1D76DB"; d = "Opened by finder agent" }
  @{ n = "agent:triage"; c = "5319E7"; d = "Processed by triage agent" }
  @{ n = "agent:fixer"; c = "FBCA04"; d = "Touched by fixer agent" }
  @{ n = "agent:qa"; c = "BFDADC"; d = "Touched by QA agent" }
  @{ n = "agent:integrator"; c = "D4C5F9"; d = "Merged by integrator" }
  @{ n = "status:needs-triage"; c = "F9D0C4"; d = "Awaiting triage vs GitHub main" }
  @{ n = "status:ready"; c = "0E8A16"; d = "Confirmed; ready to fix" }
  @{ n = "status:in-progress"; c = "FBCA04"; d = "Fixer claimed" }
  @{ n = "status:in-qa"; c = "1D76DB"; d = "PR open / QA" }
  @{ n = "status:merged"; c = "0E8A16"; d = "Merged to main" }
  @{ n = "status:rejected"; c = "B60205"; d = "Rejected / stale / duplicate" }
  @{ n = "status:blocked"; c = "E99695"; d = "Needs human / design" }
  @{ n = "sev:critical"; c = "B60205"; d = "Critical severity" }
  @{ n = "sev:high"; c = "D93F0B"; d = "High severity" }
  @{ n = "sev:medium"; c = "FBCA04"; d = "Medium severity" }
  @{ n = "sev:low"; c = "C2E0C6"; d = "Low severity" }
  @{ n = "sev:info"; c = "D4C5F9"; d = "Informational" }
  @{ n = "website"; c = "006B75"; d = "Website / CMS" }
  @{ n = "hygiene"; c = "C5DEF5"; d = "Scripts / CI / repo hygiene" }
  @{ n = "security"; c = "B60205"; d = "Security" }
  @{ n = "rpc"; c = "1D76DB"; d = "RPC gateway" }
  @{ n = "node"; c = "5319E7"; d = "Node / chain / mempool" }
  @{ n = "mining-pool"; c = "D93F0B"; d = "Mining pool" }
  @{ n = "desktop"; c = "FBCA04"; d = "Desktop clients" }
  @{ n = "review"; c = "0E8A16"; d = "From code review" }
)

Write-Host "Bootstrapping labels on $Repo"
foreach ($l in $labels) {
  $existing = gh label list --repo $Repo --json name --jq ".[].name" 2>$null
  if ($existing -split "`n" | Where-Object { $_ -eq $l.n }) {
    gh label edit $l.n --repo $Repo --color $l.c --description $l.d 2>$null | Out-Null
    Write-Host "  update $($l.n)"
  } else {
    gh label create $l.n --repo $Repo --color $l.c --description $l.d 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) {
      gh label edit $l.n --repo $Repo --color $l.c --description $l.d 2>$null | Out-Null
    }
    Write-Host "  create $($l.n)"
  }
}
Write-Host "Done."
