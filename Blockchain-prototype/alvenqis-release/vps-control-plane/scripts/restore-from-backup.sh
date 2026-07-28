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
        if member.issym() or member.islnk():
            raise SystemExit(f"links are not allowed in restore archives: {member.name}")
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
        if member.issym() or member.islnk():
            raise SystemExit(f"links are not allowed in RocksDB archive: {member.name}")
print("RocksDB backup archive path validation: ok")
PY

marker_value() {
  local key="$1"
  awk -F= -v key="$key" '$1 == key {sub(/^[^=]*=/, ""); print; exit}' "$completion_marker"
}
rocks_backup_id="$(marker_value rocksdb_backup_id)"
rocks_tip_height="$(marker_value rocksdb_tip_height)"
rocks_tip_hash="$(marker_value rocksdb_tip_hash)"
rocks_encryption="$(marker_value rocksdb_encryption)"
rocks_key_id="$(marker_value rocksdb_key_id)"
[[ "$rocks_backup_id" =~ ^[0-9]+$ && "$rocks_tip_height" =~ ^[0-9]+$ ]] || {
  echo "RocksDB backup marker has invalid numeric fields." >&2
  exit 74
}
[[ "$rocks_tip_hash" =~ ^[0-9a-f]{64}$ \
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
  [[ "$working_dir" == "$workspace" && "$config_files" == *"$workspace/compose.yaml"* ]] || {
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

cleanup() {
  if [[ "$stopped" == true ]]; then
    echo "Restore failed; attempting to restart the owned Alvenqis project." >&2
    "${ALVENQIS_COMPOSE_ARGS[@]}" \
      --profile cloudflare --profile pool --profile backup up -d --no-build || true
  fi
  if [[ -d "$stage" ]]; then
    case "$stage" in
      "$STATE_ROOT"/backups/.restore-*) rm -rf -- "$stage" ;;
      *) echo "Refusing to remove unexpected restore stage: $stage" >&2 ;;
    esac
  fi
}
trap cleanup EXIT

echo "Stopping only positively identified project $project..."
"${ALVENQIS_COMPOSE_ARGS[@]}" \
  --profile cloudflare --profile pool --profile backup stop
stopped=true

echo "Creating pre-restore safety snapshot..."
pre_stage="$stage/pre-live"
install -d -m 0700 "$pre_stage/state"
for relative in \
  data control pool config/generated stratum \
  prometheus grafana loki alloy alertmanager; do
  if [[ -e "$STATE_ROOT/$relative" ]]; then
    mkdir -p "$pre_stage/state/$(dirname "$relative")"
    cp -a --reflink=auto \
      "$STATE_ROOT/$relative" "$pre_stage/state/$(dirname "$relative")/"
  fi
done
cp -a .env "$pre_stage/.env"
tar -C "$pre_stage" -czf "$pre_dir/pre-restore-live.tar.gz" state .env
sha256sum "$pre_dir/pre-restore-live.tar.gz" > "$pre_dir/SHA256SUMS"
(
  cd "$pre_dir"
  sha256sum -c SHA256SUMS
)

echo "Extracting verified backup into an isolated stage..."
tar -xzf "$state_archive" -C "$stage"
[[ -d "$stage/state" ]] || {
  echo "Backup archive does not contain state/." >&2
  exit 74
}

for relative in \
  data control pool config/generated stratum \
  prometheus grafana loki alloy alertmanager; do
  if [[ -d "$stage/state/$relative" ]]; then
    install -d "$STATE_ROOT/$relative"
    rsync -aHAX --numeric-ids --delete \
      "$stage/state/$relative/" "$STATE_ROOT/$relative/"
  fi
done

if [[ "${RESTORE_SECRETS:-true}" == true && -f "$secrets_archive" ]]; then
  pass=/run/secrets/backup_passphrase
  [[ -s "$pass" ]] || pass="$STATE_ROOT/secrets/backup_passphrase"
  [[ -s "$pass" ]] || {
    echo "Backup passphrase is unavailable; refusing secrets restore." >&2
    exit 66
  }
  secrets_stage="$stage/decrypted"
  install -d -m 0700 "$secrets_stage"
  openssl enc -d -aes-256-cbc -salt -pbkdf2 -iter 200000 \
    -pass "file:$pass" -in "$secrets_archive" |
    tar -xzf - -C "$secrets_stage"
  [[ -d "$secrets_stage/state/secrets" ]] || {
    echo "Encrypted archive does not contain state/secrets." >&2
    exit 74
  }
  rsync -aHAX --numeric-ids --delete \
    "$secrets_stage/state/secrets/" "$STATE_ROOT/secrets/"
fi

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
line = f"ALVENQIS_STATE_ROOT={json.dumps(sys.argv[1])}"
if re.search(r"^ALVENQIS_STATE_ROOT=", content, flags=re.MULTILINE):
    content = re.sub(r"^ALVENQIS_STATE_ROOT=.*$", line, content, flags=re.MULTILINE)
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
"${ALVENQIS_COMPOSE_ARGS[@]}" \
  --profile cloudflare --profile pool --profile backup up -d --no-build
bash scripts/health-check-docker.sh

completed=true
stopped=false
cleanup
trap - EXIT
echo "Restore completed from: $backup_dir"
echo "Pre-restore snapshot: $pre_dir"
