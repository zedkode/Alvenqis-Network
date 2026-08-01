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
  local workspace="${ALVENQIS_WORKSPACE:-$PWD}"
  local dotenv_path="${1:-$workspace/.env}"
  local compose_bin_override="${2:-}"
  local registry="$workspace/compose/roles.json"
  local requested_role="${ALVENQIS_OPERATOR_ROLE:-${ALVENQIS_DEPLOYMENT_ROLE:-full-stack}}"
  local control_role="${CONTROL_ROLE:-standalone}"
  local cloudflare_mode="${CLOUDFLARE_MODE:-disabled}"
  local pool_enabled="${ENABLE_POOL:-false}"

  [[ -f "$registry" ]] || {
    echo "Missing Compose role registry: $registry" >&2
    return 66
  }
  [[ -f "$dotenv_path" ]] || {
    echo "Missing Compose dotenv file: $dotenv_path" >&2
    return 66
  }

  ALVENQIS_COMPOSE_FILES=()
  ALVENQIS_PROFILE_ARGS=()
  ALVENQIS_REQUIRED_SERVICES=()
  while IFS= read -r -d '' entry; do
    case "$entry" in
      role=*) ALVENQIS_OPERATOR_ROLE_RESOLVED="${entry#role=}" ;;
      file=*) ALVENQIS_COMPOSE_FILES+=("$workspace/compose/${entry#file=}") ;;
      profile=*) ALVENQIS_PROFILE_ARGS+=(--profile "${entry#profile=}") ;;
      service=*) ALVENQIS_REQUIRED_SERVICES+=("${entry#service=}") ;;
      *) echo "Invalid Compose role registry output: $entry" >&2; return 78 ;;
    esac
  done < <(
    python3 - "$registry" "$requested_role" "$control_role" "$pool_enabled" "$cloudflare_mode" <<'PY'
import json
import sys
from pathlib import Path

registry_path, requested_role, control_role, pool_enabled, cloudflare_mode = sys.argv[1:]
registry = json.loads(Path(registry_path).read_text(encoding="utf-8"))
roles = registry.get("roles", {})
role = requested_role.strip().lower()
if role not in roles:
    allowed = ", ".join(sorted(roles))
    raise SystemExit(f"unsupported ALVENQIS_OPERATOR_ROLE {requested_role!r}; expected one of: {allowed}")
if control_role not in {"standalone", "agent", "controller"}:
    raise SystemExit("CONTROL_ROLE must be standalone, agent or controller")
if pool_enabled.lower() not in {"true", "false"}:
    raise SystemExit("ENABLE_POOL must be true or false")
if cloudflare_mode not in {"disabled", "dns", "tunnel"}:
    raise SystemExit("CLOUDFLARE_MODE must be disabled, dns or tunnel")

selected = roles[role]
files = list(selected["files"])
profiles = list(selected["profiles"])
services = list(selected["required_services"])

def add_file(name, before=None):
    if name in files:
        return
    if before and before in files:
        files.insert(files.index(before), name)
    else:
        files.append(name)

def add_service(name):
    if name not in services:
        services.append(name)

if control_role in {"agent", "controller"}:
    add_file("rpc.yaml", before="indexer-explorer.yaml")
    add_file("project-edge.yaml")
    add_service("alvenqis-rpc")
    add_service("alvenqis-control")

project_edge_enabled = "project-edge" in profiles
if role == "full-stack" and pool_enabled.lower() == "true":
    add_file("pool.yaml", before="project-edge.yaml")
    add_service("stratum-certbot")
    add_service("alvenqis-pool")
if role in {"pool", "stratum"} and pool_enabled.lower() != "true":
    raise SystemExit(f"ALVENQIS_OPERATOR_ROLE={role} requires ENABLE_POOL=true")

if project_edge_enabled:
    add_file("cloudflare.yaml" if cloudflare_mode == "tunnel" else "direct.yaml")
    if cloudflare_mode == "tunnel":
        add_service("cloudflared")

for kind, values in (("file", files), ("profile", profiles), ("service", services)):
    for value in values:
        sys.stdout.buffer.write(f"{kind}={value}\0".encode("utf-8"))
sys.stdout.buffer.write(f"role={role}\0".encode("utf-8"))
PY
  )

  ((${#ALVENQIS_COMPOSE_FILES[@]} > 0)) || {
    echo "Compose role mapping returned no files." >&2
    return 78
  }
  pool_enabled="${pool_enabled,,}"
  if [[ "$ALVENQIS_OPERATOR_ROLE_RESOLVED" == full-stack && "$pool_enabled" == true ]]; then
    RPC_ACCESS_MODE=internal-edge
    RPC_EXPOSE_MINING=true
  elif [[ "$ALVENQIS_OPERATOR_ROLE_RESOLVED" == pool \
    || "$ALVENQIS_OPERATOR_ROLE_RESOLVED" == stratum ]]; then
    RPC_ACCESS_MODE=private-mining
    RPC_EXPOSE_MINING=true
  else
    RPC_ACCESS_MODE=public-submit
    RPC_EXPOSE_MINING=false
  fi
  export RPC_ACCESS_MODE RPC_EXPOSE_MINING
  local compose_frontend=(docker compose)
  if [[ -n "$compose_bin_override" ]]; then
    [[ -x "$compose_bin_override" ]] || {
      echo "Compose frontend is not executable: $compose_bin_override" >&2
      return 69
    }
    compose_frontend=("$compose_bin_override")
  fi
  ALVENQIS_COMPOSE_ARGS=(
    "${compose_frontend[@]}"
    --project-directory "$workspace"
    --env-file "$dotenv_path"
  )
  local compose_file
  for compose_file in "${ALVENQIS_COMPOSE_FILES[@]}"; do
    ALVENQIS_COMPOSE_ARGS+=(-f "$compose_file")
  done
  export ALVENQIS_OPERATOR_ROLE_RESOLVED
}

compose_config_files_match() {
  local config_files="$1"
  local workspace="${ALVENQIS_WORKSPACE:-$PWD}"
  [[ "$config_files" == *"$workspace/compose/base.yaml"* ]]
}

compose_has_service() {
  local expected="$1"
  local service
  for service in "${ALVENQIS_REQUIRED_SERVICES[@]:-}"; do
    [[ "$service" == "$expected" ]] && return 0
  done
  return 1
}
