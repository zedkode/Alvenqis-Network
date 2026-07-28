#!/usr/bin/env bash
set -Eeuo pipefail
node=""; host=""; email=""; controller=""; token=""; token_stdin=false; bundle=""; seeds=()
while (($#)); do case "$1" in --node-name) node="$2";shift 2;; --p2p-host|--domain) host="$2";shift 2;; --email) email="$2";shift 2;; --controller-url) controller="$2";shift 2;; --enrollment-token-stdin) token_stdin=true;shift;; --seed) seeds+=("$2");shift 2;; --release-bundle-url) bundle="$2";shift 2;; *) exit 64;; esac; done
if [[ "$token_stdin" == true ]]; then IFS= read -r token; fi
[[ -n $node && -n $host && -n $email && $controller == https://* && -n $token ]] || exit 64
[[ "$node" =~ ^[A-Za-z0-9._-]{1,64}$ ]] || { echo "Invalid node name." >&2; exit 64; }
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; repo="$(cd "$root/../.." && pwd)"; cd "$root"; base="${host#*.}"
export ALVENQIS_STATE_ROOT="${ALVENQIS_STATE_ROOT:-/var/lib/alvenqis-agents/$node}"
source scripts/lib.sh
resolve_state_root "$root"
bash "$root/scripts/prepare-state.sh"
umask 077
printf '%s' "$token" > "$STATE_ROOT/control/enrollment.token"
chmod 0600 "$STATE_ROOT/control/enrollment.token"
unset token
for n in admin_password grafana_password setup_token broker_token cloudflare_tunnel_token pool_admin_token backup_passphrase alvenqis_storage_key; do openssl rand -hex 32 > "$STATE_ROOT/secrets/$n"; chmod 0444 "$STATE_ROOT/secrets/$n"; done
for n in cloudflare_api_token r2_secret_access_key discord_webhook telegram_bot_token smtp_password; do : > "$STATE_ROOT/secrets/$n"; chmod 0444 "$STATE_ROOT/secrets/$n"; done
python3 - "$root" "$repo" "$STATE_ROOT" "$base" "$node" "$email" "$controller" "$bundle" "$host" "${seeds[@]}" <<'PY'
import json
import sys
from pathlib import Path

root, repo, state_root, base, node, email, controller, bundle, host, *seeds = sys.argv[1:]
values = {
    "COMPOSE_PROJECT_NAME": f"alvenqis-agent-{node}",
    "STACK_VERSION": "2.1.0-no-autoupdate",
    "ALVENQIS_HOST_WORKSPACE": root,
    "ALVENQIS_HOST_REPO": repo,
    "ALVENQIS_STATE_ROOT": state_root,
    "ALVENQIS_STORAGE_KEY_FILE": "/run/secrets/alvenqis_storage_key",
    "ALVENQIS_REQUIRE_STORAGE_ENCRYPTION": "true",
    "ALVENQIS_ALLOW_PLAINTEXT_STORAGE_MIGRATION": "false",
    "ALVENQIS_VERSION": "2.1.0-no-autoupdate",
    "BASE_DOMAIN": base,
    "NODE_NAME": node,
    "ADMIN_EMAIL": email,
    "CONTROL_ROLE": "agent",
    "CONTROLLER_URL": controller,
    "RELEASE_BUNDLE_URL": bundle,
    "P2P_HOST": host,
    "P2P_PORT": "20787",
    "SEED_NODES_TOML": ", ".join(json.dumps(seed) for seed in seeds),
    "CLOUDFLARE_MODE": "disabled",
    "ENABLE_POOL": "false",
    "INDEXER_INTERVAL_SECONDS": "5",
    "INDEXER_FAILURE_BACKOFF_MAX_SECONDS": "60",
    "P2P_MIN_VALIDATED_PEERS": "1",
}
Path(".env").write_text(
    "\n".join(f"{key}={json.dumps(value)}" for key, value in values.items()) + "\n",
    encoding="utf-8",
)
PY
bash "$root/scripts/prepare-state.sh"
COMPOSE_PARALLEL_LIMIT=1 docker compose --env-file .env -f compose.yaml up -d --build alvenqis-node alvenqis-rpc alvenqis-indexer alvenqis-control
