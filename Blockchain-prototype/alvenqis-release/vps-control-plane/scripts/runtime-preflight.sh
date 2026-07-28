#!/usr/bin/env bash
set -Eeuo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
source scripts/lib.sh
load_dotenv .env
resolve_state_root "$root"

assert_owner_mode() {
  local path="$1" expected_owner="$2" expected_mode="$3" label="$4"
  [[ -e "$path" && ! -L "$path" ]] || {
    echo "$label is missing or is a symlink: $path" >&2
    exit 78
  }
  local actual_owner actual_mode
  actual_owner="$(stat -c '%u:%g' "$path")"
  actual_mode="$(stat -c '%a' "$path")"
  [[ "$actual_owner" == "$expected_owner" && "$actual_mode" == "$expected_mode" ]] || {
    echo "$label must be owner=$expected_owner mode=$expected_mode; got owner=$actual_owner mode=$actual_mode" >&2
    exit 78
  }
}

assert_owner_mode "$STATE_ROOT" "0:0" "750" "Alvenqis state root"
assert_owner_mode "$STATE_ROOT/secrets" "0:0" "700" "Alvenqis secrets directory"
assert_owner_mode "$STATE_ROOT/data" "10001:10001" "750" "Alvenqis runtime data directory"
assert_owner_mode "$STATE_ROOT/data/chain" "10001:10001" "750" "Alvenqis chain directory"
storage_key="$STATE_ROOT/secrets/alvenqis_storage_key"
assert_owner_mode "$storage_key" "0:0" "444" "Alvenqis RocksDB storage key"
grep -Eq '^[0-9A-Fa-f]{64}$' "$storage_key" || {
  echo "Alvenqis RocksDB storage key must contain exactly 64 hexadecimal characters." >&2
  exit 78
}
[[ "${ALVENQIS_STORAGE_KEY_FILE:-}" == /run/secrets/alvenqis_storage_key ]] || {
  echo "ALVENQIS_STORAGE_KEY_FILE must be /run/secrets/alvenqis_storage_key." >&2
  exit 64
}
[[ "${ALVENQIS_REQUIRE_STORAGE_ENCRYPTION:-}" == true ]] || {
  echo "ALVENQIS_REQUIRE_STORAGE_ENCRYPTION=true is mandatory." >&2
  exit 64
}
[[ "${ALVENQIS_ALLOW_PLAINTEXT_STORAGE_MIGRATION:-}" == false ]] || {
  echo "Plaintext storage migration must remain disabled during normal runtime." >&2
  exit 64
}
if [[ -d "$STATE_ROOT/data/chain/state.rocksdb" ]]; then
  [[ -s "$STATE_ROOT/data/chain/state.rocksdb/CURRENT" ]] || {
    echo "Existing RocksDB state is incomplete; CURRENT is missing." >&2
    exit 78
  }
fi

command -v docker >/dev/null 2>&1 || {
  echo "Docker Engine is required." >&2
  exit 69
}
docker compose version >/dev/null 2>&1 || {
  echo "Docker Compose v2 is required." >&2
  exit 69
}

read -r host_cpus host_memory < <(docker info --format '{{.NCPU}} {{.MemTotal}}')
free_bytes="$(df -PB1 "$(dirname "$STATE_ROOT")" | awk 'NR==2 {print $4}')"
minimum_cpus="${VPS_MIN_CPU_COUNT:-6}"
minimum_memory="${VPS_MIN_MEMORY_BYTES:-11811160064}"
minimum_free_disk="${VPS_MIN_FREE_DISK_BYTES:-64424509440}"

((host_cpus >= minimum_cpus)) || {
  echo "Host has ${host_cpus} CPUs; at least ${minimum_cpus} are required for the full stack." >&2
  exit 78
}
((host_memory >= minimum_memory)) || {
  echo "Host memory ${host_memory} is below the required ${minimum_memory} bytes." >&2
  exit 78
}
((free_bytes >= minimum_free_disk)) || {
  echo "Free disk ${free_bytes} is below the required ${minimum_free_disk} bytes." >&2
  exit 78
}

python3 - "${SEED_NODES_TOML:-}" "${CONTROL_ROLE:-controller}" "${P2P_MIN_VALIDATED_PEERS:-0}" <<'PY'
import json
import re
import sys

raw, role, minimum = sys.argv[1:]
try:
    minimum_peers = int(minimum)
except ValueError as exc:
    raise SystemExit("P2P_MIN_VALIDATED_PEERS must be a non-negative integer") from exc
if minimum_peers < 0:
    raise SystemExit("P2P_MIN_VALIDATED_PEERS must be a non-negative integer")
try:
    seeds = json.loads(f"[{raw}]") if raw.strip() else []
except json.JSONDecodeError as exc:
    raise SystemExit(f"SEED_NODES_TOML is not a valid quoted seed list: {exc}") from exc
if not all(isinstance(seed, str) for seed in seeds):
    raise SystemExit("Every P2P seed must be a quoted multiaddress string")
pattern = re.compile(r"^/(?:dns4|dns6|ip4|ip6)/[^/]+/tcp/[1-9][0-9]{0,4}$")
for seed in seeds:
    if not pattern.fullmatch(seed):
        raise SystemExit(f"Invalid TCP seed multiaddress: {seed}")
if len(seeds) != len(set(seeds)):
    raise SystemExit("Duplicate P2P seeds are not allowed")
if (role != "controller" or minimum_peers > 0) and not seeds:
    raise SystemExit("Agent/readiness-enforced nodes require at least one P2P seed")
print(f"P2P seed preflight: role={role} seeds={len(seeds)} minimum_validated={minimum_peers}")
PY

for port_name in P2P_PORT STRATUM_PORT STRATUM_INTERNAL_PORT HTTP_PORT; do
  value="${!port_name:-}"
  [[ "$value" =~ ^[0-9]+$ ]] && ((value >= 1 && value <= 65535)) || {
    echo "$port_name must be between 1 and 65535." >&2
    exit 64
  }
done
if [[ "${ENABLE_POOL:-false}" == true ]]; then
  [[ -n "${POOL_ADDRESS:-}" && -n "${STRATUM_HOST:-}" ]] || {
    echo "POOL_ADDRESS and STRATUM_HOST are required when ENABLE_POOL=true." >&2
    exit 64
  }
  [[ "${P2P_PORT}" != "${STRATUM_PORT}" ]] || {
    echo "P2P_PORT and STRATUM_PORT must be different." >&2
    exit 64
  }
  [[ -s "$STATE_ROOT/secrets/cloudflare_api_token" ]] || {
    echo "Cloudflare DNS token is required for Stratum DNS-01 TLS." >&2
    exit 64
  }
fi

printf 'Host preflight: cpus=%s memory_bytes=%s free_disk_bytes=%s\n' \
  "$host_cpus" "$host_memory" "$free_bytes"
