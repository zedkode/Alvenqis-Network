use alvenqis_core::{MnemonicWordCount, Network, WalletDerivationPath};
use alvenqis_wallet::{
    balance, create_wallet, default_chain_data_dir, default_rpc_base_url_for_network,
    default_signed_tx_dir_path, default_wallet_dir_path, export_public_info,
    import_dev_private_key, import_mnemonic_wallet, sign_tx, submit_tx, verify_tx, wallet_address,
    wallet_status, WalletError, WalletResult,
};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

const WALLET_EXAMPLES: &str = "\
Examples:
  # Mainnet-candidate wallets are AES-256-GCM encrypted; set ALVENQIS_WALLET_PASSPHRASE first.
  $env:ALVENQIS_WALLET_PASSPHRASE='your-strong-passphrase'
  alvenqis-wallet --network mainnet-candidate --wallet-dir .alvenqis-local/wallets create-wallet --word-count 24
  # Prefer env/file for secrets (never put mnemonics or keys on argv):
  $env:ALVENQIS_WALLET_MNEMONIC='abandon ...'
  alvenqis-wallet --network mainnet-candidate --wallet-dir .alvenqis-local/wallets import-mnemonic --account 0 --change 0 --address-index 0
  alvenqis-wallet --network mainnet-candidate --wallet-dir .alvenqis-local/wallets import-mnemonic --phrase-file .\\phrase.txt
  alvenqis-wallet --network mainnet-candidate --wallet-dir .alvenqis-local/wallets address
  alvenqis-wallet --network mainnet-candidate --rpc-base-url http://127.0.0.1:10787 balance alve1...
  alvenqis-wallet --network mainnet-candidate --wallet-dir .alvenqis-local/wallets --signed-tx-dir .alvenqis-local/wallets/signed-txs --chain-data-dir .alvenqis-local/chain sign-tx --to alve1... --amount 1.0 --fee 0.01
";

/// Set to 1 only for legacy automation that still passes secrets on argv (discouraged).
const ALLOW_INSECURE_ARGV_ENV: &str = "ALVENQIS_WALLET_ALLOW_INSECURE_ARGV";
const MNEMONIC_ENV: &str = "ALVENQIS_WALLET_MNEMONIC";
const PRIVATE_KEY_ENV: &str = "ALVENQIS_WALLET_PRIVATE_KEY_HEX";

#[derive(Debug, Parser)]
#[command(name = "alvenqis-wallet")]
#[command(about = "Draft / Mainnet Candidate / Prototype wallet CLI for Alvenqis Network")]
#[command(after_help = WALLET_EXAMPLES)]
struct Cli {
    #[arg(long, default_value = "mainnet-candidate")]
    network: Network,
    #[arg(long)]
    wallet_dir: Option<PathBuf>,
    #[arg(long)]
    signed_tx_dir: Option<PathBuf>,
    #[arg(long)]
    rpc_base_url: Option<String>,
    #[arg(long)]
    chain_data_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    CreateWallet {
        #[arg(long, default_value_t = 24)]
        word_count: u16,
        #[arg(long, default_value_t = 0)]
        account: u32,
        #[arg(long, default_value_t = 0)]
        change: u32,
        #[arg(long, default_value_t = 0)]
        address_index: u32,
    },
    ImportMnemonic {
        /// Deprecated: secrets on argv leak via process lists and shell history.
        /// Prefer ALVENQIS_WALLET_MNEMONIC, --phrase-file, or stdin.
        #[arg(long)]
        phrase: Option<String>,
        #[arg(long)]
        phrase_file: Option<PathBuf>,
        /// BIP39 passphrase (optional). Prefer ALVENQIS_WALLET_BIP39_PASSPHRASE env.
        #[arg(long)]
        passphrase: Option<String>,
        #[arg(long, default_value_t = 0)]
        account: u32,
        #[arg(long, default_value_t = 0)]
        change: u32,
        #[arg(long, default_value_t = 0)]
        address_index: u32,
    },
    ImportPrivateKey {
        /// Deprecated: secrets on argv. Prefer ALVENQIS_WALLET_PRIVATE_KEY_HEX or --key-file.
        #[arg(long)]
        private_key_hex: Option<String>,
        #[arg(long)]
        key_file: Option<PathBuf>,
    },
    Network,
    Address,
    Balance {
        address: String,
    },
    SignTx {
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: String,
        #[arg(long)]
        fee: String,
    },
    VerifyTx {
        #[arg(long)]
        tx_file: PathBuf,
    },
    SubmitTx {
        #[arg(long)]
        tx_file: PathBuf,
    },
    ExportPublicInfo,
    WalletStatus,
}

