$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$ndkVersion = "29.0.14206865"
$sdkRoot = if ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } elseif ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA "Android\Sdk" }
$ndkRoot = if ($env:ANDROID_NDK_HOME) { $env:ANDROID_NDK_HOME } elseif ($env:ANDROID_NDK_ROOT) { $env:ANDROID_NDK_ROOT } else { Join-Path $sdkRoot "ndk\$ndkVersion" }

if (-not (Test-Path (Join-Path $ndkRoot "source.properties"))) {
    throw "Android NDK $ndkVersion was not found at '$ndkRoot'. Install it with sdkmanager 'ndk;$ndkVersion' or Android Studio SDK Manager."
}

$env:ANDROID_NDK_HOME = $ndkRoot
$env:ANDROID_NDK_ROOT = $ndkRoot

# ABI -> NDK triple for sysroot libc++_shared.so
$abiTriples = [ordered]@{
    "arm64-v8a"   = "aarch64-linux-android"
    "armeabi-v7a" = "arm-linux-androideabi"
    "x86_64"      = "x86_64-linux-android"
}

function Find-NdkHostPrebuilt {
    param([string]$Ndk)
    $prebuilt = Join-Path $Ndk "toolchains\llvm\prebuilt"
    if (-not (Test-Path $prebuilt)) {
        # Some SDK installs nest the NDK under android-ndk-rXX
        $nested = Get-ChildItem $Ndk -Directory -ErrorAction SilentlyContinue |
            Where-Object { Test-Path (Join-Path $_.FullName "toolchains\llvm\prebuilt") } |
            Select-Object -First 1
        if ($nested) {
            $prebuilt = Join-Path $nested.FullName "toolchains\llvm\prebuilt"
        }
    }
    $hostDir = Get-ChildItem $prebuilt -Directory -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $hostDir) {
        throw "Could not locate NDK LLVM prebuilt host under '$Ndk'."
    }
    return $hostDir.FullName
}

function Copy-LibcxxShared {
    param(
        [string]$Ndk,
        [string]$Abi,
        [string]$Triple,
        [string]$DestDir
    )
    $hostPrebuilt = Find-NdkHostPrebuilt -Ndk $Ndk
    $candidates = @(
        (Join-Path $hostPrebuilt "sysroot\usr\lib\$Triple\libc++_shared.so"),
        (Join-Path $hostPrebuilt "sysroot\usr\lib\$Triple\31\libc++_shared.so"),
        (Join-Path $hostPrebuilt "sysroot\usr\lib\$Triple\21\libc++_shared.so")
    )
    $src = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $src) {
        throw "libc++_shared.so not found for ABI $Abi (triple $Triple) under NDK."
    }
    New-Item -ItemType Directory -Force -Path $DestDir | Out-Null
    Copy-Item -Force $src (Join-Path $DestDir "libc++_shared.so")
}

Push-Location $root
try {
    cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o alvenqis-android/app/src/main/jniLibs build --release -p alvenqis-mobile-core
    if ($LASTEXITCODE -ne 0) { throw "Android native build failed." }

    foreach ($abi in $abiTriples.Keys) {
        $jniDir = Join-Path $root "alvenqis-android\app\src\main\jniLibs\$abi"
        $library = Join-Path $jniDir "libalvenqis_mobile_core.so"
        if (-not (Test-Path $library) -or (Get-Item $library).Length -le 4) {
            throw "Android native build did not produce '$library'."
        }
        # FiroPoW C++ is linked against NDK libc++; ship it next to the Rust .so
        Copy-LibcxxShared -Ndk $ndkRoot -Abi $abi -Triple $abiTriples[$abi] -DestDir $jniDir
    }
} finally {
    Pop-Location
}
