$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo run -p alvenqis-rpc-gateway -- --config alvenqis-rpc-gateway/config/devnet-rpc.toml
