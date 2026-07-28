#!/usr/bin/env bash

# Parse a Docker Compose dotenv file without executing it as shell code.
load_dotenv() {
  local dotenv_path="${1:-.env}"
  [[ -f "$dotenv_path" ]] || {
    echo "Missing dotenv file: $dotenv_path" >&2
    return 66
  }

  while IFS= read -r -d '' entry; do
    local key="${entry%%=*}"
    local value="${entry#*=}"
    printf -v "$key" '%s' "$value"
    export "$key"
  done < <(python3 - "$dotenv_path" <<'PY'
import json
import re
import sys
from pathlib import Path

key_re = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
for raw in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines():
    line = raw.strip()
    if not line or line.startswith("#") or "=" not in line:
        continue
    key, value = line.split("=", 1)
    key = key.strip()
    value = value.strip()
    if not key_re.fullmatch(key):
        raise SystemExit(f"invalid dotenv key: {key!r}")
    if value.startswith('"'):
        value = json.loads(value)
    elif len(value) >= 2 and value[0] == value[-1] == "'":
        value = value[1:-1]
    payload = f"{key}={value}".encode("utf-8") + b"\0"
    sys.stdout.buffer.write(payload)
PY
  )
}

resolve_state_root() {
  local workspace="${1:-$PWD}"
  local configured="${ALVENQIS_STATE_ROOT:-$workspace/state}"

  ALVENQIS_STATE_ROOT="$(
    python3 - "$workspace" "$configured" <<'PY'
import sys
from pathlib import Path

workspace = Path(sys.argv[1]).resolve()
configured = Path(sys.argv[2])
if not configured.is_absolute():
    configured = workspace / configured
resolved = configured.resolve(strict=False)

for forbidden in (
    Path("/"),
    Path("/bin"),
    Path("/boot"),
    Path("/dev"),
    Path("/etc"),
    Path("/home"),
    Path("/lib"),
    Path("/lib64"),
    Path("/opt"),
    Path("/proc"),
    Path("/root"),
    Path("/run"),
    Path("/sbin"),
    Path("/srv"),
    Path("/sys"),
    Path("/tmp"),
    Path("/usr"),
    Path("/var"),
):
    if resolved == forbidden:
        raise SystemExit(f"refusing unsafe ALVENQIS_STATE_ROOT: {resolved}")
if resolved == workspace:
    raise SystemExit("ALVENQIS_STATE_ROOT must not be the workspace itself")
print(resolved)
PY
  )"
  STATE_ROOT="$ALVENQIS_STATE_ROOT"
  export ALVENQIS_STATE_ROOT STATE_ROOT
}

compose_args() {
  ALVENQIS_COMPOSE_ARGS=(docker compose --env-file .env -f compose.yaml)
  if [[ "${CLOUDFLARE_MODE:-disabled}" != "tunnel" ]]; then
    ALVENQIS_COMPOSE_ARGS+=(-f compose.direct.yaml)
  fi

  ALVENQIS_PROFILE_ARGS=(--profile backup)
  if [[ "${CLOUDFLARE_MODE:-disabled}" == "tunnel" ]]; then
    ALVENQIS_PROFILE_ARGS+=(--profile cloudflare)
  fi
  if [[ "${ENABLE_POOL:-false}" == "true" ]]; then
    ALVENQIS_PROFILE_ARGS+=(--profile pool)
  fi
}
