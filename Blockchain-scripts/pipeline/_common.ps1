#Requires -Version 5.1
# Shared helpers for scripts/pipeline/*.ps1

function Get-RepoRoot {
  param([string]$Start = $PSScriptRoot)
  $dir = Resolve-Path (Join-Path $Start "..\..")
  return $dir.Path
}

function Get-PipelinePaths {
  param([string]$RepoRoot = (Get-RepoRoot))
  $pipeline = Join-Path $RepoRoot ".review\pipeline"
  return [pscustomobject]@{
    RepoRoot   = $RepoRoot
    Pipeline   = $pipeline
    Worktrees  = Join-Path $pipeline "worktrees"
    Runs       = Join-Path $pipeline "runs"
    Mirror     = Join-Path $pipeline "worktrees\mirror-main"
    LockFile   = Join-Path $pipeline "worktrees\.pipeline.lock"
  }
}

function Get-DefaultRepo {
  param([string]$RepoRoot = (Get-RepoRoot))
  Push-Location $RepoRoot
  try {
    $remote = git remote get-url origin 2>$null
    if ($remote -match 'github\.com[:/](.+?)(?:\.git)?\s*$') {
      return $Matches[1] -replace '\\', '/'
    }
  } finally {
    Pop-Location
  }
  return "zedkode/Alvenqis-Network"
}

function Ensure-Dir([string]$Path) {
  if (-not (Test-Path $Path)) {
    New-Item -ItemType Directory -Path $Path -Force | Out-Null
  }
}

function Invoke-Gh {
  param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$GhArgs
  )
  & gh @GhArgs
  if ($LASTEXITCODE -ne 0) {
    throw "gh failed ($LASTEXITCODE): gh $($GhArgs -join ' ')"
  }
}

function Get-IssueLabels {
  param(
    [Parameter(Mandatory = $true)][int]$Number,
    [string]$Repo = (Get-DefaultRepo)
  )
  $json = gh issue view $Number --repo $Repo --json labels | ConvertFrom-Json
  return @($json.labels | ForEach-Object { $_.name })
}

function Set-StatusLabel {
  param(
    [Parameter(Mandatory = $true)][int]$Number,
    [Parameter(Mandatory = $true)][string]$Status,
    [string]$Repo = (Get-DefaultRepo)
  )
  $all = Get-IssueLabels -Number $Number -Repo $Repo
  $statusLabels = $all | Where-Object { $_ -like 'status:*' }
  foreach ($s in $statusLabels) {
    if ($s -ne $Status) {
      gh issue edit $Number --repo $Repo --remove-label $s 2>$null | Out-Null
    }
  }
  if ($all -notcontains $Status) {
    Invoke-Gh issue edit $Number --repo $Repo --add-label $Status
  }
  if ($all -notcontains 'pipeline') {
    Invoke-Gh issue edit $Number --repo $Repo --add-label pipeline
  }
}

function Get-CargoCommand {
  $cargoShim = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
  if (Test-Path $cargoShim) {
    return @{ Exe = $cargoShim; Prefix = @("+stable-x86_64-pc-windows-msvc") }
  }
  return @{ Exe = "cargo"; Prefix = @() }
}
