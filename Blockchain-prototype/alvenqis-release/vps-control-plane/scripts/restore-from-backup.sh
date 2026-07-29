#!/usr/bin/env bash
set -Eeuo pipefail

workspace="${ALVENQIS_WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$workspace"
source scripts/lib.sh

usage() {
  cat <<'EOF'
Usage:
  RESTORE_CONFIRM=yes bash scripts/restore-from-backup.sh /absolute/backup/directory

Environment:
  RESTORE_CONFIRM=yes        Required.
  RESTORE_SECRETS=true|false Default true.
  ALVENQIS_WORKSPACE         Optional package root override.

The project is stopped for a consistent restore. Only containers positively
owned by COMPOSE_PROJECT_NAME and this Compose working directory are touched.
EOF
}

[[ "${1:-}" != "-h" && "${1:-}" != "--help" ]] || {
  usage
  exit 0
}
[[ $# -eq 1 ]] || {
  usage >&2
  exit 64
}
[[ "${RESTORE_CONFIRM:-}" == "yes" ]] || {
  echo "Refusing restore without RESTORE_CONFIRM=yes." >&2
  exit 64
}
[[ -f .env ]] || {
  echo "A live .env is required to identify the owned Compose project safely." >&2
  exit 66
}

load_dotenv .env
resolve_state_root "$workspace"
compose_args
project="${COMPOSE_PROJECT_NAME:-alvenqis-control-plane}"

install -d -m 0750 "$STATE_ROOT/backups"
exec 9>"$STATE_ROOT/backups/.backup-restore.lock"
flock -n 9 || {
  echo "Another Alvenqis backup or restore operation is already running." >&2
  exit 75
}

backup_arg="$1"
if [[ "$backup_arg" = /* ]]; then
  backup_dir="$backup_arg"
elif [[ -d "$workspace/$backup_arg" ]]; then
  backup_dir="$workspace/$backup_arg"
elif [[ "$backup_arg" == state/backups/* ]]; then
  backup_dir="$STATE_ROOT/backups/${backup_arg#state/backups/}"
else
  backup_dir="$STATE_ROOT/backups/$backup_arg"
fi
backup_dir="$(
  python3 - "$backup_dir" <<'PY'
import sys
from pathlib import Path
print(Path(sys.argv[1]).resolve(strict=False))
PY
)"

[[ -d "$backup_dir" ]] || {
  echo "Backup directory not found: $backup_dir" >&2
  exit 66
}
state_archive="$backup_dir/alvenqis-state.tar.gz"
rocks_archive="$backup_dir/alvenqis-rocksdb-backup.tar.gz"
secrets_archive="$backup_dir/alvenqis-secrets.tar.gz.enc"
sums_file="$backup_dir/SHA256SUMS"
completion_marker="$backup_dir/BACKUP_COMPLETE"
[[ -f "$state_archive" ]] || {
  echo "Missing state archive: $state_archive" >&2
  exit 66
}
[[ -f "$sums_file" ]] || {
  echo "Refusing restore without SHA256SUMS: $backup_dir" >&2
  exit 74
}
[[ -f "$rocks_archive" && -f "$completion_marker" ]] || {
  echo "Refusing restore without RocksDB backup archive and completion marker." >&2
  exit 74
}
python3 - "$sums_file" <<'PY'
import re
import sys
from pathlib import Path

expected = {
    "BACKUP_COMPLETE",
    "alvenqis-rocksdb-backup.tar.gz",
    "alvenqis-secrets.tar.gz.enc",
    "alvenqis-state.tar.gz",
}
seen = set()
for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines():
    match = re.fullmatch(r"([0-9a-f]{64}) [ *](\S+)", line)
    if not match:
        raise SystemExit("SHA256SUMS contains an invalid entry")
    name = match.group(2)
    if name not in expected or name in seen:
        raise SystemExit(f"SHA256SUMS contains an unexpected or duplicate path: {name}")
    seen.add(name)
if seen != expected:
    raise SystemExit(f"SHA256SUMS does not cover the exact backup set: {sorted(expected - seen)}")
print("Backup checksum manifest validation: ok")
PY
(
  cd "$backup_dir"
  sha256sum -c SHA256SUMS
)

python3 - "$state_archive" <<'PY'
import sys
import tarfile
from pathlib import PurePosixPath

with tarfile.open(sys.argv[1], "r:gz") as archive:
    for member in archive.getmembers():
        path = PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts:
            raise SystemExit(f"unsafe archive member: {member.name}")
        if not (member.name == ".env" or member.name == "state" or member.name.startswith("state/")):
            raise SystemExit(f"unexpected archive member: {member.name}")
        if not (member.isfile() or member.isdir()):
            raise SystemExit(f"unsupported restore archive member type: {member.name}")
print("Backup archive path validation: ok")
PY

python3 - "$rocks_archive" <<'PY'
import sys
import tarfile
from pathlib import PurePosixPath

with tarfile.open(sys.argv[1], "r:gz") as archive:
    for member in archive.getmembers():
        path = PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts:
            raise SystemExit(f"unsafe RocksDB archive member: {member.name}")
        if member.name != "rocksdb-repository" and not member.name.startswith(
            "rocksdb-repository/"
        ):
            raise SystemExit(f"unexpected RocksDB archive member: {member.name}")
        if not (member.isfile() or member.isdir()):
            raise SystemExit(f"unsupported RocksDB archive member type: {member.name}")
print("RocksDB backup archive path validation: ok")
PY

marker_value() {
  local key="$1"
  awk -F= -v key="$key" '$1 == key {sub(/^[^=]*=/, ""); print; exit}' "$completion_marker"
}
rocks_backup_id="$(marker_value rocksdb_backup_id)"
rocks_network_id="$(marker_value rocksdb_network_id)"
rocks_block_count="$(marker_value rocksdb_block_count)"
rocks_tip_height="$(marker_value rocksdb_tip_height)"
rocks_tip_hash="$(marker_value rocksdb_tip_hash)"
rocks_encryption="$(marker_value rocksdb_encryption)"
rocks_key_id="$(marker_value rocksdb_key_id)"
[[ "$rocks_backup_id" =~ ^[0-9]+$ \
  && "$rocks_block_count" =~ ^[0-9]+$ \
  && "$rocks_tip_height" =~ ^[0-9]+$ ]] || {
  echo "RocksDB backup marker has invalid numeric fields." >&2
  exit 74
}
[[ "$rocks_network_id" =~ ^[a-z0-9][a-z0-9._-]{2,63}$ \
  && "$rocks_tip_hash" =~ ^[0-9a-f]{64}$ \
  && "$rocks_key_id" =~ ^[0-9a-f]{16}$ \
  && "$rocks_encryption" == xchacha20poly1305 ]] || {
  echo "RocksDB backup marker has invalid authentication metadata." >&2
  exit 74
}

mapfile -t project_containers < <(
  docker ps -aq --filter "label=com.docker.compose.project=$project"
)
((${#project_containers[@]} > 0)) || {
  echo "No containers found for Compose project $project." >&2
  exit 69
}
for container_id in "${project_containers[@]}"; do
  name="$(docker inspect -f '{{.Name}}' "$container_id")"
  name="${name#/}"
  working_dir="$(docker inspect -f '{{index .Config.Labels "com.docker.compose.project.working_dir"}}' "$container_id")"
  config_files="$(docker inspect -f '{{index .Config.Labels "com.docker.compose.project.config_files"}}' "$container_id")"
  [[ "$working_dir" == "$workspace" ]] && compose_config_files_match "$config_files" || {
    echo "Container ownership mismatch; refusing to touch $name." >&2
    exit 73
  }
  case "$name" in
    1Panel-*|n8n|n8n-db|vaultwarden|gitea|gitea-db|uptime-kuma|rustfs|vbos|vbos-*)
      echo "Protected container unexpectedly belongs to $project: $name" >&2
      exit 73
      ;;
  esac
done

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
pre_dir="$STATE_ROOT/backups/pre-restore-$stamp"
stage="$STATE_ROOT/backups/.restore-$stamp"
install -d -m 0700 "$pre_dir" "$stage"
completed=false
stopped=false
live_mutated=false
managed_paths=(
  data control pool config/generated stratum
  prometheus grafana loki alloy alertmanager
  secrets backups/rocksdb-repository
)

cleanup() {
  local exit_code=$?
  set +e
  if [[ "$completed" != true && "$live_mutated" == true \
    && -f "$pre_dir/pre-restore-live.tar.gz" ]]; then
    echo "Restore failed after live mutation; rolling back the pre-restore snapshot." >&2
    rollback_stage="$stage/rollback"
    install -d -m 0700 "$rollback_stage"
    tar -xzf "$pre_dir/pre-restore-live.tar.gz" -C "$rollback_stage"
    for relative in "${managed_paths[@]}"; do
      source_path="$rollback_stage/state/$relative"
      destination_path="$STATE_ROOT/$relative"
      if [[ -d "$source_path" ]]; then
        install -d "$destination_path"
        rsync -aHAX --numeric-ids --delete "$source_path/" "$destination_path/"
      elif [[ -e "$destination_path" ]]; then
        case "$destination_path" in
          "$STATE_ROOT"/*) rm -rf -- "$destination_path" ;;
          *) echo "Refusing rollback removal outside state root: $destination_path" >&2 ;;
        esac
      fi
    done
    if [[ -f "$rollback_stage/.env" ]]; then
      cp -a "$rollback_stage/.env" .env
    fi
    load_dotenv .env
    resolve_state_root "$workspace"
    bash scripts/prepare-state.sh
    compose_args
    live_mutated=false
  fi
  if [[ "$stopped" == true ]]; then
    echo "Restarting the owned Alvenqis project after restore handling." >&2
    "${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" \
      up -d --no-build || true
  fi
  if [[ -d "$stage" ]]; then
    case "$stage" in
      "$STATE_ROOT"/backups/.restore-*) rm -rf -- "$stage" ;;
      *) echo "Refusing to remove unexpected restore stage: $stage" >&2 ;;
    esac
  fi
  return "$exit_code"
}
trap cleanup EXIT

echo "Extracting verified backup into an isolated stage..."
tar -xzf "$state_archive" -C "$stage"
rocks_stage="$stage/rocks"
install -d -m 0700 "$rocks_stage"
tar -xzf "$rocks_archive" -C "$rocks_stage"
[[ -d "$stage/state" ]] || {
  echo "Backup archive does not contain state/." >&2
  exit 74
}

restore_secrets="${RESTORE_SECRETS:-true}"
[[ "$restore_secrets" == true || "$restore_secrets" == false ]] || {
  echo "RESTORE_SECRETS must be true or false." >&2
  exit 64
}
if [[ "$restore_secrets" == true ]]; then
  [[ -f "$secrets_archive" ]] || {
    echo "Secrets restore requested, but the encrypted secrets archive is missing." >&2
    exit 66
  }
  pass=/run/secrets/backup_passphrase
  [[ -s "$pass" ]] || pass="$STATE_ROOT/secrets/backup_passphrase"
  [[ -s "$pass" ]] || {
    echo "Backup passphrase is unavailable; refusing secrets restore." >&2
    exit 66
  }
  secrets_stage="$stage/decrypted"
  install -d -m 0700 "$secrets_stage"
  decrypted_secrets_archive="$stage/alvenqis-secrets.tar.gz"
  openssl enc -d -aes-256-cbc -salt -pbkdf2 -iter 200000 \
    -pass "file:$pass" -in "$secrets_archive" \
    -out "$decrypted_secrets_archive"
  python3 - "$decrypted_secrets_archive" <<'PY'
import sys
import tarfile
from pathlib import PurePosixPath

with tarfile.open(sys.argv[1], "r:gz") as archive:
    for member in archive.getmembers():
        path = PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts:
            raise SystemExit(f"unsafe secrets archive member: {member.name}")
        if member.name != "state/secrets" and not member.name.startswith("state/secrets/"):
            raise SystemExit(f"unexpected secrets archive member: {member.name}")
        if not (member.isfile() or member.isdir()):
            raise SystemExit(f"unsupported secrets archive member type: {member.name}")
print("Encrypted secrets archive path validation: ok")
PY
  tar -xzf "$decrypted_secrets_archive" -C "$secrets_stage"
  rm -f -- "$decrypted_secrets_archive"
  [[ -d "$secrets_stage/state/secrets" ]] || {
    echo "Encrypted archive does not contain state/secrets." >&2
    exit 74
  }
  selected_key="$secrets_stage/state/secrets/alvenqis_storage_key"
else
  selected_key="$STATE_ROOT/secrets/alvenqis_storage_key"
fi

[[ -f "$selected_key" && ! -L "$selected_key" ]] || {
  echo "Selected RocksDB storage key is missing or unsafe." >&2
  exit 74
}
grep -Eq '^[0-9A-Fa-f]{64}$' "$selected_key" || {
  echo "Selected RocksDB storage key is invalid." >&2
  exit 74
}
actual_key_id="$(
  python3 - "$selected_key" <<'PY'
import hashlib
import sys
from pathlib import Path

raw = Path(sys.argv[1]).read_text(encoding="utf-8").strip()
print(hashlib.sha256(bytes.fromhex(raw)).hexdigest()[:16])
PY
)"
[[ "$actual_key_id" == "$rocks_key_id" ]] || {
  echo "RocksDB backup key-id does not match the selected restore key." >&2
  exit 74
}
[[ -d "$rocks_stage/rocksdb-repository" ]] || {
  echo "RocksDB backup repository is missing from the archive." >&2
  exit 74
}
candidate="$stage/state"
candidate_chain="$candidate/data/chain"
candidate_config="$candidate/config/generated/node.toml"
[[ -f "$candidate_chain/chain.sqlite3" && -f "$candidate_config" ]] || {
  echo "Backup candidate lacks canonical SQLite or generated node configuration." >&2
  exit 74
}
[[ ! -e "$candidate_chain/state.rocksdb" && ! -L "$candidate_chain/state.rocksdb" ]] || {
  echo "State archive unexpectedly contains a live RocksDB directory." >&2
  exit 74
}
chown -R 10001:10001 "$candidate/data" "$candidate/config/generated"
chown -R 10001:10001 "$rocks_stage/rocksdb-repository"
chmod 0444 "$selected_key"

status_json() {
  sed -n '/^{/,$p'
}

assert_rocks_status() {
  local payload="$1"
  local context="$2"
  local actual_network actual_blocks actual_height actual_hash actual_encryption actual_key
  read -r actual_network actual_blocks actual_height actual_hash actual_encryption actual_key < <(
    python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["network_id"], d["block_count"], d["tip_height"], d["tip_hash"], d["encryption"], d["key_id"])' \
      <<<"$payload"
  )
  [[ "$actual_network" == "$rocks_network_id" \
    && "$actual_blocks" == "$rocks_block_count" \
    && "$actual_height" == "$rocks_tip_height" \
    && "$actual_hash" == "$rocks_tip_hash" \
    && "$actual_encryption" == "$rocks_encryption" \
    && "$actual_key" == "$rocks_key_id" ]] || {
    echo "$context does not match authenticated backup metadata." >&2
    exit 74
  }
}

candidate_cli() {
  "${ALVENQIS_COMPOSE_ARGS[@]}" run --rm --no-deps \
    --entrypoint /usr/local/bin/alvenqis-node \
    -e ALVENQIS_STORAGE_KEY_FILE=/restore-key \
    -e ALVENQIS_REQUIRE_STORAGE_ENCRYPTION=true \
    -e ALVENQIS_ALLOW_PLAINTEXT_STORAGE_MIGRATION=false \
    -v "$candidate:/candidate" \
    -v "$rocks_stage/rocksdb-repository:/restore-repository" \
    -v "$selected_key:/restore-key:ro" \
    alvenqis-node "$@"
}

candidate_restore_dir="$candidate/data/.rocks-restore-$stamp"
install -d -m 0750 -o 10001 -g 10001 "$candidate_restore_dir"
restore_output="$(
  candidate_cli \
    --config /candidate/config/generated/node.toml \
    --data-dir "/candidate/data/.rocks-restore-$stamp" \
    restore-latest-rocksdb \
    --backup-repository /restore-repository
)"
restore_json="$(printf '%s\n' "$restore_output" | status_json)"
assert_rocks_status "$restore_json" "Staged RocksDB restore"
[[ -s "$candidate_restore_dir/state.rocksdb/CURRENT" ]] || {
  echo "Staged RocksDB restore did not create a complete database." >&2
  exit 74
}
rm -f -- "$candidate_restore_dir/state.rocksdb.lock"
mv -- "$candidate_restore_dir/state.rocksdb" "$candidate_chain/state.rocksdb"
rmdir -- "$candidate_restore_dir"

verify_output="$(
  candidate_cli \
    --config /candidate/config/generated/node.toml \
    --data-dir /candidate/data/chain \
    verify-rocksdb
)"
verify_json="$(printf '%s\n' "$verify_output" | status_json)"
assert_rocks_status "$verify_json" "Staged RocksDB/SQLite verification"
echo "Staged RocksDB restore and full SQLite replay verification: ok"

echo "Stopping only positively identified project $project..."
"${ALVENQIS_COMPOSE_ARGS[@]}" \
  --profile cloudflare --profile pool --profile backup stop
stopped=true

echo "Creating pre-restore safety snapshot..."
pre_stage="$stage/pre-live"
install -d -m 0700 "$pre_stage/state"
for relative in "${managed_paths[@]}"; do
  if [[ -e "$STATE_ROOT/$relative" ]]; then
    mkdir -p "$pre_stage/state/$(dirname "$relative")"
    cp -a --reflink=auto \
      "$STATE_ROOT/$relative" "$pre_stage/state/$(dirname "$relative")/"
  fi
done
cp -a .env "$pre_stage/.env"
tar -C "$pre_stage" -czf "$pre_dir/pre-restore-live.tar.gz" state .env
(
  cd "$pre_dir"
  sha256sum pre-restore-live.tar.gz > SHA256SUMS
  sha256sum -c SHA256SUMS
)
chmod 0600 "$pre_dir/pre-restore-live.tar.gz" "$pre_dir/SHA256SUMS"

live_mutated=true
for relative in \
  data control pool config/generated stratum \
  prometheus grafana loki alloy alertmanager; do
  if [[ -d "$candidate/$relative" ]]; then
    install -d "$STATE_ROOT/$relative"
    rsync -aHAX --numeric-ids --delete \
      "$candidate/$relative/" "$STATE_ROOT/$relative/"
  fi
done
if [[ "$restore_secrets" == true ]]; then
  install -d -m 0700 "$STATE_ROOT/secrets"
  rsync -aHAX --numeric-ids --delete \
    "$secrets_stage/state/secrets/" "$STATE_ROOT/secrets/"
fi
install -d -m 0750 -o 10001 -g 10001 "$STATE_ROOT/backups/rocksdb-repository"
rsync -aHAX --numeric-ids --delete \
  "$rocks_stage/rocksdb-repository/" "$STATE_ROOT/backups/rocksdb-repository/"

if [[ -f "$stage/.env" ]]; then
  cp -a "$stage/.env" .env
fi
python3 - "$STATE_ROOT" <<'PY'
import json
import os
import re
import sys
from pathlib import Path

path = Path(".env")
content = path.read_text(encoding="utf-8")
updates = {
    "ALVENQIS_STATE_ROOT": sys.argv[1],
    "ALVENQIS_STORAGE_KEY_FILE": "/run/secrets/alvenqis_storage_key",
    "ALVENQIS_REQUIRE_STORAGE_ENCRYPTION": "true",
    "ALVENQIS_ALLOW_PLAINTEXT_STORAGE_MIGRATION": "false",
}
for key, value in updates.items():
    line = f"{key}={json.dumps(value)}"
    if re.search(rf"^{key}=", content, flags=re.MULTILINE):
        content = re.sub(rf"^{key}=.*$", line, content, flags=re.MULTILINE)
    else:
        content = content.rstrip() + "\n" + line + "\n"
temporary = path.with_name(f".env.restore-{os.getpid()}")
temporary.write_text(content, encoding="utf-8")
os.chmod(temporary, 0o600)
temporary.replace(path)
PY

load_dotenv .env
resolve_state_root "$workspace"
bash scripts/prepare-state.sh
compose_args

live_verify_output="$(
  "${ALVENQIS_COMPOSE_ARGS[@]}" run --rm --no-deps \
    --entrypoint /usr/local/bin/alvenqis-node \
    alvenqis-node \
      --config /config/node.toml \
      --data-dir /data/.alvenqis-mainnet/chain \
      verify-rocksdb
)"
live_verify_json="$(printf '%s\n' "$live_verify_output" | status_json)"
assert_rocks_status "$live_verify_json" "Installed RocksDB/SQLite verification"

"${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" \
  up -d --no-build
bash scripts/health-check-docker.sh

completed=true
stopped=false
live_mutated=false
cleanup
trap - EXIT
echo "Restore completed from: $backup_dir"
echo "Pre-restore snapshot: $pre_dir"
