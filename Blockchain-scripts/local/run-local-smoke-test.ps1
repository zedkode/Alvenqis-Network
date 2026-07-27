param(
    [switch]$SkipReleaseGate,
    [switch]$Help
)

if ($Help -or $args -contains "--help" -or $args -contains "-h") {
    Write-Host "Usage: scripts/local/run-local-smoke-test.ps1 [-SkipReleaseGate]"
    Write-Host "Runs the local operator smoke test for node, RPC, wallet, indexer and explorer build readiness."
    exit 0
}

. (Join-Path $PSScriptRoot "common.ps1")

try {
    if (-not $SkipReleaseGate -and (Test-Path (Join-Path $script:WorkspaceRoot "scripts\release\release-gate.ps1"))) {
        Write-Host "Running release gate before local smoke test..."
        Invoke-ReleaseGate
    }

    & (Join-Path $PSScriptRoot "start-all.ps1") -SkipExplorer

    Write-Host (Invoke-NodeCommand -CommandArgs @("validate-chain") -CaptureOutput)
    Write-Host (Invoke-NodeCommand -CommandArgs @("mine-block") -CaptureOutput)
    Write-Host (Invoke-NodeCommand -CommandArgs @("validate-chain") -CaptureOutput)

    $health = Invoke-RestMethod -Uri "$script:RpcUrl/health" -TimeoutSec 5
    $network = Invoke-RestMethod -Uri "$script:RpcUrl/network" -TimeoutSec 5
    Write-Host ($health | ConvertTo-Json -Depth 5)
    Write-Host ($network | ConvertTo-Json -Depth 5)

    Write-Host (Invoke-WalletCommand -CommandArgs @("create-wallet") -CaptureOutput)
    Write-Host (Invoke-WalletCommand -CommandArgs @("address") -CaptureOutput)
    Write-Host (Refresh-IndexSnapshot)
    Write-Host (Invoke-IndexerCommand -CommandArgs @("status") -CaptureOutput)

    if (Test-Path $script:ExplorerDir) {
        Invoke-ExternalCommand -FilePath "npm" -Arguments @("install") -WorkingDirectory $script:ExplorerDir
        Invoke-ExternalCommand -FilePath "npm" -Arguments @("run", "build") -WorkingDirectory $script:ExplorerDir
    }

    Write-Host "local smoke test passed"
    Write-Host "logs_dir=$script:LogsDir"
} finally {
    & (Join-Path $PSScriptRoot "stop-all.ps1")
}
