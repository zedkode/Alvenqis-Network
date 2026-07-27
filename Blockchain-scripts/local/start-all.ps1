param(
    [switch]$SkipExplorer,
    [switch]$ResetOnFailure,
    [switch]$Help
)

if ($Help -or $args -contains "--help" -or $args -contains "-h") {
    Write-Host "Usage: scripts/local/start-all.ps1 [-SkipExplorer] [-ResetOnFailure]"
    Write-Host "Starts the local node, local RPC, refreshes the local index snapshot and optionally starts the explorer."
    Write-Host "-ResetOnFailure backs up and clears local chain data once if startup fails because old local data is incompatible."
    exit 0
}

. (Join-Path $PSScriptRoot "common.ps1")

Ensure-LocalDirectories
Show-LocalSummary

function Invoke-LocalStartup {
    Assert-ServicePortAvailable -Name "node" -Port 20787
    Assert-ServicePortAvailable -Name "rpc" -Port 10787
    Start-BackgroundPowerShellCommand -Name "node" -Command (Invoke-NodeStartCommandString)
    Wait-Until -TimeoutSeconds 120 -Description "local node startup" -Condition {
        if (-not (Test-ManagedProcess "node")) {
            return $true
        }

        if (-not (Test-Path (Join-Path $script:ChainDir "chain.sqlite3")) -and
            -not (Test-Path (Join-Path $script:ChainDir "chain.jsonl"))) {
            return $false
        }

        try {
            $null = Invoke-NodeCommand -CommandArgs @("node-status") -CaptureOutput
            return $true
        } catch {
            return $false
        }
    }
    Assert-ManagedProcessRunning "node"
    $nodeStatus = Invoke-NodeCommand -CommandArgs @("node-status") -CaptureOutput
    Write-Host $nodeStatus

    Start-BackgroundPowerShellCommand -Name "rpc" -Command (Invoke-RpcStartCommandString)
    Wait-ForHttpReady -Url "$script:RpcUrl/health" -TimeoutSeconds 120
    Assert-ManagedProcessRunning "rpc"

    $indexOutput = Refresh-IndexSnapshot
    Write-Host $indexOutput

    if ((Test-Path $script:ExplorerDir) -and -not $SkipExplorer) {
        Start-BackgroundPowerShellCommand -Name "explorer" -WorkingDirectory $script:ExplorerDir -Command (Get-ExplorerStartCommandString)
        Wait-ForHttpReady -Url $script:ExplorerUrl -TimeoutSeconds 90
        Assert-ManagedProcessRunning "explorer"
        Write-Host "Explorer ready at $script:ExplorerUrl"
    }

    Write-Host "Logs:"
    Write-Host "  node: $(Get-LogFilePath "node")"
    Write-Host "  rpc: $(Get-LogFilePath "rpc")"
    Write-Host "  explorer: $(Get-LogFilePath "explorer")"
}

try {
    Invoke-LocalStartup
} catch {
    $failureMessage = $_.Exception.Message
    if (-not $ResetOnFailure) {
        throw
    }

    Write-Warning "Local startup failed. Attempting one safe backup + reset retry because -ResetOnFailure was passed."
    Write-Warning $failureMessage

    & (Join-Path $PSScriptRoot "reset-local-chain.ps1")
    Invoke-LocalStartup
}
