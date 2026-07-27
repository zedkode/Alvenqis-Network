#!/usr/bin/env bash
# Probe Mainnet Candidate chain via alvenqis-browser-host one-shots.
# Usage:
#   ./scripts/browser/probe-chain.sh
#   ./scripts/browser/probe-chain.sh --local
#   ./scripts/browser/probe-chain.sh --strict
#   ./scripts/browser/probe-chain.sh --watch --interval 15 --strict
#   ./scripts/browser/probe-chain.sh --json --include-block --height 0

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

RPC=""
LOCAL=0
BUILD=0
JSON=0
QUIET=0
INCLUDE_BLOCK=0
HEIGHT=""
STRICT=0
MAX_LAG=""
WATCH=0
INTERVAL=15
MAX_ITER=0
WEBHOOK="${ALVENQIS_HEALTH_WEBHOOK_URL:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rpc) RPC="${2:-}"; shift 2 ;;
    --local) LOCAL=1; shift ;;
    --build) BUILD=1; shift ;;
    --json) JSON=1; shift ;;
    --quiet) QUIET=1; shift ;;
    --include-block) INCLUDE_BLOCK=1; shift ;;
    --height) HEIGHT="${2:-}"; INCLUDE_BLOCK=1; shift 2 ;;
    --strict) STRICT=1; shift ;;
    --max-indexer-lag) MAX_LAG="${2:-}"; shift 2 ;;
    --watch) WATCH=1; shift ;;
    --interval) INTERVAL="${2:-15}"; shift 2 ;;
    --max-iterations) MAX_ITER="${2:-0}"; shift 2 ;;
    --webhook-url) WEBHOOK="${2:-}"; shift 2 ;;
    -h|--help)
      sed -n '1,20p' "$0"
      exit 0
      ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

if [[ $BUILD -eq 1 ]]; then
  cargo build -q -p alvenqis-browser-host
fi

HOST_BIN=""
for c in "$REPO_ROOT/Blockchain-prototype/target/debug/alvenqis-browser-host" "$REPO_ROOT/Blockchain-prototype/target/release/alvenqis-browser-host"; do
  if [[ -x "$c" ]]; then HOST_BIN="$c"; break; fi
done
if [[ -z "$HOST_BIN" ]]; then
  cargo build -q -p alvenqis-browser-host
  HOST_BIN="$REPO_ROOT/Blockchain-prototype/target/debug/alvenqis-browser-host"
fi

run_host() {
  local args=()
  [[ $LOCAL -eq 1 ]] && args+=(--local)
  [[ -n "$RPC" ]] && args+=(--rpc "$RPC")
  [[ $JSON -eq 1 ]] && args+=(--json)
  args+=("$@")
  set +e
  "$HOST_BIN" "${args[@]}"
  local code=$?
  set -e
  return $code
}

notify_webhook() {
  local code="$1"
  local body="$2"
  [[ -z "$WEBHOOK" ]] && return 0
  local payload
  if command -v jq >/dev/null 2>&1; then
    payload=$(jq -nc --argjson health "${body:-{}}" --arg code "$code" \
      '{text:"Alvenqis Mainnet Candidate health FAILED",code:($code|tonumber),health:$health}')
  else
    payload="{\"text\":\"Alvenqis Mainnet Candidate health FAILED\",\"code\":$code}"
  fi
  curl -fsS -X POST -H "Content-Type: application/json" -d "$payload" "$WEBHOOK" || true
}

probe_once() {
  [[ $QUIET -eq 0 ]] && echo "=== Alvenqis chain probe $(date -Iseconds) ==="
  [[ $QUIET -eq 0 ]] && echo "--check-health"
  local health_args=(--check-health --json)
  [[ $STRICT -eq 1 ]] && health_args+=(--require-indexer-sync)
  [[ -n "$MAX_LAG" ]] && health_args+=(--max-indexer-lag "$MAX_LAG")
  local health_body
  set +e
  health_body=$(run_host "${health_args[@]}" 2>&1)
  local health_code=$?
  set -e
  [[ $QUIET -eq 0 ]] && echo "$health_body"
  if [[ $health_code -ne 0 ]]; then
    notify_webhook "$health_code" "$health_body"
  fi

  [[ $QUIET -eq 0 ]] && echo "--print-tip"
  run_host --print-tip >/dev/null || true
  [[ $QUIET -eq 0 ]] && echo "--print-chain"
  run_host --print-chain >/dev/null || true

  if [[ $INCLUDE_BLOCK -eq 1 ]]; then
    [[ $QUIET -eq 0 ]] && echo "--print-block"
    if [[ -n "$HEIGHT" ]]; then
      run_host --print-block --height "$HEIGHT" >/dev/null || true
    else
      run_host --print-block >/dev/null || true
    fi
  fi

  [[ $QUIET -eq 0 ]] && echo "health_code=$health_code"
  return $health_code
}

if [[ $WATCH -eq 1 ]]; then
  [[ $INTERVAL -lt 3 ]] && INTERVAL=3
  i=0
  last=0
  while true; do
    i=$((i + 1))
    set +e
    probe_once
    last=$?
    set -e
    if [[ $MAX_ITER -gt 0 && $i -ge $MAX_ITER ]]; then
      exit $last
    fi
    [[ $QUIET -eq 0 ]] && echo "Sleeping ${INTERVAL}s (watch)... Ctrl+C to stop"
    sleep "$INTERVAL"
  done
else
  set +e
  probe_once
  code=$?
  set -e
  exit $code
fi
