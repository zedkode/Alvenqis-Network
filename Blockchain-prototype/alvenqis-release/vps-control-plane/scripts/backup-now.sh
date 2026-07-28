#!/usr/bin/env bash
set -Eeuo pipefail

workspace="${ALVENQIS_WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$workspace"
source scripts/lib.sh
load_dotenv .env
resolve_state_root "$workspace"
compose_args

install -d -m 0750 "$STATE_ROOT/backups"
exec 9>"$STATE_ROOT/backups/.backup-restore.lock"
flock -n 9 || {
  echo "Another Alvenqis backup or restore operation is already running." >&2
  exit 75
}

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
out="$STATE_ROOT/backups/$stamp"
snapshot="$STATE_ROOT/backups/.snapshot-$stamp"
mkdir -p "$out" "$snapshot/state"
completed=false

cleanup() {
  if [[ -d "$snapshot" ]]; then
    resolved_snapshot="$(cd "$(dirname "$snapshot")" && pwd)/$(basename "$snapshot")"
    resolved_backups="$(cd "$STATE_ROOT/backups" && pwd)"
    case "$resolved_snapshot" in
      "$resolved_backups"/.snapshot-*) rm -rf -- "$resolved_snapshot" ;;
      *) echo "Refusing to remove unexpected snapshot path: $resolved_snapshot" >&2 ;;
    esac
  fi
  if [[ "$completed" != true && -d "$out" ]]; then
    resolved_out="$(cd "$(dirname "$out")" && pwd)/$(basename "$out")"
    resolved_backups="$(cd "$STATE_ROOT/backups" && pwd)"
    case "$resolved_out" in
      "$resolved_backups"/20??????T??????Z) rm -rf -- "$resolved_out" ;;
      *) echo "Refusing to remove unexpected incomplete backup path: $resolved_out" >&2 ;;
    esac
  fi
}
trap cleanup EXIT

mkdir -p "$snapshot/state/data"
rsync -aHAX --delete \
  --exclude '/chain/state.rocksdb/' \
  --exclude '/chain/state.rocksdb.lock' \
  --exclude '/chain/.alvenqis-state-restore-*/' \
  "$STATE_ROOT/data/" "$snapshot/state/data/"
for relative in \
  control pool config/generated stratum \
  prometheus grafana loki alloy alertmanager; do
  source="$STATE_ROOT/$relative"
  if [[ -e "$source" ]]; then
    mkdir -p "$snapshot/state/$(dirname "$relative")"
    cp -a --reflink=auto "$source" "$snapshot/state/$(dirname "$relative")/"
  fi
done
[[ -f .env ]] && cp -a .env "$snapshot/.env"

attempt=1
while :; do
  rocks_output="$("${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-node \
    alvenqis-node \
      --config /config/node.toml \
      --data-dir /data/.alvenqis-mainnet/chain \
      backup-rocksdb \
      --backup-repository /backups/rocksdb-repository \
      --backups-to-keep "${ROCKSDB_BACKUPS_TO_KEEP:-30}")"
  rocks_json="$(printf '%s\n' "$rocks_output" | sed -n '/^{/,$p')"
  read -r rocks_backup_id rocks_network_id rocks_block_count rocks_height rocks_hash rocks_encryption rocks_key_id < <(
    python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["backup_id"], d["network_id"], d["block_count"], d["tip_height"], d["tip_hash"], d["encryption"], d["key_id"])' \
      <<<"$rocks_json"
  )
  [[ "$rocks_encryption" == xchacha20poly1305 && "$rocks_key_id" != none ]] || {
    echo "RocksDB backup is not authenticated encrypted storage." >&2
    exit 74
  }

  python3 - "$STATE_ROOT" "$snapshot" <<'PY'
from __future__ import annotations

import sqlite3
import sys
from pathlib import Path

state_root = Path(sys.argv[1]).resolve()
snapshot = Path(sys.argv[2]).resolve()
database_count = 0

for source in sorted(state_root.rglob("*.sqlite3")):
    if "backups" in source.relative_to(state_root).parts:
        continue
    relative = source.relative_to(state_root)
    destination = snapshot / "state" / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.unlink(missing_ok=True)
    for suffix in ("-wal", "-shm"):
        Path(f"{destination}{suffix}").unlink(missing_ok=True)

    source_uri = f"file:{source.as_posix()}?mode=ro"
    with sqlite3.connect(source_uri, uri=True, timeout=30) as source_db:
        with sqlite3.connect(destination, timeout=30) as destination_db:
            source_db.backup(destination_db, pages=2048, sleep=0.05)
            result = destination_db.execute("PRAGMA integrity_check").fetchone()
            if not result or result[0] != "ok":
                raise SystemExit(f"SQLite integrity check failed for {relative}: {result}")
    database_count += 1

