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
    if ($script:DrillTranscriptStarted) {
        try { Stop-Transcript | Out-Null } catch {}
        $script:DrillTranscriptStarted = $false
    }
    Write-Error "FAIL: $Message"
    exit 1
}

function Write-Step([string]$Message) {
    Write-Host ""
    Write-Host "==> $Message"
}

function ConvertFrom-ValidationOutput([string]$Output) {
    $identity = [ordered]@{}
    foreach ($field in @('network_id', 'height', 'blocks', 'tip_hash')) {
        if ($Output -notmatch "(?:^|\s)$field=([^\s]+)") {
            throw "validate-chain output did not contain $field"
        }
        $identity[$field] = if ($field -in @('height', 'blocks')) {
            [UInt64]$Matches[1]
        } else {
            $Matches[1]
        }
    }
    $identity
}

Ensure-LocalDirectories
$chainDb = Join-Path $script:ChainDir 'chain.sqlite3'
$evidenceRoot = Join-Path $script:LocalRoot 'maturity-evidence'
New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null
$stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
$drillDir = Join-Path $evidenceRoot "sqlite-restore-$stamp"
New-Item -ItemType Directory -Force -Path $drillDir | Out-Null
$transcriptPath = Join-Path $drillDir 'drill.log'
$script:DrillTranscriptStarted = $false
Start-Transcript -Path $transcriptPath -Force | Out-Null
$script:DrillTranscriptStarted = $true

$evidence = [ordered]@{
    drill            = 'sqlite-restore'
    utc              = $stamp
    label            = 'Mainnet Candidate / Prototype'
    g4_waiver        = $false
    chain_dir        = $script:ChainDir
    drill_dir        = $drillDir
    transcript_log   = $transcriptPath
    simulate_disk    = [bool]$SimulateDiskFailure
    identity_verified = $false
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
Write-Host ("transcript: {0}" -f $transcriptPath)
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

# --- source identity ---
Write-Step 'Capture source chain identity'
try {
    $sourceValidation = Invoke-NodeCommand -CommandArgs @('validate-chain') -CaptureOutput
    Write-Host $sourceValidation
    $sourceIdentity = ConvertFrom-ValidationOutput $sourceValidation
    $evidence.source_identity = $sourceIdentity
    Add-Step 'source_identity' $true ("network_id={0} height={1} blocks={2} tip_hash={3}" -f
        $sourceIdentity.network_id, $sourceIdentity.height, $sourceIdentity.blocks, $sourceIdentity.tip_hash)
} catch {
    Add-Step 'source_identity' $false "$_"
    Fail "Source chain validation failed: $_"
}

# --- online backup ---
Write-Step 'Online backup-chain-database'
$backupDir = Join-Path $drillDir 'online-backup'
New-Item -ItemType Directory -Force -Path $backupDir | Out-Null
$backupDb = Join-Path $backupDir 'chain.sqlite3'
try {
    Invoke-NodeCommand -CommandArgs @('backup-chain-database', '--output', $backupDb) | Out-Null
    if (-not (Test-Path -LiteralPath $backupDb)) { throw 'backup file missing after command' }
    $backupSha256 = (Get-FileHash -LiteralPath $backupDb -Algorithm SHA256).Hash.ToLowerInvariant()
    $evidence.backup_db = $backupDb
    $evidence.backup_sha256 = $backupSha256
    Add-Step 'online_backup' $true "$backupDb sha256=$backupSha256"
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

$restoreSha256 = (Get-FileHash -LiteralPath $restoreDb -Algorithm SHA256).Hash.ToLowerInvariant()
$evidence.restore_db = $restoreDb
$evidence.restore_sha256 = $restoreSha256
if ($restoreSha256 -ne $backupSha256) {
    Add-Step 'restored_file_hash' $false "backup=$backupSha256 restore=$restoreSha256"
    Fail 'Restored SQLite copy hash does not match online backup'
}
Add-Step 'restored_file_hash' $true "sha256=$restoreSha256"

if (-not $SkipValidateChain) {
    Write-Step 'Isolated validate-chain + identity comparison'
    try {
        $vArgs = @(
            '--config', $script:LocalNodeConfig,
            '--data-dir', $restoreDir,
            '--mempool-dir', (Join-Path $drillDir 'isolated-mempool'),
            'validate-chain'
        )
        $restoreValidation = Invoke-CargoRun -Package 'alvenqis-node' -CliArgs $vArgs -CaptureOutput
        Write-Host $restoreValidation
        $restoreIdentity = ConvertFrom-ValidationOutput $restoreValidation
        $evidence.restore_identity = $restoreIdentity
        if (
            $restoreIdentity.network_id -ne $sourceIdentity.network_id -or
            $restoreIdentity.height -ne $sourceIdentity.height -or
            $restoreIdentity.blocks -ne $sourceIdentity.blocks -or
            $restoreIdentity.tip_hash -ne $sourceIdentity.tip_hash
        ) {
            throw 'restored chain identity does not match source'
        }
        $evidence.identity_verified = $true
        Add-Step 'restored_identity_match' $true ("network_id={0} height={1} blocks={2} tip_hash={3}" -f
            $restoreIdentity.network_id, $restoreIdentity.height, $restoreIdentity.blocks, $restoreIdentity.tip_hash)
    } catch {
        Add-Step 'restored_identity_match' $false "$_"
        Fail "Isolated validate-chain or identity comparison failed: $_"
    }
} else {
    Write-Host '  SKIP restored_identity_match — -SkipValidateChain'
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
    Stop-Transcript | Out-Null
    $script:DrillTranscriptStarted = $false
    exit 0
} else {
    Write-Host 'FAIL: one or more drill steps failed — see evidence.json'
    Stop-Transcript | Out-Null
    $script:DrillTranscriptStarted = $false
    exit 1
}
