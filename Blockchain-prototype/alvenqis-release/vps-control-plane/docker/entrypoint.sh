#!/usr/bin/env bash
set -Eeuo pipefail
component="${ALVENQIS_COMPONENT:-}"
config_dir="${ALVENQIS_CONFIG_DIR:-/config}"
chain_root="${ALVENQIS_CHAIN_ROOT:-/data/.alvenqis-mainnet}"
chain_dir="$chain_root/chain"; mempool_dir="$chain_root/mempool"; index_dir="$chain_root/indexer"
# Host-prepared volume may already contain these dirs; never fail hard on mkdir under read-only roots.
mkdir -p "$config_dir" "$chain_dir" "$mempool_dir" "$index_dir" "$chain_root/node" 2>/dev/null || true
for d in "$config_dir" "$chain_dir" "$mempool_dir" "$index_dir" "$chain_root/node"; do
  if [[ ! -d "$d" ]]; then
    echo "ERROR: required data directory missing or not writable: $d" >&2
    exit 65
  fi
done
required_env(){ local n="$1"; [[ -n "${!n:-}" ]] || { echo "ERROR: missing $n" >&2; exit 64; }; }
render(){ envsubst < "$1" > "$2.tmp"; mv "$2.tmp" "$2"; }
export BASE_DOMAIN="${BASE_DOMAIN:-example.invalid}" NODE_NAME="${NODE_NAME:-alvenqis-node}" RPC_HOST="${RPC_HOST:-rpc.${BASE_DOMAIN}}" CONTROL_HOST="${CONTROL_HOST:-control.${BASE_DOMAIN}}" POOL_HOST="${POOL_HOST:-pool.${BASE_DOMAIN}}" P2P_HOST="${P2P_HOST:-node.${BASE_DOMAIN}}" SEED_NODES_TOML="${SEED_NODES_TOML:-}"
export STRATUM_HOST="${STRATUM_HOST:-stratum.${BASE_DOMAIN}}"
export STRATUM_INTERNAL_PORT="${STRATUM_INTERNAL_PORT:-3333}"
export STRATUM_PORT="${STRATUM_PORT:-3333}"
export MAX_P2P_PEERS="${MAX_P2P_PEERS:-64}"
export RPC_ACCESS_MODE="${RPC_ACCESS_MODE:-public-submit}" RPC_EXPOSE_MINING="${RPC_EXPOSE_MINING:-false}"
[[ "$MAX_P2P_PEERS" =~ ^[0-9]+$ ]] && (( MAX_P2P_PEERS >= 8 && MAX_P2P_PEERS <= 256 )) || {
  echo "ERROR: MAX_P2P_PEERS must be between 8 and 256." >&2
  exit 64
}
export CONTROLLER_URL_TOML='""'; [[ -n "${CONTROLLER_URL:-}" ]] && CONTROLLER_URL_TOML="\"${CONTROLLER_URL}\""
case "$component" in
 node)
  render /app/templates/node.toml.template "$config_dir/node.toml"
  if [[ ! -s "$chain_dir/chain.sqlite3" && ! -s "$chain_dir/chain.jsonl" ]]; then
    alvenqis-node --config "$config_dir/node.toml" --data-dir "$chain_dir" \
      import-genesis-block \
      --genesis-file /app/docs/release/genesis.mainnet-candidate.block.json
  fi
  stop_node(){ alvenqis-node --config "$config_dir/node.toml" --data-dir "$chain_dir" --mempool-dir "$mempool_dir" shutdown || true; }
  trap stop_node TERM INT
  alvenqis-node --config "$config_dir/node.toml" --data-dir "$chain_dir" --mempool-dir "$mempool_dir" start-node & child=$!; wait "$child" ;;
 rpc)
  render /app/templates/node.toml.template "$config_dir/rpc-node.toml"; render /app/templates/rpc.toml.template "$config_dir/rpc.toml"
  exec alvenqis-rpc-gateway --config "$config_dir/rpc.toml" --node-config "$config_dir/rpc-node.toml" ;;
 indexer)
  exec /usr/local/bin/alvenqis-indexer-loop ;;
 control)
  render /app/templates/admin.toml.template "$config_dir/admin.toml"
  exec alvenqis-vps-admin --config "$config_dir/admin.toml" ;;
 pool)
  required_env POOL_ADDRESS
  render /app/templates/pool.toml.template "$config_dir/pool.toml"
  exec /usr/local/bin/alvenqis-pool-supervisor "$config_dir/pool.toml" ;;
 *) echo "ERROR: invalid ALVENQIS_COMPONENT" >&2; exit 64 ;;
esac
