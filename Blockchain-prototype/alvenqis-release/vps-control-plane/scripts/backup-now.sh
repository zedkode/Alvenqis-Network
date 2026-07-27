#!/usr/bin/env bash
set -Eeuo pipefail

workspace="${ALVENQIS_WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$workspace"
source scripts/lib.sh
load_dotenv .env

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
out="state/backups/$stamp"
snapshot="state/backups/.snapshot-$stamp"
mkdir -p "$out" "$snapshot/state"

cleanup() {
  if [[ -d "$snapshot" ]]; then
    resolved_snapshot="$(cd "$(dirname "$snapshot")" && pwd)/$(basename "$snapshot")"
    resolved_backups="$(cd state/backups && pwd)"
    case "$resolved_snapshot" in
      "$resolved_backups"/.snapshot-*) rm -rf -- "$resolved_snapshot" ;;
      *) echo "Refusing to remove unexpected snapshot path: $resolved_snapshot" >&2 ;;
    esac
  fi
}
trap cleanup EXIT

for source in state/data state/control state/pool state/config/generated; do
  if [[ -e "$source" ]]; then
    mkdir -p "$snapshot/$(dirname "$source")"
    cp -a --reflink=auto "$source" "$snapshot/$(dirname "$source")/"
  fi
done
[[ -f .env ]] && cp -a .env "$snapshot/.env"

python3 - "$workspace" "$snapshot" <<'PY'
from __future__ import annotations

import sqlite3
import sys
from pathlib import Path

workspace = Path(sys.argv[1]).resolve()
snapshot = Path(sys.argv[2]).resolve()
database_count = 0

for source in sorted((workspace / "state").rglob("*.sqlite3")):
    if "backups" in source.relative_to(workspace / "state").parts:
        continue
    relative = source.relative_to(workspace)
    destination = snapshot / relative
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

print(f"online SQLite snapshots verified: {database_count}")
PY

tar -C "$snapshot" -czf "$out/alvenqis-state.tar.gz" state .env

pass=/run/secrets/backup_passphrase
[[ -s "$pass" ]] || pass=state/secrets/backup_passphrase
tar -czf - state/secrets |
  openssl enc -aes-256-cbc -salt -pbkdf2 -iter 200000 \
    -pass "file:$pass" \
    -out "$out/alvenqis-secrets.tar.gz.enc"

sha256sum "$out"/* > "$out/SHA256SUMS"

if [[ "${BACKUP_REMOTE_ENABLED:-false}" == true ]]; then
  secret=/run/secrets/r2_secret_access_key
  [[ -s "$secret" ]] || secret=state/secrets/r2_secret_access_key
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

find state/backups -mindepth 1 -maxdepth 1 -type d -name '20????????T??????Z' \
  -mtime "+${BACKUP_RETENTION_DAYS:-30}" -exec rm -rf -- {} +

mkdir -p state/metrics
now_unix="$(date -u +%s)"
cat > state/metrics/alvenqis_backup.prom.$$ <<EOF
# HELP alvenqis_backup_last_success_unixtime Unix time of the last successful Alvenqis backup
# TYPE alvenqis_backup_last_success_unixtime gauge
alvenqis_backup_last_success_unixtime ${now_unix}
EOF
mv -f state/metrics/alvenqis_backup.prom.$$ state/metrics/alvenqis_backup.prom

cleanup
trap - EXIT
echo "Online backup completed without stopping chain services: $out"