fn main() {
    let cli = Cli::parse();
    let wallet_dir = cli
        .wallet_dir
        .or_else(|| default_wallet_dir_path(cli.network).ok())
        .unwrap_or_else(|| PathBuf::from(cli.network.default_data_root()).join("wallets"));
    let signed_tx_dir = cli
        .signed_tx_dir
        .or_else(|| default_signed_tx_dir_path(cli.network).ok())
        .unwrap_or_else(|| PathBuf::from(cli.network.default_data_root()).join("signed-txs"));
    let chain_data_dir = cli
        .chain_data_dir
        .unwrap_or_else(|| default_chain_data_dir(cli.network));
    let rpc_base_url = cli
        .rpc_base_url
        .unwrap_or_else(|| default_rpc_base_url_for_network(cli.network));

    let result = match cli.command {
        Command::CreateWallet {
            word_count,
            account,
            change,
            address_index,
        } => MnemonicWordCount::from_u16(word_count)
            .map_err(WalletError::from)
            .and_then(|count| {
                create_wallet(
                    &wallet_dir,
                    cli.network,
                    count,
                    WalletDerivationPath::new(account, change, address_index),
                )
            })
            .and_then(emit_created_wallet),
        Command::ImportMnemonic {
            phrase,
            phrase_file,
            passphrase,
            account,
            change,
            address_index,
        } => resolve_secret(
            "mnemonic",
            phrase.as_deref(),
            phrase_file.as_ref(),
            MNEMONIC_ENV,
            true,
        )
        .and_then(|phrase| {
            let bip39_pass = resolve_optional_passphrase(passphrase.as_deref())?;
            import_mnemonic_wallet(
                &wallet_dir,
                &phrase,
                &bip39_pass,
                cli.network,
                WalletDerivationPath::new(account, change, address_index),
            )
            .and_then(emit_public_wallet_only)
        }),
        Command::ImportPrivateKey {
            private_key_hex,
            key_file,
        } => resolve_secret(
            "private key hex",
            private_key_hex.as_deref(),
            key_file.as_ref(),
            PRIVATE_KEY_ENV,
            false,
        )
        .and_then(|key| {
            import_dev_private_key(&wallet_dir, key.trim(), cli.network)
                .and_then(emit_public_wallet_only)
        }),
        Command::Network => json_output(wallet_status(
            cli.network,
            &wallet_dir,
            &signed_tx_dir,
            &rpc_base_url,
        )),
        Command::Address => wallet_address(&wallet_dir),
        Command::Balance { address } => json_output(balance(&rpc_base_url, &address)),
        Command::SignTx { to, amount, fee } => sign_tx(
            &wallet_dir,
            &signed_tx_dir,
            &chain_data_dir,
            &to,
            &amount,
            &fee,
        )
        .and_then(|value| json_output(Ok(value))),
        Command::VerifyTx { tx_file } => json_output(verify_tx(&tx_file)),
        Command::SubmitTx { tx_file } => json_output(submit_tx(&rpc_base_url, &tx_file)),
        Command::ExportPublicInfo => json_output(export_public_info(&wallet_dir)),
        Command::WalletStatus => json_output(wallet_status(
            cli.network,
            &wallet_dir,
            &signed_tx_dir,
            &rpc_base_url,
        )),
    };

    match result {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("alvenqis-wallet error: {error}");
            std::process::exit(1);
        }
    }
}

fn json_output<T: Serialize>(value: WalletResult<T>) -> WalletResult<String> {
    value.and_then(|inner| serde_json::to_string_pretty(&inner).map_err(WalletError::from))
}

