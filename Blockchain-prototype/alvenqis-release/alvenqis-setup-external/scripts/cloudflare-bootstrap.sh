#!/usr/bin/env bash
set -Eeuo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
source scripts/lib.sh
load_dotenv .env
resolve_state_root "$root"

phase="all"
case "${1:-}" in
  "") ;;
  --prepare) phase="prepare" ;;
  --activate) phase="activate" ;;
  *) echo "Usage: $0 [--prepare|--activate]" >&2; exit 64 ;;
esac

workspace="${ALVENQIS_WORKSPACE:-$root}"
secrets_dir="$STATE_ROOT/secrets"
api_token_file="/run/secrets/cloudflare_api_token"
[[ -s "$api_token_file" ]] || api_token_file="$secrets_dir/cloudflare_api_token"
api="https://api.cloudflare.com/client/v4"

mode="${CLOUDFLARE_MODE:-disabled}"
[[ "$mode" != "disabled" ]] || { echo "Cloudflare automation disabled."; exit 0; }
[[ -s "$api_token_file" ]] || { echo "Cloudflare API token secret is missing." >&2; exit 64; }
token="$(cat "$api_token_file")"

: "${CLOUDFLARE_ACCOUNT_ID:?CLOUDFLARE_ACCOUNT_ID is required}"
: "${CLOUDFLARE_ZONE_ID:?CLOUDFLARE_ZONE_ID is required}"
: "${BASE_DOMAIN:?BASE_DOMAIN is required}"
FLEET_MTLS_HOST="${FLEET_MTLS_HOST:-fleet-mtls.${BASE_DOMAIN}}"
export FLEET_MTLS_HOST
: "${CONTROL_HOST:?CONTROL_HOST is required}"
: "${RPC_HOST:?RPC_HOST is required}"
: "${FLEET_HOST:?FLEET_HOST is required}"
: "${FLEET_MTLS_HOST:?FLEET_MTLS_HOST is required}"
[[ "$FLEET_MTLS_HOST" != "$FLEET_HOST" ]] || {
  echo "FLEET_MTLS_HOST must be distinct from tunneled FLEET_HOST." >&2
  exit 64
}
: "${GRAFANA_HOST:?GRAFANA_HOST is required}"
: "${PROMETHEUS_HOST:?PROMETHEUS_HOST is required}"
: "${WEBSITE_HOST:?WEBSITE_HOST is required}"
: "${WWW_HOST:?WWW_HOST is required}"
: "${EXPLORER_HOST:?EXPLORER_HOST is required}"
: "${P2P_HOST:?P2P_HOST is required}"
if [[ "${ENABLE_POOL:-false}" == "true" ]]; then
  : "${STRATUM_HOST:?STRATUM_HOST is required when ENABLE_POOL=true}"
fi

cf() {
  local method="$1"; shift
  local endpoint="$1"; shift
  curl -fsS "$api$endpoint" \
    -X "$method" \
    -H "Authorization: Bearer $token" \
    -H "Content-Type: application/json" \
    "$@"
}

assert_success() {
  local payload="$1"
  jq -e '.success == true' >/dev/null <<<"$payload" || {
    jq . >&2 <<<"$payload"
    return 1
  }
}

detect_ipv4() {
  if [[ -n "${PUBLIC_IPV4:-}" ]]; then
    printf '%s' "$PUBLIC_IPV4"
    return
  fi
  curl -4fsS https://api.ipify.org
}

upsert_dns() {
  local type="$1" name="$2" content="$3" proxied="$4"
  local existing record_id body response
  existing="$(cf GET "/zones/$CLOUDFLARE_ZONE_ID/dns_records?name=$name")"
  assert_success "$existing"
  [[ "$(jq '.result | length' <<<"$existing")" -le 1 ]] || {
    echo "Refusing to modify ambiguous duplicate DNS records for $name" >&2
    return 1
  }
  record_id="$(jq -r '.result[0].id // empty' <<<"$existing")"
  body="$(jq -n --arg type "$type" --arg name "$name" --arg content "$content" --argjson proxied "$proxied" \
    '{type:$type,name:$name,content:$content,ttl:1,proxied:$proxied,comment:"Managed by Alvenqis Docker control plane"}')"
  if [[ -n "$record_id" ]]; then
    response="$(cf PUT "/zones/$CLOUDFLARE_ZONE_ID/dns_records/$record_id" --data "$body")"
  else
    response="$(cf POST "/zones/$CLOUDFLARE_ZONE_ID/dns_records" --data "$body")"
  fi
  assert_success "$response"
  echo "DNS ready: $name -> $content ($type, proxied=$proxied)"
}

public_ip="$(detect_ipv4)"
[[ "$public_ip" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]] || {
  echo "Could not determine a valid PUBLIC_IPV4." >&2
  exit 65
}

