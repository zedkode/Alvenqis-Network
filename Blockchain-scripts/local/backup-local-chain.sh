#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: scripts/local/backup-local-chain.sh [--include-wallets]"
  echo "Creates a timestamped backup of local chain, mempool, index and logs. Wallets are excluded by default."
  exit 0
fi

include_wallets="false"
if [[ "${1:-}" == "--include-wallets" ]]; then
  include_wallets="true"
fi

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

backup_path="$(backup_local_data "$include_wallets")"
echo "backup_path=$backup_path"
