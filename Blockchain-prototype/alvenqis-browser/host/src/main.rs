mod confirm;
mod keystore;
mod native_messaging;
mod protocol;

use alvenqis_sdk_rust::{
    BlockingRpcClient, MnemonicWordCount, Network, WalletAccount, WalletDerivationPath,
};
use keystore::{
    change_passphrase, create_encrypted_wallet, default_keystore_dir, delete_wallet, export_public,
    keystore_path, unlock_wallet,
};
use protocol::{default_config_from_args, HostState, Request};
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    if args.iter().any(|a| a == "--print-info") {
        exit_result(print_info(&args));
    }

    if args.iter().any(|a| a == "--export-public") {
        run_export_public(&args);
        return;
    }

    // One-shot RPC probes (exit after print).
    if args.iter().any(|a| a == "--print-tip") {
        exit_result(run_print_tip(&args));
    }
    if args.iter().any(|a| a == "--print-status") {
        exit_result(run_print_status(&args));
    }
    if args.iter().any(|a| a == "--print-sync") {
        exit_result(run_print_sync(&args));
    }
    if args.iter().any(|a| a == "--print-supply") {
        exit_result(run_print_supply(&args));
    }
    if args.iter().any(|a| a == "--print-mempool") {
        exit_result(run_print_mempool(&args));
    }
    if args.iter().any(|a| a == "--print-indexer") {
        exit_result(run_print_indexer(&args));
    }
    if args.iter().any(|a| a == "--print-block") {
        exit_result(run_print_block(&args));
    }
    if args.iter().any(|a| a == "--print-chain") {
        exit_result(run_print_chain(&args));
    }
    if args.iter().any(|a| a == "--print-account") {
        exit_result(run_print_account(&args));
    }
    if args.iter().any(|a| a == "--check-health") {
        // Distinct exit codes for automation (see run_check_health).
        match run_check_health(&args) {
            Ok(code) => std::process::exit(code),
            Err(error) => {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|a| a == "--init-wallet") {
        if let Err(error) = run_init_wallet(&args) {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|a| a == "--import-mnemonic") {
        if let Err(error) = run_import_mnemonic(&args) {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|a| a == "--change-passphrase") {
        if let Err(error) = run_change_passphrase(&args) {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|a| a == "--delete-wallet") {
        if let Err(error) = run_delete_wallet(&args) {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        return;
    }

    let jsonl = args.iter().any(|a| a == "--jsonl" || a == "--stdio-jsonl");
    // Secure default ON (audit CR-C02). Opt out with --no-require-os-confirm or env=0/false.
    let require_os_confirm = if args.iter().any(|a| a == "--no-require-os-confirm") {
        false
    } else if let Ok(value) = env::var("ALVENQIS_HOST_REQUIRE_OS_CONFIRM") {
        !(value == "0"
            || value.eq_ignore_ascii_case("false")
            || value.eq_ignore_ascii_case("no")
            || value.eq_ignore_ascii_case("off"))
    } else {
        // Explicit flag still supported; default remains true when flag is absent.
        true
    };

    let config = default_config_from_args(&args);
    let mut state = match HostState::new(config) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("alvenqis-browser-host failed to start: {error}");
            std::process::exit(1);
        }
    };

    if let Some(dir) = arg_value(&args, "--keystore-dir") {
        state = state.with_keystore_dir(PathBuf::from(dir));
    }
    state = state.with_require_os_confirm(require_os_confirm);

    if jsonl {
        run_jsonl(&mut state);
    } else {
        run_native_messaging(&mut state);
    }
}

fn exit_result(result: Result<(), String>) -> ! {
    match result {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

fn print_help() {
    eprintln!(
        "alvenqis-browser-host {} -- Mainnet Candidate native messaging host

USAGE:
  alvenqis-browser-host [OPTIONS]

NATIVE MESSAGING / DEV:
  --jsonl                 JSON-lines protocol on stdin/stdout (dev/test)
  --local                 Use http://127.0.0.1:10787 RPC
  --rpc <url>             Custom RPC base URL
  --keystore-dir <path>   Override encrypted keystore directory
  --require-os-confirm    Confirm send/sign/submit via OS dialog (default: ON)
  --no-require-os-confirm Disable OS confirmation (dev/test ONLY — unsafe for funds)
  --print-info            Print network/keystore paths and exit
  -h, --help              Show this help

RPC ONE-SHOTS (exit after print; use --local or --rpc as needed):
  --print-tip             Chain tip (kv lines; add --json for object)
  --print-status          GET /status as JSON
  --print-sync            GET /sync/status as JSON
  --print-supply          GET /supply as JSON
  --print-mempool         GET /mempool/status as JSON
  --print-indexer         GET /indexer/status as JSON
  --print-block           Latest block (or --height N)
  --print-chain           Combined tip/status/sync/mempool/indexer/supply summary
  --print-account         Account snapshot (--address alve1... or keystore public address)
  --check-health          Probe RPC + tip (+ optional indexer); exit codes for automation
  --require-indexer-sync  Fail if indexer missing/out of sync (lag allowed = 0 unless set below)
  --max-indexer-lag <n>   Fail if indexer lag_blocks or height delta > n (enables indexer check)
  --json                  Force JSON object for --print-tip / --print-info / --check-health

Health exit codes (--check-health):
  0  healthy
  1  transport / unexpected error
  2  chain not ready (status/tip missing)
  3  indexer lag / not in sync (with --require-indexer-sync and/or --max-indexer-lag)

RECOVERY (CLI ONLY -- mnemonic never sent to the extension):
  --init-wallet           Create encrypted keystore; print recovery phrase ONCE to stderr
  --import-mnemonic       Import mnemonic into encrypted keystore
  --export-public         Print address / public metadata (no secrets)
  --change-passphrase     Re-encrypt keystore under a new passphrase
  --delete-wallet         Delete keystore after passphrase verification

Passphrases / args:
  --passphrase <text>     Or env ALVENQIS_HOST_PASSPHRASE
  --old-passphrase <text> For --change-passphrase
  --new-passphrase <text> For --change-passphrase
  --mnemonic <phrase>     For --import-mnemonic
  --height <n>            For --print-block
  --address <alve1...>      For --print-account

ENV:
  ALVENQIS_HOST_REQUIRE_OS_CONFIRM=0|1   (default: require OS confirm for sign/send/submit)
  ALVENQIS_HOST_CONFIRM=1                (allow send/sign/submit when OS confirm is on, non-GUI)
  ALVENQIS_HOST_RECOVERY_ACK=1           (allow create_wallet after phrase shown on stderr)
  ALVENQIS_HOST_HEADLESS=1               (never open OS dialogs; force env-ack paths — CI/tests)
  ALVENQIS_HOST_PASSPHRASE=...
",
        env!("CARGO_PKG_VERSION")
    );
}

fn wants_json(args: &[String]) -> bool {
    args.iter().any(|a| a == "--json")
}

fn print_info(args: &[String]) -> Result<(), String> {
    let config = default_config_from_args(args);
    let keystore_dir = resolve_keystore_dir(args, config.network);
    if wants_json(args) {
        return print_json(&serde_json::json!({
            "service": "alvenqis-browser-host",
            "version": env!("CARGO_PKG_VERSION"),
            "network_id": config.network_id(),
            "status_label": config.status_label(),
            "rpc_base_url": config.rpc_base_url,
            "keystore_dir": keystore_dir.display().to_string(),
            "keystore_file": keystore_path(&keystore_dir).display().to_string(),
            "status": "Mainnet Candidate / Prototype",
        }));
    }
    println!("service=alvenqis-browser-host");
    println!("version={}", env!("CARGO_PKG_VERSION"));
    println!("network_id={}", config.network_id());
    println!("status_label={}", config.status_label());
    println!("rpc_base_url={}", config.rpc_base_url);
    println!("keystore_dir={}", keystore_dir.display());
    println!("keystore_file={}", keystore_path(&keystore_dir).display());
    println!("status=Mainnet Candidate / Prototype");
    Ok(())
}

fn rpc_client(args: &[String]) -> Result<BlockingRpcClient, String> {
    let config = default_config_from_args(args);
    BlockingRpcClient::new(config).map_err(|e| e.to_string())
}

fn print_json(value: &impl serde::Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn run_print_tip(args: &[String]) -> Result<(), String> {
    let tip = rpc_client(args)?.tip().map_err(|e| e.to_string())?;
    if wants_json(args) {
        return print_json(&tip);
    }
    println!("height={}", tip.height);
    println!("hash={}", tip.hash);
    Ok(())
}

fn run_print_status(args: &[String]) -> Result<(), String> {
    print_json(&rpc_client(args)?.status().map_err(|e| e.to_string())?)
}

fn run_print_sync(args: &[String]) -> Result<(), String> {
    print_json(&rpc_client(args)?.sync_status().map_err(|e| e.to_string())?)
}

fn run_print_supply(args: &[String]) -> Result<(), String> {
    print_json(&rpc_client(args)?.supply().map_err(|e| e.to_string())?)
}

fn run_print_mempool(args: &[String]) -> Result<(), String> {
    print_json(
        &rpc_client(args)?
            .mempool_status()
            .map_err(|e| e.to_string())?,
    )
}

fn run_print_indexer(args: &[String]) -> Result<(), String> {
    print_json(
        &rpc_client(args)?
            .indexer_status()
            .map_err(|e| e.to_string())?,
    )
}

fn run_print_block(args: &[String]) -> Result<(), String> {
    let client = rpc_client(args)?;
    let block = if let Some(height) = arg_value(args, "--height") {
        let height: u64 = height
            .parse()
            .map_err(|_| format!("invalid --height {height}"))?;
        client.block_by_height(height).map_err(|e| e.to_string())?
    } else {
        client.block_latest().map_err(|e| e.to_string())?
    };
    print_json(&block)
}

fn run_print_chain(args: &[String]) -> Result<(), String> {
    let client = rpc_client(args)?;
    let status = client.status().map_err(|e| e.to_string())?;
    let tip = client.tip().map_err(|e| e.to_string())?;
    let sync = client.sync_status().ok();
    let mempool = client.mempool_status().ok();
    let supply = client.supply().ok();
    let indexer = client.indexer_status().ok();
    let summary = serde_json::json!({
        "network_id": status.network_id,
        "status_label": status.status_label,
        "initialized": status.initialized,
        "height": status.height,
        "block_count": status.block_count,
        "tip_hash": status.tip_hash,
        "tip": tip,
        "sync": sync,
        "mempool": mempool,
        "supply": supply,
        "indexer": indexer,
        "rpc_base_url": client.config().rpc_base_url,
        "disclaimer": "Mainnet Candidate / Prototype -- not Mainnet Live",
    });
    print_json(&summary)
}

fn run_print_account(args: &[String]) -> Result<(), String> {
    let client = rpc_client(args)?;
    let address = if let Some(address) = arg_value(args, "--address") {
        address.to_owned()
    } else {
        let dir = resolve_keystore_dir(args, Network::MainnetCandidate);
        export_public(&dir)
            .map(|view| view.address)
            .map_err(|e| format!("{e} (or pass --address alve1...)"))?
    };
    let account = client.account(&address).map_err(|e| e.to_string())?;
    print_json(&account)
}

/// Exit codes: 0 healthy, 1 error, 2 chain not ready, 3 indexer lag (when enforced).
fn run_check_health(args: &[String]) -> Result<i32, String> {
    let require_indexer_flag = args.iter().any(|a| a == "--require-indexer-sync");
    let max_indexer_lag = parse_max_indexer_lag(args)?;
    // Enforce indexer checks if either flag/threshold is present.
    let enforce_indexer = require_indexer_flag || max_indexer_lag.is_some();
    let allowed_lag = max_indexer_lag.unwrap_or(0);
    let client = rpc_client(args)?;

    let status = match client.status() {
        Ok(s) => s,
        Err(error) => {
            let report = serde_json::json!({
                "ok": false,
                "code": 1,
                "error": error.to_string(),
                "disclaimer": "Mainnet Candidate / Prototype - not Mainnet Live",
            });
            if wants_json(args) {
                print_json(&report)?;
            } else {
                eprintln!("health=fail code=1 error={error}");
            }
            return Ok(1);
        }
    };

    let tip = match client.tip() {
        Ok(t) => t,
        Err(error) => {
            let report = serde_json::json!({
                "ok": false,
                "code": 2,
                "error": format!("tip unavailable: {error}"),
                "status": status,
                "disclaimer": "Mainnet Candidate / Prototype - not Mainnet Live",
            });
            if wants_json(args) {
                print_json(&report)?;
            } else {
                eprintln!("health=fail code=2 tip unavailable: {error}");
            }
            return Ok(2);
        }
    };

    let mut warnings: Vec<String> = Vec::new();
    if !status.initialized {
        warnings.push("status.initialized=false".into());
    }
    if status.height.is_none() {
        warnings.push("status.height is null".into());
    }

    let chain_ready = status.initialized && status.height.is_some();
    if let Some(h) = status.height {
        if h != tip.height {
            warnings.push(format!(
                "status.height ({h}) != tip.height ({})",
                tip.height
            ));
        }
    }

    let indexer = client.indexer_status().ok();
    let mut indexer_ok = true;
    let mut observed_lag: Option<u64> = None;
    if let Some(ref idx) = indexer {
        if let Some(false) = idx.in_sync {
            // in_sync=false is only hard-fail when allowed_lag == 0 (strict zero-lag).
            if allowed_lag == 0 {
                indexer_ok = false;
            }
            warnings.push("indexer.in_sync=false".into());
        }
        if let Some(lag) = idx.lag_blocks {
            observed_lag = Some(observed_lag.map_or(lag, |v| v.max(lag)));
            if lag > allowed_lag {
                indexer_ok = false;
                warnings.push(format!("indexer.lag_blocks={lag} > allowed {allowed_lag}"));
            } else if lag > 0 {
                warnings.push(format!(
                    "indexer.lag_blocks={lag} (within allowed {allowed_lag})"
                ));
            }
        }
        if let (Some(ih), Some(ch)) = (idx.indexed_height, status.height.or(Some(tip.height))) {
            let delta = ch.saturating_sub(ih);
            observed_lag = Some(observed_lag.map_or(delta, |v| v.max(delta)));
            if delta > allowed_lag {
                indexer_ok = false;
                warnings.push(format!(
                    "indexer height delta {delta} (idx={ih} chain={ch}) > allowed {allowed_lag}"
                ));
            } else if delta > 0 {
                warnings.push(format!(
                    "indexer height delta {delta} (within allowed {allowed_lag})"
                ));
            }
        }
        if !idx.initialized {
            indexer_ok = false;
            warnings.push("indexer.initialized=false".into());
        }
    } else {
        indexer_ok = false;
        warnings.push("indexer_status unavailable".into());
    }

    if !chain_ready {
        let report = serde_json::json!({
            "ok": false,
            "code": 2,
            "height": tip.height,
            "tip_hash": tip.hash,
            "status": status,
            "indexer": indexer,
            "warnings": warnings,
            "max_indexer_lag": allowed_lag,
            "disclaimer": "Mainnet Candidate / Prototype - not Mainnet Live",
        });
        if wants_json(args) {
            print_json(&report)?;
        } else {
            eprintln!(
                "health=fail code=2 height={} tip={} warnings={}",
                tip.height,
                tip.hash,
                warnings.join("; ")
            );
        }
        return Ok(2);
    }

    if enforce_indexer && !indexer_ok {
        let report = serde_json::json!({
            "ok": false,
            "code": 3,
            "height": tip.height,
            "tip_hash": tip.hash,
            "status": status,
            "indexer": indexer,
            "warnings": warnings,
            "observed_indexer_lag": observed_lag,
            "max_indexer_lag": allowed_lag,
            "require_indexer_sync": require_indexer_flag,
            "disclaimer": "Mainnet Candidate / Prototype - not Mainnet Live",
        });
        if wants_json(args) {
            print_json(&report)?;
        } else {
            eprintln!(
                "health=fail code=3 height={} tip={} warnings={}",
                tip.height,
                tip.hash,
                warnings.join("; ")
            );
        }
        return Ok(3);
    }

    let report = serde_json::json!({
        "ok": true,
        "code": 0,
        "height": tip.height,
        "tip_hash": tip.hash,
        "status_label": status.status_label,
        "network_id": status.network_id,
        "indexer_in_sync": indexer.as_ref().and_then(|i| i.in_sync),
        "indexer_lag_blocks": indexer.as_ref().and_then(|i| i.lag_blocks),
        "observed_indexer_lag": observed_lag,
        "max_indexer_lag": if enforce_indexer { Some(allowed_lag) } else { None },
        "warnings": warnings,
        "require_indexer_sync": require_indexer_flag,
        "enforce_indexer": enforce_indexer,
        "disclaimer": "Mainnet Candidate / Prototype - not Mainnet Live",
    });
    if wants_json(args) {
        print_json(&report)?;
    } else {
        println!("health=ok code=0 height={} tip={}", tip.height, tip.hash);
        if !warnings.is_empty() {
            println!("warnings={}", warnings.join("; "));
        }
    }
    Ok(0)
}

fn parse_max_indexer_lag(args: &[String]) -> Result<Option<u64>, String> {
    match arg_value(args, "--max-indexer-lag") {
        None => Ok(None),
        Some(raw) => raw
            .parse::<u64>()
            .map(Some)
            .map_err(|_| format!("invalid --max-indexer-lag value: {raw}")),
    }
}

fn run_export_public(args: &[String]) {
    let config = default_config_from_args(args);
    let dir = resolve_keystore_dir(args, config.network);
    match export_public(&dir) {
        Ok(view) => match serde_json::to_string_pretty(&view) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

fn run_init_wallet(args: &[String]) -> Result<(), String> {
    let config = default_config_from_args(args);
    let dir = resolve_keystore_dir(args, config.network);
    let passphrase = require_cli_passphrase(args, "--passphrase", "ALVENQIS_HOST_PASSPHRASE")?;
    let (account, mnemonic) = WalletAccount::generate(config.network, MnemonicWordCount::Twelve)
        .map_err(|e| e.to_string())?;
    let stored = create_encrypted_wallet(&dir, config.network, &passphrase, &account)?;

    eprintln!("=== Alvenqis browser-host recovery phrase (shown ONCE) ===");
    eprintln!("{}", mnemonic.phrase);
    eprintln!("=== Store offline. This phrase is never given to the extension. ===");
    eprintln!("path={}", keystore_path(&dir).display());
    println!("address={}", stored.address);
    println!("network_id={}", stored.network_id);
    println!("status=created");
    Ok(())
}

fn run_import_mnemonic(args: &[String]) -> Result<(), String> {
    let config = default_config_from_args(args);
    let dir = resolve_keystore_dir(args, config.network);
    let passphrase = require_cli_passphrase(args, "--passphrase", "ALVENQIS_HOST_PASSPHRASE")?;
    let phrase = arg_value(args, "--mnemonic")
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "--mnemonic \"word1 word2 ...\" is required".to_owned())?;
    let account =
        WalletAccount::from_mnemonic(config.network, &phrase, "", WalletDerivationPath::default())
            .map_err(|e| e.to_string())?;
    let stored = create_encrypted_wallet(&dir, config.network, &passphrase, &account)?;
    println!("address={}", stored.address);
    println!("network_id={}", stored.network_id);
    println!("status=imported");
    println!("path={}", keystore_path(&dir).display());
    Ok(())
}

fn run_change_passphrase(args: &[String]) -> Result<(), String> {
    let config = default_config_from_args(args);
    let dir = resolve_keystore_dir(args, config.network);
    let old = require_cli_passphrase(args, "--old-passphrase", "ALVENQIS_HOST_OLD_PASSPHRASE")
        .or_else(|_| require_cli_passphrase(args, "--passphrase", "ALVENQIS_HOST_PASSPHRASE"))?;
    let new = require_cli_passphrase(args, "--new-passphrase", "ALVENQIS_HOST_NEW_PASSPHRASE")?;
    let view = change_passphrase(&dir, config.network, &old, &new)?;
    println!("address={}", view.address);
    println!("status=passphrase_changed");
    println!("path={}", view.keystore_path);
    Ok(())
}

fn run_delete_wallet(args: &[String]) -> Result<(), String> {
    let config = default_config_from_args(args);
    let dir = resolve_keystore_dir(args, config.network);
    let passphrase = require_cli_passphrase(args, "--passphrase", "ALVENQIS_HOST_PASSPHRASE")?;
    let _ = unlock_wallet(&dir, config.network, &passphrase)?;
    delete_wallet(&dir, config.network, &passphrase)?;
    println!("status=deleted");
    println!("path={}", keystore_path(&dir).display());
    Ok(())
}

fn resolve_keystore_dir(args: &[String], network: Network) -> PathBuf {
    if let Some(dir) = arg_value(args, "--keystore-dir") {
        PathBuf::from(dir)
    } else {
        default_keystore_dir(network).unwrap_or_else(|_| PathBuf::from("."))
    }
}

fn require_cli_passphrase(args: &[String], flag: &str, env_name: &str) -> Result<String, String> {
    if let Some(value) = arg_value(args, flag) {
        if !value.is_empty() {
            return Ok(value.to_owned());
        }
    }
    match env::var(env_name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(format!(
            "passphrase required via {flag} <text> or env {env_name}"
        )),
    }
}

fn arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}

fn run_native_messaging(state: &mut HostState) {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    loop {
        match native_messaging::read_message::<Request, _>(&mut stdin) {
            Ok(Some(request)) => {
                let response = state.handle(request);
                if let Err(error) = native_messaging::write_message(&mut stdout, &response) {
                    eprintln!("write failed: {error}");
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                eprintln!("read failed: {error}");
                break;
            }
        }
    }
}

fn run_jsonl(state: &mut HostState) {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<Request>(trimmed) {
                    Ok(request) => {
                        let response = state.handle(request);
                        if let Err(error) =
                            native_messaging::write_jsonl_message(&mut stdout, &response)
                        {
                            eprintln!("write failed: {error}");
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = writeln!(
                            stdout,
                            "{{\"id\":null,\"ok\":false,\"error\":\"invalid request: {error}\"}}"
                        );
                        let _ = stdout.flush();
                    }
                }
            }
            Err(error) => {
                eprintln!("read failed: {error}");
                break;
            }
        }
    }
}
