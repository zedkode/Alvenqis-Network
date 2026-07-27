param(
    [switch]$SkipChecks,
    [switch]$StageOnly,
    [switch]$AllowUnsigned,
    [ValidateSet("v2")]
    [string]$DesktopChannel = "v2"
)

# Builds Alvenqis Control Center (Tauri 2) for Windows — NSIS + portable stage.
# Product path: Tauri desktop-v2 only.

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$workspace = Join-Path $repoRoot "Blockchain-prototype"
$desktopFolder = if ($DesktopChannel -eq "v2") { "alvenqis-desktop-v2" } else { "alvenqis-desktop-tauri" }
$productName = if ($DesktopChannel -eq "v2") { "Alvenqis Control Center V2" } else { "Alvenqis Control Center" }
$binaryName = if ($DesktopChannel -eq "v2") { "alvenqis-desktop-v2.exe" } else { "alvenqis-desktop-tauri.exe" }
$artifactSlug = if ($DesktopChannel -eq "v2") { "Alvenqis-Control-Center-V2" } else { "Alvenqis-Control-Center" }
$desktop = Join-Path $workspace $desktopFolder
$artifacts = Join-Path $repoRoot "release-artifacts"
$tauriConf = Join-Path $desktop "src-tauri\tauri.conf.json"
$installerHooks = Join-Path $desktop "src-tauri\windows\installer-hooks.nsh"
$bundleRoot = Join-Path $desktop "src-tauri\target\release\bundle"
$signingConfig = $null
$signTool = $null
$thumbprint = $null
$timestampUrl = $null

if (-not (Test-Path -LiteralPath $desktop)) {
    throw "Tauri desktop project not found: $desktop"
}
if (-not (Test-Path -LiteralPath $tauriConf)) {
    throw "Missing tauri.conf.json at $tauriConf"
}
if ($DesktopChannel -eq "v2" -and -not (Test-Path -LiteralPath $installerHooks)) {
    throw "Missing complete-uninstall NSIS hooks at $installerHooks"
}

$toolchain = Join-Path $env:USERPROFILE ".rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"
if (Test-Path -LiteralPath $toolchain) {
    $env:PATH = "$toolchain;$env:PATH"
}

$tauriJson = Get-Content -Raw -LiteralPath $tauriConf | ConvertFrom-Json
$version = [string]$tauriJson.version
if ([string]::IsNullOrWhiteSpace($version)) {
    throw "Could not read version from tauri.conf.json"
}
Write-Host "Building $productName (Tauri) version $version"

if (-not $AllowUnsigned) {
    $thumbprint = ([string]$env:ALVENQIS_WINDOWS_CERT_THUMBPRINT -replace "\s", "").ToUpperInvariant()
    if ($thumbprint -notmatch "^[0-9A-F]{40}$") {
        throw "Signed builds require ALVENQIS_WINDOWS_CERT_THUMBPRINT (40 hexadecimal characters). Use -AllowUnsigned only for local development artifacts."
    }
    $certificate = @(
        Get-ChildItem -Path Cert:\CurrentUser\My, Cert:\LocalMachine\My -ErrorAction SilentlyContinue |
            Where-Object { $_.Thumbprint -eq $thumbprint -and $_.HasPrivateKey }
    ) | Select-Object -First 1
    if (-not $certificate) {
        throw "Code-signing certificate $thumbprint with private key was not found in CurrentUser/My or LocalMachine/My."
    }
    if ($certificate.NotAfter -le (Get-Date).ToUniversalTime()) {
        throw "Code-signing certificate $thumbprint is expired."
    }
    $timestampUrl = ([string]$env:ALVENQIS_WINDOWS_TIMESTAMP_URL).Trim()
    if ([string]::IsNullOrWhiteSpace($timestampUrl)) {
        $timestampUrl = "http://timestamp.digicert.com"
    }
    if ($timestampUrl -notmatch "^https?://") {
        throw "ALVENQIS_WINDOWS_TIMESTAMP_URL must be an HTTP(S) RFC 3161 timestamp endpoint."
    }
    $signingConfig = Join-Path $env:TEMP "alvenqis-tauri-signing-$PID.json"
    @{
        bundle = @{
            windows = @{
                certificateThumbprint = $thumbprint
                digestAlgorithm = "sha256"
                timestampUrl = $timestampUrl
                tsp = $true
            }
        }
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $signingConfig -Encoding utf8

    $signTool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "\\x64\\signtool\.exe$" } |
        Sort-Object FullName -Descending |
        Select-Object -First 1 -ExpandProperty FullName
    if (-not $signTool) {
        throw "signtool.exe x64 was not found in the Windows SDK."
    }
    Write-Host "Authenticode signing enabled for subject: $($certificate.Subject)"
} else {
    Write-Warning "Unsigned developer build requested explicitly. Do not publish these artifacts."
}

