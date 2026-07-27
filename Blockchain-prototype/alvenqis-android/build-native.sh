#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ndk_version="29.0.14206865"
sdk_root="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}"
ndk_root="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-${sdk_root:+$sdk_root/ndk/$ndk_version}}}"

if [[ -z "$ndk_root" || ! -f "$ndk_root/source.properties" ]]; then
  echo "Android NDK $ndk_version was not found. Set ANDROID_SDK_ROOT or ANDROID_NDK_HOME, or install ndk;$ndk_version." >&2
  exit 1
fi

export ANDROID_NDK_HOME="$ndk_root"
export ANDROID_NDK_ROOT="$ndk_root"

find_ndk_host_prebuilt() {
  local ndk="$1"
  local prebuilt="$ndk/toolchains/llvm/prebuilt"
  if [[ ! -d "$prebuilt" ]]; then
    local nested
    nested="$(find "$ndk" -maxdepth 2 -type d -path '*/toolchains/llvm/prebuilt' 2>/dev/null | head -n1 || true)"
    if [[ -n "$nested" ]]; then
      prebuilt="$nested"
    fi
  fi
  local host
  host="$(find "$prebuilt" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | head -n1 || true)"
  if [[ -z "$host" ]]; then
    echo "Could not locate NDK LLVM prebuilt host under '$ndk'." >&2
    exit 1
  fi
  echo "$host"
}

copy_libcxx_shared() {
  local abi="$1"
  local triple="$2"
  local dest="$3"
  local host_prebuilt
  host_prebuilt="$(find_ndk_host_prebuilt "$ndk_root")"
  local src=""
  for candidate in \
    "$host_prebuilt/sysroot/usr/lib/$triple/libc++_shared.so" \
    "$host_prebuilt/sysroot/usr/lib/$triple/31/libc++_shared.so" \
    "$host_prebuilt/sysroot/usr/lib/$triple/21/libc++_shared.so"
  do
    if [[ -f "$candidate" ]]; then
      src="$candidate"
      break
    fi
  done
  if [[ -z "$src" ]]; then
    echo "libc++_shared.so not found for ABI $abi (triple $triple) under NDK." >&2
    exit 1
  fi
  mkdir -p "$dest"
  cp -f "$src" "$dest/libc++_shared.so"
}

cd "$root"
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o alvenqis-android/app/src/main/jniLibs build --release -p alvenqis-mobile-core

declare -A abi_triples=(
  ["arm64-v8a"]="aarch64-linux-android"
  ["armeabi-v7a"]="arm-linux-androideabi"
  ["x86_64"]="x86_64-linux-android"
)

for abi in arm64-v8a armeabi-v7a x86_64; do
  jni_dir="$root/alvenqis-android/app/src/main/jniLibs/$abi"
  library="$jni_dir/libalvenqis_mobile_core.so"
  [[ -s "$library" ]] || { echo "Android native build did not produce $library" >&2; exit 1; }
  copy_libcxx_shared "$abi" "${abi_triples[$abi]}" "$jni_dir"
done
