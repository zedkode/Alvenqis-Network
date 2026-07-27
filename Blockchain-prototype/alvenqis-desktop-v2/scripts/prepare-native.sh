#!/usr/bin/env bash
# Prepares keystore helper + Linux sidecars for Tauri packaging.
# Usage:
#   ./scripts/prepare-native.sh
#   ./scripts/prepare-native.sh --with-sidecars
set -euo pipefail

WITH_SIDECARS=0
for arg in "$@"; do
  case "$arg" in
    --with-sidecars) WITH_SIDECARS=1 ;;
    --cpu-only-miner) echo "CPU/OpenCL miner builds were removed" >&2; exit 2 ;;
    -h|--help)
      echo "Usage: $0 [--with-sidecars]"
      exit 0
      ;;
  esac
done

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "prepare-native.sh is for Linux hosts. On Windows use prepare-native.ps1" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="$(cd "$ROOT/.." && pwd)"
HELPER_MANIFEST="$ROOT/native/keystore-helper/Cargo.toml"
BIN_DIR="$ROOT/src-tauri/binaries"
RES_DIR="$ROOT/src-tauri/resources"
RES_BIN="$RES_DIR/bin"
TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"

mkdir -p "$BIN_DIR" "$RES_DIR" "$RES_BIN"

echo "==> Building alvenqis-keystore-helper (release)"
cargo build --release --locked --manifest-path "$HELPER_MANIFEST"

HELPER_SRC="$ROOT/native/keystore-helper/target/release/alvenqis-keystore-helper"
if [[ ! -f "$HELPER_SRC" ]]; then
  echo "Missing built helper: $HELPER_SRC" >&2
  exit 1
fi

cp "$HELPER_SRC" "$BIN_DIR/alvenqis-keystore-helper-$TRIPLE"
cp "$HELPER_SRC" "$BIN_DIR/alvenqis-keystore-helper"
cp "$HELPER_SRC" "$RES_BIN/alvenqis-keystore-helper"
chmod +x "$BIN_DIR/alvenqis-keystore-helper-$TRIPLE" "$BIN_DIR/alvenqis-keystore-helper" "$RES_BIN/alvenqis-keystore-helper"
# Do not ship Windows helper leftovers into Linux packages.
rm -f "$BIN_DIR"/alvenqis-keystore-helper*.exe "$RES_BIN"/alvenqis-keystore-helper*.exe 2>/dev/null || true
echo "Staged keystore helper -> $BIN_DIR/alvenqis-keystore-helper-$TRIPLE"

stage_operator_and_assets() {
  # Operator entrypoint
  if [[ -f "$REPO/alvenqis.sh" ]]; then
    cp -f "$REPO/alvenqis.sh" "$RES_DIR/alvenqis.sh"
    chmod +x "$RES_DIR/alvenqis.sh"
    echo "  + alvenqis.sh"
  fi

  # Local operator scripts (shell only for Linux runtime)
  if [[ -d "$REPO/scripts/local" ]]; then
    mkdir -p "$RES_DIR/scripts/local"
    # Prefer shell scripts; keep .ps1 only if already present is fine for dual tree
    shopt -s nullglob
    for f in "$REPO/scripts/local"/*.sh; do
      cp -f "$f" "$RES_DIR/scripts/local/"
      chmod +x "$RES_DIR/scripts/local/$(basename "$f")"
    done
    shopt -u nullglob
    echo "  + scripts/local/*.sh"
  fi

  # Canonical configs for local stack / genesis pin
  mkdir -p "$RES_DIR/configs"
  for cfg in \
    mainnet-candidate.toml \
    genesis.mainnet-candidate.toml \
    local.toml \
    rpc.mainnet-candidate.toml \
    rpc.local.toml
  do
    if [[ -f "$REPO/configs/$cfg" ]]; then
      cp -f "$REPO/configs/$cfg" "$RES_DIR/configs/$cfg"
      echo "  + configs/$cfg"
    fi
  done

  # Optional genesis review artifacts
  if [[ -d "$REPO/docs/release" ]]; then
    mkdir -p "$RES_DIR/docs/release"
    for f in GENESIS_APPROVAL.mainnet-candidate.json GENESIS_REVIEW.mainnet-candidate.json; do
      if [[ -f "$REPO/docs/release/$f" ]]; then
        cp -f "$REPO/docs/release/$f" "$RES_DIR/docs/release/$f"
        echo "  + docs/release/$f"
      fi
    done
  fi

  # Static explorer (built frontend) if present
  if [[ -d "$REPO/alvenqis-explorer/dist" ]]; then
    rm -rf "$RES_DIR/explorer"
    mkdir -p "$RES_DIR/explorer"
    cp -a "$REPO/alvenqis-explorer/dist/." "$RES_DIR/explorer/"
    echo "  + explorer/ (from alvenqis-explorer/dist)"
  fi

  # Brand
  if [[ -f "$ROOT/logo.png" ]]; then
    cp -f "$ROOT/logo.png" "$ROOT/public/logo.png" 2>/dev/null || true
  elif [[ -f "$REPO/shared/brand/logo-mark.png" ]]; then
    cp -f "$REPO/shared/brand/logo-mark.png" "$ROOT/public/logo.png" 2>/dev/null || true
  fi
}

stage_operator_and_assets

if [[ "$WITH_SIDECARS" -eq 1 ]]; then
  echo "==> Building Linux sidecars from monorepo (release) — CUDA-only GPU miner"
  (
    cd "$REPO"
    # Product releases require compiled CUDA kernels; stubs are not shippable.
    export ALVENQIS_REQUIRE_CUDA=1
    command -v nvcc >/dev/null || {
      echo "nvcc is required to build the CUDA-only miner sidecar" >&2
      exit 1
    }
    cargo build --release --locked -p alvenqis-miner
    cargo build --release --locked -p alvenqis-node -p alvenqis-rpc-gateway -p alvenqis-indexer
  )

  # Drop Windows .exe sidecars so they never land in deb/AppImage/rpm.
  rm -f "$RES_BIN"/*.exe 2>/dev/null || true

  for bin in alvenqis-miner alvenqis-node alvenqis-rpc-gateway alvenqis-indexer; do
    src="$REPO/target/release/$bin"
    if [[ -f "$src" ]]; then
      cp -f "$src" "$RES_BIN/$bin"
      chmod +x "$RES_BIN/$bin"
      echo "  + bin/$bin"
    else
      echo "  ! missing $bin" >&2
      exit 1
    fi
  done

  cat > "$RES_DIR/MANIFEST.json" <<EOF
{
  "prepared_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "platform": "linux",
  "host_triple": "$TRIPLE",
  "keystore_helper": "bin/alvenqis-keystore-helper",
  "binaries": [
    "bin/alvenqis-miner",
    "bin/alvenqis-node",
    "bin/alvenqis-rpc-gateway",
    "bin/alvenqis-indexer"
  ],
  "operator": "alvenqis.sh",
  "mining_backend": "cuda",
  "cpu_mining": false,
  "opencl_mining": false
}
EOF
  echo "Wrote resources/MANIFEST.json"
fi

echo "Native preparation complete (Linux)."
