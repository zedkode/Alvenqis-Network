#!/usr/bin/env bash
set -Eeuo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
source scripts/lib.sh

[[ $EUID -eq 0 ]] || {
  echo "Run migrate-release-state.sh as root." >&2
  exit 77
}
[[ "${MIGRATE_CONFIRM:-}" == "yes" ]] || {
  echo "Refusing migration without MIGRATE_CONFIRM=yes." >&2
  exit 64
}
[[ -f .env ]] || {
  echo "Missing $root/.env" >&2
  exit 66
}

load_dotenv .env
project="${COMPOSE_PROJECT_NAME:-alvenqis-control-plane}"
source_root="${MIGRATION_SOURCE_STATE_ROOT:-$root/state}"
target_root="${MIGRATION_TARGET_STATE_ROOT:-${ALVENQIS_STATE_ROOT:-/var/lib/alvenqis-control-plane}}"

canonical_path() {
  python3 - "$1" <<'PY'
import sys
from pathlib import Path
print(Path(sys.argv[1]).resolve(strict=False))
PY
}

source_root="$(canonical_path "$source_root")"
target_root="$(canonical_path "$target_root")"

case "$target_root" in
  /|/bin|/boot|/dev|/etc|/home|/lib|/lib64|/opt|/proc|/root|/run|/sbin|/srv|/sys|/tmp|/usr|/var)
    echo "Refusing unsafe migration target: $target_root" >&2
    exit 64
    ;;
esac
[[ "$source_root" != "$target_root" ]] || {
  echo "Source and target state roots are identical: $source_root" >&2
  exit 0
}
[[ -d "$source_root" ]] || {
  echo "Source state root does not exist: $source_root" >&2
  exit 66
}
if [[ -f "$source_root/data/chain/state.rocksdb/CURRENT" ]]; then
  source_storage_key="$source_root/secrets/alvenqis_storage_key"
  [[ -s "$source_storage_key" ]] || {
    echo "Source RocksDB exists but its storage key is missing." >&2
    exit 78
  }
  grep -Eq '^[0-9A-Fa-f]{64}$' "$source_storage_key" || {
    echo "Source RocksDB storage key is invalid." >&2
    exit 78
  }
