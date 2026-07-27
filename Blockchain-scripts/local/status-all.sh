#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: scripts/local/status-all.sh"
  echo "Shows local process state, chain validation, RPC health and index snapshot status."
  exit 0
fi

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

ensure_local_directories
show_local_summary

echo
echo "Managed processes:"
for name in node rpc explorer; do
  if is_managed_process_running "$name"; then
    echo "  $name: running pid $(stored_pid "$name")"
  else
    echo "  $name: stopped"
  fi
done

echo
echo "Node status:"
run_node node-status || true

echo
echo "Chain validation:"
run_node validate-chain || true

echo
echo "Mempool status:"
run_node mempool-status || true

echo
echo "Indexer status:"
run_indexer status || true

echo
echo "RPC health:"
curl -fsS "$RPC_URL/health" || true

echo
echo "RPC network:"
curl -fsS "$RPC_URL/network" || true

echo
echo "Latest block:"
curl -fsS "$RPC_URL/blocks/latest" || true
echo
