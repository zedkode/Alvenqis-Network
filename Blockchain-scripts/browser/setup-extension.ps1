# End-to-end helper: build host, print install steps, optionally register.
#
# Usage:
#   .\scripts\browser\setup-extension.ps1
#   .\scripts\browser\setup-extension.ps1 -ExtensionId <id> -Browser Chrome -Build
#   .\scripts\browser\setup-extension.ps1 -ExtensionId <id> -LocalRpc
#   .\scripts\browser\setup-extension.ps1 -ExtensionId <id> -NoOsConfirm   # dev/test only
#
# OS confirm for send/sign/submit is ON by default (CR-C02).

[CmdletBinding()]
param(
    [string]$ExtensionId = "",

    [ValidateSet("Chrome", "Edge", "Brave", "All")]
    [string]$Browser = "Chrome",

    [switch]$Build,

    # Deprecated alias — host already requires OS confirm by default.
    [switch]$RequireOsConfirm,

    # Explicit opt-out for automated tests only (unsafe for funds).
    [switch]$NoOsConfirm,

    [switch]$LocalRpc
)

$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$ExtensionDir = Join-Path $RepoRoot "Blockchain-prototype\alvenqis-browser\extension"
$RegisterScript = Join-Path $PSScriptRoot "register-native-host.ps1"

Set-Location $RepoRoot

Write-Host "=== Alvenqis browser extension setup (Mainnet Candidate) ===" -ForegroundColor Cyan
Write-Host "Extension folder: $ExtensionDir"
Write-Host ""

if ($Build -or -not [string]::IsNullOrWhiteSpace($ExtensionId)) {
    Write-Host "Building alvenqis-browser-host (release)..."
    cargo build --manifest-path (Join-Path $RepoRoot "Blockchain-prototype\Cargo.toml") -p alvenqis-browser-host --release
    cargo run --manifest-path (Join-Path $RepoRoot "Blockchain-prototype\Cargo.toml") -q -p alvenqis-browser-host -- --print-info
    Write-Host ""
}

Write-Host "Step 1 — Load unpacked extension" -ForegroundColor Yellow
Write-Host "  Chrome/Edge: chrome://extensions  (or edge://extensions)"
Write-Host "  Enable Developer mode"
Write-Host "  Load unpacked -> select:"
Write-Host "    $ExtensionDir"
Write-Host "  Copy the Extension ID (32-char string under the extension name)."
Write-Host ""

if ([string]::IsNullOrWhiteSpace($ExtensionId)) {
    Write-Host "Step 2 — Register native host (after you have the ID)" -ForegroundColor Yellow
    Write-Host "  Re-run this script with -ExtensionId <id>  or:"
    Write-Host "  .\scripts\browser\register-native-host.ps1 -ExtensionId <id> -Build -Browser $Browser"
    Write-Host ""
    Write-Host "Optional flags: -LocalRpc  -Browser All  -NoOsConfirm (dev/test only)"
    Write-Host "  (OS confirm for send/sign/submit is ON by default)"
    Write-Host ""
    Write-Host "Recovery wallet (recommended BEFORE funding):" -ForegroundColor Yellow
    Write-Host '  cargo run --manifest-path (Join-Path $RepoRoot "Blockchain-prototype\Cargo.toml") -p alvenqis-browser-host -- --init-wallet --passphrase "your-long-passphrase"'
    Write-Host "  (write down the recovery phrase printed on stderr)"
    exit 0
}

Write-Host "Step 2 — Registering native host for ExtensionId=$ExtensionId Browser=$Browser" -ForegroundColor Yellow
$regArgs = @{
    ExtensionId = $ExtensionId
    Browser     = $Browser
    Build       = $true
}
if ($NoOsConfirm) { $regArgs.NoOsConfirm = $true }
elseif ($RequireOsConfirm) { $regArgs.RequireOsConfirm = $true }
if ($LocalRpc) { $regArgs.LocalRpc = $true }

& $RegisterScript @regArgs

Write-Host ""
Write-Host "Step 3 — Verify" -ForegroundColor Yellow
Write-Host "  Open the extension popup -> Ping"
Write-Host "  Prefer host CLI --init-wallet for mnemonic backup, then Unlock in the popup."
Write-Host ""
Write-Host "Done." -ForegroundColor Green
