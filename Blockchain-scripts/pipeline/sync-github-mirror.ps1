#Requires -Version 5.1
<#
.SYNOPSIS
  Fetch origin and maintain a clean worktree at origin/main for agent inspection.
.EXAMPLE
  .\scripts\pipeline\sync-github-mirror.ps1
#>
param(
  [string]$RepoRoot = "",
  [switch]$Json
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "_common.ps1")

if (-not $RepoRoot) { $RepoRoot = Get-RepoRoot }
$paths = Get-PipelinePaths -RepoRoot $RepoRoot
Ensure-Dir $paths.Worktrees

Push-Location $RepoRoot
try {
  git fetch origin --prune
  if ($LASTEXITCODE -ne 0) { throw "git fetch failed" }

  $originMain = (git rev-parse origin/main).Trim()
  $mirror = $paths.Mirror

  if (-not (Test-Path (Join-Path $mirror ".git")) -and -not (Test-Path (Join-Path $mirror ".git"))) {
    # worktree metadata may be a file pointing to main repo
  }

  $isWorktree = $false
  if (Test-Path $mirror) {
    $gitDir = git -C $mirror rev-parse --git-dir 2>$null
    if ($LASTEXITCODE -eq 0) { $isWorktree = $true }
  }

  if (-not $isWorktree) {
    if (Test-Path $mirror) {
      Remove-Item -Recurse -Force $mirror
    }
    git worktree add --detach $mirror origin/main
    if ($LASTEXITCODE -ne 0) { throw "git worktree add failed" }
  } else {
    git -C $mirror fetch origin --prune 2>$null
    git -C $mirror checkout --detach origin/main
    if ($LASTEXITCODE -ne 0) {
      git worktree remove --force $mirror 2>$null
      if (Test-Path $mirror) { Remove-Item -Recurse -Force $mirror }
      git worktree add --detach $mirror origin/main
      if ($LASTEXITCODE -ne 0) { throw "git worktree re-add failed" }
    }
    git -C $mirror reset --hard origin/main
    git -C $mirror clean -fd
  }

  $sha = (git -C $mirror rev-parse HEAD).Trim()
  $originSha = (git rev-parse origin/main).Trim()
  if ($sha -ne $originSha) {
    throw "Mirror SHA $sha != origin/main $originSha"
  }

  $result = [pscustomobject]@{
    mirror_path = $mirror
    commit      = $sha
    ref         = "origin/main"
    repo_root   = $RepoRoot
  }

  if ($Json) {
    $result | ConvertTo-Json -Compress
  } else {
    Write-Host "Mirror ready: $mirror"
    Write-Host "Commit: $sha"
  }
  return $result
} finally {
  Pop-Location
}