/// Recovery phrase goes to stderr once; stdout JSON is public-only (audit CR-H02).
fn emit_created_wallet(created: alvenqis_wallet::CreatedWallet) -> WalletResult<String> {
    eprintln!("=== ALVENQIS RECOVERY MNEMONIC (write down offline; shown once) ===");
    eprintln!("{}", created.mnemonic);
    eprintln!("=== END MNEMONIC — not written to stdout or wallet JSON ===");
    let public = serde_json::json!({
        "network_id": created.network_id,
        "network_name": created.network_name,
        "status_label": created.status_label,
        "address": created.address,
        "public_key_hex": created.public_key_hex,
        "derivation_path": created.derivation_path,
        "wallet_seed_standard_id": created.wallet_seed_standard_id,
        "key_derivation_policy_id": created.key_derivation_policy_id,
        "mnemonic_word_count": created.mnemonic_word_count,
        "mnemonic_shown_on_stderr": true,
        "warning": created.warning,
    });
    serde_json::to_string_pretty(&public).map_err(WalletError::from)
}

/// Import success: never echo private_key_hex / encrypted blob secrets (audit CR-H02).
fn emit_public_wallet_only(wallet: alvenqis_wallet::StoredWallet) -> WalletResult<String> {
    let public = serde_json::json!({
        "schema": wallet.schema,
        "network_id": wallet.network_id,
        "network_name": wallet.network_name,
        "status_label": wallet.status_label,
        "address": wallet.address,
        "public_key_hex": wallet.public_key_hex,
        "encrypted": wallet.encrypted.is_some(),
        "key_origin": wallet.key_origin,
        "derivation_path": wallet.derivation_path,
        "wallet_seed_standard_id": wallet.wallet_seed_standard_id,
        "key_derivation_policy_id": wallet.key_derivation_policy_id,
        "warning": "Secrets are not printed. private_key_hex and ciphertext are omitted from CLI output.",
    });
    serde_json::to_string_pretty(&public).map_err(WalletError::from)
}

fn insecure_argv_allowed() -> bool {
    std::env::var(ALLOW_INSECURE_ARGV_ENV)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}

fn resolve_optional_passphrase(argv: Option<&str>) -> WalletResult<String> {
    if let Ok(env_value) = std::env::var("ALVENQIS_WALLET_BIP39_PASSPHRASE") {
        return Ok(env_value);
    }
    if let Some(value) = argv {
        if !value.is_empty() && !insecure_argv_allowed() {
            return Err(WalletError::Input(format!(
                "BIP39 passphrase on argv is blocked; use ALVENQIS_WALLET_BIP39_PASSPHRASE or set {ALLOW_INSECURE_ARGV_ENV}=1 (discouraged)"
            )));
        }
        return Ok(value.to_owned());
    }
    Ok(String::new())
}

/// Resolve a secret from file, env, optional stdin, or (discouraged) argv.
fn resolve_secret(
    label: &str,
    argv_value: Option<&str>,
    file: Option<&PathBuf>,
    env_name: &str,
    allow_stdin: bool,
) -> WalletResult<String> {
    if let Some(path) = file {
        let content = std::fs::read_to_string(path).map_err(WalletError::from)?;
        let trimmed = content.trim().to_owned();
        if trimmed.is_empty() {
            return Err(WalletError::Input(format!(
                "{label} file is empty: {}",
                path.display()
            )));
        }
        return Ok(trimmed);
    }
    if let Ok(env_value) = std::env::var(env_name) {
        let trimmed = env_value.trim().to_owned();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    if let Some(value) = argv_value {
        if !insecure_argv_allowed() {
            return Err(WalletError::Input(format!(
                "{label} on argv is blocked (audit CR-H01). Use {env_name}, --*-file, or stdin. \
                 Emergency override: {ALLOW_INSECURE_ARGV_ENV}=1"
            )));
        }
        eprintln!(
            "warning: reading {label} from argv is insecure; migrate to {env_name} or a file"
        );
        return Ok(value.to_owned());
    }
    if allow_stdin && !atty_stderr_hint() {
        // When stdin is piped, accept the secret from stdin.
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(WalletError::from)?;
        let trimmed = buf.trim().to_owned();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    Err(WalletError::Input(format!(
        "missing {label}: set {env_name}, pass --*-file, pipe on stdin, or (discouraged) use argv with {ALLOW_INSECURE_ARGV_ENV}=1"
    )))
}

fn atty_stderr_hint() -> bool {
    // Treat non-TTY stdin as piped secret input.
    !io::stdin().is_terminal()
}
