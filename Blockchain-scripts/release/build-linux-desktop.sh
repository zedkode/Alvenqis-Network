#!/usr/bin/env bash
# Build Alvenqis Control Center (Tauri) for Linux desktops.
# Produces: .deb (Ubuntu/Debian), .AppImage (Arch + portable), .rpm (Fedora/openSUSE)
#
# Usage:
#   bash Blockchain-scripts/release/build-linux-desktop.sh
#   bash Blockchain-scripts/release/build-linux-desktop.sh --bundles deb,appimage
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
prototype="$root/Blockchain-prototype"
desktop="$prototype/alvenqis-desktop-v2"
output="$root/release-artifacts/linux-v2"
bundles="deb,appimage,rpm"

while (($#)); do
  case "$1" in
    -h|--help)
      cat <<'EOF'
Usage: bash Blockchain-scripts/release/build-linux-desktop.sh [options]

Options:
  --bundles LIST           Comma-separated Tauri bundles (default: deb,appimage,rpm)
  -h, --help               Show this help

Same product shell as Windows: wallet, optional NVIDIA CUDA FiroPoW miner,
node/RPC sidecars, explorer resources, recovery phrase (native keystore).
AMD/Intel GPUs cannot mine — wallet UI still works without NVIDIA.

Distro mapping:
  Ubuntu / Debian           → install the .deb
  Arch / CachyOS / pacman   → .AppImage or packaging/arch/PKGBUILD
  Fedora / RHEL             → .rpm when requested (not the primary QA focus)

Build host must be Linux x86_64 with Rust, Node 20+, and distro Tauri deps
(see Blockchain-docs/human/operator/LINUX_DESKTOP.md for deps and QA notes).
EOF
      exit 0
      ;;
    --bundles)
      bundles="${2:-}"; shift 2 || true
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 2
      ;;
  esac
done

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "ERROR: Linux packages must be built on a Linux host (got $(uname -s))." >&2
  echo "Use WSL2 Ubuntu, a VM, or CI linux-x64 runner." >&2
  exit 1
fi

command -v cargo >/dev/null || { echo "cargo is required" >&2; exit 1; }
command -v npm >/dev/null || { echo "npm is required" >&2; exit 1; }
command -v pkg-config >/dev/null || { echo "pkg-config is required (install build-essential / base-devel)" >&2; exit 1; }

# Detect missing WebKitGTK early with a clear message.
if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
  cat <<'EOF' >&2
Missing webkit2gtk-4.1 development package.

  Ubuntu/Debian:  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
                    libssl-dev libayatana-appindicator3-dev librsvg2-dev libsecret-1-dev \
                    libsoup-3.0-dev libjavascriptcoregtk-4.1-dev patchelf fakeroot zenity
  Fedora:         sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
                    libappindicator-gtk3-devel librsvg2-devel libsecret-devel zenity
  Arch:           sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file \
                    openssl appmenu-gtk-module libappindicator-gtk3 librsvg libsecret zenity
EOF
  exit 1
fi

version="$(
  node -e "const j=require('$desktop/package.json'); process.stdout.write(j.version||'')" 2>/dev/null \
    || sed -n 's/^.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$desktop/package.json" | head -1
)"
echo "==> Alvenqis Control Center Linux build version=${version:-unknown} bundles=$bundles"

mkdir -p "$output"
rm -f "$output"/*.AppImage "$output"/*.deb "$output"/*.rpm "$output"/*.pacman \
  "$output"/alvenqis-control-center "$output"/SHA256SUMS 2>/dev/null || true
rm -rf "$desktop/src-tauri/target/release/bundle"

# Brand assets
if [[ -f "$desktop/logo.png" ]]; then
  cp -f "$desktop/logo.png" "$desktop/public/logo.png"
fi
if [[ -f "$root/shared/brand/logo-mark.png" ]]; then
  cp -f "$root/shared/brand/logo-mark.png" "$desktop/public/logo-mark.png"
fi

export ALVENQIS_REQUIRE_CUDA=1

(
  cd "$desktop"
  if [[ -f package-lock.json ]]; then
    npm ci
  else
    npm install
  fi

  echo "==> Preparing native (CUDA-only GPU miner + sidecars)"
  bash ./scripts/prepare-native.sh --with-sidecars

  # Arch sometimes needs NO_STRIP for AppImage strip tools
  if grep -qi arch /etc/os-release 2>/dev/null; then
    export NO_STRIP="${NO_STRIP:-true}"
    echo "NOTE: Arch host detected; NO_STRIP=$NO_STRIP"
  fi

  npx tauri build --bundles "$bundles"
)

# Collect Tauri linux bundles
bundle_root="$desktop/src-tauri/target/release/bundle"
if [[ -d "$bundle_root" ]]; then
  find "$bundle_root" -type f \
    \( -name '*.AppImage' -o -name '*.deb' -o -name '*.rpm' -o -name '*.pacman' \) \
    -print0 | while IFS= read -r -d '' f; do
      cp -f "$f" "$output/"
      echo "  + $(basename "$f")"
    done
fi

if ! find "$output" -maxdepth 1 -type f \
  \( -name '*.AppImage' -o -name '*.deb' -o -name '*.rpm' -o -name '*.pacman' \) | grep -q .; then
  if [[ -f "$desktop/src-tauri/target/release/alvenqis-desktop-v2" ]]; then
    cp -f "$desktop/src-tauri/target/release/alvenqis-desktop-v2" "$output/alvenqis-control-center"
    chmod +x "$output/alvenqis-control-center"
    echo "NOTE: no deb/AppImage/rpm produced; shipped raw binary -> $output/alvenqis-control-center"
  else
    echo "Linux packaging produced no artifacts" >&2
    exit 1
  fi
fi

# Install hints
cat > "$output/INSTALL.txt" <<EOF
Alvenqis Control Center ${version:-?} — Linux (parity with Windows Tauri shell)

Mainnet Candidate / Prototype — not public Mainnet.

Features (same UI as Windows):
  - Wallet + native recovery phrase (libsecret / keyring)
  - FiroPoW 0.9.4 mining on NVIDIA CUDA GPUs only
  - Node / RPC / indexer sidecars, pool view, explorer, settings, auto-update hooks

Ubuntu / Debian:
  sudo apt install ./Alvenqis*.deb
  # or: sudo dpkg -i ./*.deb && sudo apt-get install -f

Arch Linux / CachyOS / other pacman derivatives:
  chmod +x ./*.AppImage && ./*.AppImage
  # Or: packaging/arch/PKGBUILD (makepkg -si)
  # Mining is NVIDIA CUDA only — AMD/Intel GPUs cannot mine.

Fedora / RHEL / openSUSE (rpm when present; not the primary QA focus):
  sudo dnf install ./*.rpm

Runtime deps (deb pulls most automatically):
  WebKitGTK 4.1, GTK3, AppIndicator, libsecret, zenity
  Mining GPU: supported NVIDIA driver (CUDA only; no AMD/OpenCL/CPU mining)

Wallet keyring needs a Secret Service agent (GNOME Keyring / KWallet).
Operator guide: Blockchain-docs/human/operator/LINUX_DESKTOP.md
Maturity: Blockchain-docs/human/release/NETWORK_MATURITY.md
EOF

(
  cd "$output"
  find . -maxdepth 1 -type f ! -name SHA256SUMS ! -name INSTALL.txt -print0 \
    | sort -z | xargs -0 sha256sum > SHA256SUMS
)

echo ""
echo "Linux desktop packages: $output"
ls -la "$output"
echo "Done version=${version:-unknown}"
