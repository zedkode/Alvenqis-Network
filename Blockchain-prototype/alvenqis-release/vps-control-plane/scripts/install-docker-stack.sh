#!/usr/bin/env bash
set -Eeuo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; repo="$(cd "$root/../.." && pwd)"; cd "$root"
command -v docker >/dev/null && docker compose version >/dev/null || { echo "Docker Engine + Compose v2 required" >&2; exit 69; }
export ALVENQIS_STATE_ROOT="${ALVENQIS_STATE_ROOT:-/var/lib/alvenqis-control-plane}"
bash "$root/scripts/prepare-state.sh"
source scripts/lib.sh
resolve_state_root "$root"
storage_key="$STATE_ROOT/secrets/alvenqis_storage_key"
rocks_current="$STATE_ROOT/data/chain/state.rocksdb/CURRENT"
if [[ -f "$rocks_current" && ! -s "$storage_key" ]]; then
  echo "Existing RocksDB state has no storage key; refusing to generate a replacement." >&2
  exit 78
fi
if [[ -s "$storage_key" ]]; then
  grep -Eq '^[0-9A-Fa-f]{64}$' "$storage_key" || {
    echo "Existing Alvenqis storage key is invalid." >&2
    exit 78
  }
else
  openssl rand -hex 32 > "$storage_key"
fi
chmod 0444 "$storage_key"
for n in setup_token broker_token admin_password grafana_password pool_admin_token backup_passphrase cloudflare_tunnel_token; do [[ -s "$STATE_ROOT/secrets/$n" && "$(cat "$STATE_ROOT/secrets/$n")" != validation-placeholder ]] || openssl rand -hex 32 > "$STATE_ROOT/secrets/$n"; chmod 0444 "$STATE_ROOT/secrets/$n"; done
for n in cloudflare_api_token r2_secret_access_key discord_webhook telegram_bot_token smtp_password; do [[ -e "$STATE_ROOT/secrets/$n" ]] || : > "$STATE_ROOT/secrets/$n"; chmod 0444 "$STATE_ROOT/secrets/$n"; done
cat > .installer.env <<EOF
ALVENQIS_HOST_WORKSPACE=$root
ALVENQIS_HOST_REPO=$repo
ALVENQIS_STATE_ROOT=$STATE_ROOT
OPS_BOOTSTRAP_PORT=${OPS_BOOTSTRAP_PORT:-8080}
ALVENQIS_VERSION=${ALVENQIS_VERSION:-2.1.0-no-autoupdate}
ALVENQIS_OPS_IMAGE=${ALVENQIS_OPS_IMAGE:-ghcr.io/zedkode/alvenqis-ops}
EOF
docker compose --env-file .installer.env -f installer.compose.yaml up -d --build --force-recreate
cat <<EOF
Create this SSH tunnel: ssh -N -L 18080:127.0.0.1:${OPS_BOOTSTRAP_PORT:-8080} root@SERVER_IP
Open: http://127.0.0.1:18080/?token=$(cat "$STATE_ROOT/secrets/setup_token")
EOF
