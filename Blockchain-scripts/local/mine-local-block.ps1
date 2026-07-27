param([switch]$Help)

if ($Help -or $args -contains "--help" -or $args -contains "-h") {
    Write-Host "Usage: scripts/local/mine-local-block.ps1"
    Write-Host "Mines one local block, refreshes the index snapshot, validates the chain and prints the latest block if RPC is running."
    exit 0
}

. (Join-Path $PSScriptRoot "common.ps1")

Write-Host (Invoke-NodeCommand -CommandArgs @("mine-block") -CaptureOutput)
Write-Host (Invoke-NodeCommand -CommandArgs @("validate-chain") -CaptureOutput)
Write-Host (Refresh-IndexSnapshot)

try {
    $latestBlock = Invoke-RestMethod -Uri "$script:RpcUrl/blocks/latest" -TimeoutSec 5
    $latestBlock | ConvertTo-Json -Depth 6
} catch {
    Write-Warning "Latest block could not be fetched from RPC: $($_.Exception.Message)"
}
