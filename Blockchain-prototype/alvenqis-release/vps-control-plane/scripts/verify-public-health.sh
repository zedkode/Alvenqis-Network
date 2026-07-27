#!/usr/bin/env bash
set -Eeuo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
source scripts/lib.sh
load_dotenv .env

[[ "${CLOUDFLARE_MODE:-disabled}" != disabled ]] || {
  echo "Public verification skipped because Cloudflare automation is disabled."
  exit 0
}

urls=("https://${RPC_HOST}/health" "https://${FLEET_HOST}/")
[[ "${ENABLE_POOL:-false}" != true ]] || urls+=("https://${RPC_HOST}/pool/health" "https://${POOL_HOST}/health")

for url in "${urls[@]}"; do
  ready=false
  for _ in {1..24}; do
    if curl -fsS --connect-timeout 5 --max-time 12 "$url" >/dev/null; then
      ready=true
      break
    fi
    sleep 5
  done
  [[ "$ready" == true ]] || {
    echo "Public endpoint failed after DNS/tunnel activation: $url" >&2
    exit 1
  }
  echo "Public endpoint healthy: $url"
done
