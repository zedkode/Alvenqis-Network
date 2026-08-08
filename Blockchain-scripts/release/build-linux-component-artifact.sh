#!/usr/bin/env bash
# Build one independently publishable Linux x86_64 component artifact.
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
workspace="$repo_root/Blockchain-prototype"
component="${1:-}"
output_dir="${2:-$repo_root/release-artifacts/components}"
build_root="${CARGO_TARGET_DIR:-$repo_root/.alvenqis-local/build/component-release}"

usage() {
  cat <<'EOF'
Usage: build-linux-component-artifact.sh COMPONENT [OUTPUT_DIR]

Components:
  node, rpc, indexer, explorer, pool, wallet-cli, miner

Each invocation tests and packages only the selected component. The miner build
requires a Linux x86_64 CUDA toolchain and ALVENQIS_REQUIRE_CUDA=1.
EOF
}

case "$component" in
  node|rpc|indexer|explorer|pool|wallet-cli|miner) ;;
  -h|--help|"")
    usage
    [[ -n "$component" ]] && exit 0 || exit 2
    ;;
  *)
    echo "Unsupported component: $component" >&2
    usage >&2
    exit 2
    ;;
esac

command -v tar >/dev/null || { echo "tar is required" >&2; exit 127; }
command -v sha256sum >/dev/null || { echo "sha256sum is required" >&2; exit 127; }

mkdir -p "$output_dir" "$build_root"
stage_root="$(mktemp -d)"
trap 'rm -rf "$stage_root"' EXIT

package_rust_component() {
  local package="$1"
  local binary="$2"
  local version
  local package_stage="$stage_root/$binary-linux-x86_64"
  local feature_args=()

  version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$workspace/$package/Cargo.toml" | head -n 1)"
  [[ -n "$version" ]] || { echo "Could not read version for $package" >&2; exit 1; }
  [[ "$component" == node ]] && feature_args=(--features storage-rocksdb)

  (
    cd "$workspace"
    CARGO_TARGET_DIR="$build_root" cargo test --locked -p "$package" "${feature_args[@]}"
    if [[ "$component" == miner ]]; then
      ALVENQIS_REQUIRE_CUDA=1 CARGO_TARGET_DIR="$build_root" \
        cargo build --locked --release -p "$package"
    else
      CARGO_TARGET_DIR="$build_root" \
        cargo build --locked --release -p "$package" "${feature_args[@]}"
    fi
  )

  install -D -m 0755 "$build_root/release/$binary" "$package_stage/bin/$binary"
  cp "$workspace/$package/Cargo.toml" "$package_stage/Cargo.toml"
  cp "$repo_root/README.md" "$package_stage/README-REPOSITORY.md"

  case "$component" in
    node)
      mkdir -p "$package_stage/configs" "$package_stage/docs"
      cp "$workspace/configs/mainnet-candidate.toml" "$package_stage/configs/"
      cp "$workspace/configs/genesis.mainnet-candidate.toml" "$package_stage/configs/"
      cp "$repo_root/Blockchain-docs/human/operator/INDEPENDENT_NODE_OPERATOR_GUIDE.md" "$package_stage/docs/"
      ;;
    rpc)
      mkdir -p "$package_stage/configs"
      cp "$workspace/configs/rpc.mainnet-candidate.toml" "$package_stage/configs/"
      ;;
    pool)
      mkdir -p "$package_stage/configs"
      cp "$workspace/alvenqis-release/alvenqis-setup-external/configs/pool.toml" "$package_stage/configs/pool.example.toml"
      ;;
    miner)
      mkdir -p "$package_stage/docs"
      cp "$repo_root/Blockchain-docs/human/mining/GPU_MINING.md" "$package_stage/docs/"
      ;;
  esac

  artifact="$output_dir/${binary}-${version}-linux-x86_64.tar.gz"
  tar -C "$stage_root" -czf "$artifact" "$(basename "$package_stage")"
}

package_explorer() {
  local explorer="$workspace/alvenqis-explorer"
  local version
  local package_stage

  command -v npm >/dev/null || { echo "npm is required for explorer" >&2; exit 127; }
  version="$(node -e "const p=require('$explorer/package.json'); process.stdout.write(p.version)")"
  package_stage="$stage_root/alvenqis-explorer-linux-x86_64"
  (
    cd "$explorer"
    npm ci
    npm run lint
    npm run build
  )
  mkdir -p "$package_stage"
  cp -R "$explorer/dist" "$package_stage/dist"
  cp "$explorer/package.json" "$explorer/package-lock.json" "$package_stage/"
  cp "$explorer/README.md" "$package_stage/"
  artifact="$output_dir/alvenqis-explorer-${version}-linux-x86_64.tar.gz"
  tar -C "$stage_root" -czf "$artifact" "$(basename "$package_stage")"
}

case "$component" in
  node) package_rust_component alvenqis-node alvenqis-node ;;
  rpc) package_rust_component alvenqis-rpc-gateway alvenqis-rpc-gateway ;;
  indexer) package_rust_component alvenqis-indexer alvenqis-indexer ;;
  explorer) package_explorer ;;
  pool) package_rust_component alvenqis-mining-pool alvenqis-mining-pool ;;
  wallet-cli) package_rust_component alvenqis-wallet alvenqis-wallet ;;
  miner) package_rust_component alvenqis-miner alvenqis-miner ;;
esac

artifact_name="$(basename "$artifact")"
(
  cd "$output_dir"
  sha256sum "$artifact_name" > "$artifact_name.sha256"
  sha256sum -c "$artifact_name.sha256"
)
echo "$artifact"
echo "$artifact.sha256"
