use alvenqis_core::{Network, Transaction};
use alvenqis_node::{
    approve_genesis, backup_chain_database, balance, default_data_dir, default_mempool_dir,
    default_miner_address, export_genesis_block, format_status, genesis_approval_status,
    genesis_hash_hex_from_config, genesis_review_manifest, import_genesis_block, mempool_status,
    mine_block, mine_pending_block, node_status, peers, print_chain, queue_peer_ban,
    queue_peer_unban, runtime_dir_for_data_dir, shutdown, start_node, state, status,
    submit_transaction, validate_chain, verify_database_integrity, write_genesis_review_manifest,
    NetworkConfig, DEFAULT_CONFIG_PATH,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

const NODE_EXAMPLES: &str = "\
Examples:
  alvenqis-node --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool start-node
  alvenqis-node --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool node-status
  alvenqis-node --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool mine-block
  alvenqis-node --config configs/local.toml --data-dir .alvenqis-local/chain --mempool-dir .alvenqis-local/mempool validate-chain
  alvenqis-node --data-dir .alvenqis-local/chain backup-chain-database --output .alvenqis-local/backups/chain.sqlite3
  alvenqis-node --data-dir .alvenqis-local/chain verify-chain-database
  alvenqis-node --config configs/mainnet-candidate.toml export-genesis-review --output docs/release/GENESIS_REVIEW.mainnet-candidate.json
  alvenqis-node --config configs/mainnet-candidate.toml approve-genesis --review-file docs/release/GENESIS_REVIEW.mainnet-candidate.json --approved-by \"Operator\" --output docs/release/GENESIS_APPROVAL.mainnet-candidate.json
  alvenqis-node --config configs/mainnet-candidate.toml export-genesis-block --output docs/release/genesis.mainnet-candidate.block.json
  alvenqis-node --config configs/mainnet-candidate.toml --data-dir /var/lib/alvenqis/.alvenqis-mainnet/chain import-genesis-block --genesis-file docs/release/genesis.mainnet-candidate.block.json --force
";

#[derive(Debug, Parser)]
#[command(name = "alvenqis-node")]
#[command(about = "Draft / Mainnet Candidate / Prototype CLI for Alvenqis Network")]
#[command(after_help = NODE_EXAMPLES)]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(long)]
    mempool_dir: Option<PathBuf>,
    #[arg(long)]
    miner_address: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    StartNode {
        #[arg(long, default_value_t = false)]
        force_genesis: bool,
    },
    NodeStatus,
    MineBlock,
    Peers,
    BanPeer {
        peer_id: String,
        #[arg(long)]
        reason: String,
        /// Ban duration in seconds; 0 means permanent.
        #[arg(long, default_value_t = 3_600)]
        duration_seconds: u64,
    },
    UnbanPeer {
        peer_id: String,
    },
    Shutdown,
    PrintGenesisHash,
    ExportGenesisReview {
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Export pre-mined genesis block JSON (mines once on this machine — use for VPS import).
    ExportGenesisBlock {
        #[arg(long)]
        output: PathBuf,
    },
    /// Import pre-mined genesis into empty data-dir (no mining on server / no GPU needed).
    ImportGenesisBlock {
        #[arg(long)]
        genesis_file: PathBuf,
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    ApproveGenesis {
        #[arg(long)]
        review_file: PathBuf,
        #[arg(long)]
        approved_by: String,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    GenesisApprovalStatus,
    Status,
    MinePendingBlock,
    PrintChain,
    ValidateChain,
    /// Create a transactionally consistent online SQLite backup.
    BackupChainDatabase {
        #[arg(long)]
        output: PathBuf,
    },
    /// Run SQLite integrity_check against the canonical chain database.
    VerifyChainDatabase,
    MempoolStatus,
    Balance {
        address: String,
    },
    State,
    SubmitTx {
        #[arg(long)]
        tx_file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let default_config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
    let config_path = cli.config.unwrap_or(default_config_path);
    let loaded_config = NetworkConfig::load_from_path(&config_path).ok();
    let configured_network = loaded_config
        .as_ref()
        .map(|config| config.network)
        .unwrap_or_else(|| Network::MainnetCandidate);
    let max_mempool_transactions = loaded_config
        .as_ref()
        .map(|config| config.max_mempool_transactions)
        .unwrap_or(1024);
    let data_dir = cli
        .data_dir
        .unwrap_or_else(|| default_data_dir(configured_network));
    let mempool_dir = cli
        .mempool_dir
        .unwrap_or_else(|| default_mempool_dir(configured_network));
    let miner_address = cli
        .miner_address
        .unwrap_or_else(|| default_miner_address(configured_network));

    let result = match cli.command {
        Command::StartNode { force_genesis } => {
            start_node(&config_path, &data_dir, &mempool_dir, force_genesis).map(|_| {
                format!(
                    "stopped network_id={} data_dir={} mempool_dir={}",
                    configured_network.network_id(),
                    data_dir.display(),
                    mempool_dir.display()
                )
            })
        }
        Command::NodeStatus => {
            node_status(&config_path, &data_dir, &mempool_dir).and_then(|summary| {
                serde_json::to_string_pretty(&summary).map_err(alvenqis_node::NodeError::from)
            })
        }
        Command::MineBlock => mine_block(&config_path, &data_dir, &mempool_dir, &miner_address),
        Command::Peers => peers(&config_path, &data_dir).and_then(|summary| {
            serde_json::to_string_pretty(&summary).map_err(alvenqis_node::NodeError::from)
        }),
        Command::BanPeer {
            peer_id,
            reason,
            duration_seconds,
        } => queue_peer_ban(
            &runtime_dir_for_data_dir(&data_dir),
            &peer_id,
            &reason,
            duration_seconds,
        )
        .map(|request_id| format!("queued peer ban request_id={request_id} peer_id={peer_id}")),
        Command::UnbanPeer { peer_id } => {
            queue_peer_unban(&runtime_dir_for_data_dir(&data_dir), &peer_id).map(|request_id| {
                format!("queued peer unban request_id={request_id} peer_id={peer_id}")
            })
        }
        Command::Shutdown => shutdown(configured_network, &data_dir),
        Command::PrintGenesisHash => genesis_hash_hex_from_config(&config_path),
        Command::ExportGenesisBlock { output } => export_genesis_block(&config_path, &output)
            .and_then(|genesis| {
                Ok(format!(
                    "exported genesis block hash={} height={} output={}",
                    alvenqis_core::hash_to_hex(&genesis.hash()?),
                    genesis.header.height,
                    output.display()
                ))
            }),
        Command::ImportGenesisBlock {
            genesis_file,
            force,
        } => import_genesis_block(&config_path, &data_dir, &genesis_file, force).map(|hash| {
            format!(
                "imported genesis hash={} data_dir={}",
                hash,
                data_dir.display()
            )
        }),
        Command::ExportGenesisReview { output } => match output {
            Some(path) => write_genesis_review_manifest(&config_path, &path).map(|manifest| {
                format!(
                    "exported genesis review network_id={} review_hash={} output={}",
                    manifest.network_id,
                    manifest.review_hash,
                    path.display()
                )
            }),
            None => genesis_review_manifest(&config_path).and_then(|manifest| {
                serde_json::to_string_pretty(&manifest).map_err(alvenqis_node::NodeError::from)
            }),
        },
        Command::ApproveGenesis {
            review_file,
            approved_by,
            notes,
            output,
        } => approve_genesis(
            &config_path,
            &review_file,
            &approved_by,
            notes.as_deref(),
            output.as_deref(),
        )
        .and_then(|summary| {
            serde_json::to_string_pretty(&summary).map_err(alvenqis_node::NodeError::from)
        }),
        Command::GenesisApprovalStatus => {
            genesis_approval_status(&config_path).and_then(|summary| {
                serde_json::to_string_pretty(&summary).map_err(alvenqis_node::NodeError::from)
            })
        }
        Command::Status => status(&config_path, &data_dir).map(|report| format_status(&report)),
        Command::MinePendingBlock => {
            mine_pending_block(&config_path, &data_dir, &mempool_dir, &miner_address).and_then(
                |summary| {
                    serde_json::to_string_pretty(&summary).map_err(alvenqis_node::NodeError::from)
                },
            )
        }
        Command::PrintChain => print_chain(&config_path, &data_dir),
        Command::ValidateChain => validate_chain(&config_path, &data_dir).map(|summary| {
            format!(
                "valid network_id={} network={} height={} blocks={} tip_hash={}",
                summary.network_id,
                summary.network_name,
                summary.height,
                summary.block_count,
                summary.tip_hash
            )
        }),
        Command::BackupChainDatabase { output } => {
            backup_chain_database(&data_dir, &output).map(|_| {
                format!(
                    "backed up chain database data_dir={} output={}",
                    data_dir.display(),
                    output.display()
                )
            })
        }
        Command::VerifyChainDatabase => verify_database_integrity(&data_dir).map(|_| {
            format!(
                "valid SQLite chain database data_dir={}",
                data_dir.display()
            )
        }),
        Command::Balance { address } => {
            balance(&config_path, &data_dir, &address).and_then(|summary| {
                serde_json::to_string_pretty(&summary).map_err(alvenqis_node::NodeError::from)
            })
        }
        Command::State => state(&config_path, &data_dir).and_then(|summary| {
            serde_json::to_string_pretty(&summary).map_err(alvenqis_node::NodeError::from)
        }),
        Command::MempoolStatus => mempool_status(&data_dir, &mempool_dir).and_then(|summary| {
            serde_json::to_string_pretty(&summary).map_err(alvenqis_node::NodeError::from)
        }),
        Command::SubmitTx { tx_file } => {
            let content = std::fs::read_to_string(&tx_file).map_err(alvenqis_node::NodeError::from);
            content
                .and_then(|value| serde_json::from_str::<Transaction>(&value).map_err(Into::into))
                .and_then(|transaction| {
                    submit_transaction(
                        &data_dir,
                        &mempool_dir,
                        max_mempool_transactions,
                        &transaction,
                    )
                })
                .and_then(|summary| {
                    serde_json::to_string_pretty(&summary).map_err(alvenqis_node::NodeError::from)
                })
        }
    };

    match result {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("alvenqis-node error: {error}");
            std::process::exit(1);
        }
    }
}
