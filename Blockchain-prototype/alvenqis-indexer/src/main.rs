use alvenqis_core::Network;
use alvenqis_indexer::{
    default_index_dir_for_network, default_network_root, find_address, find_block,
    find_transaction, index_chain, indexer_status_with_chain, latest_block, load_index,
    reset_index, sync_index, watch_index,
};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

const INDEXER_EXAMPLES: &str = "\
Examples:
  alvenqis-indexer --network mainnet-candidate --chain-data-dir .alvenqis-local/chain --index-dir .alvenqis-local/indexer index-chain
  alvenqis-indexer --network mainnet-candidate --chain-data-dir .alvenqis-local/chain --index-dir .alvenqis-local/indexer sync
  alvenqis-indexer --network mainnet-candidate --chain-data-dir .alvenqis-local/chain --index-dir .alvenqis-local/indexer watch --interval-seconds 5
  alvenqis-indexer --network mainnet-candidate --index-dir .alvenqis-local/indexer status
  alvenqis-indexer --network mainnet-candidate --index-dir .alvenqis-local/indexer latest-block
";

#[derive(Debug, Parser)]
#[command(name = "alvenqis-indexer")]
#[command(about = "Draft / Mainnet Candidate / Prototype indexer CLI for Alvenqis Network")]
#[command(after_help = INDEXER_EXAMPLES)]
struct Cli {
    #[arg(long, default_value = "mainnet-candidate")]
    network: Network,
    #[arg(long)]
    chain_data_dir: Option<PathBuf>,
    #[arg(long)]
    index_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Full rebuild of the index from the chain file.
    IndexChain,
    /// Rebuild only when chain tip differs from the index tip (reorg-safe).
    Sync,
    /// Continuously poll the chain and reindex on tip change (operator helper).
    Watch {
        #[arg(long, default_value_t = 5)]
        interval_seconds: u64,
        /// Stop after N polls (tests / one-shot soak). Omit for forever.
        #[arg(long)]
        max_iterations: Option<u64>,
    },
    Status,
    ResetIndex,
    PrintIndexSummary,
    FindBlock {
        height: u64,
    },
    FindTx {
        tx_hash: String,
    },
    FindAddress {
        address: String,
    },
    LatestBlock,
}

fn main() {
    let cli = Cli::parse();
    let chain_data_dir = cli
        .chain_data_dir
        .unwrap_or_else(|| default_network_root(cli.network).join("chain"));
    let index_dir = cli
        .index_dir
        .unwrap_or_else(|| default_index_dir_for_network(cli.network));
    let result = match cli.command {
        Command::IndexChain => json_output(index_chain(&chain_data_dir, &index_dir)),
        Command::Sync => json_output(sync_index(&chain_data_dir, &index_dir)),
        Command::Watch {
            interval_seconds,
            max_iterations,
        } => watch_index(
            &chain_data_dir,
            &index_dir,
            interval_seconds,
            max_iterations,
        )
        .map(|_| {
            format!(
                "watch status=ok chain={} index={}",
                chain_data_dir.display(),
                index_dir.display()
            )
        }),
        Command::Status => json_output(indexer_status_with_chain(
            &index_dir,
            Some(chain_data_dir.as_path()),
        )),
        Command::ResetIndex => reset_index(&index_dir).map(|_| {
            format!(
                "reset status=ok mode=\"Draft / Local Indexer / Prototype\" index_dir={}",
                index_dir.display()
            )
        }),
        Command::PrintIndexSummary => load_index(&index_dir)
            .and_then(|index| serde_json::to_string_pretty(&index.summary).map_err(Into::into)),
        Command::FindBlock { height } => json_output(find_block(&index_dir, height)),
        Command::FindTx { tx_hash } => json_output(find_transaction(&index_dir, &tx_hash)),
        Command::FindAddress { address } => json_output(find_address(&index_dir, &address)),
        Command::LatestBlock => json_output(latest_block(&index_dir)),
    };

    match result {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("alvenqis-indexer error: {error}");
            std::process::exit(1);
        }
    }
}

fn json_output<T: Serialize>(
    value: alvenqis_indexer::IndexerResult<T>,
) -> alvenqis_indexer::IndexerResult<String> {
    value.and_then(|inner| serde_json::to_string_pretty(&inner).map_err(Into::into))
}
