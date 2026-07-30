#!/usr/bin/env bash
set -Eeuo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
[[ -f .env ]] || {
  echo "Missing $root/.env." >&2
  exit 66
}
source scripts/lib.sh
load_dotenv .env
resolve_state_root "$root"
compose_args "$root/.env"
compose_has_service alvenqis-pool || {
  echo "The selected operator role does not include alvenqis-pool." >&2
  exit 64
}

stratum_host="${STRATUM_HOST:-}"
stratum_port="${STRATUM_PORT:-3333}"
miner_address="${ALVENQIS_SMOKE_MINER_ADDRESS:-${POOL_ADDRESS:-}}"
[[ -n "$stratum_host" ]] || {
  echo "Set STRATUM_HOST explicitly." >&2
  exit 64
}
[[ -n "$miner_address" ]] || {
  echo "Set ALVENQIS_SMOKE_MINER_ADDRESS to a valid Alvenqis address." >&2
  exit 64
}

template="$("${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" exec -T alvenqis-pool \
  curl -fsS --max-time 45 \
  "http://alvenqis-rpc:10787/mining/template?miner_address=${miner_address}")" || {
  echo "Docker-private pool mining template request failed." >&2
  exit 1
}
python3 -c 'import json,sys; d=json.load(sys.stdin); assert d.get("template_id") and d.get("network_id") == "alvenqis-mainnet-candidate"' <<<"$template"

command -v openssl >/dev/null 2>&1 || {
  echo "openssl is required for the Stratum TLS smoke check." >&2
  exit 127
}
certificate="$(timeout 20 openssl s_client \
  -connect "127.0.0.1:${stratum_port}" \
  -servername "$stratum_host" \
  -verify_hostname "$stratum_host" \
  -verify_return_error </dev/null 2>&1)"
grep -q 'Verify return code: 0 (ok)' <<<"$certificate" || {
  echo "$certificate" >&2
  exit 1
}
echo "PASS: Solo mining template and Stratum TLS certificate verification succeeded."
