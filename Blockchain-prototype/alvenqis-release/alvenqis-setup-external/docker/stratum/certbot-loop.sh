#!/bin/sh
set -eu

token_file=/run/secrets/cloudflare_api_token
credentials=/tmp/cloudflare.ini
cert_root=/etc/letsencrypt

test -s "$token_file" || {
  echo "Cloudflare API token is required for Stratum DNS-01 TLS." >&2
  exit 64
}
test -n "${STRATUM_HOST:-}" || {
  echo "STRATUM_HOST is required." >&2
  exit 64
}
test -n "${ADMIN_EMAIL:-}" || {
  echo "ADMIN_EMAIL is required." >&2
  exit 64
}

umask 077
printf 'dns_cloudflare_api_token = %s\n' "$(cat "$token_file")" > "$credentials"

while :; do
  if certbot certonly \
    --non-interactive \
    --agree-tos \
    --email "$ADMIN_EMAIL" \
    --dns-cloudflare \
    --dns-cloudflare-credentials "$credentials" \
    --dns-cloudflare-propagation-seconds 30 \
    --cert-name "$STRATUM_HOST" \
    --keep-until-expiring \
    -d "$STRATUM_HOST"; then
    # Pool container runs as uid/gid 10001 and must read the TLS material.
    # Certbot defaults to root-only keys; open read access for the pool without
    # publishing the private key world-writable.
    find "$cert_root" -type d -exec chmod 755 {} + 2>/dev/null || true
    find "$cert_root" -type f -exec chmod 644 {} + 2>/dev/null || true
    find "$cert_root" -name 'privkey*.pem' -exec chmod 640 {} + 2>/dev/null || true
    chgrp -R 10001 "$cert_root" 2>/dev/null || true
    printf 'ready\n' > "$cert_root/.stratum-cert-ready"
    sleep 43200
  else
    echo "Stratum certificate issue/renewal failed; retrying in 5 minutes." >&2
    sleep 300
  fi
done
