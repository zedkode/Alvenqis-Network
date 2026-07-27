#!/usr/bin/env bash
set -Eeuo pipefail

website_dir="${ALVENQIS_WEBSITE_DIR:-/opt/alvenqis/repo/Blockchain-prototype/alvenqis-website}"
runtime_name="${ALVENQIS_WEBSITE_RUNTIME_NAME:-alvenqis-website}"
container_name="${ALVENQIS_WEBSITE_CONTAINER_NAME:-alvenqis-website-runtime}"
host_port="${ALVENQIS_WEBSITE_PORT:-18081}"
core_db="${ONEPANEL_CORE_DB:-/home/1panel/1panel/db/core.db}"
agent_db="${ONEPANEL_AGENT_DB:-/home/1panel/1panel/db/agent.db}"
backup_dir="${ONEPANEL_BACKUP_DIR:-/home/1panel/1panel/db/backups}"

for required in sqlite3 jq curl openssl md5sum sort 1pctl; do
  command -v "$required" >/dev/null 2>&1 || {
    echo "ERROR: required command is missing: $required" >&2
    exit 69
  }
done
[[ -f "$website_dir/package.json" ]] || {
  echo "ERROR: website package not found at $website_dir" >&2
  exit 66
}
[[ -f "$core_db" && -f "$agent_db" ]] || {
  echo "ERROR: 1Panel databases were not found" >&2
  exit 66
}
[[ "$host_port" =~ ^[0-9]+$ ]] && ((host_port >= 1 && host_port <= 65535)) || {
  echo "ERROR: invalid website port: $host_port" >&2
  exit 64
}

tmp_dir="$(mktemp -d)"
mkdir -p "$backup_dir"
backup_path="$backup_dir/core-before-alvenqis-runtime-$(date -u +%Y%m%dT%H%M%SZ).db"
sqlite3 "$core_db" ".backup '$backup_path'"
chmod 0600 "$backup_path"

setting_hex() {
  sqlite3 "$core_db" "SELECT hex(COALESCE(value,'')) FROM settings WHERE key='$1' LIMIT 1;"
}

old_api_status_hex="$(setting_hex ApiInterfaceStatus)"
old_api_key_hex="$(setting_hex ApiKey)"
old_ip_whitelist_hex="$(setting_hex IpWhiteList)"
panel_port="$(sqlite3 "$core_db" "SELECT value FROM settings WHERE key='ServerPort' LIMIT 1;")"
[[ "$panel_port" =~ ^[0-9]+$ ]] || {
  echo "ERROR: invalid 1Panel server port" >&2
  exit 65
}

api_restored=false
restore_api_settings() {
  if [[ "$api_restored" == true ]]; then
    return
  fi
  set +e
  1pctl stop core >/dev/null 2>&1
  sqlite3 "$core_db" "
    UPDATE settings SET value=CAST(X'$old_api_status_hex' AS TEXT), updated_at=CURRENT_TIMESTAMP WHERE key='ApiInterfaceStatus';
    UPDATE settings SET value=CAST(X'$old_api_key_hex' AS TEXT), updated_at=CURRENT_TIMESTAMP WHERE key='ApiKey';
    UPDATE settings SET value=CAST(X'$old_ip_whitelist_hex' AS TEXT), updated_at=CURRENT_TIMESTAMP WHERE key='IpWhiteList';
  " >/dev/null
  1pctl start core >/dev/null 2>&1
  api_restored=true
  set -e
}
cleanup() {
  restore_api_settings
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

api_key="$(openssl rand -hex 32)"
1pctl stop core >/dev/null
sqlite3 "$core_db" "
  UPDATE settings SET value='Enable', updated_at=CURRENT_TIMESTAMP WHERE key='ApiInterfaceStatus';
  UPDATE settings SET value='$api_key', updated_at=CURRENT_TIMESTAMP WHERE key='ApiKey';
  UPDATE settings SET value='127.0.0.1', updated_at=CURRENT_TIMESTAMP WHERE key='IpWhiteList';
"
1pctl start core >/dev/null

api_base="http://127.0.0.1:${panel_port}/api/v2"
for _ in $(seq 1 30); do
  if curl -sS --max-time 2 -o /dev/null "http://127.0.0.1:${panel_port}/" 2>/dev/null; then
    break
  fi
  sleep 1
done

api_call() {
  local endpoint="$1"
  local request_file="$2"
  local response_file="$tmp_dir/response.json"
  local timestamp token http_code
  timestamp="$(date +%s)"
  token="$(printf '1panel%s%s' "$api_key" "$timestamp" | md5sum | awk '{print $1}')"
  http_code="$(curl -sS --max-time 60 \
    -o "$response_file" \
    -w '%{http_code}' \
    -H 'Content-Type: application/json' \
    -H "1Panel-Token: $token" \
    -H "1Panel-Timestamp: $timestamp" \
    --data-binary "@$request_file" \
    "$api_base$endpoint")"
  if [[ "$http_code" != 200 ]] || ! jq -e '.code == 200' "$response_file" >/dev/null 2>&1; then
    message="$(jq -r '.message // .msg // "unknown 1Panel API error"' "$response_file" 2>/dev/null || true)"
    echo "ERROR: 1Panel API request failed (HTTP $http_code): $message" >&2
    exit 70
  fi
}

node_runtime="$(
  sqlite3 -separator '|' "$agent_db" "
    SELECT d.id, d.version
    FROM app_details d
    JOIN apps a ON a.id=d.app_id
    WHERE a.key='node' AND d.status='Normal' AND d.version LIKE '24.%';
  " | sort -t'|' -k2,2V | tail -n 1
)"
app_detail_id="${node_runtime%%|*}"
node_version="${node_runtime#*|}"
[[ "$app_detail_id" =~ ^[0-9]+$ && "$node_version" == 24.* ]] || {
  echo "ERROR: no supported Node.js 24 runtime exists in the 1Panel app catalog" >&2
  exit 65
}

