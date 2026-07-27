param(
    [Parameter(Position = 0)]
    [string]$Command = "help",
    [switch]$SkipExplorer,
    [switch]$ResetOnFailure,
    [switch]$NoBackup,
    [switch]$IncludeWallets,
    [switch]$SkipReleaseGate,
    [string]$Service = "node",
    [int]$Tail = 80
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

function Show-Usage {
    Write-Host "Usage: scripts/local/alvenqis-local.ps1 <command> [options]"
    Write-Host ""
    Write-Host "Commands:"
    Write-Host "  start         Start node, RPC and explorer."
    Write-Host "  stop          Stop managed local services."
    Write-Host "  restart       Stop then start the local services."
    Write-Host "  status        Show process, chain, mempool, RPC and latest-block status."
    Write-Host "  mine          Mine one block, validate the chain and refresh the index."
    Write-Host "  backup        Backup local chain, mempool, index and logs."
    Write-Host "  reset         Stop services and clear local chain data."
    Write-Host "  smoke         Run the local smoke test."
    Write-Host "  logs          Print the tail of node, rpc or explorer logs."
    Write-Host "  help          Show this message."
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -SkipExplorer       Do not start the explorer during start/restart."
    Write-Host "  -ResetOnFailure     Backup + reset local chain once if start fails."
    Write-Host "  -NoBackup           Allow reset without creating a backup."
    Write-Host "  -IncludeWallets     Include wallets in the backup command."
    Write-Host "  -SkipReleaseGate    Skip release-gate execution in the smoke test."
    Write-Host "  -Service <name>     Log target for the logs command: node, rpc, explorer."
    Write-Host "  -Tail <count>       Number of log lines to show for the logs command."
}

switch ($Command.ToLowerInvariant()) {
    "start" {
        & (Join-Path $scriptDir "start-all.ps1") -SkipExplorer:$SkipExplorer -ResetOnFailure:$ResetOnFailure
    }
    "stop" {
        & (Join-Path $scriptDir "stop-all.ps1")
    }
    "restart" {
        & (Join-Path $scriptDir "stop-all.ps1")
        & (Join-Path $scriptDir "start-all.ps1") -SkipExplorer:$SkipExplorer -ResetOnFailure:$ResetOnFailure
    }
    "status" {
        & (Join-Path $scriptDir "status-all.ps1")
    }
    "mine" {
        & (Join-Path $scriptDir "mine-local-block.ps1")
    }
    "backup" {
        & (Join-Path $scriptDir "backup-local-chain.ps1") -IncludeWallets:$IncludeWallets
    }
    "reset" {
        & (Join-Path $scriptDir "reset-local-chain.ps1") -NoBackup:$NoBackup
    }
    "smoke" {
        & (Join-Path $scriptDir "run-local-smoke-test.ps1") -SkipReleaseGate:$SkipReleaseGate
    }
    "logs" {
        . (Join-Path $scriptDir "common.ps1")
        $selected = $Service.ToLowerInvariant()
        if ($selected -notin @("node", "rpc", "explorer")) {
            throw "unsupported service '$Service'. expected: node, rpc, explorer"
        }

        $stdoutPath = Get-LogFilePath $selected
        $stderrPath = Get-ErrLogFilePath $selected

        Write-Host "stdout: $stdoutPath"
        if (Test-Path $stdoutPath) {
            Get-Content -Path $stdoutPath -Tail $Tail
        } else {
            Write-Host "(missing)"
        }

        Write-Host ""
        Write-Host "stderr: $stderrPath"
        if (Test-Path $stderrPath) {
            Get-Content -Path $stderrPath -Tail $Tail
        } else {
            Write-Host "(missing)"
        }
    }
    "help" {
        Show-Usage
    }
    default {
        Show-Usage
        throw "unknown command '$Command'"
    }
}
