#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
prototype="$repo_root/Blockchain-prototype"
output_dir="$repo_root/release-artifacts"
output="$output_dir/alvenqis-setup-external.tar.gz"
contents_file="$(mktemp "${TMPDIR:-/tmp}/alvenqis-setup-external-contents.XXXXXX")"
trap 'rm -f "$contents_file"' EXIT

cd "$repo_root"
git diff --quiet && git diff --cached --quiet || {
  echo "Refusing to build a release archive from a dirty working tree." >&2
  exit 73
}

# The Dockerfiles build the selected external-host applications from this
# source bundle. Include every referenced Cargo member and web application;
# exclude secrets, runtime state, desktop packages, and build output.
paths=(
  .dockerignore .gitattributes
  Blockchain-prototype/Cargo.toml Blockchain-prototype/Cargo.lock
  Blockchain-prototype/VERSION Blockchain-prototype/clippy.toml
  Blockchain-prototype/configs Blockchain-docs/human/release Blockchain-prototype/shared
  Blockchain-prototype/alvenqis-core Blockchain-prototype/alvenqis-node
  Blockchain-prototype/alvenqis-rpc-gateway Blockchain-prototype/alvenqis-wallet
  Blockchain-prototype/alvenqis-sdk-rust Blockchain-prototype/alvenqis-browser/host
  Blockchain-prototype/alvenqis-indexer Blockchain-prototype/alvenqis-miner
  Blockchain-prototype/alvenqis-mining-pool
  Blockchain-prototype/alvenqis-explorer Blockchain-prototype/alvenqis-website
  Blockchain-prototype/alvenqis-release/alvenqis-setup-external
)

mkdir -p "$output_dir"
git archive --format=tar HEAD -- "${paths[@]}" | gzip -9 > "$output"
(cd "$output_dir" && sha256sum "$(basename "$output")" > "$(basename "$output").sha256")

tar -tzf "$output" > "$contents_file"
required_entries=(
  Blockchain-prototype/alvenqis-release/alvenqis-setup-external/compose/base.yaml
  Blockchain-prototype/alvenqis-release/alvenqis-setup-external/compose/installer.yaml
  Blockchain-prototype/alvenqis-release/alvenqis-setup-external/compose/roles.json
  Blockchain-prototype/alvenqis-release/alvenqis-setup-external/APPLICATIONS.md
  Blockchain-prototype/alvenqis-release/alvenqis-setup-external/scripts/install-docker-stack.sh
  Blockchain-prototype/alvenqis-release/alvenqis-setup-external/docker/Dockerfile
  Blockchain-prototype/alvenqis-explorer/Dockerfile
  Blockchain-prototype/alvenqis-website/Dockerfile
)
for required_entry in "${required_entries[@]}"; do
  grep -Fxq "$required_entry" "$contents_file" || {
    echo "Required file is missing from Setup External archive: $required_entry" >&2
    exit 1
  }
done

if grep -Eq '(^|/)\.env$|(^|/)state/[^.]|(^|/)(target|node_modules|\.artifacts)/' "$contents_file"; then
  echo "Forbidden runtime or generated file entered the Docker release archive." >&2
  exit 1
fi

echo "$output"