# P2P must remain DNS-only because standard Cloudflare HTTP proxy/Tunnel is not
# a transparent public TCP proxy for arbitrary blockchain clients.
upsert_dns A "$P2P_HOST" "$public_ip" false
# Fleet report and certificate-rotation traffic terminates as client-authenticated
# TLS at the gateway. Keep this hostname on direct, DNS-only transport; never
# route it through the HTTP tunnel used by the enrollment hostname.
upsert_dns A "$FLEET_MTLS_HOST" "$public_ip" false
if [[ "${ENABLE_POOL:-false}" == "true" ]]; then
  # Standard Cloudflare Tunnel/HTTP proxy is not a transparent public Stratum proxy.
  upsert_dns A "$STRATUM_HOST" "$public_ip" false
fi

[[ "$mode" == "tunnel" ]] || { echo "Unsupported CLOUDFLARE_MODE: $mode" >&2; exit 64; }
tunnel_name="${CLOUDFLARE_TUNNEL_NAME:-alvenqis-setup-external}"

list="$(cf GET "/accounts/$CLOUDFLARE_ACCOUNT_ID/cfd_tunnel?is_deleted=false&name=$tunnel_name")"
assert_success "$list"
tunnel_id="$(jq -r --arg n "$tunnel_name" '.result[] | select(.name == $n) | .id' <<<"$list" | head -n1)"

if [[ -z "$tunnel_id" ]]; then
  created="$(cf POST "/accounts/$CLOUDFLARE_ACCOUNT_ID/cfd_tunnel" \
    --data "$(jq -n --arg name "$tunnel_name" '{name:$name,config_src:"cloudflare"}')")"
  assert_success "$created"
  tunnel_id="$(jq -r '.result.id' <<<"$created")"
  tunnel_token="$(jq -r '.result.token // empty' <<<"$created")"
  echo "Created Cloudflare Tunnel: $tunnel_name ($tunnel_id)"
else
  tunnel_token=""
  echo "Reusing Cloudflare Tunnel: $tunnel_name ($tunnel_id)"
fi

if [[ -z "$tunnel_token" ]]; then
  token_response="$(cf GET "/accounts/$CLOUDFLARE_ACCOUNT_ID/cfd_tunnel/$tunnel_id/token")"
  assert_success "$token_response"
  tunnel_token="$(jq -r '.result' <<<"$token_response")"
fi
[[ -n "$tunnel_token" && "$tunnel_token" != "null" ]] || {
  echo "Cloudflare did not return a tunnel token." >&2
  exit 69
}
printf '%s\n' "$tunnel_token" > "$secrets_dir/cloudflare_tunnel_token"
chmod 0444 "$secrets_dir/cloudflare_tunnel_token"

if [[ "$phase" == "prepare" ]]; then
  echo "Cloudflare Tunnel token prepared; production ingress and DNS were not changed."
  exit 0
fi

ingress="$(jq -n \
  --arg control "$CONTROL_HOST" \
  --arg rpc "$RPC_HOST" \
  --arg fleet "$FLEET_HOST" \
  --arg grafana "$GRAFANA_HOST" \
  --arg prometheus "$PROMETHEUS_HOST" \
  --arg pool "$POOL_HOST" \
  --arg website "$WEBSITE_HOST" \
  --arg www "$WWW_HOST" \
  --arg explorer "$EXPLORER_HOST" \
  --argjson pool_enabled "$([[ "${ENABLE_POOL:-false}" == "true" ]] && echo true || echo false)" \
  '{
    config: {
      ingress: (
        [
          {hostname:$control, service:"http://gateway:8080"},
          {hostname:$rpc, service:"http://gateway:8080"},
          {hostname:$fleet, service:"http://gateway:8080"},
          {hostname:$grafana, service:"http://gateway:8080"},
          {hostname:$prometheus, service:"http://gateway:8080"},
          {hostname:$website, service:"http://gateway:8080"},
          {hostname:$www, service:"http://gateway:8080"},
          {hostname:$explorer, service:"http://gateway:8080"}
        ]
        + (if $pool_enabled then [{hostname:$pool, service:"http://gateway:8080"}] else [] end)
        + [{service:"http_status:404"}]
      ),
      originRequest: {connectTimeout:30}
    }
  }')"
configured="$(cf PUT "/accounts/$CLOUDFLARE_ACCOUNT_ID/cfd_tunnel/$tunnel_id/configurations" --data "$ingress")"
assert_success "$configured"

for host in "$CONTROL_HOST" "$RPC_HOST" "$FLEET_HOST" "$GRAFANA_HOST" "$PROMETHEUS_HOST" "$WEBSITE_HOST" "$WWW_HOST" "$EXPLORER_HOST"; do
  upsert_dns CNAME "$host" "$tunnel_id.cfargotunnel.com" true
done
if [[ "${ENABLE_POOL:-false}" == "true" ]]; then
  upsert_dns CNAME "$POOL_HOST" "$tunnel_id.cfargotunnel.com" true
fi
echo "Cloudflare Tunnel and DNS configuration completed."
