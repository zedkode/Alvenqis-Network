#!/usr/bin/env bash
# Restore a backup produced by scripts/backup-now.sh.
# Destructive to live state — requires explicit RESTORE_CONFIRM=yes.
# Restore never removes Docker volumes; chain-volume wipes are forbidden here.
set -Eeuo pipefail

workspace="${ALVENQIS_WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$workspace"
source scripts/lib.sh

usage() {
  cat <<'EOF'
Usage:
  RESTORE_CONFIRM=yes ./scripts/restore-from-backup.sh state/backups/<UTC-stamp>

Environment:
  RESTORE_CONFIRM=yes          Required. Refuse without this exact value.
  RESTORE_SECRETS=true|false   Default true. Decrypt alvenqis-secrets.tar.gz.enc when present.
  CHAIN_SNAPSHOT_STOP_SERVICES Default true (same as backup-now.sh).
  ALVENQIS_WORKSPACE           Optional package root override.

Restores from the directory created by backup-now.sh:
  alvenqis-state.tar.gz
  alvenqis-secrets.tar.gz.enc  (optional decrypt)
  SHA256SUMS                   (verified when present)
EOF
}

[[ "${1:-}" != "-h" && "${1:-}" != "--help" ]] || { usage; exit 0; }
[[ $# -ge 1 ]] || { usage >&2; exit 64; }

backup_arg="$1"
if [[ "$backup_arg" = /* ]]; then
  backup_dir="$backup_arg"
else
  backup_dir="$workspace/$backup_arg"
fi
backup_dir="$(cd "$(dirname "$backup_dir")" && pwd)/$(basename "$backup_dir")"

[[ "${RESTORE_CONFIRM:-}" == "yes" ]] || {
  echo "Refusing restore without RESTORE_CONFIRM=yes" >&2
  echo "Example: RESTORE_CONFIRM=yes $0 state/backups/<UTC-stamp>" >&2
  exit 64
}

[[ -d "$backup_dir" ]] || {
  echo "Backup directory not found: $backup_dir" >&2
  exit 66
}

state_archive="$backup_dir/alvenqis-state.tar.gz"
secrets_archive="$backup_dir/alvenqis-secrets.tar.gz.enc"
sums_file="$backup_dir/SHA256SUMS"

[[ -f "$state_archive" ]] || {
  echo "Missing state archive: $state_archive" >&2
  exit 66
}

if [[ -f "$sums_file" ]]; then
  echo "Verifying SHA256SUMS in $backup_dir"
  (
    cd "$backup_dir"
    sha256sum -c SHA256SUMS
  )
else
  echo "WARN: no SHA256SUMS in backup dir; continuing without checksum verification"
fi

# .env is required by compose_args / load_dotenv for stop+start. Prefer live .env;
# if missing, extract only .env from the archive first.
if [[ ! -f .env ]]; then
  echo "No live .env — extracting .env from state archive first"
  tar -xzf "$state_archive" .env
fi
load_dotenv .env
compose_args

services=(alvenqis-indexer alvenqis-control alvenqis-rpc alvenqis-node)
if [[ "${ENABLE_POOL:-false}" == true ]]; then
  services=(alvenqis-pool "${services[@]}")
fi

stopped=false
cleanup() {
  if [[ "$stopped" == true ]]; then
    echo "Restarting services after restore..."
    "${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" up -d "${services[@]}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [[ "${CHAIN_SNAPSHOT_STOP_SERVICES:-true}" == true ]]; then
  echo "Stopping chain services for consistent restore: ${services[*]}"
  "${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" stop "${services[@]}" || true
  stopped=true
fi

# Preserve a pre-restore safety copy of live paths that will be overwritten.
pre_stamp="$(date -u +%Y%m%dT%H%M%SZ)"
pre_dir="state/backups/pre-restore-$pre_stamp"
mkdir -p "$pre_dir"
pre_items=(state/data state/control state/pool state/config/generated .env)
pre_existing=()
for i in "${pre_items[@]}"; do
  [[ -e $i ]] && pre_existing+=("$i")
done
if ((${#pre_existing[@]} > 0)); then
  tar -czf "$pre_dir/pre-restore-live.tar.gz" "${pre_existing[@]}"
  echo "Pre-restore live snapshot: $pre_dir/pre-restore-live.tar.gz"
fi

echo "Extracting state archive into $workspace"
tar -xzf "$state_archive" -C "$workspace"

if [[ "${RESTORE_SECRETS:-true}" == true && -f "$secrets_archive" ]]; then
  pass=/run/secrets/backup_passphrase
  [[ -s $pass ]] || pass=state/secrets/backup_passphrase
  if [[ ! -s $pass ]]; then
    echo "WARN: passphrase not found; skipping secrets decrypt ($secrets_archive)" >&2
  else
    echo "Decrypting secrets archive"
    openssl enc -d -aes-256-cbc -salt -pbkdf2 -iter 200000 -pass "file:$pass" \
      -in "$secrets_archive" | tar -xzf - -C "$workspace"
  fi
elif [[ -f "$secrets_archive" ]]; then
  echo "Skipping secrets restore (RESTORE_SECRETS=${RESTORE_SECRETS:-true})"
fi

# Reload dotenv after extract in case .env was restored
if [[ -f .env ]]; then
  load_dotenv .env
  compose_args
fi

echo "Starting services"
"${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" up -d "${services[@]}"
stopped=false
trap - EXIT

echo "Restore completed from: $backup_dir"
echo "Next: ./scripts/health-check-docker.sh"
echo "Then (laptop or host): ./scripts/smoke-public-candidate.sh"