chain_database = snapshot / "state" / "data" / "chain" / "chain.sqlite3"
if not chain_database.is_file():
    raise SystemExit(f"canonical SQLite snapshot is missing: {chain_database}")
with sqlite3.connect(chain_database, timeout=30) as chain_db:
    tip = chain_db.execute(
        "SELECT height, hash FROM canonical_blocks ORDER BY height DESC LIMIT 1"
    ).fetchone()
if tip is None:
    raise SystemExit("canonical SQLite snapshot has no tip")
(snapshot / "sqlite-chain-tip").write_text(f"{tip[0]} {tip[1]}\n", encoding="utf-8")
print(f"online SQLite snapshots verified: {database_count}")
PY
  read -r sqlite_height sqlite_hash < "$snapshot/sqlite-chain-tip"
  if [[ "$sqlite_height" == "$rocks_height" && "$sqlite_hash" == "$rocks_hash" ]]; then
    break
  fi
  if ((attempt >= 5)); then
    echo "Could not obtain matching RocksDB/SQLite backup tips after $attempt attempts." >&2
    exit 74
  fi
  attempt=$((attempt + 1))
  sleep 2
done

tar -C "$STATE_ROOT/backups" -czf "$out/alvenqis-rocksdb-backup.tar.gz" \
  rocksdb-repository

archive_items=(state)
[[ -f "$snapshot/.env" ]] && archive_items+=(.env)
tar --dereference -C "$snapshot" -czf "$out/alvenqis-state.tar.gz" "${archive_items[@]}"

pass=/run/secrets/backup_passphrase
[[ -s "$pass" ]] || pass="$STATE_ROOT/secrets/backup_passphrase"
mkdir -p "$snapshot/state"
cp -a "$STATE_ROOT/secrets" "$snapshot/state/secrets"
tar -C "$snapshot" -czf - state/secrets |
  openssl enc -aes-256-cbc -salt -pbkdf2 -iter 200000 \
    -pass "file:$pass" \
    -out "$out/alvenqis-secrets.tar.gz.enc"
rm -rf -- "$snapshot/state/secrets"

cat > "$out/BACKUP_COMPLETE" <<EOF
created_utc=$stamp
state_root=$STATE_ROOT
sqlite_integrity=ok
rocksdb_backup_id=$rocks_backup_id
rocksdb_network_id=$rocks_network_id
rocksdb_block_count=$rocks_block_count
rocksdb_tip_height=$rocks_height
rocksdb_tip_hash=$rocks_hash
rocksdb_encryption=$rocks_encryption
rocksdb_key_id=$rocks_key_id
EOF
(
  cd "$out"
  sha256sum \
    BACKUP_COMPLETE \
    alvenqis-rocksdb-backup.tar.gz \
    alvenqis-secrets.tar.gz.enc \
    alvenqis-state.tar.gz > SHA256SUMS
)

if [[ "${BACKUP_REMOTE_ENABLED:-false}" == true ]]; then
  secret=/run/secrets/r2_secret_access_key
  [[ -s "$secret" ]] || secret="$STATE_ROOT/secrets/r2_secret_access_key"
  export RCLONE_CONFIG_R2_TYPE=s3
  export RCLONE_CONFIG_R2_PROVIDER=Other
  export RCLONE_CONFIG_R2_ENDPOINT="$R2_ENDPOINT"
  export RCLONE_CONFIG_R2_ACCESS_KEY_ID="$R2_ACCESS_KEY_ID"
  export RCLONE_CONFIG_R2_SECRET_ACCESS_KEY
  RCLONE_CONFIG_R2_SECRET_ACCESS_KEY="$(cat "$secret")"
  export RCLONE_CONFIG_R2_REGION="${R2_REGION:-auto}"
  rclone copy "$out" "r2:${R2_BUCKET}/alvenqis/$stamp" --checksum
  unset RCLONE_CONFIG_R2_SECRET_ACCESS_KEY
fi

find "$STATE_ROOT/backups" -mindepth 1 -maxdepth 1 -type d -name '20??????T??????Z' \
  -mtime "+${BACKUP_RETENTION_DAYS:-30}" -exec rm -rf -- {} +

mkdir -p "$STATE_ROOT/metrics"
now_unix="$(date -u +%s)"
cat > "$STATE_ROOT/metrics/alvenqis_backup.prom.$$" <<EOF
# HELP alvenqis_backup_last_success_unixtime Unix time of the last successful Alvenqis backup
# TYPE alvenqis_backup_last_success_unixtime gauge
alvenqis_backup_last_success_unixtime ${now_unix}
EOF
mv -f "$STATE_ROOT/metrics/alvenqis_backup.prom.$$" "$STATE_ROOT/metrics/alvenqis_backup.prom"

completed=true
cleanup
trap - EXIT
echo "Online backup completed without stopping chain services: $out"
