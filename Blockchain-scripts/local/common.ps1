Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Repo root (contains Blockchain-prototype / Blockchain-scripts / Blockchain-docs).
$script:RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
# Implemented monorepo workspace (Cargo.toml lives here).
$script:WorkspaceRoot = Join-Path $script:RepoRoot "Blockchain-prototype"
$script:SidecarDir = Join-Path $script:RepoRoot "bin"
$script:IsPackaged = Test-Path (Join-Path $script:SidecarDir "alvenqis-node.exe")
$legacyLocalRoot = if ($script:IsPackaged) {
    Join-Path $env:LOCALAPPDATA "Alvenqis\ControlCenter\.alvenqis-local"
} else {
    Join-Path $script:RepoRoot ".alvenqis-local"
}
$script:LocalRoot = if ($env:ALVENQIS_LOCAL_ROOT) {
    $env:ALVENQIS_LOCAL_ROOT
} elseif ($script:IsPackaged) {
    Join-Path $env:LOCALAPPDATA "Alvenqis\ControlCenter\.alvenqis-local"
} elseif ((Test-Path -LiteralPath $legacyLocalRoot) -and -not (Test-Path -LiteralPath (Join-Path $script:RepoRoot ".alvenqis-local"))) {
    $legacyLocalRoot
} else {
    Join-Path $script:RepoRoot ".alvenqis-local"
}
$script:ChainDir = Join-Path $script:LocalRoot "chain"
$script:MempoolDir = Join-Path $script:LocalRoot "mempool"
$script:IndexerDir = Join-Path $script:LocalRoot "indexer"
$newWalletDir = Join-Path $env:USERPROFILE ".alvenqis-mainnet\wallets"
$legacyWalletDir = Join-Path $env:USERPROFILE ".alvenqis-mainnet\wallets"
$script:WalletDir = if ((Test-Path -LiteralPath $legacyWalletDir) -and -not (Test-Path -LiteralPath $newWalletDir)) {
    $legacyWalletDir
} else {
    $newWalletDir
}
$script:SignedTxDir = Join-Path $script:WalletDir "signed-txs"
$script:LogsDir = Join-Path $script:LocalRoot "logs"
$script:BackupsDir = Join-Path $script:LocalRoot "backups"
$script:BuildDir = if ($env:ALVENQIS_BUILD_DIR) {
    $env:ALVENQIS_BUILD_DIR
} else {
    Join-Path $script:LocalRoot "build\target"
}
$script:LocalNodeConfig = if ($env:ALVENQIS_LOCAL_NODE_CONFIG) {
    $env:ALVENQIS_LOCAL_NODE_CONFIG
} elseif ($script:IsPackaged) {
    Join-Path $script:LocalRoot "node.toml"
} else {
    Join-Path $script:WorkspaceRoot "configs\local.toml"
}
$script:LocalRpcConfig = if ($script:IsPackaged) {
    Join-Path $script:LocalRoot "rpc.local.toml"
} else {
    Join-Path $script:WorkspaceRoot "configs\rpc.local.toml"
}
$script:ExplorerDir = Join-Path $script:WorkspaceRoot "alvenqis-explorer"
$script:ExplorerEnvExample = Join-Path $script:WorkspaceRoot "configs\explorer.local.example.env"
$script:RpcUrl = "http://127.0.0.1:10787"
$script:ExplorerUrl = "http://127.0.0.1:4173"
$rustToolchainBin = Join-Path $env:USERPROFILE ".rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"
$script:ManagedPathValue = if (Test-Path $rustToolchainBin) {
    "$rustToolchainBin;$env:USERPROFILE\.cargo\bin;$env:PATH"
} else {
    "$env:USERPROFILE\.cargo\bin;$env:PATH"
}
$cargoShim = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (Test-Path $cargoShim) {
    $script:CargoPath = $cargoShim
    $script:CargoPrefix = @("+stable-x86_64-pc-windows-msvc")
} else {
    $script:CargoPath = "cargo"
    $script:CargoPrefix = @()
}

function Quote-PowerShell {
    param([string]$Value)
    "'" + $Value.Replace("'", "''") + "'"
}

function Join-PowerShellArgs {
    param([string[]]$Arguments)
    ($Arguments | ForEach-Object { Quote-PowerShell $_ }) -join " "
}

