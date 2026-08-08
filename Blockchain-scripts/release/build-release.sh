#!/usr/bin/env bash
set -euo pipefail

"$(dirname "$0")/check-release.sh"

output_dir="$(pwd)/release-artifacts/mainnet-candidate"
rm -rf "$output_dir"
mkdir -p "$output_dir"

cargo build --workspace --release
cp README.md "$output_dir/"
cp -R configs "$output_dir/"
cp -R docs/release "$output_dir/"

echo "Release artifacts prepared in $output_dir"
