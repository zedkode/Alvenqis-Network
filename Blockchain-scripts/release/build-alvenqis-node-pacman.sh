#!/usr/bin/env bash
# Packages the release alvenqis-node binary as a pacman package via nfpm.
# Expects cargo build --release -p alvenqis-node to have run already
# The caller must provide the release tag in RELEASE_TAG.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
workspace="$repo_root/Blockchain-prototype"
cd "$repo_root"

NFPM_VERSION="2.43.3"

tag="${RELEASE_TAG:?RELEASE_TAG (e.g. v0.3.2-candidate.1) is required}"
# pacman versions cannot contain hyphens; v0.3.2-candidate.1 -> 0.3.2.candidate.1
export ALVENQIS_NODE_VERSION="$(echo "${tag#v}" | tr '-' '.')"
target_dir="${CARGO_TARGET_DIR:-$workspace/target}"

test -x "$target_dir/release/alvenqis-node" || { echo "alvenqis-node release binary missing; build it first" >&2; exit 1; }
# nfpm only expands env vars in selected fields (version), not in content
# src paths, so stage the binary at the fixed path the config references.
install -D -m 0755 "$target_dir/release/alvenqis-node" release-artifacts/nfpm-stage/alvenqis-node

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL "https://github.com/goreleaser/nfpm/releases/download/v${NFPM_VERSION}/nfpm_${NFPM_VERSION}_Linux_x86_64.tar.gz" | tar -xz -C "$tmp" nfpm

mkdir -p release-artifacts
"$tmp/nfpm" package \
  --config Blockchain-scripts/release/nfpm-alvenqis-node.yaml \
  --packager archlinux \
  --target release-artifacts/
ls release-artifacts/alvenqis-node-*.pkg.tar.zst
