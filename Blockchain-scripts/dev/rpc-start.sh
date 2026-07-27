#!/usr/bin/env bash
set -euo pipefail
cargo run -p alvenqis-rpc-gateway -- --config alvenqis-rpc-gateway/config/devnet-rpc.toml
