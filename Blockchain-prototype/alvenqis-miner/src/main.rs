use alvenqis_miner::{
    default_activity_path, ActivityLog, BackendConfig, CudaMiningCoordinator, FileWorkSource,
    MinerConfig, MinerError, MiningBackend, MiningJob, MiningMode, MiningSubmitRequest,
    PoolWorkSource, Result, RpcWorkSource, StratumEndpoint, StratumWorkSource, SubmitStatus,
    WorkSource, WorkSourceConfig,
};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
#[command(
    name = "alvenqis-miner",
    version,
    about = "Alvenqis FiroPoW 0.9.4 CUDA-only GPU miner — no CPU/OpenCL mining"
)]
struct Cli {
    #[arg(long, default_value = "alvenqis-miner/config.toml")]
    config: PathBuf,
    #[arg(long, default_value_t = false)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate and print the effective miner configuration.
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Fetch and validate one work template without mining it.
    Status,
    /// List available NVIDIA CUDA mining devices.
    Devices {
        /// Filter: all | gpu | cuda
        #[arg(long, default_value = "all")]
        backend: String,
        /// Emit JSON (also accepted as global `alvenqis-miner --json devices`).
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Benchmark backends with a live or synthetic job.
    Benchmark {
        /// Device filter: all | gpu | cuda
        #[arg(long, default_value = "all")]
        device: String,
        #[arg(long, default_value_t = 5)]
        seconds: u64,
    },
    /// Mine and submit blocks until stopped, or one accepted block with --once.
    Mine {
        #[arg(long)]
        once: bool,
        /// Override config backend mode (cuda; auto/gpu are compatibility aliases).
        #[arg(long)]
        mode: Option<String>,
        #[arg(long)]
        gpu_device: Option<String>,
        #[arg(long)]
        gpu_intensity: Option<u8>,
    },
    /// Alias for mine (start mining).
    Start {
        #[arg(long)]
        once: bool,
        #[arg(long)]
        mode: Option<String>,
        #[arg(long)]
        gpu_device: Option<String>,
        #[arg(long)]
        gpu_intensity: Option<u8>,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigAction {
    /// Print effective configuration (default).
    Show,
    /// Validate configuration file only.
    Validate,
}

#[derive(Serialize)]
struct StatusOutput {
    status: &'static str,
    source: String,
    network_id: String,
    template_id: String,
    height: u64,
    difficulty_leading_zero_bits: u8,
    expires_at_unix_seconds: u64,
    backend_mode: String,
    cuda_kernels_built: bool,
}

#[derive(Serialize)]
struct MiningMetrics<'a> {
    status: &'a str,
    network_id: &'a str,
    template_id: &'a str,
    height: u64,
    /// Network / full-block difficulty (leading zero bits).
    difficulty_leading_zero_bits: u8,
    /// Share target for pool (or same as network for solo).
    share_difficulty_leading_zero_bits: u8,
    /// Estimated seconds to next share at current hashrate (0 if unknown).
    eta_share_seconds: f64,
    /// Estimated seconds to next full block at current hashrate (0 if unknown).
    eta_block_seconds: f64,
    backend_mode: &'a str,
    active_backend: &'a str,
    hashrate_hs: f64,
    hashes_attempted: u64,
    accepted_blocks: u64,
    accepted_shares: u64,
    rejected_local: u64,
    stale: u64,
    gpu_devices: usize,
    last_error: Option<&'a str>,
    work_source: &'a str,
    updated_at_unix_seconds: u64,
}

fn eta_seconds(hashrate_hs: f64, leading_zero_bits: u8) -> f64 {
    if hashrate_hs <= 1.0 || leading_zero_bits >= 63 {
        return 0.0;
    }
    let expected_hashes = 2_f64.powi(i32::from(leading_zero_bits));
    expected_hashes / hashrate_hs
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Devices { backend, json } => {
            // Devices does not require a full config file.
            // Accept both `alvenqis-miner --json devices` and `alvenqis-miner devices --json`.
            devices(backend, cli.json || *json)
        }
        Command::Config { action } => {
            let config = MinerConfig::load(&cli.config)?;
            match action.as_ref().unwrap_or(&ConfigAction::Show) {
                ConfigAction::Show => {
                    println!("{}", serde_json::to_string_pretty(&config)?);
                    Ok(())
                }
                ConfigAction::Validate => {
                    config.validate()?;
                    if cli.json {
                        println!(
                            r#"{{"status":"valid","schema_version":{}}}"#,
                            config.schema_version
                        );
                    } else {
                        println!(
                            "configuration valid (schema_version={})",
                            config.schema_version
                        );
                    }
                    Ok(())
                }
            }
        }
        Command::Status => {
            let config = MinerConfig::load(&cli.config)?;
            status(&config)
        }
        Command::Benchmark { device, seconds } => {
            let config = MinerConfig::load(&cli.config).unwrap_or_else(|_| synthetic_config());
            benchmark(&config, device, *seconds, cli.json)
        }
        Command::Mine {
            once,
            mode,
            gpu_device,
            gpu_intensity,
        }
        | Command::Start {
            once,
            mode,
            gpu_device,
            gpu_intensity,
        } => {
            let mut config = MinerConfig::load(&cli.config)?;
            if let Some(mode) = mode {
                MiningMode::parse(mode)?;
                config.backend_mode = mode.clone();
            }
            if let Some(device) = gpu_device {
                config.gpu_devices = vec![device.clone()];
            }
            if let Some(intensity) = gpu_intensity {
                config.gpu_intensity = *intensity;
            }
            config.validate()?;
            mine(&config, *once)
        }
    }
}

fn cuda_feature_enabled() -> bool {
    cfg!(feature = "gpu-cuda")
}

fn devices(backend_filter: &str, json: bool) -> Result<()> {
    let filter = backend_filter.to_ascii_lowercase();
    let report = alvenqis_miner::enumerate_device_report();
    let filtered: Vec<_> = report
        .devices
        .into_iter()
        .filter(|d| match filter.as_str() {
            "all" => true,
            "cpu" | "opencl" => false,
            "gpu" => matches!(d.backend, alvenqis_miner::BackendId::GpuCuda),
            "cuda" | "nvidia" => matches!(d.backend, alvenqis_miner::BackendId::GpuCuda),
            _ => true,
        })
        .collect();
    if json {
        // Bare array keeps Control Center / scripting stable.
        println!("{}", serde_json::to_string_pretty(&filtered)?);
    } else if filtered.is_empty() {
        println!("No mining devices matched filter '{backend_filter}'.");
        if !cuda_feature_enabled() {
            println!("Note: this binary was built without CUDA kernels.");
        }
        for note in &report.notes {
            eprintln!("note: {note}");
        }
    } else {
        for d in &filtered {
            println!(
                "[{:?}] {} — {} ({}) cu={:?} mem={:?}",
                d.backend, d.id, d.name, d.vendor, d.compute_units, d.global_memory_bytes
            );
        }
        for note in &report.notes {
            eprintln!("note: {note}");
        }
    }
    Ok(())
}

fn status(config: &MinerConfig) -> Result<()> {
    let source = make_source(config)?;
    let template = source.fetch_template(&config.miner_address)?;
    source.validate_and_build(&template, &config.miner_address)?;
    let output = StatusOutput {
        status: "ready",
        source: source.description(),
        network_id: template.network_id,
        template_id: template.template_id,
        height: template.height,
        difficulty_leading_zero_bits: template.difficulty_leading_zero_bits,
        expires_at_unix_seconds: template.expires_at_unix_seconds,
        backend_mode: config.backend_mode.clone(),
        cuda_kernels_built: cuda_feature_enabled(),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn synthetic_config() -> MinerConfig {
    use alvenqis_core::{Address, Network, PrivateKey};
    // Always generate a valid Alvenqis address (never legacy foreign HRPs).
    let miner_address =
        Address::from_public_key_for_network(&PrivateKey::generate().public_key(), Network::Devnet)
            .to_string();
    MinerConfig {
        schema_version: alvenqis_miner::MINER_CONFIG_SCHEMA_VERSION,
        miner_address,
        nonce_batch_size: 50_000,
        template_refresh_seconds: 5,
        status_interval_seconds: 10,
        metrics_path: None,
        activity_log_path: None,
        backend_mode: "cuda".into(),
        gpu_intensity: 75,
        gpu_batch_size: 0,
        gpu_devices: Vec::new(),
        kernel_validation: true,
        source: WorkSourceConfig::LocalFile {
            template_path: PathBuf::from("template.json"),
            submission_path: PathBuf::from("submit.json"),
        },
    }
}

fn make_job_from_template(source: &dyn WorkSource, config: &MinerConfig) -> Result<MiningJob> {
    let template = source.fetch_template(&config.miner_address)?;
    let block = source.validate_and_build(&template, &config.miner_address)?;
    Ok(MiningJob {
        template_id: template.template_id,
        difficulty_leading_zero_bits: template.difficulty_leading_zero_bits,
        start_nonce: template.nonce_start,
        max_nonces: config.nonce_batch_size,
        block,
    })
}

fn synthetic_job() -> MiningJob {
    use alvenqis_core::{Amount, Hash, Network, PrivateKey, Transaction};
    let address = alvenqis_core::Address::from_public_key_for_network(
        &PrivateKey::generate().public_key(),
        Network::Devnet,
    )
    .to_string();
    let transaction = Transaction::coinbase(1, address, Amount::from_atomic(1)).expect("coinbase");
    let block =
        alvenqis_core::Block::new(Network::Devnet, 1, Hash::zero(), 0, 1, 0, vec![transaction])
            .expect("block");
    MiningJob {
        template_id: "benchmark".into(),
        block,
        difficulty_leading_zero_bits: 255,
        start_nonce: 0,
        max_nonces: 50_000,
    }
}

fn benchmark(config: &MinerConfig, device: &str, seconds: u64, json: bool) -> Result<()> {
    let duration = Duration::from_secs(seconds.max(1));
    let job = match make_source(config).and_then(|s| make_job_from_template(s.as_ref(), config)) {
        Ok(job) => job,
        Err(_) => synthetic_job(),
    };
    let mut results = Vec::new();
    let filter = device.to_ascii_lowercase();

    if filter == "cpu" {
        return Err(MinerError::Config(
            "CPU/OpenCL mining is disabled; use cuda".into(),
        ));
    }

    if matches!(
        filter.as_str(),
        "all" | "gpu" | "cuda" | "combined" | "auto"
    ) || filter.starts_with("cuda")
    {
        #[cfg(feature = "gpu-cuda")]
        {
            let mut cuda = alvenqis_miner::CudaGpuBackend::default();
            match cuda.initialize(BackendConfig {
                batch_size: config.effective_gpu_batch(),
                device_ids: config.gpu_devices.clone(),
                intensity: config.effective_gpu_intensity(),
            }) {
                Ok(()) => match cuda.benchmark(&job, duration) {
                    Ok(r) => results.push(r),
                    Err(e) => eprintln!("cuda benchmark error: {e}"),
                },
                Err(e) => eprintln!("cuda init skipped: {e}"),
            }
        }
        #[cfg(not(feature = "gpu-cuda"))]
        {
            eprintln!("cuda benchmark unavailable: binary built without gpu-cuda");
        }
    }

    if filter.starts_with("opencl") {
        return Err(MinerError::Config(
            "OpenCL mining was removed because it did not execute FiroPoW on the GPU".into(),
        ));
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for r in &results {
            println!(
                "[{:?}] {} — {:.0} H/s over {} ms (hashes={} errors={})",
                r.backend,
                r.device_id,
                r.hashrate_hs,
                r.duration_ms,
                r.hashes,
                r.errors.len()
            );
            for e in &r.errors {
                eprintln!("  error: {e}");
            }
        }
    }
    Ok(())
}

fn build_backend(config: &MinerConfig) -> Result<Box<dyn MiningBackend>> {
    let mode = MiningMode::parse(&config.backend_mode)?;
    let gpu_batch_size = config.effective_gpu_batch();

    match mode {
        MiningMode::Gpu | MiningMode::Cuda | MiningMode::Auto => {
            let mut cuda = CudaMiningCoordinator::new(mode);
            cuda.initialize(BackendConfig {
                batch_size: gpu_batch_size.max(256),
                device_ids: config.gpu_devices.clone(),
                intensity: config.effective_gpu_intensity(),
            })?;
            Ok(Box::new(cuda))
        }
    }
}

fn mine(config: &MinerConfig, once: bool) -> Result<()> {
    let activity_path = config
        .activity_log_path
        .clone()
        .unwrap_or_else(|| default_activity_path(config.metrics_path.as_deref(), Path::new(".")));
    // Prefer metrics parent when metrics_path is set (stable Control Center layout).
    let activity_path = if config.activity_log_path.is_none() {
        default_activity_path(config.metrics_path.as_deref(), Path::new("."))
    } else {
        activity_path
    };
    let log = ActivityLog::open(Some(activity_path.clone()))?;
    log.info(
        "miner_start",
        format!(
            "activity_log={} metrics={:?} backend={} batch={} once={once} address={}",
            activity_path.display(),
            config.metrics_path,
            config.backend_mode,
            config.effective_nonce_batch(),
            config.miner_address
        ),
    );

    let source = make_source(config)?;
    log.info("work_source", source.description());
    let mut backend = build_backend(config)?;
    log.info(
        "backend_ready",
        format!(
            "mode={} name={} gpu_feature={}",
            config.backend_mode,
            backend.backend_name(),
            cuda_feature_enabled()
        ),
    );
    eprintln!(
        "mining backend={} name={} gpu_feature={}",
        config.backend_mode,
        backend.backend_name(),
        cuda_feature_enabled()
    );

    let mut current_template_id = String::new();
    let mut work_fingerprint = String::new();
    let mut nonce_cursor = 0_u64;
    let mut last_template_fetch = Instant::now()
        .checked_sub(Duration::from_secs(config.template_refresh_seconds))
        .unwrap_or_else(Instant::now);
    let mut last_status = Instant::now();
    let mut template = None;
    let mut total_hashes = 0_u64;
    let mut accepted_blocks = 0_u64;
    let mut accepted_shares = 0_u64;
    let mut rejected_local = 0_u64;
    let mut stale_count = 0_u64;
    // Exponential moving average — live hashrate, not cumulative lifetime average.
    let mut hashrate_ema = 0.0_f64;
    let mut window_hashes = 0_u64;
    let mut window_started = Instant::now();
    let mut last_share_error: Option<String> = None;
    let mut force_template_refresh = false;
    let work_source_desc = source.description();
    let is_pool = work_source_desc.starts_with("pool ");

    // Reset metrics immediately so UI does not show leftover "mining" from a previous run.
    let _ = write_metrics(
        config,
        MiningMetrics {
            status: "starting",
            network_id: "",
            template_id: "",
            height: 0,
            difficulty_leading_zero_bits: 0,
            share_difficulty_leading_zero_bits: 0,
            eta_share_seconds: 0.0,
            eta_block_seconds: 0.0,
            backend_mode: &config.backend_mode,
            active_backend: backend.backend_name(),
            hashrate_hs: 0.0,
            hashes_attempted: 0,
            accepted_blocks: 0,
            accepted_shares: 0,
            rejected_local: 0,
            stale: 0,
            gpu_devices: 0,
            last_error: None,
            work_source: &work_source_desc,
            updated_at_unix_seconds: unix_now(),
        },
    );

    loop {
        if template.is_none()
            || force_template_refresh
            || last_template_fetch.elapsed() >= Duration::from_secs(config.template_refresh_seconds)
        {
            force_template_refresh = false;
            log.trace_action("template_fetch_begin", source.description());
            match source.fetch_template(&config.miner_address) {
                Ok(next) => {
                    // Soft-fail validation so a bad/foreign template never kills the process.
                    match source.validate_and_build(&next, &config.miner_address) {
                        Ok(block) => {
                            // Work identity: tip + merkle + difficulty + timestamp (PoW preimage).
                            // template_id alone changes on every poll if the gateway does not sticky-cache.
                            let fingerprint = format!(
                                "{}:{}:{}:{}:{}",
                                next.height,
                                next.previous_hash,
                                next.merkle_root,
                                next.difficulty_leading_zero_bits,
                                next.timestamp
                            );
                            let work_changed = fingerprint != work_fingerprint;
                            let share_bits = next
                                .share_difficulty_leading_zero_bits
                                .unwrap_or(next.difficulty_leading_zero_bits)
                                .min(next.difficulty_leading_zero_bits);
                            if work_changed {
                                work_fingerprint = fingerprint;
                                nonce_cursor = next.nonce_start;
                                log.info(
                                    "template_switch",
                                    format!(
                                        "template={} height={} net_diff={} share_diff={} nonce_start={} source={}",
                                        next.template_id,
                                        next.height,
                                        next.difficulty_leading_zero_bits,
                                        share_bits,
                                        next.nonce_start,
                                        work_source_desc
                                    ),
                                );
                                eprintln!(
                                    "template={} height={} net_diff={} share_diff={} source={}",
                                    next.template_id,
                                    next.height,
                                    next.difficulty_leading_zero_bits,
                                    share_bits,
                                    work_source_desc
                                );
                                if !is_pool && next.difficulty_leading_zero_bits >= 28 {
                                    eprintln!(
                                        "WARNING: solo mining at net_diff={} (~2^{} hashes/block). Prefer pool mode for shares (share_diff often ~18). ETA at 100 MH/s ≈ {:.0}s.",
                                        next.difficulty_leading_zero_bits,
                                        next.difficulty_leading_zero_bits,
                                        eta_seconds(100_000_000.0, next.difficulty_leading_zero_bits)
                                    );
                                }
                            } else if next.template_id != current_template_id {
                                // Same work, new id (gateway re-issued): keep nonce progress, update id for submit.
                                log.debug(
                                    "template_id_rotated",
                                    format!("{} -> {}", current_template_id, next.template_id),
                                );
                                eprintln!(
                                    "template id rotated (work unchanged) {} -> {}",
                                    current_template_id, next.template_id
                                );
                            }
                            current_template_id.clone_from(&next.template_id);
                            let tip_share_bits = next
                                .share_difficulty_leading_zero_bits
                                .unwrap_or(next.difficulty_leading_zero_bits)
                                .min(next.difficulty_leading_zero_bits);
                            let tip_net_diff = next.difficulty_leading_zero_bits;
                            let tip_height = next.height;
                            let tip_network_id = next.network_id.clone();
                            template = Some((next, block));
                            last_template_fetch = Instant::now();
                            last_share_error = None;
                            // Publish telemetry immediately so the Control Center shows live work
                            // (height/template) even while the first long FiroPoW batch runs.
                            let status_label = if total_hashes == 0 {
                                "building_dag"
                            } else {
                                "mining"
                            };
                            let _ = write_metrics(
                                config,
                                MiningMetrics {
                                    status: status_label,
                                    network_id: &tip_network_id,
                                    template_id: &current_template_id,
                                    height: tip_height,
                                    difficulty_leading_zero_bits: tip_net_diff,
                                    share_difficulty_leading_zero_bits: tip_share_bits,
                                    eta_share_seconds: eta_seconds(hashrate_ema, tip_share_bits),
                                    eta_block_seconds: eta_seconds(hashrate_ema, tip_net_diff),
                                    backend_mode: &config.backend_mode,
                                    active_backend: backend.backend_name(),
                                    hashrate_hs: hashrate_ema,
                                    hashes_attempted: total_hashes,
                                    accepted_blocks,
                                    accepted_shares,
                                    rejected_local,
                                    stale: stale_count,
                                    gpu_devices: backend.metrics().active_devices,
                                    last_error: if total_hashes == 0 {
                                        Some("warming FiroPoW DAG / first GPU batch")
                                    } else {
                                        None
                                    },
                                    work_source: &work_source_desc,
                                    updated_at_unix_seconds: unix_now(),
                                },
                            );
                        }
                        Err(error) => {
                            let err = error.to_string();
                            log.error("template_invalid", &err);
                            eprintln!("template invalid (retrying): {err}");
                            last_share_error = Some(err.clone());
                            let keep_mining = template.is_some();
                            let status = if keep_mining {
                                "refreshing_work"
                            } else {
                                "waiting_work"
                            };
                            let _ = write_metrics(
                                config,
                                MiningMetrics {
                                    status,
                                    network_id: &next.network_id,
                                    template_id: &next.template_id,
                                    height: next.height,
                                    difficulty_leading_zero_bits: next.difficulty_leading_zero_bits,
                                    share_difficulty_leading_zero_bits: next
                                        .share_difficulty_leading_zero_bits
                                        .unwrap_or(next.difficulty_leading_zero_bits),
                                    eta_share_seconds: 0.0,
                                    eta_block_seconds: 0.0,
                                    backend_mode: &config.backend_mode,
                                    active_backend: backend.backend_name(),
                                    hashrate_hs: if keep_mining { hashrate_ema } else { 0.0 },
                                    hashes_attempted: total_hashes,
                                    accepted_blocks,
                                    accepted_shares,
                                    rejected_local,
                                    stale: stale_count,
                                    gpu_devices: backend.metrics().active_devices,
                                    last_error: Some(&err),
                                    work_source: &work_source_desc,
                                    updated_at_unix_seconds: unix_now(),
                                },
                            );
                            if keep_mining {
                                // Keep hashing previous valid job; retry refresh later.
                                last_template_fetch = Instant::now();
                            } else {
                                thread::sleep(Duration::from_secs(2));
                                last_template_fetch = Instant::now()
                                    .checked_sub(Duration::from_secs(
                                        config.template_refresh_seconds,
                                    ))
                                    .unwrap_or_else(Instant::now);
                                continue;
                            }
                        }
                    }
                }
                Err(error) if template.is_some() => {
                    log.warn("template_refresh_failed", error.to_string());
                    eprintln!("template refresh failed: {error}");
                    last_template_fetch = Instant::now();
                }
                Err(error) => {
                    // Startup / RPC restarts: keep retrying instead of exiting the process.
                    log.error("template_fetch_failed", error.to_string());
                    eprintln!("template fetch failed (retrying): {error}");
                    let err = error.to_string();
                    let _ = write_metrics(
                        config,
                        MiningMetrics {
                            status: "waiting_work",
                            network_id: "",
                            template_id: "",
                            height: 0,
                            difficulty_leading_zero_bits: 0,
                            share_difficulty_leading_zero_bits: 0,
                            eta_share_seconds: 0.0,
                            eta_block_seconds: 0.0,
                            backend_mode: &config.backend_mode,
                            active_backend: backend.backend_name(),
                            hashrate_hs: 0.0,
                            hashes_attempted: total_hashes,
                            accepted_blocks,
                            accepted_shares,
                            rejected_local,
                            stale: stale_count,
                            gpu_devices: backend.metrics().active_devices,
                            last_error: Some(&err),
                            work_source: &work_source_desc,
                            updated_at_unix_seconds: unix_now(),
                        },
                    );
                    thread::sleep(Duration::from_secs(2));
                    last_template_fetch = Instant::now()
                        .checked_sub(Duration::from_secs(config.template_refresh_seconds))
                        .unwrap_or_else(Instant::now);
                    continue;
                }
            }
        }

        let Some((active_template, block)) = template.as_ref() else {
            // Should only happen if template fetch left None; retry without panicking.
            force_template_refresh = true;
            thread::sleep(Duration::from_millis(200));
            continue;
        };
        let batch_started = Instant::now();
        let share_difficulty = active_template
            .share_difficulty_leading_zero_bits
            .unwrap_or(active_template.difficulty_leading_zero_bits)
            .min(active_template.difficulty_leading_zero_bits);

        let lease = config.effective_nonce_batch();
        let job = MiningJob {
            template_id: active_template.template_id.clone(),
            block: block.clone(),
            difficulty_leading_zero_bits: share_difficulty,
            start_nonce: nonce_cursor,
            max_nonces: lease,
        };

        log.trace_action(
            "batch_begin",
            format!(
                "template={} height={} share_diff={} nonce={} backend={}",
                active_template.template_id,
                active_template.height,
                share_difficulty,
                nonce_cursor,
                backend.backend_name()
            ),
        );
        // Publish a heartbeat *before* the (potentially long) FiroPoW batch so the
        // Control Center is never stuck at 0 while the first epoch DAG builds on CPU.
        {
            let net_diff = active_template.difficulty_leading_zero_bits;
            let status_label = if total_hashes == 0 {
                "building_dag"
            } else {
                "mining"
            };
            let _ = write_metrics(
                config,
                MiningMetrics {
                    status: status_label,
                    network_id: &active_template.network_id,
                    template_id: &current_template_id,
                    height: active_template.height,
                    difficulty_leading_zero_bits: net_diff,
                    share_difficulty_leading_zero_bits: share_difficulty,
                    eta_share_seconds: eta_seconds(hashrate_ema, share_difficulty),
                    eta_block_seconds: eta_seconds(hashrate_ema, net_diff),
                    backend_mode: &config.backend_mode,
                    active_backend: backend.backend_name(),
                    hashrate_hs: hashrate_ema,
                    hashes_attempted: total_hashes,
                    accepted_blocks,
                    accepted_shares,
                    rejected_local,
                    stale: stale_count,
                    gpu_devices: backend.metrics().active_devices,
                    last_error: if total_hashes == 0 {
                        Some("warming FiroPoW DAG / first GPU batch")
                    } else {
                        last_share_error.as_deref()
                    },
                    work_source: &work_source_desc,
                    updated_at_unix_seconds: unix_now(),
                },
            );
        }
        let result = match backend.mine_batch(&job) {
            Ok(result) => result,
            Err(error) => {
                // Soft-fail GPU/kernel errors so one bad dispatch does not kill the miner.
                let err = error.to_string();
                log.error("batch_failed", &err);
                eprintln!("CUDA batch failed (retrying): {err}");
                last_share_error = Some(err.clone());
                let net_diff = active_template.difficulty_leading_zero_bits;
                let _ = write_metrics(
                    config,
                    MiningMetrics {
                        status: "gpu_error",
                        network_id: &active_template.network_id,
                        template_id: &current_template_id,
                        height: active_template.height,
                        difficulty_leading_zero_bits: net_diff,
                        share_difficulty_leading_zero_bits: share_difficulty,
                        eta_share_seconds: 0.0,
                        eta_block_seconds: 0.0,
                        backend_mode: &config.backend_mode,
                        active_backend: backend.backend_name(),
                        hashrate_hs: hashrate_ema,
                        hashes_attempted: total_hashes,
                        accepted_blocks,
                        accepted_shares,
                        rejected_local,
                        stale: stale_count,
                        gpu_devices: backend.metrics().active_devices,
                        last_error: Some(&err),
                        work_source: &work_source_desc,
                        updated_at_unix_seconds: unix_now(),
                    },
                );
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };
        let metrics = backend.metrics();
        // Prefer exact CUDA work counter; fall back to lease so hashrate never sticks at 0
        // after a successful dispatch that under-reports atomic counters.
        let advance = lease.max(1);
        let delta = metrics.hashes_attempted.saturating_sub(total_hashes);
        let batch_hashes = if delta > 0 {
            delta.min(advance.saturating_mul(2)).max(1)
        } else if metrics.hashrate_hs > 0.0 {
            // Backend has a live rate but counter not yet aligned — estimate from rate.
            let secs = batch_started.elapsed().as_secs_f64().max(0.001);
            ((metrics.hashrate_hs * secs) as u64).clamp(1, advance)
        } else {
            // Successful empty report: count the full lease so UI shows real work.
            advance
        };
        total_hashes = total_hashes.saturating_add(batch_hashes);
        window_hashes = window_hashes.saturating_add(batch_hashes);

        // Instant batch rate + short rolling window + EMA → stable live hashrate.
        let batch_secs = batch_started.elapsed().as_secs_f64().max(0.001);
        let batch_rate = metrics
            .hashrate_hs
            .max(batch_hashes as f64 / batch_secs)
            .max(1.0);
        let window_secs = window_started.elapsed().as_secs_f64().max(0.001);
        let window_rate = window_hashes as f64 / window_secs;
        // Reset rolling window every ~10s so the displayed rate stays live.
        if window_secs >= 10.0 {
            window_hashes = 0;
            window_started = Instant::now();
        }
        let sample = batch_rate.max(window_rate * 0.5);
        hashrate_ema = if hashrate_ema <= 0.0 {
            sample
        } else {
            // Fast EMA (~2–3 samples half-life) so UI tracks live rate without lifetime growth.
            0.45 * sample + 0.55 * hashrate_ema
        };
        let hashrate = hashrate_ema;
        // Contiguous non-overlapping cursor: next batch starts exactly after this lease.
        nonce_cursor = nonce_cursor.saturating_add(advance);
        rejected_local = rejected_local.max(metrics.rejected_local);
        stale_count = stale_count.max(metrics.stale);
        // Clear only the temporary DAG warm-up message after real work starts.
        if total_hashes > 0 {
            if let Some(err) = last_share_error.as_deref() {
                if err.contains("warming") || err.contains("DAG") {
                    last_share_error = None;
                }
            }
        }
        log.trace_action(
            "batch_end",
            format!(
                "hashes={batch_hashes} hashrate_hs={hashrate:.0} total={total_hashes} next_nonce={nonce_cursor} found={}",
                result.is_some()
            ),
        );

        // Always refresh metrics after each batch so the UI hashrate is live even when
        // status_interval is longer than a single FiroPoW batch (or batch is long).
        {
            let net_diff = active_template.difficulty_leading_zero_bits;
            let eta_share = eta_seconds(hashrate, share_difficulty);
            let eta_block = eta_seconds(hashrate, net_diff);
            let status_label = if last_share_error
                .as_deref()
                .is_some_and(|e| e.contains("stale") || e.contains("expired"))
            {
                "refreshing_work"
            } else {
                "mining"
            };
            let combined_error = last_share_error
                .as_deref()
                .or(metrics.last_error.as_deref());
            let _ = write_metrics(
                config,
                MiningMetrics {
                    status: status_label,
                    network_id: &active_template.network_id,
                    template_id: &current_template_id,
                    height: active_template.height,
                    difficulty_leading_zero_bits: net_diff,
                    share_difficulty_leading_zero_bits: share_difficulty,
                    eta_share_seconds: eta_share,
                    eta_block_seconds: eta_block,
                    backend_mode: &config.backend_mode,
                    active_backend: backend.backend_name(),
                    hashrate_hs: hashrate,
                    hashes_attempted: total_hashes,
                    accepted_blocks,
                    accepted_shares,
                    rejected_local,
                    stale: stale_count,
                    gpu_devices: metrics.active_devices,
                    last_error: combined_error,
                    work_source: &work_source_desc,
                    updated_at_unix_seconds: unix_now(),
                },
            );
        }

        if let Some(solution) = result {
            // Always advance past the found nonce so the next batch cannot re-submit it.
            let past_solution = solution.nonce.wrapping_add(1);
            if past_solution > nonce_cursor
                || nonce_cursor.wrapping_sub(past_solution) > (1u64 << 62)
            {
                // Normal case: solution is behind/at cursor window; push cursor past it.
                // Also handle wrap when past_solution wraps below cursor near u64::MAX.
                nonce_cursor = past_solution;
            }

            // Extra core validation before submit (FiroPoW final + mix).
            let core_out = match block.pow_hash_with_nonce(solution.nonce) {
                Ok(o) => o,
                Err(_) => {
                    rejected_local = rejected_local.saturating_add(1);
                    continue;
                }
            };
            log.info(
                "solution_found",
                format!(
                    "nonce={} final={} mix={} share_diff={}",
                    solution.nonce,
                    alvenqis_core::hash_to_hex(&solution.final_hash),
                    alvenqis_core::hash_to_hex(&solution.mix_hash),
                    share_difficulty
                ),
            );
            if core_out.final_hash != solution.final_hash
                || core_out.mix_hash != solution.mix_hash
                || !alvenqis_core::check_pow(&core_out.final_hash, share_difficulty)
            {
                log.warn(
                    "solution_rejected_local",
                    format!("nonce={} core FiroPoW validation failed", solution.nonce),
                );
                eprintln!(
                    "rejecting local solution nonce={} (FiroPoW validation failed)",
                    solution.nonce
                );
                rejected_local = rejected_local.saturating_add(1);
            } else {
                let request = MiningSubmitRequest::from_solution(
                    active_template.template_id.clone(),
                    solution.nonce,
                    solution.final_hash,
                    solution.mix_hash,
                );
                log.trace_action(
                    "submit_begin",
                    format!(
                        "template={} nonce={} hash={}",
                        request.template_id, request.nonce, request.block_hash
                    ),
                );
                match source.submit(&request) {
                    Ok(response) => {
                        log.info(
                            "submit_response",
                            format!(
                                "status={:?} reason={:?} height={:?}",
                                response.status, response.reason, response.height
                            ),
                        );
                        // Compact stdout: pretty JSON every share floods miner.log + panel.
                        match response.status {
                            SubmitStatus::Accepted => {
                                println!(
                                    "accepted block height={:?} hash={}",
                                    response.height, response.block_hash
                                );
                            }
                            SubmitStatus::PendingLocal => {
                                // Count only; periodic status line reports share totals.
                            }
                            other => {
                                println!("submit status={other:?} reason={:?}", response.reason);
                            }
                        }
                        if response.status == SubmitStatus::Accepted {
                            accepted_blocks = accepted_blocks.saturating_add(1);
                            accepted_shares = accepted_shares.saturating_add(1);
                        }
                        if response.status == SubmitStatus::PendingLocal {
                            accepted_shares = accepted_shares.saturating_add(1);
                        }
                        match response.status {
                            SubmitStatus::Accepted if once => return Ok(()),
                            SubmitStatus::PendingLocal if once => return Ok(()),
                            // Share accepted by pool (or already recorded) — keep mining.
                            SubmitStatus::PendingLocal => {
                                last_share_error = None;
                            }
                            SubmitStatus::Accepted => {
                                last_share_error = None;
                                template = None;
                                force_template_refresh = true;
                                thread::sleep(Duration::from_millis(250));
                                continue;
                            }
                            SubmitStatus::Stale => {
                                stale_count = stale_count.saturating_add(1);
                                last_share_error = response
                                    .reason
                                    .clone()
                                    .or_else(|| Some("stale work".into()));
                                eprintln!(
                                    "share stale — refreshing work ({})",
                                    last_share_error.as_deref().unwrap_or("stale")
                                );
                                template = None;
                                force_template_refresh = true;
                                thread::sleep(Duration::from_millis(200));
                                continue;
                            }
                            _ => {
                                last_share_error = response.reason.clone();
                                template = None;
                                force_template_refresh = true;
                                thread::sleep(Duration::from_millis(250));
                                continue;
                            }
                        }
                    }
                    // Soft-fail pool/RPC share errors so one bad share does not kill the miner.
                    Err(error) => {
                        let msg = error.to_string();
                        log.error("submit_failed", &msg);
                        eprintln!("share submit failed (continuing): {msg}");
                        rejected_local = rejected_local.saturating_add(1);
                        last_share_error = Some(msg.clone());
                        if msg.contains("stale")
                            || msg.contains("expired")
                            || msg.contains("unknown job")
                            || msg.contains("409")
                            || msg.contains("Conflict")
                        {
                            stale_count = stale_count.saturating_add(1);
                            template = None;
                            force_template_refresh = true;
                            thread::sleep(Duration::from_millis(200));
                            continue;
                        }
                    }
                }
            }
        }

        if last_status.elapsed() >= Duration::from_secs(config.status_interval_seconds) {
            let backend_error = metrics.last_error.clone();
            let status_label = if last_share_error
                .as_deref()
                .is_some_and(|e| e.contains("stale") || e.contains("expired"))
            {
                "refreshing_work"
            } else {
                "mining"
            };
            let net_diff = active_template.difficulty_leading_zero_bits;
            let eta_share = eta_seconds(hashrate, share_difficulty);
            let eta_block = eta_seconds(hashrate, net_diff);
            log.info(
                "status",
                format!(
                    "source={} mode={} template={} height={} net_diff={net_diff} share_diff={share_difficulty} next_nonce={nonce_cursor} hashrate_hs={hashrate:.0} eta_share_s={eta_share:.1} eta_block_s={eta_block:.0} total_hashes={total_hashes} shares={accepted_shares} blocks={accepted_blocks} stale={stale_count} devices={}",
                    work_source_desc,
                    config.backend_mode,
                    current_template_id,
                    active_template.height,
                    metrics.active_devices
                ),
            );
            eprintln!(
                "mining source={} net_diff={net_diff} share_diff={share_difficulty} hashrate={hashrate:.0} H/s eta_share={eta_share:.1}s eta_block={eta_block:.0}s shares={accepted_shares} blocks={accepted_blocks} stale={stale_count}",
                work_source_desc
            );
            let combined_error = last_share_error.as_deref().or(backend_error.as_deref());
            write_metrics(
                config,
                MiningMetrics {
                    status: status_label,
                    network_id: &active_template.network_id,
                    template_id: &current_template_id,
                    height: active_template.height,
                    difficulty_leading_zero_bits: net_diff,
                    share_difficulty_leading_zero_bits: share_difficulty,
                    eta_share_seconds: eta_share,
                    eta_block_seconds: eta_block,
                    backend_mode: &config.backend_mode,
                    active_backend: backend.backend_name(),
                    hashrate_hs: hashrate,
                    hashes_attempted: total_hashes,
                    accepted_blocks,
                    accepted_shares,
                    rejected_local,
                    stale: stale_count,
                    // Exact selected GPU count (never subtract 1 — that zeroed single-GPU UI).
                    gpu_devices: metrics.active_devices,
                    last_error: combined_error,
                    work_source: &work_source_desc,
                    updated_at_unix_seconds: unix_now(),
                },
            )?;
            last_status = Instant::now();
        }
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn write_metrics(config: &MinerConfig, metrics: MiningMetrics<'_>) -> Result<()> {
    let Some(path) = &config.metrics_path else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = atomic_write_file::AtomicWriteFile::options()
        .open(path)
        .map_err(MinerError::Io)?;
    serde_json::to_writer_pretty(&mut file, &metrics)?;
    file.commit().map_err(MinerError::Io)
}

fn make_source(config: &MinerConfig) -> Result<Box<dyn WorkSource>> {
    match &config.source {
        WorkSourceConfig::Rpc {
            url,
            timeout_seconds,
        } => Ok(Box::new(RpcWorkSource::new(
            url.clone(),
            Duration::from_secs(*timeout_seconds),
        )?)),
        WorkSourceConfig::Pool {
            url,
            worker_name,
            timeout_seconds,
        } => Ok(Box::new(PoolWorkSource::new(
            url.clone(),
            config.miner_address.clone(),
            worker_name.clone(),
            Duration::from_secs(*timeout_seconds),
        )?)),
        WorkSourceConfig::LocalFile {
            template_path,
            submission_path,
        } => Ok(Box::new(FileWorkSource::new(
            template_path.clone(),
            submission_path.clone(),
        ))),
        WorkSourceConfig::Stratum {
            host,
            port,
            use_tls,
            skip_tls_verify,
            worker_name,
            password,
            timeout_seconds,
        } => Ok(Box::new(StratumWorkSource::new(
            StratumEndpoint {
                host: host.clone(),
                port: *port,
                use_tls: *use_tls,
                skip_tls_verify: *skip_tls_verify,
                worker_name: worker_name.clone(),
                password: password.clone(),
                timeout_seconds: *timeout_seconds,
            },
            config.miner_address.clone(),
        )?)),
    }
}
