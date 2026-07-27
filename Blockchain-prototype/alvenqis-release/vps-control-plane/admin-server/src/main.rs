use alvenqis_vps_admin::{router, run_agent_reporter, AdminConfig, AdminState, FleetStore};
use axum::serve;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

#[derive(Debug, Parser)]
#[command(name = "alvenqis-vps-admin")]
#[command(about = "Loopback-only Alvenqis VPS fleet control plane")]
struct Cli {
    #[arg(long, default_value = "/etc/alvenqis-control/admin.toml")]
    config: PathBuf,
    #[arg(long)]
    check_config: bool,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("alvenqis-vps-admin error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let config = AdminConfig::load(&cli.config)?;
    if cli.check_config {
        println!(
            "valid admin config={} network_id={} bind={}:{}",
            cli.config.display(),
            config.network_id,
            config.bind_host,
            config.bind_port
        );
        return Ok(());
    }
    let store = FleetStore::load(config.state_dir.clone())?;
    let state = AdminState::new(config.clone(), store)?;
    tokio::spawn(run_agent_reporter(state.clone()));
    let address: SocketAddr = format!("{}:{}", config.bind_host, config.bind_port)
        .parse()
        .map_err(|error| format!("invalid bind address: {error}"))?;
    let listener = TcpListener::bind(address)
        .await
        .map_err(|error| error.to_string())?;
    println!(
        "alvenqis-vps-admin listening on http://{}:{} ({})",
        config.bind_host, config.bind_port, config.status_label
    );
    serve(listener, router(state))
        .await
        .map_err(|error| error.to_string())
}