function Ensure-LocalDirectories {
    foreach ($path in @(
        $script:LocalRoot,
        $script:ChainDir,
        $script:MempoolDir,
        $script:IndexerDir,
        $script:WalletDir,
        $script:SignedTxDir,
        $script:LogsDir,
        $script:BackupsDir,
        $script:BuildDir
    )) {
        New-Item -ItemType Directory -Force -Path $path | Out-Null
    }
    if ($script:IsPackaged) {
        if (-not (Test-Path -LiteralPath $script:LocalNodeConfig)) {
            Copy-Item -LiteralPath (Join-Path $script:WorkspaceRoot "configs\mainnet-candidate.toml") -Destination $script:LocalNodeConfig -ErrorAction Stop
        }
        @"
bind_host = "127.0.0.1"
bind_port = 10787
network = "mainnet-candidate"
network_id = "alvenqis-mainnet-candidate"
human_name = "Alvenqis Mainnet Candidate"
status_label = "Planned / Mainnet Candidate"
address_prefix = "alve"
chain_data_path = "$($script:ChainDir.Replace('\', '\\'))"
indexer_data_path = "$($script:IndexerDir.Replace('\', '\\'))"
mempool_data_path = "$($script:MempoolDir.Replace('\', '\\'))"
public_rpc_allowed = false
max_mempool_transactions = 2048
max_request_body_bytes = 65536
cors_allowed_origin = "http://127.0.0.1:10787"
explorer_static_path = "$($script:WorkspaceRoot.Replace('\', '\\'))\\explorer"
allow_mainnet_candidate = true
"@ | Set-Content -LiteralPath $script:LocalRpcConfig -Encoding UTF8
    }
}

function Get-PidFilePath {
    param([string]$Name)
    Join-Path $script:LogsDir "$Name.pid"
}

function Get-LogFilePath {
    param([string]$Name)
    Join-Path $script:LogsDir "$Name.log"
}

function Get-ErrLogFilePath {
    param([string]$Name)
    Join-Path $script:LogsDir "$Name.err.log"
}

function Get-BootstrapFilePath {
    param([string]$Name)
    Join-Path $script:LogsDir "$Name.bootstrap.ps1"
}

function Get-StoredPid {
    param([string]$Name)
    $pidFile = Get-PidFilePath $Name
    if (-not (Test-Path $pidFile)) {
        return $null
    }
    $content = (Get-Content $pidFile -Raw).Trim()
    if (-not $content) {
        return $null
    }
    [int]$content
}

function Test-ManagedProcess {
    param([string]$Name)
    $pidValue = Get-StoredPid $Name
    if ($null -eq $pidValue) {
        return $false
    }
    try {
        Get-Process -Id $pidValue -ErrorAction Stop | Out-Null
        return $true
    } catch {
        Remove-Item (Get-PidFilePath $Name) -ErrorAction SilentlyContinue
        return $false
    }
}

