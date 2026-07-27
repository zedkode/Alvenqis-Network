#!/usr/bin/env bash
set -Eeuo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ $# -gt 0 ]]; then
  source_root="$1"
elif [[ -d /var/lib/alvenqis/.alvenqis-mainnet ]]; then
  source_root=/var/lib/alvenqis/.alvenqis-mainnet
elif [[ -d /var/lib/alvenqis/.alvenqis-mainnet ]]; then
  source_root=/var/lib/alvenqis/.alvenqis-mainnet
else
  source_root=/var/lib/alvenqis/.alvenqis-mainnet
fi
[[ $EUID -eq 0 ]] || { echo "Run with sudo to read systemd-era data." >&2; exit 77; }
[[ -d "$source_root" ]] || { echo "Source not found: $source_root" >&2; exit 66; }

if systemctl is-active --quiet alvenqis-node 2>/dev/null \
  || systemctl is-active --quiet alvenqis-node 2>/dev/null; then
  echo "Stop the legacy services before migration:" >&2
  echo "  sudo systemctl stop alvenqis-indexer-refresh.timer alvenqis-rpc alvenqis-node alvenqis-indexer-refresh.timer alvenqis-rpc alvenqis-node" >&2
  exit 1
fi

mkdir -p state/data/{chain,mempool,indexer,node} state/control state/pool
for name in chain mempool indexer node; do
  [[ -d "$source_root/$name" ]] && rsync -aHAX "$source_root/$name/" "state/data/$name/"
done
for source in /var/lib/alvenqis-control /var/lib/alvenqis-control; do
  [[ -d "$source" ]] && rsync -aHAX "$source/" state/control/
done
for source in /var/lib/alvenqis-pool /var/lib/alvenqis-pool; do
  [[ -d "$source" ]] && rsync -aHAX "$source/" state/pool/
done

chown -R 10001:10001 state/data state/control state/pool
echo "Legacy data migrated. The source directories were not deleted."
