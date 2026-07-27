#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: scripts/local/mine-local-block.sh"
  echo "Mines one local block, refreshes the index snapshot, validates the chain and prints the latest block if RPC is running."
  exit 0
fi

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

run_node mine-block
run_node validate-chain
refresh_index_snapshot
curl -fsS "$RPC_URL/blocks/latest" || true
echo
