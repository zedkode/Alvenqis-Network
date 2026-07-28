#!/usr/bin/env bash
set -Eeuo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
[[ $EUID -eq 0 ]] || { echo "prepare-state.sh must run as root" >&2; exit 77; }
source scripts/lib.sh
[[ ! -f .env ]] || load_dotenv .env
resolve_state_root "$root"

install -d -m 0700 "$STATE_ROOT/secrets"

create_owned() {
  local uid="$1" gid="$2"
  shift 2
  local path
  for path in "$@"; do
    install -d -m 0750 -o "$uid" -g "$gid" "$path"
    chown -R "$uid:$gid" "$path"
  done
}

create_owned 10001 10001 \
  "$STATE_ROOT/config/generated" \
  "$STATE_ROOT/data" \
  "$STATE_ROOT/data/chain" "$STATE_ROOT/data/mempool" \
  "$STATE_ROOT/data/indexer" "$STATE_ROOT/data/node" \
  "$STATE_ROOT/control" "$STATE_ROOT/pool" "$STATE_ROOT/loki"
create_owned 65534 65534 "$STATE_ROOT/prometheus" "$STATE_ROOT/alertmanager"
create_owned 472 472 "$STATE_ROOT/grafana"
create_owned 473 473 "$STATE_ROOT/alloy"
create_owned 0 0 \
  "$STATE_ROOT/backups" "$STATE_ROOT/ops" "$STATE_ROOT/repair-backups"

install -d -m 0755 -o 0 -g 0 "$STATE_ROOT/metrics"
chmod 0755 "$STATE_ROOT/metrics"
find "$STATE_ROOT/metrics" -maxdepth 1 -type f -exec chmod 0644 {} +

# Certbot owns the Stratum tree, while the pool runs as uid/gid 10001.
# Reapplying host state permissions must preserve that read-only group access.
install -d -m 0750 -o 0 -g 0 "$STATE_ROOT/stratum"
if [[ -d "$STATE_ROOT/stratum/letsencrypt" ]]; then
  find "$STATE_ROOT/stratum/letsencrypt" -type d -exec chmod 0755 {} +
  find "$STATE_ROOT/stratum/letsencrypt" -type f -exec chmod 0644 {} +
  find "$STATE_ROOT/stratum/letsencrypt" -type f -name 'privkey*.pem' \
    -exec chown 0:10001 {} + -exec chmod 0640 {} +
  chgrp -R 10001 "$STATE_ROOT/stratum/letsencrypt"
fi

# Compose file-backed secrets are bind mounts, so the source mode is retained.
# The parent directory remains root-only (0700); read-only file mode allows only
# the explicitly mounted secret to be read by non-root container users.
find "$STATE_ROOT/secrets" -maxdepth 1 -type f -exec chmod 0444 {} +

echo "Alvenqis state permissions prepared at $STATE_ROOT."
