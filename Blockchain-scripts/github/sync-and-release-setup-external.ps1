[CmdletBinding()]
param(
  [string]$Message,
  [switch]$NoWait,
  [switch]$SyncOnly,
  [Alias("h")]
  [switch]$Help
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot
$expectedRepository = "zedkode/Alvenqis-Network"
$workflow = "alvenqis-setup-external-release.yml"
$workspace = Join-Path $repoRoot "Blockchain-prototype"
$versionFile = "Blockchain-prototype/alvenqis-release/alvenqis-setup-external/VERSION"
$env:CARGO_TARGET_DIR = Join-Path $repoRoot "target-msvc"
$gh = @(
  (Join-Path $env:ProgramFiles "GitHub CLI\gh.exe"),
  (Join-Path $env:LOCALAPPDATA "Programs\GitHub CLI\gh.exe")
) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $gh) {
  $command = Get-Command gh -ErrorAction SilentlyContinue
  if ($command) { $gh = $command.Source }
}
if (-not $gh) { throw "GitHub CLI is required. Install GitHub.cli with winget." }

function Invoke-Checked {
  param([scriptblock]$Command, [string]$Failure)
  & $Command
  if ($LASTEXITCODE -ne 0) { throw $Failure }
}

function Invoke-Cargo {
  param([string[]]$Arguments)
  $rustup = Join-Path $env:USERPROFILE ".cargo\bin\rustup.exe"
  Push-Location $workspace
  try {
    if (Test-Path -LiteralPath $rustup) {
      & $rustup run stable-x86_64-pc-windows-msvc cargo @Arguments
    } else {
      & cargo @Arguments
    }
    if ($LASTEXITCODE -ne 0) { throw "cargo $($Arguments -join ' ') failed" }
  } finally {
    Pop-Location
  }
}

if ($Help) {
  Write-Host "Usage: .\Blockchain-scripts\github\sync-and-release-setup-external.ps1 [-Message TEXT] [-NoWait] [-SyncOnly] [-Help]"
  exit 0
}

$remote = git remote get-url origin
if ($LASTEXITCODE -ne 0 -or $remote -notmatch "github\.com[/:]zedkode/Alvenqis-Network(?:\.git)?$") {
  throw "origin must point to https://github.com/$expectedRepository"
}
if ((git branch --show-current) -ne "main") { throw "Setup External releases are allowed only from main." }
Invoke-Checked { & $gh auth status --hostname github.com } "GitHub CLI is not authenticated."
$repo = & $gh repo view --json nameWithOwner,viewerPermission | ConvertFrom-Json
if ($repo.nameWithOwner -ne $expectedRepository -or $repo.viewerPermission -notin @("ADMIN", "MAINTAIN", "WRITE")) {
  throw "Authenticated account cannot publish $expectedRepository."
}

Write-Host "[1/8] Scanning repository before staging"
& Blockchain-scripts/git/check-forbidden-files.ps1
& Blockchain-scripts/security/check-secrets.ps1
& Blockchain-scripts/security/check-repo-hygiene.ps1
& Blockchain-scripts/security/check-config-safety.ps1

Write-Host "[2/8] Committing coherent local changes when present"
git add --all
if ($LASTEXITCODE -ne 0) { throw "git add failed" }
git diff --cached --quiet
if ($LASTEXITCODE -eq 1) {
  if (-not $Message) {
    $Message = "release(setup-external): bundle update $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
  }
  Invoke-Checked { git commit -m $Message } "git commit failed"
} elseif ($LASTEXITCODE -ne 0) {
  throw "cannot inspect staged changes"
} else {
  Write-Host "No local changes require a commit."
}

Write-Host "[3/8] Integrating origin/main"
Invoke-Checked { git fetch origin --prune } "git fetch failed"
Invoke-Checked { git pull --rebase origin main } "git rebase failed. Resolve conflicts, then rerun the script."

Write-Host "[4/8] Running release gates"
& Blockchain-scripts/git/check-forbidden-files.ps1
& Blockchain-scripts/security/check-secrets.ps1
& Blockchain-scripts/security/check-repo-hygiene.ps1
& Blockchain-scripts/security/check-config-safety.ps1
Invoke-Cargo @("fmt", "--all", "--check")
Invoke-Cargo @("test", "--workspace", "--locked")
Invoke-Cargo @("clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings")

if (git status --porcelain) { throw "Release checks changed the working tree; refusing to tag." }
Write-Host "[5/8] Pushing main"
Invoke-Checked { git push origin main } "push to origin/main failed"

if ($SyncOnly) {
  Write-Host "Main synchronized. Setup External release was intentionally skipped."
  exit 0
}

$version = (Get-Content -LiteralPath $versionFile -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw "Invalid VPS VERSION: $version" }
$prefix = "setup-external-v$version-rc."
$existingHeadTag = @(git tag --points-at HEAD --list "$prefix*") | Select-Object -First 1
if ($existingHeadTag) {
  Write-Host "Current commit is already released as $existingHeadTag. No duplicate tag created."
  exit 0
}
$remoteTags = @(git ls-remote --tags origin "$prefix*")
$numbers = foreach ($line in $remoteTags) {
  if ($line -match [regex]::Escape("refs/tags/$prefix") + '(\d+)$') { [int]$Matches[1] }
}
$next = if ($numbers) { ($numbers | Measure-Object -Maximum).Maximum + 1 } else { 1 }
$tag = "$prefix$next"

Write-Host "[6/8] Creating $tag"
Invoke-Checked { git tag -a $tag -m "alvenqis-setup-external $tag" } "tag creation failed"
try {
  Invoke-Checked { git push origin $tag } "tag push failed"
} catch {
  git tag -d $tag | Out-Null
  throw
}

if ($NoWait) {
  Write-Host "Triggered $workflow for $tag."
  exit 0
}

Write-Host "[7/8] Waiting for GitHub Actions"
$runId = $null
for ($attempt = 0; $attempt -lt 30 -and -not $runId; $attempt++) {
  Start-Sleep -Seconds 2
  $runs = & $gh run list --workflow $workflow --limit 20 --json databaseId,headBranch,event | ConvertFrom-Json
  $matchingRun = $runs | Where-Object { $_.headBranch -eq $tag } | Select-Object -First 1
  if ($matchingRun) { $runId = $matchingRun.databaseId }
}
if (-not $runId) { throw "GitHub Actions run for $tag was not discovered." }
Invoke-Checked { & $gh run watch $runId --exit-status } "Setup External release workflow failed."

Write-Host "[8/8] Verifying published release assets"
$release = & $gh release view $tag --json url,isPrerelease,assets | ConvertFrom-Json
$requiredAssets = @(
  "alvenqis-setup-external.tar.gz",
  "alvenqis-setup-external.tar.gz.sha256",
  "setup-external-contents.txt"
)
foreach ($asset in $requiredAssets) {
  if ($asset -notin @($release.assets.name)) { throw "Published release is missing $asset" }
}
Write-Host "Release published: $($release.url)"
