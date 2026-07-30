#!/bin/sh
set -eu

required_variables="
ADMIN_USER
CONTROL_HOST
RPC_HOST
FLEET_HOST
GRAFANA_HOST
PROMETHEUS_HOST
POOL_HOST
STRATUM_HOST
STRATUM_PORT
WEBSITE_HOST
WWW_HOST
EXPLORER_HOST
"

for variable in $required_variables; do
  eval "value=\${$variable:-}"
  if [ -z "$value" ]; then
    echo "$variable is required" >&2
    exit 64
  fi
done

password="$(cat /run/secrets/admin_password)"
if [ -z "$password" ]; then
  echo "admin_password secret is empty" >&2
  exit 64
fi

umask 077
htpasswd -mbn "$ADMIN_USER" "$password" > /tmp/admin.htpasswd
unset password

envsubst '${CONTROL_HOST} ${RPC_HOST} ${FLEET_HOST} ${GRAFANA_HOST} ${PROMETHEUS_HOST} ${POOL_HOST} ${STRATUM_HOST} ${STRATUM_PORT} ${WEBSITE_HOST} ${WWW_HOST} ${EXPLORER_HOST}' \
  < /etc/alvenqis/nginx.conf.template \
  > /tmp/nginx.conf

nginx -t -c /tmp/nginx.conf
exec nginx -c /tmp/nginx.conf -g 'daemon off;'
