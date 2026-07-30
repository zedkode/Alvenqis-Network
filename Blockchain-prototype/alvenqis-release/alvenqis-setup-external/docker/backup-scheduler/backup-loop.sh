#!/bin/sh
set -eu
sleep "${BACKUP_INITIAL_DELAY_SECONDS:-3600}"
while true; do
 token="$(cat "${BROKER_TOKEN_FILE:-/run/secrets/broker_token}")"
 curl -fsS -X POST "${BROKER_URL:-http://docker-broker:8090}/v1/action" -H "Content-Type: application/json" -H "X-Alvenqis-Broker-Token: $token" --data '{"action":"backup"}' || true
 sleep "${BACKUP_INTERVAL_SECONDS:-86400}"
done
