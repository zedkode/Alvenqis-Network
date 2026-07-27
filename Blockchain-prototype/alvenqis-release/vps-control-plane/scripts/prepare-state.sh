#!/usr/bin/env bash
set -Eeuo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
[[ $EUID -eq 0 ]] || { echo "prepare-state.sh must run as root" >&2; exit 77; }

install -d -m 0700 state/secrets

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
  state/config/generated \
  state/data \
  state/data/chain state/data/mempool state/data/indexer state/data/node \
  state/control state/pool state/loki
create_owned 65534 65534 state/prometheus state/alertmanager
create_owned 472 472 state/grafana
create_owned 1000 1000 state/caddy/data state/caddy/config
create_owned 473 473 state/alloy
create_owned 0 0 state/backups state/metrics state/ops state/repair-backups

# Compose file-backed secrets are bind mounts, so the source mode is retained.
# The parent directory remains root-only (0700); read-only file mode allows only
# the explicitly mounted secret to be read by non-root container users.
find state/secrets -maxdepth 1 -type f -exec chmod 0444 {} +

echo "Alvenqis state permissions prepared."
