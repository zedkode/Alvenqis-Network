#!/usr/bin/env bash
set -Eeuo pipefail

base_url="${ALVENQIS_PUBLIC_RPC_URL:-https://rpcnode.dohotstudio.com}"
stratum_host="${STRATUM_HOST:-stratum.dohotstudio.com}"
stratum_port="${STRATUM_PORT:-3333}"

code="$(curl -sS -o /tmp/alvenqis-retired-mining.txt -w '%{http_code}' \
  "${base_url%/}/mining/template" || true)"
[[ "$code" == 410 ]] || {
  echo "Expected public mining HTTP to be retired with 410, got $code." >&2
  exit 1
}

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
echo "PASS: HTTP mining is retired and Stratum TLS certificate verification succeeded."
