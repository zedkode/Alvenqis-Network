#!/usr/bin/env bash
set -Eeuo pipefail
workspace="${ALVENQIS_WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$workspace"
source scripts/lib.sh
load_dotenv .env
compose_args
required=(alvenqis-node alvenqis-rpc alvenqis-indexer alvenqis-control docker-broker alvenqis-ops alvenqis-website alvenqis-explorer gateway prometheus alertmanager blackbox-exporter node-exporter cadvisor alvenqis-metrics-exporter loki alloy grafana backup-scheduler); deadline=$((SECONDS+420))
[[ "${CLOUDFLARE_MODE:-disabled}" == tunnel ]] && required+=(cloudflared)
[[ "${ENABLE_POOL:-false}" == true ]] && required+=(alvenqis-pool stratum-certbot)
while ((SECONDS<deadline)); do
  fail=0
  for service in "${required[@]}"; do
    container="$("${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" ps -q "$service" 2>/dev/null || true)"
    [[ -n "$container" ]] || { fail=1; continue; }
    status="$(docker inspect -f '{{.State.Status}}' "$container")"
    health="$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' "$container")"
    [[ "$status" == running && "$health" == healthy ]] || fail=1
  done
  ((fail == 0)) && break
  sleep 5
done

memory_limit_total=0
for service in "${required[@]}"; do
  container="$("${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" ps -q "$service")"
  [[ "$(docker inspect -f '{{.State.Status}}' "$container")" == running ]] || {
    echo "Container is not running: $service" >&2
    exit 1
  }
  [[ "$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' "$container")" == healthy ]] || {
    echo "Container is not healthy: $service" >&2
    docker inspect -f '{{json .State.Health}}' "$container" >&2 || true
    exit 1
  }
  memory_limit="$(docker inspect -f '{{.HostConfig.Memory}}' "$container")"
  ((memory_limit > 0)) || {
    echo "Container has no memory limit: $service" >&2
    exit 1
  }
  memory_limit_total=$((memory_limit_total + memory_limit))
done

budget_bytes="${CONTAINER_MEMORY_BUDGET_BYTES:-11274289152}"
((memory_limit_total <= budget_bytes)) || {
  echo "Container memory limits exceed the VPS budget: ${memory_limit_total} > ${budget_bytes}" >&2
  exit 1
}
printf 'Container memory-limit budget: %s / %s bytes\n' "$memory_limit_total" "$budget_bytes"

"${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-rpc curl -fsS http://127.0.0.1:10787/health >/dev/null
status_json="$("${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-rpc curl -fsS http://127.0.0.1:10787/status)"
p2p_json="$("${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-rpc curl -fsS http://127.0.0.1:10787/p2p/status)"
read -r initialized height index_lag < <(python3 -c 'import json,sys; d=json.load(sys.stdin); print(str(bool(d.get("initialized"))).lower(), d.get("height", -1), d.get("index_lag_blocks", -1))' <<<"$status_json")
read -r configured connected validated < <(python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("configured_seed_count", -1), d.get("connected_peer_count", -1), d.get("validated_peer_count", -1))' <<<"$p2p_json")
[[ "$initialized" == true && "$height" -ge 0 ]] || {
  echo "RPC readiness failed: chain is not initialized." >&2
  exit 1
}
[[ "$configured" -ge 0 && "$connected" -ge 0 && "$validated" -ge 0 ]] || {
  echo "P2P readiness failed: invalid live counters." >&2
  exit 1
}
if [[ -n "${SEED_NODES_TOML:-}" && "$configured" -eq 0 ]]; then
  echo "P2P readiness failed: seeds were configured but the node reports zero configured seeds." >&2
  exit 1
fi
minimum_peers="${P2P_MIN_VALIDATED_PEERS:-0}"
[[ "$minimum_peers" =~ ^[0-9]+$ ]] || {
  echo "P2P_MIN_VALIDATED_PEERS must be a non-negative integer." >&2
  exit 64
}
((validated >= minimum_peers)) || {
  echo "P2P readiness failed: validated peers ${validated} < required ${minimum_peers}." >&2
  exit 1
}
printf 'Live chain/P2P: height=%s index_lag=%s seeds=%s connected=%s validated=%s\n' \
  "$height" "$index_lag" "$configured" "$connected" "$validated"

if command -v ss >/dev/null 2>&1; then
  ss -ltn | grep -Eq "[:.]${P2P_PORT:-20787}[[:space:]]" || {
    echo "Host TCP P2P port ${P2P_PORT:-20787} is not listening." >&2
    exit 1
  }
fi

# Mining methods are reachable only on the Docker-internal RPC route used by
# the pool. The public gateway boundary must retire them with HTTP 410.
public_mining_code="$("${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-rpc curl -sS -o /dev/null -w '%{http_code}' -H "Host: ${RPC_HOST}" "http://gateway:8080/mining/template")"
[[ "$public_mining_code" == 410 ]] || {
  echo "Public mining route must return HTTP 410, got $public_mining_code" >&2
  exit 1
}
"${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-control curl -fsS http://127.0.0.1:10788/health >/dev/null
"${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-metrics-exporter curl -fsS http://127.0.0.1:9101/health >/dev/null
"${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-rpc curl -fsS -H "Host: ${WEBSITE_HOST}" http://gateway:8080/healthz >/dev/null
"${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-rpc curl -fsS -H "Host: ${EXPLORER_HOST}" http://gateway:8080/healthz >/dev/null
if [[ "${ENABLE_POOL:-false}" == true ]]; then
  private_template="$("${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-pool curl -fsS --max-time 20 "http://alvenqis-rpc:10787/mining/template?miner_address=${POOL_ADDRESS}")"
  python3 -c 'import json,sys; d=json.load(sys.stdin); assert d.get("template_id") and d.get("network_id") == "alvenqis-mainnet-candidate"' <<<"$private_template" || {
    echo "Private mining RPC did not return a valid live template." >&2
    exit 1
  }
  pool_health="$("${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-pool curl -fsS http://127.0.0.1:30787/health)"
  echo "$pool_health" | grep -q '"stratum_tls":true' || {
    echo "Pool health does not confirm Stratum TLS: $pool_health" >&2
    exit 1
  }
  echo "$pool_health" | grep -q '"http_mining_api":false' || {
    echo "Pool health does not confirm retired HTTP mining API: $pool_health" >&2
    exit 1
  }
  pool_status="$("${ALVENQIS_COMPOSE_ARGS[@]}" exec -T alvenqis-pool curl -fsS http://127.0.0.1:30787/api/v1/pool/status)"
  echo "$pool_status" | grep -q '"upstream_status":"healthy"' || {
    echo "Pool upstream is degraded: $pool_status" >&2
    exit 1
  }
  timeout 20 openssl s_client \
    -connect "127.0.0.1:${STRATUM_PORT:-3333}" \
    -servername "$STRATUM_HOST" \
    -verify_hostname "$STRATUM_HOST" \
    -verify_return_error </dev/null 2>&1 | grep -q 'Verify return code: 0 (ok)' || {
      echo "Stratum TLS readiness failed for ${STRATUM_HOST}:${STRATUM_PORT:-3333}." >&2
      exit 1
    }
fi
echo "Alvenqis Docker stack health and readiness checks passed."
