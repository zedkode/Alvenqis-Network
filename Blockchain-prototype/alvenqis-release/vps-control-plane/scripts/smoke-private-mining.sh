#!/usr/bin/env bash
set -Eeuo pipefail

base_url="${ALVENQIS_PUBLIC_RPC_URL:-https://rpcnode.dohotstudio.com}"
stratum_host="${STRATUM_HOST:-stratum.dohotstudio.com}"
stratum_port="${STRATUM_PORT:-3333}"
miner_address="${ALVENQIS_SMOKE_MINER_ADDRESS:-${POOL_ADDRESS:-}}"
[[ -n "$miner_address" ]] || {
  echo "Set ALVENQIS_SMOKE_MINER_ADDRESS to a valid Alvenqis address." >&2
  exit 64
}

template="$(curl -fsS --max-time 45 \
  "${base_url%/}/mining/template?miner_address=${miner_address}")" || {
  echo "Public solo mining template request failed." >&2
  exit 1
}
python3 -c 'import json,sys; d=json.load(sys.stdin); assert d.get("template_id") and d.get("network_id") == "alvenqis-mainnet-candidate"' <<<"$template"

command -v openssl >/dev/null 2>&1 || {
  echo "openssl is required for the Stratum TLS smoke check." >&2
  exit 127
}
certificate="$(timeout 20 openssl s_client \
  -connect "${stratum_host}:${stratum_port}" \
  -servername "$stratum_host" \
  -verify_return_error </dev/null 2>&1)"
grep -q 'Verify return code: 0 (ok)' <<<"$certificate" || {
  echo "$certificate" >&2
  exit 1
}
echo "PASS: Solo mining template and Stratum TLS certificate verification succeeded."
