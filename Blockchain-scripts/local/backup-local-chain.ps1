param(
    [switch]$IncludeWallets,
    [switch]$Help
)

if ($Help -or $args -contains "--help" -or $args -contains "-h") {
    Write-Host "Usage: scripts/local/backup-local-chain.ps1 [-IncludeWallets]"
    Write-Host "Creates a timestamped backup of local chain, mempool, index and logs. Wallets are excluded by default."
    exit 0
}

. (Join-Path $PSScriptRoot "common.ps1")

$backupPath = Backup-LocalData -IncludeWallets:$IncludeWallets
Write-Host "backup_path=$backupPath"
