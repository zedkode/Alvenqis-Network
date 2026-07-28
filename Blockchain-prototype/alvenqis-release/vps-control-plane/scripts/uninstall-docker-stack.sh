#!/usr/bin/env bash
set -Eeuo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$root"; purge=false; [[ "${1:-}" == --purge-data ]] && purge=true
if [[ -f .env ]]; then
  source scripts/lib.sh
  load_dotenv .env
  resolve_state_root "$root"
  compose_args
  "${ALVENQIS_COMPOSE_ARGS[@]}" --profile cloudflare --profile pool --profile backup down --remove-orphans || true
fi
[[ -f .installer.env ]] && docker compose --env-file .installer.env -f installer.compose.yaml down --remove-orphans || true
if $purge; then
  [[ -n "${STATE_ROOT:-}" ]] || {
    echo "Cannot purge without a validated ALVENQIS_STATE_ROOT." >&2
    exit 64
  }
  read -r -p "Type PURGE $STATE_ROOT: " answer
  if [[ "$answer" == "PURGE $STATE_ROOT" ]]; then
    rm -rf --one-file-system -- "$STATE_ROOT"
    rm -f -- .env .installer.env
  else
    echo "Purge cancelled; persistent data preserved at $STATE_ROOT."
  fi
fi
