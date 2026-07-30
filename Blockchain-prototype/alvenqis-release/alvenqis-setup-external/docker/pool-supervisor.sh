#!/usr/bin/env bash
set -uo pipefail

config="${1:?pool config path is required}"
cert_file="/certs/live/${STRATUM_HOST:?STRATUM_HOST is required}/fullchain.pem"
key_file="/certs/live/${STRATUM_HOST}/privkey.pem"
child=""

stop_child() {
  [[ -z "$child" ]] || kill -TERM "$child" 2>/dev/null || true
}
trap stop_child TERM INT

certificate_fingerprint() {
  sha256sum "$cert_file" "$key_file" 2>/dev/null | sha256sum | cut -d' ' -f1
}

while :; do
  [[ -s "$cert_file" && -s "$key_file" ]] || {
    echo "Stratum TLS material is unavailable." >&2
    exit 65
  }
  fingerprint="$(certificate_fingerprint)"
  alvenqis-mining-pool --config "$config" &
  child=$!

  while kill -0 "$child" 2>/dev/null; do
    sleep 30
    current="$(certificate_fingerprint)"
    if [[ -n "$current" && "$current" != "$fingerprint" ]]; then
      echo "Stratum TLS certificate changed; restarting pool listener." >&2
      kill -TERM "$child" 2>/dev/null || true
      wait "$child" 2>/dev/null || true
      child=""
      break
    fi
  done

  if [[ -n "$child" ]]; then
    wait "$child"
    exit $?
  fi
done
