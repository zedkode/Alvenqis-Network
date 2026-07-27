#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: scripts/local/reset-local-chain.sh [--no-backup]"
  echo "Stops local processes, creates a backup unless --no-backup is passed, and clears local chain, mempool and index data."
  exit 0
fi

no_backup="false"
if [[ "${1:-}" == "--no-backup" ]]; then
  no_backup="true"
fi

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

bash "$SCRIPT_DIR/stop-all.sh"

backup_path=""
if [[ "$no_backup" != "true" ]]; then
  backup_path="$(backup_local_data false)"
else
  echo "Skipping local backup because --no-backup was explicitly passed." >&2
fi

if [[ "$no_backup" != "true" && -z "$backup_path" ]]; then
  echo "Refusing reset because no backup was created." >&2
  exit 1
fi

clear_local_chain_state
echo "local reset complete"
[[ -n "$backup_path" ]] && echo "backup_path=$backup_path"
