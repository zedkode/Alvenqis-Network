use alvenqis_pingora_gateway::tls::build_mtls_settings;
use alvenqis_pingora_gateway::{limits::ConnectionRateFilter, routes::RatePolicy};
use alvenqis_pingora_gateway::{GatewayConfig, GatewayProxy};
use clap::Parser as _;
use log::LevelFilter;
use pingora::proxy::http_proxy_service;
use pingora::server::Server;
use pingora::services::background::background_service;
use serde_json::json;
use std::io::Write as _;

#[derive(Debug, clap::Parser)]
#[command(
    name = "alvenqis-pingora-gateway",
    version,
    about = "Project-operated Alvenqis edge gateway"
)]
struct Arguments {
    /// Validate environment, secrets, and mTLS material without opening listeners.
    #[arg(long)]
    check_config: bool,
}

fn main() {
    init_logging();

    if let Err(error) = run() {
        eprintln!("gateway startup validation failed: {error}");
        std::process::exit(1);
    }
}

fn init_logging() {
    let application_level = match std::env::var("GATEWAY_LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_owned())
        .to_ascii_lowercase()
        .as_str()
    {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };
    let mut builder = env_logger::Builder::new();
    // Pingora can log raw malformed request bytes before the application
    // callback runs. Suppress dependency logs and emit only bounded,
    // application-owned structured records.
    builder
        .filter_level(LevelFilter::Off)
        .filter_module("alvenqis_pingora_gateway", application_level)
        .format(|buffer, record| {
            let value = json!({
                "timestamp": buffer.timestamp_millis().to_string(),
                "level": record.level().as_str(),
                "target": record.target(),
                "message": record.args().to_string(),
            });
            writeln!(buffer, "{value}")
        })
        .init();
}

fn run() -> Result<(), String> {
    let arguments = Arguments::parse();
    let config = GatewayConfig::load()?;
    let mtls_settings = build_mtls_settings(&config.hosts, &config.pki)?;

    if arguments.check_config {
        println!(
            "gateway configuration valid: http={}, mtls={}, metrics={}",
            config.http_bind, config.mtls_bind, config.metrics_bind
        );
        return Ok(());
    }

    let http_bind = config.http_bind.to_string();
    let mtls_bind = config.mtls_bind.to_string();
    let metrics_bind = config.metrics_bind.to_string();
    let connection_filter = ConnectionRateFilter::new(
        config.limiter_max_keys,
        RatePolicy {
            requests_per_second: f64::from(config.connection_rate_per_second),
            burst: config.connection_burst,
            concurrent: u32::MAX,
        },
    );
    let (gateway, health_checker) = GatewayProxy::new(config);

    let mut server = Server::new_with_opt_and_conf(None, GatewayProxy::server_configuration());
    server.bootstrap();

    let mut gateway_service = http_proxy_service(&server.configuration, gateway);
    gateway_service.set_connection_filter(connection_filter);
    gateway_service.add_tcp(&http_bind);
    gateway_service.add_tls_with_settings(&mtls_bind, None, mtls_settings);
    server.add_service(gateway_service);

    let mut metrics_service = pingora_prometheus::prometheus_http_service();
    metrics_service.add_tcp(&metrics_bind);
    server.add_service(metrics_service);
    server.add_service(background_service(
        "alvenqis upstream health",
        health_checker,
    ));

    server.run_forever();
}
