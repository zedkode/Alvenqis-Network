# Prepares keystore helper + optional Windows sidecars for Tauri packaging.
# Usage:
#   .\scripts\prepare-native.ps1
#   .\scripts\prepare-native.ps1 -WithSidecars

param(
  [switch]$WithSidecars
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Repo = Split-Path -Parent $Root
$WorkspaceRoot = Split-Path -Parent $Repo
$HelperManifest = Join-Path $Root "native\keystore-helper\Cargo.toml"
$BinDir = Join-Path $Root "src-tauri\binaries"
$ResDir = Join-Path $Root "src-tauri\resources"
$ResBin = Join-Path $ResDir "bin"
$TargetTriple = "x86_64-pc-windows-msvc"
$TauriManifest = Join-Path $Root "src-tauri\Cargo.toml"
$DefaultTauriTarget = Join-Path $Root "src-tauri\target"

# Cargo build-script artifacts embed their OUT_DIR as an absolute path. After
# moving this app inside the monorepo, Cargo can otherwise reuse a valid-looking
# build script that still points to the old checkout and Tauri fails while
# loading generated plugin permissions. Only clean the app-owned default target
# when stale root-output markers are present. Respect an explicit shared
# CARGO_TARGET_DIR instead of cleaning a directory this script does not own.
if (-not $env:CARGO_TARGET_DIR -and (Test-Path -LiteralPath $DefaultTauriTarget)) {
  $expectedTargetPrefix = [System.IO.Path]::GetFullPath($DefaultTauriTarget).TrimEnd("\") + "\"
  $staleRootOutput = Get-ChildItem -LiteralPath $DefaultTauriTarget -Filter "root-output" -File -Recurse -ErrorAction SilentlyContinue |
    Where-Object {
      $recordedPath = (Get-Content -LiteralPath $_.FullName -Raw -ErrorAction SilentlyContinue).Trim()
      if ($recordedPath.StartsWith("\\?\")) {
        $recordedPath = $recordedPath.Substring(4)
      }
      $recordedPath -and -not $recordedPath.StartsWith(
        $expectedTargetPrefix,
        [System.StringComparison]::OrdinalIgnoreCase
      )
    } |
    Select-Object -First 1

  if ($staleRootOutput) {
    Write-Host "==> Cleaning stale Tauri Cargo cache left by a previous repository path"
    cargo clean --manifest-path $TauriManifest
    if ($LASTEXITCODE -ne 0) { throw "stale Tauri cache cleanup failed" }
  }
}

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $ResDir | Out-Null
New-Item -ItemType Directory -Force -Path $ResBin | Out-Null

# Never mix Linux ELF sidecars into a Windows bundle.
foreach ($staleLinuxBinary in @(
  "alvenqis-keystore-helper",
  "alvenqis-miner",
  "alvenqis-node",
  "alvenqis-rpc-gateway",
  "alvenqis-indexer"
)) {
  Remove-Item -LiteralPath (Join-Path $BinDir $staleLinuxBinary) -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath (Join-Path $ResBin $staleLinuxBinary) -Force -ErrorAction SilentlyContinue
}

Write-Host "==> Building alvenqis-keystore-helper (release)"
cargo build --release --locked --manifest-path $HelperManifest
if ($LASTEXITCODE -ne 0) { throw "keystore helper build failed" }

$HelperSrc = Join-Path $Root "native\keystore-helper\target\release\alvenqis-keystore-helper.exe"
if (-not (Test-Path $HelperSrc)) {
  throw "Missing built helper: $HelperSrc"
}

$HelperDst = Join-Path $BinDir "alvenqis-keystore-helper-$TargetTriple.exe"
Copy-Item $HelperSrc $HelperDst -Force
Copy-Item $HelperSrc (Join-Path $BinDir "alvenqis-keystore-helper.exe") -Force
Copy-Item $HelperSrc (Join-Path $ResBin "alvenqis-keystore-helper.exe") -Force
Write-Host "Staged keystore helper -> $HelperDst"

$repoOperator = Join-Path $Repo "alvenqis.ps1"
if (Test-Path $repoOperator) {
  Copy-Item $repoOperator (Join-Path $ResDir "alvenqis.ps1") -Force
  Write-Host "  + alvenqis.ps1"
}

$logoCandidates = @(
  (Join-Path $Root "logo.png"),
  (Join-Path $Repo "logo.png"),
  (Join-Path $Repo "shared\brand\logo-mark.png")
)
foreach ($logo in $logoCandidates) {
  if (Test-Path $logo) {
    Copy-Item $logo (Join-Path $Root "public\logo.png") -Force
    break
  }
}

$configsDir = Join-Path $ResDir "configs"
New-Item -ItemType Directory -Force -Path $configsDir | Out-Null
foreach ($configName in @(
  "mainnet-candidate.toml",
  "genesis.mainnet-candidate.toml",
  "local.toml",
  "rpc.mainnet-candidate.toml",
  "rpc.local.toml"
)) {
  $configSource = Join-Path $Repo "configs\$configName"
  if (Test-Path -LiteralPath $configSource) {
    Copy-Item -LiteralPath $configSource -Destination (Join-Path $configsDir $configName) -Force
    Write-Host "  + configs\$configName"
  }
}

$releaseDocsSource = Join-Path $WorkspaceRoot "Blockchain-docs\human\release"
$releaseDocsDestination = Join-Path $ResDir "docs\release"
New-Item -ItemType Directory -Force -Path $releaseDocsDestination | Out-Null
foreach ($releaseFile in @(
  "GENESIS_APPROVAL.mainnet-candidate.json",
  "GENESIS_REVIEW.mainnet-candidate.json",
  "genesis.mainnet-candidate.block.json",
  "NETWORK_MATURITY.md"
)) {
  $releaseSource = Join-Path $releaseDocsSource $releaseFile
  if (-not (Test-Path -LiteralPath $releaseSource)) {
    throw "Missing required release document: $releaseSource"
  }
  Copy-Item -LiteralPath $releaseSource -Destination (Join-Path $releaseDocsDestination $releaseFile) -Force
  Write-Host "  + docs\release\$releaseFile"
}

if ($WithSidecars) {
  Write-Host "==> Building monorepo release sidecars (Windows)"
  Push-Location $Repo
  try {
    # Release sidecars must contain real CUDA kernels; no stub/fallback is shippable.
    $env:ALVENQIS_REQUIRE_CUDA = "1"
    cargo build --release --locked -p alvenqis-miner
    if ($LASTEXITCODE -ne 0) { throw "CUDA-enabled alvenqis-miner build failed" }
    cargo build --release --locked -p alvenqis-node -p alvenqis-rpc-gateway -p alvenqis-indexer
    if ($LASTEXITCODE -ne 0) { throw "Alvenqis sidecar build failed" }
  } finally {
    Pop-Location
  }

  $bins = @("alvenqis-miner", "alvenqis-node", "alvenqis-rpc-gateway", "alvenqis-indexer")
  foreach ($name in $bins) {
    $src = Join-Path $Repo "target\release\$name.exe"
    if (Test-Path $src) {
      Copy-Item $src (Join-Path $ResBin "$name.exe") -Force
      Write-Host "  + bin\$name.exe"
    } else {
      Write-Host "  ! missing $name.exe"
    }
  }

  $manifest = @{
    prepared_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    platform = "windows"
    keystore_helper = "bin/alvenqis-keystore-helper.exe"
    binaries = @(
      "bin/alvenqis-miner.exe",
      "bin/alvenqis-node.exe",
      "bin/alvenqis-rpc-gateway.exe",
      "bin/alvenqis-indexer.exe"
    )
    operator = "alvenqis.ps1"
    mining_backend = "cuda"
    cpu_mining = $false
    opencl_mining = $false
  } | ConvertTo-Json
  Set-Content -Path (Join-Path $ResDir "MANIFEST.json") -Value $manifest -Encoding UTF8
  Write-Host "Wrote resources/MANIFEST.json"
}

Write-Host "Native preparation complete."