function Assert-ServicePortAvailable {
    param(
        [string]$Name,
        [int]$Port
    )

    if (Test-ManagedProcess $Name) {
        return
    }

    $listeners = @(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
    if ($listeners.Count -eq 0) {
        return
    }

    $owners = foreach ($listener in $listeners) {
        $process = Get-Process -Id $listener.OwningProcess -ErrorAction SilentlyContinue
        if ($process) {
            "pid=$($process.Id) process=$($process.ProcessName) path=$($process.Path)"
        } else {
            "pid=$($listener.OwningProcess) process=unknown"
        }
    }
    throw "Cannot start $Name because port $Port is already owned by another process: $($owners -join '; '). Stop the other Alvenqis installation before starting this stack."
}

function Get-ManagedProcessState {
    param([string]$Name)

    $pidValue = Get-StoredPid $Name
    if ($null -eq $pidValue) {
        return "stopped"
    }

    try {
        $null = Get-Process -Id $pidValue -ErrorAction Stop
        return "running"
    } catch {
        return "stale"
    }
}

function Remove-ManagedPidFile {
    param([string]$Name)
    Remove-Item (Get-PidFilePath $Name) -ErrorAction SilentlyContinue
}

function Remove-ManagedRuntimeFiles {
    param([string]$Name)
    foreach ($path in @(
        Get-PidFilePath $Name,
        Get-BootstrapFilePath $Name
    )) {
        Remove-Item -LiteralPath $path -ErrorAction SilentlyContinue
    }
}

function Get-RecentLogText {
    param(
        [string]$Name,
        [switch]$ErrorLog,
        [int]$Tail = 40
    )

    $path = if ($ErrorLog) { Get-ErrLogFilePath $Name } else { Get-LogFilePath $Name }
    if (-not (Test-Path $path)) {
        return ""
    }

    (Get-Content -Path $path -Tail $Tail -ErrorAction SilentlyContinue) -join [Environment]::NewLine
}

function Invoke-ExternalCommand {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$WorkingDirectory = $script:WorkspaceRoot,
        [switch]$CaptureOutput
    )

    Ensure-LocalDirectories
    $previousPath = $env:PATH
    $previousCargoTargetDir = $env:CARGO_TARGET_DIR
    $env:PATH = $script:ManagedPathValue
    $env:CARGO_TARGET_DIR = $script:BuildDir

    try {
        Push-Location $WorkingDirectory
        if ($CaptureOutput) {
            $stdoutPath = Join-Path $script:LogsDir ("capture-" + [System.Guid]::NewGuid().ToString("N") + ".stdout.log")
            $stderrPath = Join-Path $script:LogsDir ("capture-" + [System.Guid]::NewGuid().ToString("N") + ".stderr.log")

            try {
                $process = Start-Process -FilePath $FilePath `
                    -ArgumentList $Arguments `
                    -WorkingDirectory $WorkingDirectory `
                    -RedirectStandardOutput $stdoutPath `
                    -RedirectStandardError $stderrPath `
                    -PassThru `
                    -Wait `
                    -WindowStyle Hidden

                $stdout = if (Test-Path $stdoutPath) { Get-Content $stdoutPath -Raw } else { "" }
                $stderr = if (Test-Path $stderrPath) { Get-Content $stderrPath -Raw } else { "" }
                $text = ($stdout + $stderr).Trim()

                if ($process.ExitCode -ne 0) {
                    throw $text
                }

                return $text
            } finally {
                Remove-Item -LiteralPath $stdoutPath -ErrorAction SilentlyContinue
                Remove-Item -LiteralPath $stderrPath -ErrorAction SilentlyContinue
            }
        }

        & $FilePath @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "$FilePath exited with code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
        $env:PATH = $previousPath
        if ($null -ne $previousCargoTargetDir -and $previousCargoTargetDir -ne "") {
            $env:CARGO_TARGET_DIR = $previousCargoTargetDir
        } else {
            Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
        }
    }
}

function Invoke-CargoRun {
    param(
        [string]$Package,
        [string[]]$CliArgs,
        [switch]$CaptureOutput
    )

    $sidecar = Join-Path $script:SidecarDir "$Package.exe"
    if (Test-Path -LiteralPath $sidecar) {
        Invoke-ExternalCommand -FilePath $sidecar -Arguments $CliArgs -CaptureOutput:$CaptureOutput
    } else {
        $arguments = @() + $script:CargoPrefix + @("run", "-p", $Package, "--") + $CliArgs
        Invoke-ExternalCommand -FilePath $script:CargoPath -Arguments $arguments -CaptureOutput:$CaptureOutput
    }
}

function Invoke-NodeCommand {
    param([string[]]$CommandArgs, [switch]$CaptureOutput)
    $args = @(
        "--config", $script:LocalNodeConfig,
        "--data-dir", $script:ChainDir,
        "--mempool-dir", $script:MempoolDir
    ) + $CommandArgs
    Invoke-CargoRun -Package "alvenqis-node" -CliArgs $args -CaptureOutput:$CaptureOutput
}

function Invoke-WalletCommand {
    param([string[]]$CommandArgs, [switch]$CaptureOutput)
    $args = @(
        "--network", "mainnet-candidate",
        "--wallet-dir", $script:WalletDir,
        "--signed-tx-dir", $script:SignedTxDir,
        "--rpc-base-url", $script:RpcUrl,
        "--chain-data-dir", $script:ChainDir
    ) + $CommandArgs
    Invoke-CargoRun -Package "alvenqis-wallet" -CliArgs $args -CaptureOutput:$CaptureOutput
}

function Invoke-IndexerCommand {
    param([string[]]$CommandArgs, [switch]$CaptureOutput)
    $args = @(
        "--network", "mainnet-candidate",
        "--chain-data-dir", $script:ChainDir,
        "--index-dir", $script:IndexerDir
    ) + $CommandArgs
    Invoke-CargoRun -Package "alvenqis-indexer" -CliArgs $args -CaptureOutput:$CaptureOutput
}

function Invoke-RpcStartCommandString {
    $sidecar = Join-Path $script:SidecarDir "alvenqis-rpc-gateway.exe"
    if (Test-Path -LiteralPath $sidecar) {
        return "& " + (Quote-PowerShell $sidecar) + " --config " + (Quote-PowerShell $script:LocalRpcConfig)
    }
    "& " + (Quote-PowerShell $script:CargoPath) + " " + (Join-PowerShellArgs (@() + $script:CargoPrefix + @(
        "run", "-p", "alvenqis-rpc-gateway", "--", "--config", $script:LocalRpcConfig
    )))
}

function Invoke-NodeStartCommandString {
    $sidecar = Join-Path $script:SidecarDir "alvenqis-node.exe"
    if (Test-Path -LiteralPath $sidecar) {
        return "& " + (Quote-PowerShell $sidecar) + " " + (Join-PowerShellArgs @(
            "--config", $script:LocalNodeConfig,
            "--data-dir", $script:ChainDir,
            "--mempool-dir", $script:MempoolDir,
            "start-node"
        ))
    }
    "& " + (Quote-PowerShell $script:CargoPath) + " " + (Join-PowerShellArgs (@() + $script:CargoPrefix + @(
        "run", "-p", "alvenqis-node", "--",
        "--config", $script:LocalNodeConfig,
        "--data-dir", $script:ChainDir,
        "--mempool-dir", $script:MempoolDir,
        "start-node"
    )))
}

function Get-ExplorerStartCommandString {
    $viteCli = Join-Path $script:ExplorerDir "node_modules\vite\bin\vite.js"
    @(
        "if (-not (Test-Path " + (Quote-PowerShell (Join-Path $script:ExplorerDir "node_modules")) + ")) { & npm install }",
        '$env:VITE_ALVENQIS_RPC_URL = ' + (Quote-PowerShell $script:RpcUrl) + ";",
        "& node " + (Quote-PowerShell $viteCli) + " --host 127.0.0.1 --port 4173"
    ) -join "; "
}

function Start-BackgroundPowerShellCommand {
    param(
        [string]$Name,
        [string]$Command,
        [string]$WorkingDirectory = $script:WorkspaceRoot
    )

    Ensure-LocalDirectories
    if (Test-ManagedProcess $Name) {
        Write-Host "$Name is already running with pid $(Get-StoredPid $Name)."
        return
    }

    $wrapper = @(
        '$ErrorActionPreference = ''Stop''',
        "Set-Location " + (Quote-PowerShell $WorkingDirectory),
        '$env:PATH = ' + (Quote-PowerShell $script:ManagedPathValue),
        '$env:CARGO_TARGET_DIR = ' + (Quote-PowerShell $script:BuildDir),
        $Command
    ) -join [Environment]::NewLine

    $bootstrapPath = Get-BootstrapFilePath $Name
    Set-Content -Path $bootstrapPath -Value $wrapper -Encoding UTF8

    $process = Start-Process -FilePath "powershell.exe" `
        -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $bootstrapPath) `
        -WorkingDirectory $WorkingDirectory `
        -RedirectStandardOutput (Get-LogFilePath $Name) `
        -RedirectStandardError (Get-ErrLogFilePath $Name) `
        -PassThru `
        -WindowStyle Hidden

    Set-Content -Path (Get-PidFilePath $Name) -Value $process.Id
    Write-Host "Started $Name pid=$($process.Id) log=$(Get-LogFilePath $Name)"
}

function Assert-ManagedProcessRunning {
    param([string]$Name)

    if (Test-ManagedProcess $Name) {
        return
    }

    $stderr = Get-RecentLogText -Name $Name -ErrorLog
    $stdout = Get-RecentLogText -Name $Name
    $details = @()
    if ($stderr) {
        $details += "stderr:"
        $details += $stderr
    }
    if ($stdout) {
        $details += "stdout:"
        $details += $stdout
    }

    $message = "$Name failed to stay running."
    if ($details.Count -gt 0) {
        $message += [Environment]::NewLine + ($details -join [Environment]::NewLine)
    }

    throw $message
}

function Wait-Until {
    param(
        [scriptblock]$Condition,
        [int]$TimeoutSeconds,
        [string]$Description
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        if (& $Condition) {
            return
        }
        Start-Sleep -Seconds 1
    }

    throw "Timed out waiting for $Description"
}

function Wait-ForHttpReady {
    param(
        [string]$Url,
        [int]$TimeoutSeconds = 60
    )

    Wait-Until -TimeoutSeconds $TimeoutSeconds -Description $Url -Condition {
        try {
            $response = Invoke-WebRequest -UseBasicParsing -Uri $Url -TimeoutSec 3
            return $response.StatusCode -ge 200 -and $response.StatusCode -lt 500
        } catch {
            return $false
        }
    }
}

function Refresh-IndexSnapshot {
    Ensure-LocalDirectories
    try {
        $output = Invoke-IndexerCommand -CommandArgs @("index-chain") -CaptureOutput
        Set-Content -Path (Get-LogFilePath "indexer-refresh") -Value $output
        Remove-Item (Get-ErrLogFilePath "indexer-refresh") -ErrorAction SilentlyContinue
        return $output
    } catch {
        Set-Content -Path (Get-ErrLogFilePath "indexer-refresh") -Value $_.Exception.Message
        throw
    }
}

function Stop-ManagedProcess {
    param([string]$Name)
    if (-not (Test-ManagedProcess $Name)) {
        Remove-ManagedRuntimeFiles $Name
        return
    }
    $pidValue = Get-StoredPid $Name
    if ($null -eq $pidValue) {
        return
    }
    try {
        Stop-Process -Id $pidValue -Force -ErrorAction Stop
        Write-Host "Stopped $Name pid=$pidValue"
    } catch {
        Write-Warning "Failed to stop $Name pid=${pidValue}: $($_.Exception.Message)"
    } finally {
        Remove-ManagedRuntimeFiles $Name
    }
}

function Stop-WorkspaceBinaryProcess {
    param([string]$ProcessName)

    $expectedPrefixes = @(
        [System.IO.Path]::GetFullPath($script:BuildDir),
        [System.IO.Path]::GetFullPath($script:SidecarDir)
    )
    $candidates = Get-CimInstance Win32_Process -Filter ("Name = '" + $ProcessName + ".exe'") -ErrorAction SilentlyContinue
    foreach ($candidate in @($candidates)) {
        if (-not $candidate.ExecutablePath) {
            continue
        }

        $fullPath = [System.IO.Path]::GetFullPath($candidate.ExecutablePath)
        $isManagedBinary = $expectedPrefixes | Where-Object {
            $fullPath.StartsWith($_, [System.StringComparison]::OrdinalIgnoreCase)
        }
        if (-not $isManagedBinary) {
            continue
        }

        try {
            Stop-Process -Id $candidate.ProcessId -Force -ErrorAction Stop
            Write-Host "Stopped $ProcessName.exe pid=$($candidate.ProcessId)"
        } catch {
            Write-Warning "Failed to stop $ProcessName.exe pid=$($candidate.ProcessId): $($_.Exception.Message)"
        }
    }
}

function Stop-NodeProcess {
    if (-not (Test-ManagedProcess "node")) {
        Stop-WorkspaceBinaryProcess "alvenqis-node"
        return
    }

    try {
        $null = Invoke-NodeCommand -CommandArgs @("shutdown") -CaptureOutput
    } catch {
        Write-Warning "Graceful node shutdown failed: $($_.Exception.Message)"
    }

    try {
        Wait-Until -TimeoutSeconds 15 -Description "node shutdown" -Condition {
            -not (Test-ManagedProcess "node")
        }
    } catch {
        Stop-ManagedProcess "node"
    } finally {
        Stop-WorkspaceBinaryProcess "alvenqis-node"
    }
}

function Stop-ExplorerProcess {
    Stop-ManagedProcess "explorer"

    $viteCli = (Join-Path $script:ExplorerDir "node_modules\vite\bin\vite.js").ToLowerInvariant()
    $explorerRoot = $script:ExplorerDir.ToLowerInvariant()
    $candidates = Get-CimInstance Win32_Process -Filter "Name = 'node.exe'" -ErrorAction SilentlyContinue
    foreach ($candidate in @($candidates)) {
        if (-not $candidate.CommandLine) {
            continue
        }

        $commandLine = $candidate.CommandLine.ToLowerInvariant()
        if ($commandLine.Contains($viteCli) -or $commandLine.Contains($explorerRoot)) {
            try {
                Stop-Process -Id $candidate.ProcessId -Force -ErrorAction Stop
                Write-Host "Stopped explorer node.exe pid=$($candidate.ProcessId)"
            } catch {
                Write-Warning "Failed to stop explorer node.exe pid=$($candidate.ProcessId): $($_.Exception.Message)"
            }
        }
    }
}

function Invoke-ReleaseGate {
    $scriptPath = Join-Path $script:WorkspaceRoot "scripts\release\release-gate.ps1"
    & $scriptPath
    if ($LASTEXITCODE -ne 0) {
        throw "release gate failed"
    }
}

function Get-LatestBackupPath {
    if (-not (Test-Path $script:BackupsDir)) {
        return $null
    }
    Get-ChildItem -Path $script:BackupsDir -Directory -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTimeUtc -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}

function Backup-LocalData {
    param([switch]$IncludeWallets)

    Ensure-LocalDirectories
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $backupPath = Join-Path $script:BackupsDir "local-backup-$timestamp"
    New-Item -ItemType Directory -Force -Path $backupPath | Out-Null

    $chainDatabase = Join-Path $script:ChainDir "chain.sqlite3"
    $legacyChain = Join-Path $script:ChainDir "chain.jsonl"
    if ((Test-Path -LiteralPath $chainDatabase) -or (Test-Path -LiteralPath $legacyChain)) {
        $chainBackupDir = Join-Path $backupPath "chain"
        New-Item -ItemType Directory -Force -Path $chainBackupDir | Out-Null
        Invoke-NodeCommand -CommandArgs @(
            "backup-chain-database", "--output", (Join-Path $chainBackupDir "chain.sqlite3")
        ) | Out-Null
        if (Test-Path -LiteralPath $legacyChain) {
            Copy-Item -LiteralPath $legacyChain -Destination (Join-Path $chainBackupDir "chain.jsonl") -Force
        }
    }

    foreach ($entry in @(
        @{ Source = $script:MempoolDir; Name = "mempool" },
        @{ Source = $script:IndexerDir; Name = "indexer" },
        @{ Source = $script:LogsDir; Name = "logs" }
    )) {
        if (Test-Path $entry.Source) {
            Copy-Item -LiteralPath $entry.Source -Destination (Join-Path $backupPath $entry.Name) -Recurse -Force
        }
    }

    $genesisMarker = Join-Path $script:LocalRoot "genesis-info.json"
    if (Test-Path $genesisMarker) {
        Copy-Item -LiteralPath $genesisMarker -Destination (Join-Path $backupPath "genesis-info.json") -Force
    }
    if ($IncludeWallets) {
        Write-Warning "Wallet backup is intentionally excluded from repository-local backups. Back up $script:WalletDir separately to encrypted offline storage."
    }

    $backupPath
}

function Clear-LocalChainState {
    Ensure-LocalDirectories
    foreach ($path in @($script:ChainDir, $script:MempoolDir, $script:IndexerDir)) {
        if (Test-Path $path) {
            Remove-Item -LiteralPath $path -Recurse -Force
        }
        New-Item -ItemType Directory -Force -Path $path | Out-Null
    }

    $genesisMarker = Join-Path $script:LocalRoot "genesis-info.json"
    Remove-Item -LiteralPath $genesisMarker -ErrorAction SilentlyContinue
    Remove-ManagedRuntimeFiles "node"
    Remove-ManagedRuntimeFiles "rpc"
    Remove-ManagedRuntimeFiles "explorer"
}

function Show-LocalSummary {
    Write-Host "Local root: $script:LocalRoot"
    Write-Host "Chain dir: $script:ChainDir"
    Write-Host "Mempool dir: $script:MempoolDir"
    Write-Host "Indexer dir: $script:IndexerDir"
    Write-Host "Wallet dir: $script:WalletDir"
    Write-Host "Logs dir: $script:LogsDir"
    Write-Host "RPC URL: $script:RpcUrl"
    Write-Host "Explorer URL: $script:ExplorerUrl"
}
