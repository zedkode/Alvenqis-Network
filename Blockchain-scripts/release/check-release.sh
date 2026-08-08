#!/usr/bin/env bash
set -euo pipefail

echo "Running Alvenqis mainnet-candidate release checks..."
cargo fmt --all --check
cargo test --workspace --tests
cargo clippy --workspace --all-targets -- -D warnings

if [[ -f alvenqis-explorer/package.json ]]; then
  pushd alvenqis-explorer >/dev/null
  npm install
  npm run build
  popd >/dev/null
fi

echo "Release checks passed."
