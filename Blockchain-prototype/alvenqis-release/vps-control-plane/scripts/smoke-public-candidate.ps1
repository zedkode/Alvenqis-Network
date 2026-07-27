# Public Mainnet Candidate smoke (Windows operator laptop). No SSH, no secrets.
# Exit 0 only when health + status pass and public mining stays disabled.
[CmdletBinding()]
param(
    [string]$BaseUrl = $(if ($env:ALVENQIS_PUBLIC_RPC) { $env:ALVENQIS_PUBLIC_RPC } else { 'https://rpcnode.dohotstudio.com' }),
    [string]$ExpectedGenesisTip = $(if ($env:ALVENQIS_EXPECTED_GENESIS_TIP) { $env:ALVENQIS_EXPECTED_GENESIS_TIP } else { '0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5' }),
    [string]$ExpectedNetworkId = $(if ($env:ALVENQIS_EXPECTED_NETWORK_ID) { $env:ALVENQIS_EXPECTED_NETWORK_ID } else { 'alvenqis-mainnet-candidate' })
)

$ErrorActionPreference = 'Stop'
$BaseUrl = $BaseUrl.TrimEnd('/')

function Fail {
    param([string]$Message)
    Write-Error "FAIL: $Message"
    exit 1
}

Write-Host "Public candidate smoke against: $BaseUrl"
Write-Host ("UTC: {0}" -f [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))

# --- /health ---
try {
    $health = Invoke-RestMethod -Uri "$BaseUrl/health" -Method Get -TimeoutSec 20
} catch {
    Fail "/health request failed: $_"
}

if ($health.ok -ne $true) {
    Fail ("/health ok != true: {0}" -f ($health | ConvertTo-Json -Compress))
}
$mode = [string]$health.mode
if ($mode -notmatch '(?i)mining disabled') {
    Fail "mode must mention mining disabled for public profile: $mode"
}
Write-Host ("health ok mode={0} network_id={1}" -f $mode, $health.network_id)

# --- /status ---
try {
    $status = Invoke-RestMethod -Uri "$BaseUrl/status" -Method Get -TimeoutSec 20
} catch {
    Fail "/status request failed: $_"
}

if ($status.initialized -ne $true) {
    Fail ("/status initialized != true: {0}" -f ($status | ConvertTo-Json -Compress))
}
if ($status.network_id -ne $ExpectedNetworkId) {
    Fail ("network_id {0} != {1}" -f $status.network_id, $ExpectedNetworkId)
}
if (-not $status.tip_hash) {
    Fail 'missing tip_hash'
}
if ($status.height -eq 0 -and ([string]$status.tip_hash).ToLowerInvariant() -ne $ExpectedGenesisTip.ToLowerInvariant()) {
    Fail ("height 0 tip {0} != pinned genesis {1}" -f $status.tip_hash, $ExpectedGenesisTip)
}
Write-Host ("status ok initialized=true network_id={0} height={1} tip_hash={2} index_in_sync={3} index_lag_blocks={4}" -f `
    $status.network_id, $status.height, $status.tip_hash, $status.index_in_sync, $status.index_lag_blocks)

# --- /mining/template must stay unavailable on public profile ---
$miningCode = $null
try {
    $resp = Invoke-WebRequest -Uri "$BaseUrl/mining/template" -Method Get -TimeoutSec 20 -UseBasicParsing
    $miningCode = [int]$resp.StatusCode
} catch {
    if ($_.Exception.Response) {
        $miningCode = [int]$_.Exception.Response.StatusCode
    } else {
        Fail "/mining/template request failed without HTTP status: $_"
    }
}

if ($miningCode -eq 200) {
    Fail '/mining/template returned 200 - public mining must stay disabled'
}
if ($miningCode -ge 500) {
    Fail ("/mining/template HTTP {0} (server error)" -f $miningCode)
}
if ($miningCode -ge 400 -and $miningCode -lt 500) {
    Write-Host ("mining template correctly unavailable HTTP {0}" -f $miningCode)
} else {
    Write-Host ("mining template not publicly available HTTP {0}" -f $miningCode)
}

Write-Host ("PASS: public Mainnet Candidate smoke OK ({0})" -f $BaseUrl)
Write-Host 'NOTE: This does not prove VPS monorepo revision, backup, or restore drills.'
exit 0
