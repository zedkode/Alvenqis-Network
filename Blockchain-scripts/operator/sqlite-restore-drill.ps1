# Task 3 Drill A/B: SQLite online backup + isolated restore (+ optional disk-failure sim).
# Mainnet Candidate / Prototype only — not G4 launch approval.
[CmdletBinding()]
param(
    [switch]$SimulateDiskFailure,
    [switch]$ConfirmLiveRestore,
    [switch]$SkipValidateChain
)

$ErrorActionPreference = 'Stop'
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
. (Join-Path $scriptDir '..\local\common.ps1')

function Fail([string]$Message) {
    Write-Error "FAIL: $Message"
    exit 1
}

function Write-Step([string]$Message) {
    Write-Host ""
    Write-Host "==> $Message"
}

Ensure-LocalDirectories
$chainDb = Join-Path $script:ChainDir 'chain.sqlite3'
$evidenceRoot = Join-Path $script:LocalRoot 'maturity-evidence'
New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null
$stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
$drillDir = Join-Path $evidenceRoot "sqlite-restore-$stamp"
New-Item -ItemType Directory -Force -Path $drillDir | Out-Null

$evidence = [ordered]@{
    drill            = 'sqlite-restore'
    utc              = $stamp
    label            = 'Mainnet Candidate / Prototype'
    g4_waiver        = $false
    chain_dir        = $script:ChainDir
    drill_dir        = $drillDir
    simulate_disk    = [bool]$SimulateDiskFailure
    steps            = @()
    pass             = $false
}

function Add-Step([string]$Name, [bool]$Ok, [string]$Detail = '') {
    $evidence.steps += [ordered]@{ name = $Name; ok = $Ok; detail = $Detail }
    if ($Ok) { Write-Host "  OK  $Name$(if ($Detail) { " — $Detail" })" }
    else { Write-Host "  FAIL $Name$(if ($Detail) { " — $Detail" })" }
}

Write-Host 'Alvenqis Task 3 — SQLite restore drill'
Write-Host ("UTC: {0}" -f $stamp)
Write-Host ("chain_dir: {0}" -f $script:ChainDir)
Write-Host ("drill_dir: {0}" -f $drillDir)
Write-Host 'NOTE: This is rehearsal evidence, not Mainnet Live / G4 approval.'

if (-not (Test-Path -LiteralPath $chainDb)) {
    Fail "No chain.sqlite3 at $chainDb. Initialize local candidate chain first (LOCAL_RUNBOOK / start-all)."
}

# --- preflight integrity ---
Write-Step 'Preflight verify-chain-database'
try {
    Invoke-NodeCommand -CommandArgs @('verify-chain-database') | Out-Null
    Add-Step 'preflight_integrity' $true 'verify-chain-database'
} catch {
    Add-Step 'preflight_integrity' $false "$_"
    Fail "Live chain failed integrity check: $_"
}

# capture tip via status if possible
$preTip = $null
$preHeight = $null
try {
    $statusOut = Invoke-NodeCommand -CommandArgs @('status') -CaptureOutput
    if ($statusOut -match 'tip[_\s-]*hash[=:\s]+([0-9a-fA-F]{64})') { $preTip = $Matches[1] }
    if ($statusOut -match 'height[=:\s]+(\d+)') { $preHeight = [int]$Matches[1] }
    $evidence.pre_status_snippet = if ($statusOut.Length -gt 500) { $statusOut.Substring(0, 500) } else { $statusOut }
} catch {
    Write-Host "  (status optional) $_"
}
$evidence.pre_tip_hash = $preTip
$evidence.pre_height = $preHeight

