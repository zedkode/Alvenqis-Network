#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
prototype="$repo_root/Blockchain-prototype"
output_dir="$repo_root/release-artifacts"
output="$output_dir/alvenqis-docker-control-plane.tar.gz"

cd "$repo_root"
git diff --quiet && git diff --cached --quiet || {
  echo "Refusing to build a release archive from a dirty working tree." >&2
  exit 73
}

# The Dockerfile builds the selected VPS binaries from the prototype workspace.
# Include Cargo members + control plane; exclude frontends, secrets, build output.
paths=(
  .dockerignore .gitattributes
  Blockchain-prototype/Cargo.toml Blockchain-prototype/Cargo.lock
  Blockchain-prototype/VERSION Blockchain-prototype/clippy.toml
  Blockchain-prototype/configs Blockchain-docs/human/release Blockchain-prototype/shared
  Blockchain-prototype/alvenqis-core Blockchain-prototype/alvenqis-node
  Blockchain-prototype/alvenqis-rpc-gateway Blockchain-prototype/alvenqis-wallet
  Blockchain-prototype/alvenqis-sdk-rust Blockchain-prototype/alvenqis-browser/host
  Blockchain-prototype/alvenqis-indexer Blockchain-prototype/alvenqis-miner
  Blockchain-prototype/alvenqis-mining-pool Blockchain-prototype/alvenqis-mobile-core
  Blockchain-prototype/alvenqis-release/vps-control-plane
)

mkdir -p "$output_dir"
git archive --format=tar HEAD -- "${paths[@]}" | gzip -9 > "$output"
(cd "$output_dir" && sha256sum "$(basename "$output")" > "$(basename "$output").sha256")

tar -tzf "$output" | grep -Fxq 'Blockchain-prototype/alvenqis-release/vps-control-plane/compose.yaml'
tar -tzf "$output" | grep -Fxq 'Blockchain-prototype/alvenqis-release/vps-control-plane/scripts/install-docker-stack.sh'
tar -tzf "$output" | grep -Fxq 'Blockchain-prototype/alvenqis-release/vps-control-plane/docker/Dockerfile'

if tar -tzf "$output" | grep -Eq '(^|/)(\.env|state/[^.].*|target/|node_modules/|\.artifacts/)'; then
  echo "Forbidden runtime or generated file entered the Docker release archive." >&2
  exit 1
fi

echo "$output"
