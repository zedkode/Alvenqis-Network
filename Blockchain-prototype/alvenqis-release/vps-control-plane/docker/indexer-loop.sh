#!/usr/bin/env bash
set -uo pipefail

chain_dir="${ALVENQIS_CHAIN_ROOT:-/data/.alvenqis-mainnet}/chain"
index_dir="${ALVENQIS_CHAIN_ROOT:-/data/.alvenqis-mainnet}/indexer"
interval="${INDEXER_INTERVAL_SECONDS:-5}"
max_backoff="${INDEXER_FAILURE_BACKOFF_MAX_SECONDS:-60}"

[[ "$interval" =~ ^[0-9]+$ ]] && (( interval >= 1 && interval <= 60 )) || {
  echo "INDEXER_INTERVAL_SECONDS must be between 1 and 60." >&2
  exit 64
}
[[ "$max_backoff" =~ ^[0-9]+$ ]] && (( max_backoff >= 5 && max_backoff <= 300 )) || {
  echo "INDEXER_FAILURE_BACKOFF_MAX_SECONDS must be between 5 and 300." >&2
  exit 64
}

backoff=2
while :; do
  started="$(date +%s)"
  if alvenqis-indexer \
    --network mainnet-candidate \
    --chain-data-dir "$chain_dir" \
    --index-dir "$index_dir" \
    sync; then
    finished="$(date +%s)"
    printf '%s\n' "$(date -u +%FT%TZ)" > "$index_dir/.last-success"
    printf 'duration_seconds=%s\n' "$((finished - started))" > "$index_dir/.last-run"
    rm -f "$index_dir/.last-error"
    backoff=2
    sleep "$interval"
  else
    printf '%s sync failed\n' "$(date -u +%FT%TZ)" > "$index_dir/.last-error"
    echo "Indexer sync failed; retrying in ${backoff}s." >&2
    sleep "$backoff"
    backoff=$((backoff * 2))
    (( backoff <= max_backoff )) || backoff="$max_backoff"
  fi
done
