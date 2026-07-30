use alvenqis_core::Network;
use alvenqis_node::{default_miner_address, init_devnet, mine_dev_blocks, storage, validate_chain};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tempfile::tempdir;

const DEFAULT_BLOCK_COUNT: u64 = 1_000;

#[derive(Debug, Deserialize, Serialize)]
struct ColdRebuildMeasurement {
    block_count: usize,
    height: u64,
    cold_rebuild_ms: u128,
    peak_rss_kib: Option<u64>,
    sqlite_bytes: u64,
}

#[derive(Debug, Serialize)]
struct LongChainBenchmarkReport {
    requested_child_blocks: u64,
    fixture_build_ms: u128,
    measurement: ColdRebuildMeasurement,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("long-chain rebuild benchmark failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.first().is_some_and(|value| value == "--measure") {
        return run_measurement(&arguments[1..]);
    }

    let requested_child_blocks = parse_block_count(&arguments)?;
    let temporary = tempdir()?;
    let config_path = temporary.path().join("alvenqis-devnet/config/devnet.toml");
    let data_dir = temporary.path().join(".alvenqis-dev/chain");
    write_devnet_config(&config_path)?;

    let miner_address = default_miner_address(Network::Devnet);
    init_devnet(&config_path, &data_dir, &miner_address)?;
    let fixture_started = Instant::now();
    let fixture = mine_dev_blocks(
        &config_path,
        &data_dir,
        &miner_address,
        requested_child_blocks,
    )?;
    let fixture_build_ms = fixture_started.elapsed().as_millis();

    let executable = std::env::current_exe()?;
    let output = Command::new(executable)
        .arg("--measure")
        .arg(&config_path)
        .arg(&data_dir)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "cold child process exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    let measurement: ColdRebuildMeasurement = serde_json::from_slice(&output.stdout)?;
    if measurement.block_count != fixture.block_count || measurement.height != fixture.height {
        return Err(format!(
            "cold rebuild returned blocks={} height={}, expected blocks={} height={}",
            measurement.block_count, measurement.height, fixture.block_count, fixture.height
        )
        .into());
    }

    let report = LongChainBenchmarkReport {
        requested_child_blocks,
        fixture_build_ms,
        measurement,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn parse_block_count(arguments: &[OsString]) -> Result<u64, Box<dyn Error>> {
    match arguments {
        [] => Ok(DEFAULT_BLOCK_COUNT),
        [flag, value] if flag == "--blocks" => {
            let value = value
                .to_str()
                .ok_or("--blocks must be valid UTF-8")?
                .parse::<u64>()?;
            if value == 0 {
                return Err("--blocks must be greater than zero".into());
            }
            Ok(value)
        }
        _ => Err("usage: cargo run -p alvenqis-node --example long_chain_rebuild --release -- [--blocks COUNT]".into()),
    }
}

fn run_measurement(arguments: &[OsString]) -> Result<(), Box<dyn Error>> {
    let [config_path, data_dir] = arguments else {
        return Err("internal --measure requires CONFIG_PATH and DATA_DIR".into());
    };
    let config_path = PathBuf::from(config_path);
    let data_dir = PathBuf::from(data_dir);
    let started = Instant::now();
    let summary = validate_chain(&config_path, &data_dir)?;
    let measurement = ColdRebuildMeasurement {
        block_count: summary.block_count,
        height: summary.height,
        cold_rebuild_ms: started.elapsed().as_millis(),
        peak_rss_kib: peak_rss_kib(),
        sqlite_bytes: sqlite_storage_bytes(&data_dir)?,
    };
    println!("{}", serde_json::to_string(&measurement)?);
    Ok(())
}

fn write_devnet_config(path: &Path) -> Result<(), Box<dyn Error>> {
    let network = Network::Devnet;
    let chain_magic_hex = network
        .chain_magic_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let content = format!(
        r#"network = "devnet"
network_id = "{network_id}"
human_name = "{human_name}"
status_label = "{status_label}"
block_time_seconds = 60
difficulty_leading_zero_bits = 4
ticker = "ALVE"
address_prefix = "{address_prefix}"
max_supply = "60000000"
halving_interval = 1576800
initial_block_reward = "19.02587519"
default_rpc_port = {rpc_port}
default_p2p_port = {p2p_port}
max_mempool_transactions = 8
genesis_config_path = "{genesis_config_path}"
chain_magic_hex = "{chain_magic_hex}"
allow_mainnet_candidate = false
"#,
        network_id = network.network_id(),
        human_name = network.human_name(),
        status_label = network.status_label(),
        address_prefix = network.address_prefix(),
        rpc_port = network.default_rpc_port(),
        p2p_port = network.default_p2p_port(),
        genesis_config_path = network.genesis_config_path(),
    );
    fs::create_dir_all(path.parent().ok_or("config path must have a parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

fn sqlite_storage_bytes(data_dir: &Path) -> Result<u64, Box<dyn Error>> {
    let database = storage::chain_database_path(data_dir);
    let mut bytes = fs::metadata(&database)?.len();
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", database.display()));
        if let Ok(metadata) = fs::metadata(sidecar) {
            bytes = bytes.saturating_add(metadata.len());
        }
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn peak_rss_kib() -> Option<u64> {
    fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")
                .and_then(|value| value.split_whitespace().next())
                .and_then(|value| value.parse().ok())
        })
}

#[cfg(not(target_os = "linux"))]
fn peak_rss_kib() -> Option<u64> {
    None
}
