use alvenqis_rpc_gateway::{router, RpcConfig, RpcState};
use axum::serve;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

const DEFAULT_RPC_CONFIG_PATH: &str = "configs/rpc.mainnet-candidate.toml";
const DEFAULT_NODE_CONFIG_PATH: &str = "configs/mainnet-candidate.toml";
const RPC_EXAMPLES: &str = "\
Examples:
  alvenqis-rpc-gateway --config configs/rpc.local.toml --node-config configs/local.toml
  alvenqis-rpc-gateway --config configs/rpc.mainnet-candidate.toml --node-config configs/mainnet-candidate.toml
";

#[derive(Debug, Parser)]
#[command(name = "alvenqis-rpc-gateway")]
#[command(about = "Mainnet Candidate RPC gateway with explicit endpoint exposure profiles")]
#[command(after_help = RPC_EXAMPLES)]
struct Cli {
    #[arg(long, default_value = DEFAULT_RPC_CONFIG_PATH)]
    config: PathBuf,
    #[arg(long, default_value = DEFAULT_NODE_CONFIG_PATH)]
    node_config: PathBuf,
    #[arg(long, default_value_t = false)]
    check_config: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = async {
        let config = RpcConfig::load_from_path(&cli.config)?;
        if cli.check_config {
            println!(
                "valid RPC config={} network_id={} access_mode={:?}",
                cli.config.display(),
                config.network_id,
                config.access_mode
            );
            return Ok::<(), alvenqis_rpc_gateway::RpcError>(());
        }
        let state = RpcState::new(config.clone()).with_node_config_path(cli.node_config.clone());
        // Warm FiroPoW light/DAG epoch cache so the first /mining/template does not
        // peg a CPU core for minutes (validator hosts have no CUDA).
        println!("alvenqis-rpc-gateway: prewarming FiroPoW epoch cache (CPU)...");
        match tokio::task::spawn_blocking(|| alvenqis_core::firopow::firopow_prewarm(0)).await {
            Ok(Ok(())) => println!("alvenqis-rpc-gateway: FiroPoW prewarm complete"),
            Ok(Err(error)) => eprintln!("alvenqis-rpc-gateway: FiroPoW prewarm failed: {error}"),
            Err(error) => eprintln!("alvenqis-rpc-gateway: FiroPoW prewarm join failed: {error}"),
        }
        let app = router(state);
        let addr: SocketAddr = format!("{}:{}", config.bind_host, config.bind_port)
            .parse::<SocketAddr>()
            .map_err(|error| alvenqis_rpc_gateway::RpcError::Config(error.to_string()))?;

        let listener = TcpListener::bind(addr).await?;
        println!(
            "alvenqis-rpc-gateway listening on http://{}:{} (config={}, node_config={}, network_id={}, {}, access_mode={:?})",
            config.bind_host,
            config.bind_port,
            cli.config.display(),
            cli.node_config.display(),
            config.network_id,
            config.status_label,
            config.access_mode
        );
        // ConnectInfo enables per-IP write rate limits (audit CR-H04).
        serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;
        Ok::<(), alvenqis_rpc_gateway::RpcError>(())
    }
    .await;

    if let Err(error) = result {
        eprintln!("alvenqis-rpc-gateway error: {error}");
        std::process::exit(1);
    }
}