fi
if [[ -e "$target_root" && -n "$(find "$target_root" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
  echo "Migration target is not empty: $target_root" >&2
  exit 73
fi

for command in docker python3 rsync sha256sum tar; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "Required command is missing: $command" >&2
    exit 69
  }
done
docker compose version >/dev/null

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
  [[ "$working_dir" == "$root" ]] && compose_config_files_match "$config_files" || {
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

mapfile -t foreign_mounts < <(
  for container_id in $(docker ps -aq); do
    container_project="$(docker inspect -f '{{index .Config.Labels "com.docker.compose.project"}}' "$container_id")"
    [[ "$container_project" == "$project" ]] && continue
    docker inspect -f '{{range .Mounts}}{{$.Name}}|{{.Source}}{{"\n"}}{{end}}' "$container_id" |
      awk -F '|' -v source="$source_root" 'index($2, source) == 1 {print $1 " " $2}'
  done
)
((${#foreign_mounts[@]} == 0)) || {
  printf 'A non-project container mounts the source state; refusing migration:\n%s\n' \
    "${foreign_mounts[*]}" >&2
  exit 73
}

source_bytes="$(du -sx --block-size=1 "$source_root" | awk '{print $1}')"
target_parent="$(dirname "$target_root")"
install -d -m 0750 "$target_parent"
available_bytes="$(df -PB1 "$target_parent" | awk 'NR == 2 {print $4}')"
required_bytes=$((source_bytes * 3 + 268435456))
((available_bytes >= required_bytes)) || {
  echo "Insufficient disk for verified migration: need $required_bytes bytes, have $available_bytes." >&2
  exit 78
}

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
backup_root="/var/backups/alvenqis-control-plane/pre-state-migration-$stamp"
install -d -m 0700 "$backup_root"
cp -a .env "$backup_root/control-plane.env.before"

echo "Creating verified online application backup before migration..."
ALVENQIS_STATE_ROOT="$source_root" bash scripts/backup-now.sh
latest_online="$(
  find "$source_root/backups" -mindepth 1 -maxdepth 1 -type d \
    -name '20??????T??????Z' -printf '%T@ %p\n' |
    sort -nr | awk 'NR == 1 {print $2}'
)"
[[ -n "$latest_online" && -f "$latest_online/BACKUP_COMPLETE" ]] || {
  echo "Online backup did not produce a completion marker." >&2
  exit 74
}
(
  cd "$latest_online"
  sha256sum -c SHA256SUMS
)

compose_args
stopped=false
completed=false

restart_source_stack() {
  if [[ "$stopped" == true && "$completed" != true ]]; then
    echo "Migration failed; restarting the source-backed Alvenqis project." >&2
    cp -a "$backup_root/control-plane.env.before" .env
    export ALVENQIS_STATE_ROOT="$source_root"
    export STATE_ROOT="$source_root"
    load_dotenv .env
    compose_args
    "${ALVENQIS_COMPOSE_ARGS[@]}" \
      --profile cloudflare --profile pool --profile backup \
      up -d --no-build --force-recreate || true
  fi
}
trap restart_source_stack EXIT

echo "Stopping only positively identified project $project for an offline snapshot..."
"${ALVENQIS_COMPOSE_ARGS[@]}" \
  --profile cloudflare --profile pool --profile backup stop
stopped=true

echo "Creating offline full-state archive..."
tar --acls --xattrs --numeric-owner -C "$source_root" -czf "$backup_root/full-state.tar.gz" .
sha256sum "$backup_root/full-state.tar.gz" > "$backup_root/SHA256SUMS"
(
  cd "$backup_root"
  sha256sum -c SHA256SUMS
)

install -d -m 0750 "$target_root"
rsync -aHAX --numeric-ids --delete "$source_root/" "$target_root/"
differences="$(
  rsync -aHAXn --numeric-ids --delete --itemize-changes \
    "$source_root/" "$target_root/"
)"
[[ -z "$differences" ]] || {
  printf 'State verification failed after rsync:\n%s\n' "$differences" >&2
  exit 74
}

python3 - "$target_root" <<'PY'
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
        content = re.sub(
            rf"^{key}=.*$",
            line,
            content,
            flags=re.MULTILINE,
        )
    else:
        content = content.rstrip() + "\n" + line + "\n"
temporary = path.with_name(f".env.migration-{os.getpid()}")
temporary.write_text(content, encoding="utf-8")
os.chmod(temporary, 0o600)
temporary.replace(path)
PY

load_dotenv .env
resolve_state_root "$root"
[[ "$STATE_ROOT" == "$target_root" ]] || {
  echo "Updated dotenv did not resolve to the migration target." >&2
  exit 74
}
bash scripts/prepare-state.sh
if [[ -f "$target_root/data/chain/state.rocksdb/CURRENT" ]]; then
  cmp -s "$source_root/secrets/alvenqis_storage_key" \
    "$target_root/secrets/alvenqis_storage_key" || {
    echo "Migrated RocksDB storage key does not match the source." >&2
    exit 74
  }
fi
compose_args

echo "Starting Alvenqis with release-independent state..."
"${ALVENQIS_COMPOSE_ARGS[@]}" \
  --profile cloudflare --profile pool --profile backup \
  up -d --no-build --force-recreate

deadline=$((SECONDS + 420))
while ((SECONDS < deadline)); do
  unhealthy=0
  for container_id in $(docker ps -aq --filter "label=com.docker.compose.project=$project"); do
    status="$(docker inspect -f '{{.State.Status}}' "$container_id")"
    health="$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' "$container_id")"
    [[ "$status" == running && "$health" == healthy ]] || {
      unhealthy=1
      break
    }
  done
  ((unhealthy == 0)) && break
  sleep 5
done
for container_id in $(docker ps -aq --filter "label=com.docker.compose.project=$project"); do
  name="$(docker inspect -f '{{.Name}}' "$container_id")"
  status="$(docker inspect -f '{{.State.Status}}' "$container_id")"
  health="$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' "$container_id")"
  [[ "$status" == running && "$health" == healthy ]] || {
    echo "Migrated container is not healthy: ${name#/} status=$status health=$health" >&2
    exit 74
  }
done

mapfile -t source_mounts < <(
  for container_id in $(docker ps -aq --filter "label=com.docker.compose.project=$project"); do
    docker inspect -f '{{range .Mounts}}{{$.Name}}|{{.Source}}{{"\n"}}{{end}}' "$container_id" |
      awk -F '|' -v source="$source_root" 'index($2, source) == 1 {print $1 " " $2}'
  done
)
((${#source_mounts[@]} == 0)) || {
  printf 'A migrated container still mounts release state:\n%s\n' \
    "${source_mounts[*]}" >&2
  exit 74
}

ALVENQIS_STATE_ROOT="$target_root" bash scripts/backup-now.sh
latest_stable="$(
  find "$target_root/backups" -mindepth 1 -maxdepth 1 -type d \
    -name '20??????T??????Z' -printf '%T@ %p\n' |
    sort -nr | awk 'NR == 1 {print $2}'
)"
[[ -n "$latest_stable" && -f "$latest_stable/BACKUP_COMPLETE" ]] || {
  echo "Stable-root backup validation failed." >&2
  exit 74
}
(
  cd "$latest_stable"
  sha256sum -c SHA256SUMS
)

runtime_health=healthy
set +e
bash scripts/health-check-docker.sh \
  > >(tee "$backup_root/post-migration-health.log") \
  2> >(tee -a "$backup_root/post-migration-health.log" >&2)
health_exit=$?
set -e
if ((health_exit != 0)); then
  runtime_health=degraded
  if [[ "${MIGRATION_ALLOW_RUNTIME_DEGRADED:-}" != "yes" ]]; then
    echo "Full runtime health failed; set MIGRATION_ALLOW_RUNTIME_DEGRADED=yes only for a documented pre-existing degradation." >&2
    exit "$health_exit"
  fi
  echo "WARNING: storage migration passed, but full runtime health remains degraded." >&2
fi

cat > "$backup_root/MIGRATION_COMPLETE" <<EOF
completed_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)
compose_project=$project
source_root=$source_root
target_root=$target_root
online_backup=$latest_online
stable_backup=$latest_stable
runtime_health=$runtime_health
EOF
cat > "$root/STATE_MIGRATED_TO" <<EOF
$target_root
Migration evidence: $backup_root/MIGRATION_COMPLETE
EOF

completed=true
stopped=false
trap - EXIT
echo "Verified state migration complete: $source_root -> $target_root"
echo "The source copy remains read-only rollback material until a separate cleanup approval."
