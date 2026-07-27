#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMAND="${1:-help}"
shift || true

case "$COMMAND" in
  start) exec "$ROOT/scripts/local/start-all.sh" "$@" ;;
  stop) exec "$ROOT/scripts/local/stop-all.sh" ;;
  restart)
    "$ROOT/scripts/local/stop-all.sh"
    exec "$ROOT/scripts/local/start-all.sh" "$@"
    ;;
  status) exec "$ROOT/scripts/local/status-all.sh" ;;
  mine) exec "$ROOT/scripts/local/mine-local-block.sh" ;;
  backup) exec "$ROOT/scripts/local/backup-local-chain.sh" ;;
  validate)
    source "$ROOT/scripts/local/common.sh"
    exec 3>&1
    run_node validate-chain
    ;;
  miner-start)
    address="${1:-}"
    # $2 was threads (deprecated — continuous CPU mining removed from product)
    mode="${3:-solo}"
    pool_url="${4:-}"
    worker_name="${5:-linux-01}"
    [[ -n "$address" ]] || { echo "miner address is required" >&2; exit 2; }
    source "$ROOT/scripts/local/common.sh"
    ensure_local_directories
    miner_dir="$LOCAL_ROOT/miner"
    mkdir -p "$miner_dir"
    config="$miner_dir/config.toml"
    if [[ "$mode" == "pool" ]]; then
      [[ "$pool_url" == http://* || "$pool_url" == https://* ]] || { echo "valid pool URL is required" >&2; exit 2; }
      source_config="[source]
kind = \"pool\"
url = \"$pool_url\"
worker_name = \"$worker_name\"
timeout_seconds = 10"
    else
      source_config="[source]
kind = \"rpc\"
url = \"$RPC_URL\"
timeout_seconds = 10"
    fi
    cat >"$config" <<EOF
schema_version = 3
miner_address = "$address"
threads = 1
nonce_batch_size = 1048576
template_refresh_seconds = 5
status_interval_seconds = 10
backend_mode = "auto"
gpu_intensity = 90
kernel_validation = true
metrics_path = "$miner_dir/metrics.json"

$source_config
EOF
    if [[ "$PACKAGED" == "true" ]]; then
      command="cd $(printf %q "$ROOT") && $(printf %q "$ROOT/bin/alvenqis-miner") --config $(printf %q "$config") mine"
    else
      command="cd $(printf %q "$ROOT") && env CARGO_TARGET_DIR=$(printf %q "$BUILD_DIR") $(printf %q "$CARGO_BIN") run -p alvenqis-miner --release -- --config $(printf %q "$config") mine"
    fi
    start_background_process "miner" "$ROOT" "$command"
    ;;
  miner-stop)
    source "$ROOT/scripts/local/common.sh"
    stop_managed_process miner
    ;;
  help|--help|-h)
    echo "Usage: ./alvenqis.sh start|stop|restart|status|mine|validate|backup|miner-start|miner-stop"
    echo "Linux desktop runtime for Alvenqis Mainnet Candidate / Prototype."
    ;;
  *) echo "unknown command: $COMMAND" >&2; exit 2 ;;
esac
