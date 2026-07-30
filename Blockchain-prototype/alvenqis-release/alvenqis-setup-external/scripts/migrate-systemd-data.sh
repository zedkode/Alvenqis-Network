#!/usr/bin/env bash
set -Eeuo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
source scripts/lib.sh
[[ ! -f .env ]] || load_dotenv .env
export ALVENQIS_STATE_ROOT="${ALVENQIS_STATE_ROOT:-/var/lib/alvenqis-setup-external}"
resolve_state_root "$root"

if [[ $# -gt 0 ]]; then
  source_root="$1"
elif [[ -d /var/lib/alvenqis/.alvenqis-mainnet ]]; then
  source_root=/var/lib/alvenqis/.alvenqis-mainnet
else
  source_root=/var/lib/alvenqis/.alvenqis-mainnet
fi
[[ $EUID -eq 0 ]] || { echo "Run with sudo to read systemd-era data." >&2; exit 77; }
[[ -d "$source_root" ]] || { echo "Source not found: $source_root" >&2; exit 66; }

if systemctl is-active --quiet alvenqis-node 2>/dev/null \
  || systemctl is-active --quiet alvenqis-rpc 2>/dev/null; then
  echo "Stop the legacy services before migration:" >&2
  echo "  sudo systemctl stop alvenqis-indexer-refresh.timer alvenqis-rpc alvenqis-node" >&2
  exit 1
fi

bash scripts/prepare-state.sh
storage_key="$STATE_ROOT/secrets/alvenqis_storage_key"
source_storage_key="${ALVENQIS_STORAGE_KEY_SOURCE:-$source_root/secrets/alvenqis_storage_key}"
if [[ -f "$source_root/chain/state.rocksdb/CURRENT" ]]; then
  [[ -s "$source_storage_key" ]] || {
    echo "Legacy RocksDB exists but ALVENQIS_STORAGE_KEY_SOURCE is unavailable." >&2
    exit 78
  }
  grep -Eq '^[0-9A-Fa-f]{64}$' "$source_storage_key" || {
    echo "Legacy RocksDB storage key is invalid." >&2
    exit 78
  }
  if [[ -s "$storage_key" ]] && ! cmp -s "$source_storage_key" "$storage_key"; then
    echo "Destination storage key does not match legacy RocksDB." >&2
    exit 78
  fi
  install -m 0444 "$source_storage_key" "$storage_key"
elif [[ ! -s "$storage_key" ]]; then
  openssl rand -hex 32 > "$storage_key"
  chmod 0444 "$storage_key"
fi
for name in chain mempool indexer node; do
  [[ -d "$source_root/$name" ]] && rsync -aHAX "$source_root/$name/" "$STATE_ROOT/data/$name/"
done
for source in /var/lib/alvenqis-control; do
  [[ -d "$source" ]] && rsync -aHAX "$source/" "$STATE_ROOT/control/"
done
for source in /var/lib/alvenqis-pool; do
  [[ -d "$source" ]] && rsync -aHAX "$source/" "$STATE_ROOT/pool/"
done

chown -R 10001:10001 "$STATE_ROOT/data" "$STATE_ROOT/control" "$STATE_ROOT/pool"
echo "Legacy data migrated to $STATE_ROOT. The source directories were not deleted."
