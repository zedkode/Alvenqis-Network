# Alvenqis Mobile for Android

Status: **Mainnet Candidate / Prototype / not public mainnet**
Version: **1.0.0** (aligned with Windows/Linux Control Center)
**Minimum OS: Android 12 (API 31)+**

Alvenqis Mobile mirrors the desktop Control Center for **wallet + network monitoring** (read-only chain / pool visibility).

## Mining policy (hard rule)

**Mobile platforms are not for mining.**

- No miner start/stop that executes PoW.
- No share submission, GPU/CPU mining backends, or pool worker registration from Android.
- The Mining tab is explicitly **DISABLED** and points users to **Windows/Linux Tauri Control Center**.

## Desktop parity (read-only)

| Desktop area | Android |
|--------------|---------|
| Overview / sync / balance | Yes |
| Wallet create / import (12 or 24 words) | Yes (Android Keystore AES-GCM) |
| Network / RPC endpoint | Yes (official VPS default) |
| Explorer tip + recent blocks | Yes (when RPC lists blocks) |
| Pool status (PPLNS metrics) | Yes (via `/pool/api/v1/pool/status`) |
| Mining | Disabled (policy) |
| Tx signing / broadcast | Desktop only (this candidate) |

## Security

- Recovery words shown only at create; back up offline.
- Network security config: **HTTPS required** for public RPC; cleartext only for `127.0.0.1` / emulator (`10.0.2.2`).
- Official client uses `https://rpcnode.dohotstudio.com` (TLS). Port `20787` is P2P, not HTTP RPC.

## Build prerequisites

- **Android 12+** devices/emulators (minSdk **31**)
- JDK 17+
- Android SDK 35, NDK 29.x
- Rust targets + `cargo-ndk`

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
cd alvenqis-android
./gradlew clean assembleDebug verifyDebugNativeLibraries
```

Windows:

```powershell
cd alvenqis-android
.\gradlew.bat clean assembleDebug verifyDebugNativeLibraries
```

APK: `app/build/outputs/apk/debug/app-debug.apk`
Native libs: `arm64-v8a`, `armeabi-v7a`, `x86_64`.

## Native tests

```bash
cd alvenqis-android
./gradlew connectedDebugAndroidTest
```

Instrumented tests load `libalvenqis_mobile_core.so` and verify 24-word (and import path) wallet derivation.

## Product line

Keep `versionName` / `VERSION` in lockstep with desktop (`1.0.0`). Bump `versionCode` on each Play/internal release.
