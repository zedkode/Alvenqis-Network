param([switch]$Help)

if ($Help -or $args -contains "--help" -or $args -contains "-h") {
    Write-Host "Usage: scripts/local/status-all.ps1"
    Write-Host "Shows local process state, chain validation, RPC health and index snapshot status."
    exit 0
}

. (Join-Path $PSScriptRoot "common.ps1")

Ensure-LocalDirectories
Show-LocalSummary

Write-Host ""
Write-Host "Managed processes:"
foreach ($name in @("node", "rpc", "explorer")) {
    $state = Get-ManagedProcessState $name
    if ($state -eq "running") {
        Write-Host "  ${name}: running pid=$(Get-StoredPid $name)"
        continue
    }

    if ($state -eq "stale") {
        Write-Host "  ${name}: stale pid file present"
        continue
    }

    Write-Host "  ${name}: stopped"
}

Write-Host ""
Write-Host "Node status:"
try {
    Write-Host (Invoke-NodeCommand -CommandArgs @("node-status") -CaptureOutput)
} catch {
    Write-Warning $_.Exception.Message
}

Write-Host ""
Write-Host "Chain validation:"
try {
    Write-Host (Invoke-NodeCommand -CommandArgs @("validate-chain") -CaptureOutput)
} catch {
    Write-Warning $_.Exception.Message
}

Write-Host ""
Write-Host "Mempool status:"
try {
    Write-Host (Invoke-NodeCommand -CommandArgs @("mempool-status") -CaptureOutput)
} catch {
    Write-Warning $_.Exception.Message
}

Write-Host ""
Write-Host "Indexer status:"
try {
    Write-Host (Invoke-IndexerCommand -CommandArgs @("status") -CaptureOutput)
} catch {
    Write-Warning $_.Exception.Message
}

Write-Host ""
Write-Host "RPC health:"
try {
    $health = Invoke-RestMethod -Uri "$script:RpcUrl/health" -TimeoutSec 5
    $health | ConvertTo-Json -Depth 5
} catch {
    Write-Warning "RPC health unavailable: $($_.Exception.Message)"
}

Write-Host ""
Write-Host "RPC network:"
try {
    $network = Invoke-RestMethod -Uri "$script:RpcUrl/network" -TimeoutSec 5
    $network | ConvertTo-Json -Depth 5
} catch {
    Write-Warning "RPC network unavailable: $($_.Exception.Message)"
}

Write-Host ""
Write-Host "Latest block:"
try {
    $latestBlock = Invoke-RestMethod -Uri "$script:RpcUrl/blocks/latest" -TimeoutSec 5
    $latestBlock | ConvertTo-Json -Depth 6
} catch {
    Write-Warning "Latest block unavailable: $($_.Exception.Message)"
}
