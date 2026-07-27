# Shared repo path helpers for PowerShell scripts.
# Dot-source from Blockchain-scripts/<area>/*.ps1:
#   . (Join-Path $PSScriptRoot "..\lib\repo-paths.ps1")
function Get-AlvenqisRepoRoot {
  param([string]$StartDir = $PSScriptRoot)
  $dir = $StartDir
  while ($dir) {
    if ((Test-Path (Join-Path $dir "Blockchain-prototype\Cargo.toml")) -or (Test-Path (Join-Path $dir "init.md"))) {
      return (Resolve-Path $dir).Path
    }
    $parent = Split-Path $dir -Parent
    if (-not $parent -or $parent -eq $dir) { break }
    $dir = $parent
  }
  throw "Could not locate Alvenqis repo root from $StartDir"
}
function Get-AlvenqisPrototypeRoot {
  param([string]$RepoRoot = (Get-AlvenqisRepoRoot))
  Join-Path $RepoRoot "Blockchain-prototype"
}
function Get-AlvenqisScriptsRoot {
  param([string]$RepoRoot = (Get-AlvenqisRepoRoot))
  Join-Path $RepoRoot "Blockchain-scripts"
}
function Get-AlvenqisDocsRoot {
  param([string]$RepoRoot = (Get-AlvenqisRepoRoot))
  Join-Path $RepoRoot "Blockchain-docs"
}