# --- online backup ---
Write-Step 'Online backup-chain-database'
$backupDir = Join-Path $drillDir 'online-backup'
New-Item -ItemType Directory -Force -Path $backupDir | Out-Null
$backupDb = Join-Path $backupDir 'chain.sqlite3'
try {
    Invoke-NodeCommand -CommandArgs @('backup-chain-database', '--output', $backupDb) | Out-Null
    if (-not (Test-Path -LiteralPath $backupDb)) { throw 'backup file missing after command' }
    Add-Step 'online_backup' $true $backupDb
} catch {
    Add-Step 'online_backup' $false "$_"
    Fail "Online backup failed: $_"
}

# --- isolated restore ---
Write-Step 'Isolated restore + integrity'
$restoreDir = Join-Path $drillDir 'isolated-restore'
New-Item -ItemType Directory -Force -Path $restoreDir | Out-Null
$restoreDb = Join-Path $restoreDir 'chain.sqlite3'
Copy-Item -LiteralPath $backupDb -Destination $restoreDb -Force
try {
    # verify against restore data-dir
    $args = @(
        '--config', $script:LocalNodeConfig,
        '--data-dir', $restoreDir,
        '--mempool-dir', (Join-Path $drillDir 'isolated-mempool'),
        'verify-chain-database'
    )
    Invoke-CargoRun -Package 'alvenqis-node' -CliArgs $args | Out-Null
    Add-Step 'isolated_integrity' $true $restoreDb
} catch {
    Add-Step 'isolated_integrity' $false "$_"
    Fail "Isolated restore integrity failed: $_"
}

if (-not $SkipValidateChain) {
    Write-Step 'Isolated validate-chain'
    try {
        $vArgs = @(
            '--config', $script:LocalNodeConfig,
            '--data-dir', $restoreDir,
            '--mempool-dir', (Join-Path $drillDir 'isolated-mempool'),
            'validate-chain'
        )
        Invoke-CargoRun -Package 'alvenqis-node' -CliArgs $vArgs | Out-Null
        Add-Step 'isolated_validate_chain' $true
    } catch {
        Add-Step 'isolated_validate_chain' $false "$_"
        Write-Warning "validate-chain failed on restore (recorded): $_"
    }
}

# --- optional disk failure on live ---
if ($SimulateDiskFailure) {
    Write-Step 'Disk-failure simulation (LIVE)'
    if (-not $ConfirmLiveRestore) {
        Fail 'Refusing live disk-failure sim without -ConfirmLiveRestore'
    }
    if (Test-ManagedProcess 'node') {
        Write-Host '  Stopping managed local node...'
        & (Join-Path $scriptDir '..\local\stop-all.ps1') -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
    }
    $failedName = "chain.sqlite3.failed-$stamp"
    $failedPath = Join-Path $script:ChainDir $failedName
    Move-Item -LiteralPath $chainDb -Destination $failedPath -Force
    Copy-Item -LiteralPath $backupDb -Destination $chainDb -Force
    try {
        Invoke-NodeCommand -CommandArgs @('verify-chain-database') | Out-Null
        Add-Step 'live_disk_failure_restore' $true "failed_saved_as=$failedName"
    } catch {
        Add-Step 'live_disk_failure_restore' $false "$_"
        # attempt roll back
        if (Test-Path -LiteralPath $failedPath) {
            Move-Item -LiteralPath $failedPath -Destination $chainDb -Force
        }
        Fail "Live restore after simulated failure failed (attempted rollback): $_"
    }
    Write-Host '  Live DB restored from online backup. Restart local stack and check /status.'
}

$evidence.pass = -not ($evidence.steps | Where-Object { -not $_.ok } | Select-Object -First 1)
$evidencePath = Join-Path $drillDir 'evidence.json'
($evidence | ConvertTo-Json -Depth 6) | Set-Content -Path $evidencePath -Encoding utf8
Write-Host ""
Write-Host ("evidence: {0}" -f $evidencePath)
if ($evidence.pass) {
    Write-Host 'PASS: Drill A SQLite backup/restore evidence recorded (not G4).'
    exit 0
} else {
    Write-Host 'FAIL: one or more drill steps failed — see evidence.json'
    exit 1
}
