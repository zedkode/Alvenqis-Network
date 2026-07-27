#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: scripts/local/start-all.sh [--skip-explorer]"
  echo "Starts the local node, local RPC, refreshes the index snapshot and optionally starts the explorer."
  exit 0
fi

skip_explorer="false"
if [[ "${1:-}" == "--skip-explorer" ]]; then
  skip_explorer="true"
fi

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

ensure_local_directories
show_local_summary

start_background_process "node" "$WORKSPACE_ROOT" "$(node_start_command)"
for _ in $(seq 1 120); do
  if is_managed_process_running node && { [[ -f "$CHAIN_DIR/chain.sqlite3" ]] || [[ -f "$CHAIN_DIR/chain.jsonl" ]]; }; then
    break
  fi
  sleep 1
done

start_background_process "rpc" "$WORKSPACE_ROOT" "$(rpc_start_command)"
wait_for_http_ready "$RPC_URL/health" 120 1

refresh_index_snapshot

if [[ -d "$EXPLORER_DIR" && "$skip_explorer" != "true" ]]; then
  start_background_process "explorer" "$EXPLORER_DIR" "cd \"$EXPLORER_DIR\" && if [[ ! -d node_modules ]]; then npm install; fi && env VITE_ALVENQIS_RPC_URL=\"$RPC_URL\" npm run dev -- --host 127.0.0.1 --port 4173"
  echo "Explorer dev server requested at $EXPLORER_URL"
fi

echo "Logs:"
echo "  node: $(log_file_for node)"
echo "  rpc: $(log_file_for rpc)"
echo "  explorer: $(log_file_for explorer)"
