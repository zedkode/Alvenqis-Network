#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: scripts/local/run-local-smoke-test.sh [--skip-release-gate]"
  echo "Runs the local operator smoke test for node, RPC, wallet, indexer and explorer build readiness."
  exit 0
fi

skip_release_gate="false"
if [[ "${1:-}" == "--skip-release-gate" ]]; then
  skip_release_gate="true"
fi

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

cleanup() {
  bash "$SCRIPT_DIR/stop-all.sh" >/dev/null 2>&1 || true
}

trap cleanup EXIT

if [[ "$skip_release_gate" != "true" && -f "$WORKSPACE_ROOT/scripts/release/release-gate.sh" ]]; then
  echo "Running release gate before local smoke test..."
  run_release_gate
fi

bash "$SCRIPT_DIR/start-all.sh" --skip-explorer
run_node validate-chain
run_node mine-block
run_node validate-chain
curl -fsS "$RPC_URL/health"
echo
curl -fsS "$RPC_URL/network"
echo
run_wallet create-wallet
run_wallet address
refresh_index_snapshot
run_indexer status

if [[ -d "$EXPLORER_DIR" ]]; then
  (
    cd "$EXPLORER_DIR"
    npm install
    npm run build
  )
fi

echo "local smoke test passed"
echo "logs_dir=$LOG_DIR"
