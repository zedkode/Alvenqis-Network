#Requires -Version 5.1
<#
.SYNOPSIS
  Pipeline cycle entrypoint (scaffolding for Grok orchestrator).

  This script prepares GitHub-main mirror, run folder, and prints the queue.
  Multi-agent spawn remains host-driven (see .review/pipeline/ORCHESTRATOR.md).

.EXAMPLE
  .\scripts\pipeline\run-cycle.ps1 -Mode drain
  .\scripts\pipeline\run-cycle.ps1 -Mode once -ImportBacklog
#>
param(
  [ValidateSet("once", "drain", "watch")]
  [string]$Mode = "drain",
  [string]$Repo = "",
  [switch]$ImportBacklog,
  [switch]$BootstrapLabels,
  [int]$WatchSeconds = 3600
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")
if (-not $Repo) { $Repo = Get-DefaultRepo }

$repoRoot = Get-RepoRoot
$paths = Get-PipelinePaths -RepoRoot $repoRoot
Ensure-Dir $paths.Runs
Ensure-Dir $paths.Worktrees

$runId = Get-Date -Format "yyyyMMdd-HHmmss"
$runDir = Join-Path $paths.Runs $runId
Ensure-Dir $runDir

Write-Host "=== Alvenqis pipeline cycle ===" -ForegroundColor Green
Write-Host "Mode: $Mode"
Write-Host "Repo: $Repo"
Write-Host "Run:  $runDir"

if ($BootstrapLabels) {
  & (Join-Path $PSScriptRoot "bootstrap-labels.ps1") -Repo $Repo
}

$mirror = & (Join-Path $PSScriptRoot "sync-github-mirror.ps1") -RepoRoot $repoRoot
$sha = $mirror.commit

if ($ImportBacklog) {
  & (Join-Path $PSScriptRoot "import-backlog.ps1") -Repo $Repo
}

$state = [pscustomobject]@{
  run_id      = $runId
  mode        = $Mode
  repo        = $Repo
  mirror_sha  = $sha
  mirror_path = $mirror.mirror_path
  started_at  = (Get-Date).ToString("o")
  note        = "Host orchestrator should spawn Finder/Triage/Fixer/QA/Integrator per ORCHESTRATOR.md using mirror_path only."
}
$state | ConvertTo-Json | Set-Content (Join-Path $runDir "state.json") -Encoding UTF8

Write-Host "`n--- Queue ---"
& (Join-Path $PSScriptRoot "list-queue.ps1") -Repo $Repo

$summary = @"
# Pipeline run $runId

- **Mode:** $Mode
- **Repo:** $Repo
- **Mirror SHA:** ``$sha``
- **Mirror path:** ``$($mirror.mirror_path)``
- **Started:** $($state.started_at)

## Next (host orchestrator)

1. Triage all ``status:needs-triage`` against mirror (``triage-apply.ps1``)
2. Claim ``status:ready`` issues (``claim-issue.ps1``) with worktrees from ``origin/main``
3. Open PRs (``open-fix-pr.ps1``)
4. QA (``run-checks.ps1`` + ``qa-report.ps1``)
5. Merge (``merge-if-green.ps1``)

**GitHub is source of truth. Do not inspect dirty local Desktop trees.**
"@
Set-Content -Path (Join-Path $runDir "summary.md") -Value $summary -Encoding UTF8
Write-Host "`nWrote $runDir\summary.md"
Write-Host "Mirror commit for agents: $sha"

if ($Mode -eq "watch") {
  Write-Host "Watch mode: re-run this script on an interval (host loop). Sleeping $WatchSeconds s once as a placeholder."
  Start-Sleep -Seconds $WatchSeconds
}

Write-Host "Cycle scaffold complete."
