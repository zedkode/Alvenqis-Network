#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: scripts/local/stop-all.sh"
  echo "Stops local managed Alvenqis processes if they are running."
  exit 0
fi

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

stop_node_process
stop_managed_process rpc
stop_managed_process explorer

echo "Managed local processes stopped."
echo "Logs remain under $LOG_DIR"
