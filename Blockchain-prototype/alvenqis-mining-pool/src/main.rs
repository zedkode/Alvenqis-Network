use alvenqis_mining_pool::{router, serve_stratum, PoolConfig, PoolState, PoolStore};
use axum::serve;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

#[derive(Debug, Parser)]
#[command(
    name = "alvenqis-mining-pool",
    about = "Alvenqis pooled mining coordinator prototype"
)]
struct Cli {
    #[arg(long, default_value = "alvenqis-mining-pool/config.toml")]
    config: PathBuf,
    #[arg(long)]
    check_config: bool,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("alvenqis-mining-pool error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = PoolConfig::load(&cli.config)?;
    if cli.check_config {
        println!(
            "valid pool config={} network_id={} bind={}:{}",
            cli.config.display(),
            config.network_id,
            config.bind_host,
            config.bind_port
        );
        return Ok(());
    }
    let store = PoolStore::load(config.data_dir.clone(), config.max_stored_shares)?;
    let state = PoolState::new(config.clone(), store)?;
    let address: SocketAddr = format!("{}:{}", config.bind_host, config.bind_port).parse()?;
    let listener = TcpListener::bind(address).await?;
    println!(
        "alvenqis-mining-pool listening on http://{address} ({})",
        config.status_label
    );
    let http = serve(
        listener,
        router(state.clone()).into_make_service_with_connect_info::<SocketAddr>(),
    );
    if let Some(stratum) = config.stratum {
        tokio::select! {
            result = http => result?,
            result = serve_stratum(state, stratum) => result?,
        }
    } else {
        http.await?;
    }
    Ok(())
}
