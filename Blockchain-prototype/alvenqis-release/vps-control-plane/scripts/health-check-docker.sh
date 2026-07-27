#!/usr/bin/env bash
set -Eeuo pipefail
workspace="${ALVENQIS_WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$workspace"
source scripts/lib.sh
load_dotenv .env
compose_args
required=(alvenqis-node alvenqis-rpc alvenqis-indexer alvenqis-control docker-broker alvenqis-ops caddy prometheus alertmanager blackbox-exporter node-exporter alvenqis-metrics-exporter loki alloy grafana backup-scheduler); deadline=$((SECONDS+420))
[[ "${CLOUDFLARE_MODE:-disabled}" == tunnel ]] && required+=(cloudflared)
[[ "${ENABLE_POOL:-false}" == true ]] && required+=(alvenqis-pool stratum-certbot)
while ((SECONDS<deadline)); do fail=0; for s in "${required[@]}"; do c="$("${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" ps -q "$s" 2>/dev/null || true)"; [[ -n $c ]] || { fail=1; continue; }; st="$(docker inspect -f '{{.State.Status}}' "$c")"; h="$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$c")"; [[ $st == running && $h != unhealthy ]] || fail=1; done; ((fail==0)) && break; sleep 5; done
for s in "${required[@]}"; do c="$("${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" ps -q "$s")"; [[ "$(docker inspect -f '{{.State.Status}}' "$c")" == running ]] || exit 1; [[ "$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$c")" != unhealthy ]] || exit 1; done
"${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-rpc curl -fsS http://127.0.0.1:10787/health >/dev/null
# Mining methods exist only on the private gateway for the Stratum pool. The
# public Caddy boundary must retire them with HTTP 410.
public_mining_code="$("${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-rpc curl -sS -o /dev/null -w '%{http_code}' -H "Host: ${RPC_HOST}" http://caddy/mining/template)"
[[ "$public_mining_code" == 410 ]] || {
  echo "Public mining route must return HTTP 410, got $public_mining_code" >&2
  exit 1
}
"${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-control curl -fsS http://127.0.0.1:10788/health >/dev/null
"${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-metrics-exporter curl -fsS http://127.0.0.1:9101/health >/dev/null
if [[ "${ENABLE_POOL:-false}" == true ]]; then
  pool_health="$("${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-pool curl -fsS http://127.0.0.1:30787/health)"
  echo "$pool_health" | grep -q '"stratum_tls":true' || {
    echo "Pool health does not confirm Stratum TLS: $pool_health" >&2
    exit 1
  }
  echo "$pool_health" | grep -q '"http_mining_api":false' || {
    echo "Pool health does not confirm retired HTTP mining API: $pool_health" >&2
    exit 1
  }
fi
echo "Alvenqis Docker stack healthy; PostgreSQL intentionally absent until a real DB adapter exists."
