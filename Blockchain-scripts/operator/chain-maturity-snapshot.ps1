# Task 3 public/local chain maturity snapshot (read-only). No secrets.
# Records height/tip/index lag for SESSION_LOG evidence. Not G4 approval.
[CmdletBinding()]
param(
    [string]$RpcUrl = $(if ($env:ALVENQIS_RPC_URL) { $env:ALVENQIS_RPC_URL } else { 'https://rpcnode.dohotstudio.com' }),
    [string]$OutDir = ''
)

$ErrorActionPreference = 'Stop'
$RpcUrl = $RpcUrl.TrimEnd('/')
$stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')

function Fail([string]$Message) {
    Write-Error "FAIL: $Message"
    exit 1
}

Write-Host 'Alvenqis Task 3 — chain maturity snapshot'
Write-Host ("RPC: {0}" -f $RpcUrl)
Write-Host ("UTC: {0}" -f $stamp)

try {
    $health = Invoke-RestMethod -Uri "$RpcUrl/health" -TimeoutSec 20
    $status = Invoke-RestMethod -Uri "$RpcUrl/status" -TimeoutSec 20
} catch {
    Fail "RPC request failed: $_"
}

if ($health.ok -ne $true) { Fail 'health.ok != true' }
if ($status.initialized -ne $true) { Fail 'status.initialized != true' }

$idx = $null
try {
    $idx = Invoke-RestMethod -Uri "$RpcUrl/indexer/status" -TimeoutSec 20
} catch {
    Write-Host 'indexer/status unavailable (warning)'
}

$snap = [ordered]@{
    utc              = $stamp
    rpc_url          = $RpcUrl
    label            = 'Mainnet Candidate / Prototype'
    g4_waiver        = $false
    health_mode      = [string]$health.mode
    network_id       = [string]$status.network_id
    height           = $status.height
    tip_hash         = [string]$status.tip_hash
    block_count      = $status.block_count
    cumulative_work  = [string]$status.cumulative_work
    index_in_sync    = $status.index_in_sync
    index_lag_blocks = $status.index_lag_blocks
    index_height     = $status.index_height
    indexer_in_sync  = if ($idx) { $idx.in_sync } else { $null }
    pass             = $true
}

Write-Host ("height={0} tip={1}" -f $snap.height, $snap.tip_hash)
Write-Host ("index_in_sync={0} lag={1}" -f $snap.index_in_sync, $snap.index_lag_blocks)
Write-Host ("mode={0}" -f $snap.health_mode)

if ($OutDir) {
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
    $path = Join-Path $OutDir "chain-maturity-snapshot-$stamp.json"
    ($snap | ConvertTo-Json -Depth 4) | Set-Content -Path $path -Encoding utf8
    Write-Host ("wrote {0}" -f $path)
}

Write-Host 'PASS: snapshot captured (rehearsal only; not Mainnet Live).'
exit 0
