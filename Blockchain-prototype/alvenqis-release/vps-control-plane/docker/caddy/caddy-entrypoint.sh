#!/bin/sh
set -eu

password="$(cat /run/secrets/admin_password)"
if [ -z "$password" ]; then
  echo "admin_password secret is empty" >&2
  exit 64
fi

export ADMIN_PASSWORD_HASH
ADMIN_PASSWORD_HASH="$(caddy hash-password --plaintext "$password")"

if [ "${CLOUDFLARE_MODE:-disabled}" = "tunnel" ]; then
  export CADDY_SCHEME=http
else
  export CADDY_SCHEME=https
fi

caddy validate --config /etc/caddy/Caddyfile.template --adapter caddyfile
exec caddy run --config /etc/caddy/Caddyfile.template --adapter caddyfile
