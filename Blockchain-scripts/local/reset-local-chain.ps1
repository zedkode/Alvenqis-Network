param(
    [switch]$NoBackup,
    [switch]$Help
)

if ($Help -or $args -contains "--help" -or $args -contains "-h") {
    Write-Host "Usage: scripts/local/reset-local-chain.ps1 [-NoBackup]"
    Write-Host "Stops local processes, creates a backup unless -NoBackup is passed, and clears local chain, mempool and index data."
    exit 0
}

. (Join-Path $PSScriptRoot "common.ps1")

& (Join-Path $PSScriptRoot "stop-all.ps1")

$hasData = @($script:ChainDir, $script:MempoolDir, $script:IndexerDir) | Where-Object { Test-Path $_ }
$backupPath = $null
if (-not $NoBackup -and $hasData.Count -gt 0) {
    $backupPath = Backup-LocalData
} elseif ($NoBackup) {
    Write-Warning "Skipping local backup because -NoBackup was explicitly passed."
}

if (-not $NoBackup -and $hasData.Count -gt 0 -and -not $backupPath) {
    throw "refusing reset because no backup was created"
}

Clear-LocalChainState
Write-Host "local reset complete"
if ($backupPath) {
    Write-Host "backup_path=$backupPath"
}
