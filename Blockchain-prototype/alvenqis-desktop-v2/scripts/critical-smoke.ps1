# Critical-path smoke for Tauri Control Center (no UI).
# Usage: .\scripts\critical-smoke.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Repo = Split-Path -Parent $Root
$Res = Join-Path $Root "src-tauri\resources"

function Assert-True($Condition, $Message) {
  if (-not $Condition) { throw $Message }
}

Write-Host "==> prepare:native"
Push-Location $Root
try {
  npm run prepare:native
  if ($LASTEXITCODE -ne 0) { throw "prepare:native failed" }

  Write-Host "==> TypeScript check"
  npx tsc --noEmit
  if ($LASTEXITCODE -ne 0) { throw "tsc failed" }

  Write-Host "==> cargo test health (src-tauri)"
  Push-Location (Join-Path $Root "src-tauri")
  try {
    cargo test --lib health::tests -- --nocapture
    if ($LASTEXITCODE -ne 0) { throw "cargo health tests failed" }
    cargo check
    if ($LASTEXITCODE -ne 0) { throw "cargo check failed" }
  } finally {
    Pop-Location
  }

  $helper = Join-Path $Root "src-tauri\binaries\alvenqis-keystore-helper.exe"
  Assert-True (Test-Path $helper) "Keystore helper not staged: $helper"

  Write-Host "==> keystore helper self-call (metadata)"
  $json = '{"command":"metadata"}'
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = $helper
  $psi.RedirectStandardInput = $true
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.UseShellExecute = $false
  $psi.CreateNoWindow = $true
  $psi.EnvironmentVariables["ALVENQIS_RPC_URL"] = "https://rpcnode.dohotstudio.com"
  $p = [System.Diagnostics.Process]::Start($psi)
  $p.StandardInput.Write($json)
  $p.StandardInput.Close()
  $stdout = $p.StandardOutput.ReadToEnd()
  $stderr = $p.StandardError.ReadToEnd()
  $p.WaitForExit()
  if ($p.ExitCode -ne 0) {
    throw "keystore helper metadata failed ($($p.ExitCode)): $stderr"
  }
  Write-Host "keystore helper OK (stdout length $($stdout.Length))"

  $operator = Join-Path $Repo "alvenqis.ps1"
  Assert-True (Test-Path $operator) "Monorepo operator missing: $operator"

  Write-Host "==> prepare:native:sidecars (if not already staged)"
  npm run prepare:native:sidecars
  if ($LASTEXITCODE -ne 0) { throw "prepare:native:sidecars failed" }

  Write-Host "==> validate staged resources"
  $required = @(
    "bin\alvenqis-node.exe",
    "bin\alvenqis-rpc-gateway.exe",
    "bin\alvenqis-miner.exe",
    "bin\alvenqis-indexer.exe",
    "bin\alvenqis-keystore-helper.exe",
    "alvenqis.ps1",
    "scripts\local\alvenqis-local.ps1",
    "scripts\local\common.ps1",
    "configs\mainnet-candidate.toml",
    "MANIFEST.json"
  )
  foreach ($rel in $required) {
    $path = Join-Path $Res $rel
    Assert-True (Test-Path $path) "Staged resources missing: $rel"
  }
  Write-Host "staged resources OK"

  Write-Host "==> operator status against staged resources"
  $local = Join-Path $env:TEMP "alvenqis-tauri-critical\.alvenqis-local"
  New-Item -ItemType Directory -Force -Path $local | Out-Null
  $prevWs = $env:ALVENQIS_WORKSPACE_ROOT
  $prevLocal = $env:ALVENQIS_LOCAL_ROOT
  try {
    $env:ALVENQIS_WORKSPACE_ROOT = $Res
    $env:ALVENQIS_LOCAL_ROOT = $local
    powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Res "alvenqis.ps1") status | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "staged operator status failed" }
    Write-Host "staged operator status OK"
  } finally {
    if ($null -eq $prevWs) { Remove-Item Env:ALVENQIS_WORKSPACE_ROOT -ErrorAction SilentlyContinue }
    else { $env:ALVENQIS_WORKSPACE_ROOT = $prevWs }
    if ($null -eq $prevLocal) { Remove-Item Env:ALVENQIS_LOCAL_ROOT -ErrorAction SilentlyContinue }
    else { $env:ALVENQIS_LOCAL_ROOT = $prevLocal }
  }

  $p2pBusy = $false
  try {
    $conn = Get-NetTCPConnection -LocalPort 20787 -State Listen -ErrorAction SilentlyContinue
    if ($conn) { $p2pBusy = $true }
  } catch { }

  Write-Host ""
  Write-Host "CRITICAL SMOKE PASSED"
  if ($p2pBusy) {
    Write-Host "NOTE: port 20787 is currently in use (existing Control Center install)."
    Write-Host "      Stack start cannot be smoke-tested until that process is stopped."
  } else {
    Write-Host "Ports free: stack start can be tested with staged resources."
  }
  Write-Host "Next: npm run tauri:dev  |  npm run tauri:build"
}
finally {
  Pop-Location
}