runtime_id="$(sqlite3 "$agent_db" "SELECT id FROM runtimes WHERE name='$runtime_name' LIMIT 1;")"
common_jq=(
  --arg name "$runtime_name"
  --arg image "1panel/node:$node_version"
  --arg version "$node_version"
  --arg codeDir "$website_dir"
  --arg container "$container_name"
  --argjson port "$host_port"
)
if [[ -n "$runtime_id" ]]; then
  jq -n "${common_jq[@]}" --argjson id "$runtime_id" '{
    id:$id,
    name:$name,
    image:$image,
    version:$version,
    rebuild:true,
    source:"https://registry.npmjs.org/",
    codeDir:$codeDir,
    remark:"Alvenqis public website managed by 1Panel",
    install:true,
    params:{
      PACKAGE_MANAGER:"npm",
      HOST_IP:"127.0.0.1",
      CUSTOM_SCRIPT:"0",
      EXEC_SCRIPT:"start",
      CONTAINER_NAME:$container,
      PANEL_APP_PORT_HTTP:$port
    },
    exposedPorts:[{hostPort:$port,containerPort:$port,hostIP:"127.0.0.1",protocol:"tcp"}],
    environments:[
      {key:"NODE_ENV",value:"production"},
      {key:"HOST",value:"0.0.0.0"},
      {key:"PORT",value:($port|tostring)},
      {key:"VITE_ALVENQIS_RPC_URL",value:"https://rpcnode.dohotstudio.com"},
      {key:"VITE_ALVENQIS_POOL_URL",value:"https://pool.dohotstudio.com"}
    ],
    volumes:[],
    extraHosts:[]
  }' > "$tmp_dir/request.json"
  api_call "/runtimes/update" "$tmp_dir/request.json"
  action="updated"
else
  task_id="$(tr -d '\r\n' </proc/sys/kernel/random/uuid)"
  jq -n "${common_jq[@]}" --argjson appDetailId "$app_detail_id" --arg taskID "$task_id" '{
    appDetailId:$appDetailId,
    name:$name,
    resource:"remote",
    image:$image,
    type:"node",
    version:$version,
    source:"https://registry.npmjs.org/",
    codeDir:$codeDir,
    remark:"Alvenqis public website managed by 1Panel",
    taskID:$taskID,
    install:true,
    params:{
      PACKAGE_MANAGER:"npm",
      HOST_IP:"127.0.0.1",
      CUSTOM_SCRIPT:"0",
      EXEC_SCRIPT:"start",
      CONTAINER_NAME:$container,
      PANEL_APP_PORT_HTTP:$port
    },
    exposedPorts:[{hostPort:$port,containerPort:$port,hostIP:"127.0.0.1",protocol:"tcp"}],
    environments:[
      {key:"NODE_ENV",value:"production"},
      {key:"HOST",value:"0.0.0.0"},
      {key:"PORT",value:($port|tostring)},
      {key:"VITE_ALVENQIS_RPC_URL",value:"https://rpcnode.dohotstudio.com"},
      {key:"VITE_ALVENQIS_POOL_URL",value:"https://pool.dohotstudio.com"}
    ],
    volumes:[],
    extraHosts:[]
  }' > "$tmp_dir/request.json"
  api_call "/runtimes" "$tmp_dir/request.json"
  action="created"
fi

restore_api_settings

for _ in $(seq 1 120); do
  if docker inspect "$container_name" >/dev/null 2>&1 &&
     curl -fsS --max-time 3 "http://127.0.0.1:${host_port}/healthz" >/dev/null 2>&1; then
    runtime_status="$(sqlite3 "$agent_db" "SELECT status FROM runtimes WHERE name='$runtime_name' LIMIT 1;")"
    echo "1Panel runtime $action: name=$runtime_name node=$node_version status=$runtime_status"
    echo "Website health: http://127.0.0.1:${host_port}/healthz"
    exit 0
  fi
  sleep 5
done

runtime_status="$(sqlite3 "$agent_db" "SELECT status FROM runtimes WHERE name='$runtime_name' LIMIT 1;")"
echo "ERROR: 1Panel runtime did not become healthy (status=$runtime_status)" >&2
exit 70
