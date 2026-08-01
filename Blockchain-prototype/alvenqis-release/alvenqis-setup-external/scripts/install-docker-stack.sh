#!/usr/bin/env bash
set -Eeuo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; repo="$(cd "$root/../.." && pwd)"; cd "$root"
operator_role="${ALVENQIS_OPERATOR_ROLE:-node}"
while (($#)); do
  case "$1" in
    --role) operator_role="${2:-}"; shift 2 ;;
    *) echo "Usage: $0 [--role node|validator|rpc|indexer|indexer-explorer|explorer|pool|stratum|full-stack]" >&2; exit 64 ;;
  esac
done
python3 - "$root/compose/roles.json" "$operator_role" <<'PY'
import json
import sys
from pathlib import Path

roles = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))["roles"]
if sys.argv[2] not in roles:
    raise SystemExit(f"unsupported operator role: {sys.argv[2]}")
PY
command -v docker >/dev/null && docker compose version >/dev/null || { echo "Docker Engine + Compose v2 required" >&2; exit 69; }
default_state_root="/var/lib/alvenqis/$operator_role"
[[ "$operator_role" == full-stack ]] && default_state_root="/var/lib/alvenqis-setup-external"
export ALVENQIS_STATE_ROOT="${ALVENQIS_STATE_ROOT:-$default_state_root}"
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
for n in setup_token broker_token admin_password admin_viewer_password grafana_password pool_admin_token backup_passphrase cloudflare_tunnel_token; do [[ -s "$STATE_ROOT/secrets/$n" && "$(cat "$STATE_ROOT/secrets/$n")" != validation-placeholder ]] || openssl rand -hex 32 > "$STATE_ROOT/secrets/$n"; chmod 0444 "$STATE_ROOT/secrets/$n"; done
control_proxy_token="$STATE_ROOT/secrets/control_proxy_token"
if [[ -s "$control_proxy_token" && "$(cat "$control_proxy_token")" != validation-placeholder ]]; then
  grep -Eq '^[0-9A-Fa-f]{64}$' "$control_proxy_token" || {
    echo "Existing control proxy token is invalid; refusing to replace it." >&2
    exit 78
  }
else
  openssl rand -hex 32 > "$control_proxy_token"
fi
chmod 0444 "$control_proxy_token"
for n in cloudflare_api_token r2_secret_access_key discord_webhook telegram_bot_token smtp_password; do [[ -e "$STATE_ROOT/secrets/$n" ]] || : > "$STATE_ROOT/secrets/$n"; chmod 0444 "$STATE_ROOT/secrets/$n"; done
cat > .installer.env <<EOF
ALVENQIS_HOST_WORKSPACE=$root
ALVENQIS_HOST_REPO=$repo
ALVENQIS_STATE_ROOT=$STATE_ROOT
OPS_BOOTSTRAP_PORT=${OPS_BOOTSTRAP_PORT:-8080}
ALVENQIS_VERSION=${ALVENQIS_VERSION:-2.1.0-no-autoupdate}
ALVENQIS_OPS_IMAGE=${ALVENQIS_OPS_IMAGE:-ghcr.io/zedkode/alvenqis-ops}
ALVENQIS_OPERATOR_ROLE=$operator_role
EOF
docker compose --project-directory "$root" --env-file .installer.env -f compose/installer.yaml config --quiet
docker compose --project-directory "$root" --env-file .installer.env -f compose/installer.yaml up -d --build --force-recreate
cat <<EOF
Create this SSH tunnel: ssh -N -L 18080:127.0.0.1:${OPS_BOOTSTRAP_PORT:-8080} root@SERVER_IP
Open: http://127.0.0.1:18080/?token=$(cat "$STATE_ROOT/secrets/setup_token")
EOF
