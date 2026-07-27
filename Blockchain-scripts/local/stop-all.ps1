param([switch]$Help)

if ($Help -or $args -contains "--help" -or $args -contains "-h") {
    Write-Host "Usage: scripts/local/stop-all.ps1"
    Write-Host "Stops local managed Alvenqis processes if they are running."
    exit 0
}

. (Join-Path $PSScriptRoot "common.ps1")

Stop-NodeProcess
Stop-ManagedProcess "rpc"
Stop-ExplorerProcess
Stop-WorkspaceBinaryProcess "alvenqis-rpc-gateway"

Write-Host "Managed local processes stopped."
Write-Host "Logs remain under $script:LogsDir"
