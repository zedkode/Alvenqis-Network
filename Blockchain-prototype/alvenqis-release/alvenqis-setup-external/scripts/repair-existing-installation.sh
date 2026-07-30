#!/usr/bin/env bash
set -Eeuo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; repo="$(cd "$root/../.." && pwd)"; cd "$root"; [[ -f .env ]] || { echo "Missing .env" >&2; exit 66; }
source scripts/lib.sh
load_dotenv .env
export ALVENQIS_STATE_ROOT="${ALVENQIS_STATE_ROOT:-/var/lib/alvenqis-setup-external}"
resolve_state_root "$root"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"; mkdir -p "$STATE_ROOT/repair-backups/$stamp" "$STATE_ROOT/secrets"; cp -a .env "$STATE_ROOT/repair-backups/$stamp/"
# Preserve disabled rollback state from the former updater design.
if [[ -d "$STATE_ROOT/rollback" ]]; then
  mkdir -p "$STATE_ROOT/legacy-disabled"
  mv "$STATE_ROOT/rollback" "$STATE_ROOT/legacy-disabled/rollback-$stamp"
fi
storage_key="$STATE_ROOT/secrets/alvenqis_storage_key"
rocks_current="$STATE_ROOT/data/chain/state.rocksdb/CURRENT"
if [[ -f "$rocks_current" && ! -s "$storage_key" ]]; then
  echo "Encrypted RocksDB exists but its storage key is missing; refusing repair." >&2
  exit 78
fi
if [[ -s "$storage_key" ]]; then
  grep -Eq '^[0-9A-Fa-f]{64}$' "$storage_key" || {
    echo "Invalid Alvenqis storage key; refusing to replace it." >&2
    exit 78
  }
else
  openssl rand -hex 32 > "$storage_key"
fi
chmod 0444 "$storage_key"
for n in broker_token setup_token admin_password grafana_password pool_admin_token backup_passphrase cloudflare_tunnel_token; do [[ -s "$STATE_ROOT/secrets/$n" && "$(cat "$STATE_ROOT/secrets/$n")" != validation-placeholder ]] || openssl rand -hex 32 > "$STATE_ROOT/secrets/$n"; chmod 0444 "$STATE_ROOT/secrets/$n"; done
for n in cloudflare_api_token r2_secret_access_key discord_webhook telegram_bot_token smtp_password; do [[ -e "$STATE_ROOT/secrets/$n" ]] || : > "$STATE_ROOT/secrets/$n"; chmod 0444 "$STATE_ROOT/secrets/$n"; done
python3 - "$root" "$repo" "$STATE_ROOT" <<'P'
from pathlib import Path
import re,sys
p=Path('.env'); s=p.read_text(); root,repo,state_root=sys.argv[1:]
def set_value(key, value):
 global s
 line=f"{key}={__import__('json').dumps(value)}"
 s=re.sub(rf'^{key}=.*$',line,s,flags=re.M) if re.search(rf'^{key}=',s,re.M) else s+'\n'+line+'\n'
for k,v in [('STACK_VERSION','2.1.0-no-autoupdate'),('ALVENQIS_HOST_WORKSPACE',root),('ALVENQIS_HOST_REPO',repo),('ALVENQIS_STATE_ROOT',state_root),('ALVENQIS_BACKUP_IMAGE','ghcr.io/zedkode/alvenqis-backup-scheduler')]:
 set_value(k,v)
legacy_defaults = {
 'NODE_MEMORY_LIMIT': ('3G', '2304M'),
 'RPC_MEMORY_LIMIT': ('3G', '1536M'),
 'CONTROL_MEMORY_LIMIT': ('1G', '384M'),
 'INDEXER_MEMORY_LIMIT': ('1G', '768M'),
 'INDEXER_INTERVAL_SECONDS': ('15', '5'),
 'PROMETHEUS_RETENTION': ('30d', '15d'),
 'LOKI_RETENTION_HOURS': ('720', '168'),
}
for key,(old,new) in legacy_defaults.items():
 match=re.search(rf'^{key}=(.*)$',s,flags=re.M)
 if match is None or match.group(1).strip().strip('"').strip("'") == old:
  set_value(key,new)
for key,value in [('INDEXER_FAILURE_BACKOFF_MAX_SECONDS','60'),('PROMETHEUS_RETENTION_SIZE','8GB'),('P2P_MIN_VALIDATED_PEERS','0'),('MAX_P2P_PEERS','64'),('CONTAINER_LOG_MAX_SIZE','20m'),('CONTAINER_LOG_MAX_FILES','3')]:
 if not re.search(rf'^{key}=',s,flags=re.M):
  set_value(key,value)
for key,value in [('ALVENQIS_STORAGE_KEY_FILE','/run/secrets/alvenqis_storage_key'),('ALVENQIS_REQUIRE_STORAGE_ENCRYPTION','true'),('ALVENQIS_ALLOW_PLAINTEXT_STORAGE_MIGRATION','false')]:
 set_value(key,value)
for k in ['POSTGRES_DB','POSTGRES_USER','POSTGRES_MEMORY_LIMIT']: s=re.sub(rf'^{k}=.*\n?','',s,flags=re.M)
p.write_text(s)
P
bash "$root/scripts/prepare-state.sh"
# Stop and preserve old host services. Unit files and data are not deleted.
legacy_units=(alvenqis-indexer-refresh.timer alvenqis-auto-update.timer alvenqis-mining-pool alvenqis-vps-admin alvenqis-rpc alvenqis-node alvenqis-indexer-refresh.timer alvenqis-auto-update.timer alvenqis-mining-pool alvenqis-vps-admin alvenqis-rpc alvenqis-node)
for unit in "${legacy_units[@]}"; do
  systemctl disable --now "$unit" 2>/dev/null || true
done

# Preserve conflicting containers under a timestamped legacy name instead of
# deleting them. Legacy-named containers are stopped in place.
for old in alvenqis-node alvenqis-rpc alvenqis-indexer alvenqis-control alvenqis-pool alvenqis-ops alvenqis-caddy alvenqis-cloudflared alvenqis-docker-broker alvenqis-updater alvenqis-cadvisor alvenqis-watchtower; do
  docker update --restart=no "$old" >/dev/null 2>&1 || true
  docker stop "$old" >/dev/null 2>&1 || true
done
for current in alvenqis-node alvenqis-rpc alvenqis-indexer alvenqis-control alvenqis-pool alvenqis-ops alvenqis-caddy alvenqis-cloudflared alvenqis-docker-broker; do
  if docker container inspect "$current" >/dev/null 2>&1; then
    docker update --restart=no "$current" >/dev/null
    docker stop "$current" >/dev/null 2>&1 || true
    docker rename "$current" "legacy-${stamp}-${current}"
  fi
done

source scripts/lib.sh
load_dotenv .env
compose_args
if [[ "${CLOUDFLARE_MODE:-disabled}" == tunnel ]]; then
  bash scripts/cloudflare-bootstrap.sh --prepare
fi
COMPOSE_PARALLEL_LIMIT=1 "${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" up -d --build
bash scripts/health-check-docker.sh
if [[ "${CLOUDFLARE_MODE:-disabled}" != disabled ]]; then
  bash scripts/cloudflare-bootstrap.sh --activate
  bash scripts/verify-public-health.sh
fi
echo "Repair complete; backup: $STATE_ROOT/repair-backups/$stamp"
