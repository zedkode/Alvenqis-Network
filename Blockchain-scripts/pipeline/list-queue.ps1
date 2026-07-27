#Requires -Version 5.1
param(
  [string]$Repo = "",
  [ValidateSet("all", "needs-triage", "ready", "in-progress", "in-qa", "blocked")]
  [string]$Status = "all"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }

$issues = gh issue list --repo $Repo --state open --limit 100 --json number,title,labels,createdAt |
  ConvertFrom-Json

$rows = foreach ($i in $issues) {
  $labs = @($i.labels | ForEach-Object { $_.name })
  if ($labs -notcontains "pipeline" -and $labs -notcontains "review") { continue }
  $st = ($labs | Where-Object { $_ -like "status:*" } | Select-Object -First 1)
  if (-not $st) { $st = "status:?" }
  $sev = ($labs | Where-Object { $_ -like "sev:*" } | Select-Object -First 1)
  if ($Status -ne "all" -and $st -ne "status:$Status") { continue }
  [pscustomobject]@{
    Number = $i.number
    Status = $st
    Sev    = $sev
    Title  = $i.title
    Labels = ($labs -join ", ")
  }
}

$order = @{
  "status:needs-triage" = 0
  "status:ready"        = 1
  "status:in-progress"  = 2
  "status:in-qa"        = 3
  "status:blocked"      = 4
  "status:?"            = 9
}
$rows |
  Sort-Object { if ($order.ContainsKey($_.Status)) { $order[$_.Status] } else { 8 } }, Number |
  Format-Table Number, Status, Sev, Title -AutoSize

Write-Host ("Total: {0}" -f @($rows).Count)
