#!/usr/bin/env bash
set -Eeuo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; repo="$(cd "$root/../.." && pwd)"; cd "$root"
command -v docker >/dev/null && docker compose version >/dev/null || { echo "Docker Engine + Compose v2 required" >&2; exit 69; }
export ALVENQIS_STATE_ROOT="${ALVENQIS_STATE_ROOT:-/var/lib/alvenqis-control-plane}"
bash "$root/scripts/prepare-state.sh"
source scripts/lib.sh
resolve_state_root "$root"
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
