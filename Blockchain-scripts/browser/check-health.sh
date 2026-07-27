#!/usr/bin/env bash
# Thin wrapper around alvenqis-browser-host --check-health for CI / cron.
#
# Usage:
#   ./scripts/browser/check-health.sh
#   ./scripts/browser/check-health.sh --strict
#   ./scripts/browser/check-health.sh --rpc https://rpcnode.dohotstudio.com --json
#   ./scripts/browser/check-health.sh --strict --webhook-url "$URL"
#
# Exit codes: same as host --check-health (0/1/2/3)

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

RPC=""
STRICT=0
MAX_LAG=""
JSON=1
BUILD=0
WEBHOOK=""
LOCAL=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rpc) RPC="${2:-}"; shift 2 ;;
    --local) LOCAL=1; shift ;;
    --strict) STRICT=1; shift ;;
    --max-indexer-lag) MAX_LAG="${2:-}"; shift 2 ;;
    --json) JSON=1; shift ;;
    --no-json) JSON=0; shift ;;
    --build) BUILD=1; shift ;;
    --webhook-url) WEBHOOK="${2:-}"; shift 2 ;;
    -h|--help)
      sed -n '1,22p' "$0"
      exit 0
      ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

if [[ $BUILD -eq 1 ]]; then
  cargo build -q -p alvenqis-browser-host --release
fi

HOST_BIN=""
for c in \
  "$REPO_ROOT/Blockchain-prototype/target/release/alvenqis-browser-host" \
  "$REPO_ROOT/Blockchain-prototype/target/debug/alvenqis-browser-host"
do
  if [[ -x "$c" ]]; then HOST_BIN="$c"; break; fi
done
if [[ -z "$HOST_BIN" ]]; then
  cargo build -q -p alvenqis-browser-host
  HOST_BIN="$REPO_ROOT/Blockchain-prototype/target/debug/alvenqis-browser-host"
fi

ARGS=(--check-health)
[[ $JSON -eq 1 ]] && ARGS+=(--json)
[[ $LOCAL -eq 1 ]] && ARGS+=(--local)
[[ -n "$RPC" ]] && ARGS+=(--rpc "$RPC")
[[ $STRICT -eq 1 ]] && ARGS+=(--require-indexer-sync)
[[ -n "$MAX_LAG" ]] && ARGS+=(--max-indexer-lag "$MAX_LAG")

TMP="$(mktemp)"
set +e
"$HOST_BIN" "${ARGS[@]}" | tee "$TMP"
CODE=${PIPESTATUS[0]}
set -e

if [[ $CODE -ne 0 && -n "$WEBHOOK" ]]; then
  BODY="$(cat "$TMP" 2>/dev/null || echo '{}')"
  if command -v jq >/dev/null 2>&1; then
    PAYLOAD=$(jq -nc --argjson health "$BODY" --arg code "$CODE" \
      '{text:"Alvenqis Mainnet Candidate health FAILED",code:($code|tonumber),health:$health}')
  else
    PAYLOAD="{\"text\":\"Alvenqis Mainnet Candidate health FAILED\",\"code\":$CODE}"
  fi
  curl -fsS -X POST -H "Content-Type: application/json" -d "$PAYLOAD" "$WEBHOOK" || true
fi

rm -f "$TMP"
exit "$CODE"
