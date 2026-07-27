param(
    [Parameter(Mandatory = $true)]
    [string]$RepoPath
)

$ErrorActionPreference = 'Stop'

$resolvedRepo = (Resolve-Path $RepoPath).Path
$source = Join-Path $PSScriptRoot 'workflows'
$target = Join-Path $resolvedRepo '.github\workflows'

if (-not (Test-Path (Join-Path $resolvedRepo '.git'))) {
    throw "RepoPath is not a Git repository: $resolvedRepo"
}

New-Item -ItemType Directory -Force -Path $target | Out-Null

$timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$backup = Join-Path $resolvedRepo ".github\workflows-backup-$timestamp"
New-Item -ItemType Directory -Force -Path $backup | Out-Null

Get-ChildItem -Path $target -File -ErrorAction SilentlyContinue |
    Copy-Item -Destination $backup -Force

@('candidate-release.yml', 'rust.yml') | ForEach-Object {
    $obsolete = Join-Path $target $_
    if (Test-Path $obsolete) {
        Remove-Item $obsolete -Force
    }
}

Get-ChildItem -Path $source -Filter '*.yml' -File |
    Copy-Item -Destination $target -Force

$releaseToolsTarget = Join-Path $resolvedRepo 'scripts\release'
New-Item -ItemType Directory -Force -Path $releaseToolsTarget | Out-Null
Copy-Item (Join-Path $PSScriptRoot 'alvenqis-release.ps1') -Destination $releaseToolsTarget -Force
Copy-Item (Join-Path $PSScriptRoot 'alvenqis-release.cmd') -Destination $releaseToolsTarget -Force

Write-Host "Installed rebuilt workflows into: $target"
Write-Host "Installed interactive release manager into: $releaseToolsTarget"
Write-Host "Backup of previous workflows: $backup"
Write-Host "Next: review 'git diff -- .github/workflows scripts/release' and commit the changes."
Write-Host "After commit, run: .\scripts\release\alvenqis-release.cmd"