if (-not $SkipChecks) {
    Push-Location $workspace
    try {
        cargo fmt --all --check
        if ($LASTEXITCODE -ne 0) { throw "cargo fmt failed" }
        cargo test --workspace --locked
        if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
        cargo clippy --workspace --all-targets --locked -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed" }
    } finally {
        Pop-Location
    }
}

New-Item -ItemType Directory -Force -Path $artifacts | Out-Null
Get-ChildItem -LiteralPath $artifacts -File | Where-Object {
    $_.Name -like "$productName`_*-setup.exe" -or
    $_.Name -like "$artifactSlug-*-Windows-x64-Portable.zip" -or
    $_.Name -eq "README-UPDATES-$DesktopChannel.txt"
} | Remove-Item -Force
Get-ChildItem -LiteralPath $artifacts -Directory -Filter ".portable-stage-*" | ForEach-Object {
    if (-not $_.FullName.StartsWith($artifacts, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clean unexpected portable stage: $($_.FullName)"
    }
    Remove-Item -LiteralPath $_.FullName -Recurse -Force
}

# Tauri retains bundles from earlier versions; never recollect stale installers.
if (Test-Path -LiteralPath $bundleRoot) {
    $resolvedBundleRoot = (Resolve-Path -LiteralPath $bundleRoot).Path
    if (-not $resolvedBundleRoot.StartsWith($desktop, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clean unexpected bundle path: $resolvedBundleRoot"
    }
    Remove-Item -LiteralPath $resolvedBundleRoot -Recurse -Force
}

Push-Location $desktop
try {
    npm ci
    if ($LASTEXITCODE -ne 0) { throw "npm ci failed in $desktopFolder" }

    # Keystore helper + miner/node sidecars for full operator builds.
    npm run prepare:native:sidecars
    if ($LASTEXITCODE -ne 0) { throw "prepare:native:sidecars failed" }

    if ($signingConfig) {
        $prepackageExecutables = @(
            Get-ChildItem -LiteralPath (Join-Path $desktop "src-tauri\binaries") -Filter "*.exe" -File -ErrorAction SilentlyContinue
            Get-ChildItem -LiteralPath (Join-Path $desktop "src-tauri\resources\bin") -Filter "*.exe" -File -ErrorAction SilentlyContinue
            Get-Item -LiteralPath (Join-Path $desktop "native\keystore-helper\target\release\alvenqis-keystore-helper.exe") -ErrorAction SilentlyContinue
        ) | Where-Object { $_ } | Sort-Object FullName -Unique
        foreach ($executable in $prepackageExecutables) {
            & $signTool sign /sha1 $thumbprint /fd SHA256 /tr $timestampUrl /td SHA256 $executable.FullName
            if ($LASTEXITCODE -ne 0) {
                throw "Authenticode signing failed before packaging: $($executable.FullName)"
            }
        }
        Write-Host "Signed $($prepackageExecutables.Count) helper/sidecar executable(s) before packaging."
    }

    if ($StageOnly) {
        npm run build
        if ($LASTEXITCODE -ne 0) { throw "frontend build failed" }
        npx tauri build --no-bundle
        if ($LASTEXITCODE -ne 0) { throw "tauri build --no-bundle failed" }
        $exeCandidates = @(
            (Join-Path $desktop "src-tauri\target\release\$binaryName"),
            (Join-Path $desktop "src-tauri\target\release\$productName.exe")
        ) | Where-Object { Test-Path -LiteralPath $_ }
        if (-not $exeCandidates) {
            throw "StageOnly: release binary not found under src-tauri/target/release"
        }
        $stage = Join-Path $artifacts "tauri-stage-$version"
        if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
        New-Item -ItemType Directory -Force -Path $stage | Out-Null
        Copy-Item -LiteralPath $exeCandidates[0] -Destination (Join-Path $stage "$productName.exe")
        Write-Host "StageOnly binary: $stage"
        exit 0
    }

    if ($signingConfig) {
        npx tauri build --bundles nsis --config $signingConfig
    } else {
        npx tauri build --bundles nsis
    }
    if ($LASTEXITCODE -ne 0) { throw "Tauri NSIS build failed" }
} finally {
    Pop-Location
    if ($signingConfig -and (Test-Path -LiteralPath $signingConfig)) {
        Remove-Item -LiteralPath $signingConfig -Force
    }
}

# Collect installers into release-artifacts with versioned names
if (-not (Test-Path -LiteralPath $bundleRoot)) {
    throw "No Tauri bundle directory at $bundleRoot"
}

$copied = @()
Get-ChildItem -Path $bundleRoot -Recurse -File | Where-Object {
    $_.Extension -eq ".exe" -or $_.Name -like "*.nsis.zip"
} | ForEach-Object {
    $dest = Join-Path $artifacts $_.Name
    Copy-Item -LiteralPath $_.FullName -Destination $dest -Force
    $copied += $dest
    Write-Host "Artifact: $dest"
}

# Portable zip of the exact application executable when present.
$releaseExePath = Join-Path $desktop "src-tauri\target\release\$binaryName"
$releaseExe = if (Test-Path -LiteralPath $releaseExePath) { Get-Item -LiteralPath $releaseExePath } else { $null }
if ($releaseExe) {
    $portable = Join-Path $artifacts "$artifactSlug-$version-Windows-x64-Portable.zip"
    $portableStage = Join-Path $artifacts ".portable-stage-$version"
    if (Test-Path -LiteralPath $portable) { Remove-Item -LiteralPath $portable -Force }
    if (Test-Path -LiteralPath $portableStage) { Remove-Item -LiteralPath $portableStage -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $portableStage | Out-Null
    Copy-Item -LiteralPath $releaseExe.FullName -Destination (Join-Path $portableStage "$productName.exe")
    $releaseResources = Join-Path $desktop "src-tauri\target\release\resources"
    if (-not (Test-Path -LiteralPath $releaseResources)) {
        throw "Portable package resources are missing: $releaseResources"
    }
    Copy-Item -LiteralPath $releaseResources -Destination (Join-Path $portableStage "resources") -Recurse
    $keystoreHelper = Join-Path $desktop "src-tauri\target\release\alvenqis-keystore-helper.exe"
    if (-not (Test-Path -LiteralPath $keystoreHelper)) {
        throw "Portable package keystore helper is missing: $keystoreHelper"
    }
    Copy-Item -LiteralPath $keystoreHelper -Destination (Join-Path $portableStage "alvenqis-keystore-helper.exe")
    & tar.exe -a -cf $portable -C $portableStage .
    if ($LASTEXITCODE -ne 0) { throw "Portable zip creation failed" }
    $portableEntries = @(& tar.exe -tf $portable)
    if ($LASTEXITCODE -ne 0) { throw "Portable zip verification failed" }
    foreach ($requiredEntry in @(
        "./$productName.exe",
        "./resources/bin/alvenqis-miner.exe",
        "./alvenqis-keystore-helper.exe"
    )) {
        if ($portableEntries -notcontains $requiredEntry) {
            throw "Portable zip is incomplete; missing $requiredEntry"
        }
    }
    Remove-Item -LiteralPath $portableStage -Recurse -Force
    $copied += $portable
    Write-Host "Portable package: $portable"
}

# Honest updater note (no legacy electron-builder latest.yml)
$readme = Join-Path $artifacts "README-UPDATES-$DesktopChannel.txt"
@(
    "$productName Tauri $version",
    "The app checks GitHub Releases but requires explicit approval before installation.",
    "Each selected asset must match the release SHA256SUMS file before execution.",
    $(if ($AllowUnsigned) { "DEVELOPER BUILD: artifacts are unsigned and must not be published." } else { "Windows executables are Authenticode-signed and RFC 3161 timestamped; verify the publisher and checksum before approval." }),
    "Do not use legacy electron-builder latest.yml channels."
) | Set-Content -LiteralPath $readme -Encoding utf8

if (-not $copied) {
    throw "No Windows installer artifacts were produced under $bundleRoot"
}

if (-not $AllowUnsigned) {
    $signedFiles = @($copied | Where-Object { [System.IO.Path]::GetExtension($_) -eq ".exe" })
    if ($releaseExe) {
        $signedFiles += $releaseExe.FullName
    }
    $signedFiles += @(
        Get-ChildItem -LiteralPath (Join-Path $desktop "src-tauri\resources\bin") -Filter "*.exe" -File -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty FullName
    )
    $keystoreHelper = Join-Path $desktop "src-tauri\target\release\alvenqis-keystore-helper.exe"
    if (Test-Path -LiteralPath $keystoreHelper) {
        $signedFiles += $keystoreHelper
    }
    foreach ($signedFile in ($signedFiles | Sort-Object -Unique)) {
        & $signTool verify /pa /all /v $signedFile
        if ($LASTEXITCODE -ne 0) {
            throw "Authenticode verification failed: $signedFile"
        }
    }
    Write-Host "Authenticode verification passed for $($signedFiles.Count) executable artifact(s)."
}

Write-Host "$productName Windows packaging complete ($($copied.Count) artifacts) version $version"
