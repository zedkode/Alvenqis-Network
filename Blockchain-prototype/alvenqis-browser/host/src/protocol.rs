use crate::confirm::{confirm_recovery_backup, confirm_send};
use crate::keystore::{
    change_passphrase, create_encrypted_wallet, default_keystore_dir, delete_wallet, export_public,
    keystore_exists, keystore_path, load_stored, unlock_wallet,
};
use alvenqis_sdk_rust::{
    Amount, BlockingRpcClient, MnemonicWordCount, Network, NetworkConfig, SignedTransfer,
    TransferBuilder, WalletAccount, ATOMIC_UNITS_PER_ALVE,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use zeroize::Zeroize;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub id: Value,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            id,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Value, error: impl Into<String>) -> Self {
        Self {
            id,
            ok: false,
            result: None,
            error: Some(error.into()),
        }
    }
}

pub struct HostState {
    pub config: NetworkConfig,
    pub rpc: BlockingRpcClient,
    pub session: Option<WalletAccount>,
    pub keystore_dir: PathBuf,
    /// When true, prepare_and_sign / send require OS (or env) confirmation.
    pub require_os_confirm: bool,
}

impl HostState {
    pub fn new(config: NetworkConfig) -> Result<Self, String> {
        let keystore_dir = default_keystore_dir(config.network)?;
        let rpc = BlockingRpcClient::new(config.clone()).map_err(|e| e.to_string())?;
        Ok(Self {
            config,
            rpc,
            session: None,
            keystore_dir,
            // Secure default: every sign/send/submit requires OS (or env) confirmation (CR-C02).
            require_os_confirm: true,
        })
    }

    pub fn with_keystore_dir(mut self, dir: PathBuf) -> Self {
        self.keystore_dir = dir;
        self
    }

    pub fn with_require_os_confirm(mut self, enabled: bool) -> Self {
        self.require_os_confirm = enabled;
        self
    }

    pub fn handle(&mut self, request: Request) -> Response {
        let id = request.id.clone();
        match request.method.as_str() {
            "ping" => Response::ok(
                id,
                serde_json::json!({
                    "service": "alvenqis-browser-host",
                    "version": env!("CARGO_PKG_VERSION"),
                    "status": "Mainnet Candidate / Prototype",
                    "require_os_confirm": self.require_os_confirm,
                }),
            ),
            "network_info" => Response::ok(
                id,
                serde_json::json!({
                    "network_id": self.config.network_id(),
                    "status_label": self.config.status_label(),
                    "address_prefix": self.config.address_prefix(),
                    "rpc_base_url": self.config.rpc_base_url,
                }),
            ),
            "keystore_status" => {
                let exists = keystore_exists(&self.keystore_dir);
                let public = if exists {
                    load_stored(&self.keystore_dir).ok().map(|w| {
                        serde_json::json!({
                            "address": w.address,
                            "network_id": w.network_id,
                            "status_label": w.status_label,
                        })
                    })
                } else {
                    None
                };
                Response::ok(
                    id,
                    serde_json::json!({
                        "exists": exists,
                        "path": keystore_path(&self.keystore_dir).display().to_string(),
                        "unlocked": self.session.is_some(),
                        "session_address": self.session.as_ref().map(|a| a.address_string()),
                        "public": public,
                        "require_os_confirm": self.require_os_confirm,
                        "warning": "Passphrase crosses native messaging briefly; mnemonic never leaves the host. Sign/send/submit require OS confirm by default."
                    }),
                )
            }
            "session_status" => {
                let (unlocked, address) = match &self.session {
                    Some(account) => (true, Some(account.address_string())),
                    None => (false, None),
                };
                Response::ok(
                    id,
                    serde_json::json!({
                        "unlocked": unlocked,
                        "address": address,
                        "keystore_exists": keystore_exists(&self.keystore_dir),
                        "warning": "Unlocked session is in host RAM only until lock/clear."
                    }),
                )
            }
            "create_wallet" => {
                let passphrase = match require_passphrase(&request.params) {
                    Ok(p) => p,
                    Err(e) => return Response::err(id, e),
                };
                if keystore_exists(&self.keystore_dir) {
                    return Response::err(
                        id,
                        "keystore already exists; unlock it or remove the file manually",
                    );
                }
                match WalletAccount::generate(self.config.network, MnemonicWordCount::Twelve) {
                    Ok((account, mut mnemonic)) => {
                        // CR-C01: refuse spendable keystore without one-time OS/host recovery display.
                        // Mnemonic is NEVER returned in native-messaging JSON.
                        let ack = confirm_recovery_backup(&mnemonic.phrase);
                        // Zeroize phrase in process memory as soon as acknowledgement completes.
                        mnemonic.phrase.zeroize();
                        drop(mnemonic);
                        if let Err(error) = ack {
                            return Response::err(id, error);
                        }
                        match create_encrypted_wallet(
                            &self.keystore_dir,
                            self.config.network,
                            &passphrase,
                            &account,
                        ) {
                            Ok(stored) => {
                                let address = account.address_string();
                                self.session = Some(account);
                                Response::ok(
                                    id,
                                    serde_json::json!({
                                        "address": stored.address,
                                        "path": keystore_path(&self.keystore_dir).display().to_string(),
                                        "unlocked": true,
                                        "session_address": address,
                                        "recovery_acknowledged": true,
                                        "mnemonic_returned": false,
                                        "warning": "Encrypted keystore created. Recovery phrase was shown once via host OS/stderr confirm and is NOT returned to the extension. Prefer host CLI --init-wallet for offline backup workflows."
                                    }),
                                )
                            }
                            Err(error) => Response::err(id, error),
                        }
                    }
                    Err(error) => Response::err(id, error.to_string()),
                }
            }
            "export_public" => match export_public(&self.keystore_dir) {
                Ok(view) => match serde_json::to_value(view) {
                    Ok(value) => Response::ok(id, value),
                    Err(error) => Response::err(id, error.to_string()),
                },
                Err(error) => Response::err(id, error),
            },
            "change_passphrase" => {
                let old_passphrase = request
                    .params
                    .get("old_passphrase")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let new_passphrase = request
                    .params
                    .get("new_passphrase")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if old_passphrase.is_empty() || new_passphrase.is_empty() {
                    return Response::err(
                        id,
                        "params.old_passphrase and params.new_passphrase are required",
                    );
                }
                match change_passphrase(
                    &self.keystore_dir,
                    self.config.network,
                    old_passphrase,
                    new_passphrase,
                ) {
                    Ok(view) => {
                        // Force re-unlock with the new passphrase.
                        self.session = None;
                        match serde_json::to_value(view) {
                            Ok(mut value) => {
                                if let Some(obj) = value.as_object_mut() {
                                    obj.insert("unlocked".to_owned(), serde_json::json!(false));
                                    obj.insert(
                                        "warning".to_owned(),
                                        serde_json::json!(
                                            "Passphrase changed. Session locked; unlock with the new passphrase."
                                        ),
                                    );
                                }
                                Response::ok(id, value)
                            }
                            Err(error) => Response::err(id, error.to_string()),
                        }
                    }
                    Err(error) => Response::err(id, error),
                }
            }
            "delete_wallet" => {
                let passphrase = match require_passphrase(&request.params) {
                    Ok(p) => p,
                    Err(e) => return Response::err(id, e),
                };
                match delete_wallet(&self.keystore_dir, self.config.network, &passphrase) {
                    Ok(()) => {
                        self.session = None;
                        Response::ok(
                            id,
                            serde_json::json!({
                                "deleted": true,
                                "warning": "Keystore file removed. Recovery requires a mnemonic backed up via host CLI --init-wallet / --import-mnemonic."
                            }),
                        )
                    }
                    Err(error) => Response::err(id, error),
                }
            }
            "unlock" => {
                let passphrase = match require_passphrase(&request.params) {
                    Ok(p) => p,
                    Err(e) => return Response::err(id, e),
                };
                match unlock_wallet(&self.keystore_dir, self.config.network, &passphrase) {
                    Ok(account) => {
                        let address = account.address_string();
                        self.session = Some(account);
                        Response::ok(
                            id,
                            serde_json::json!({
                                "unlocked": true,
                                "address": address
                            }),
                        )
                    }
                    Err(error) => Response::err(id, error),
                }
            }
            "lock" => {
                self.session = None;
                Response::ok(id, serde_json::json!({ "unlocked": false }))
            }
            // CR-C01: ephemeral sessions discarded the only recovery material and allowed
            // permanent fund loss. Creation is only via create_wallet (OS recovery ack) or
            // host CLI --init-wallet / --import-mnemonic.
            "create_session" => Response::err(
                id,
                "create_session is disabled: it discarded the recovery phrase and risked permanent fund loss. \
Use create_wallet (OS recovery backup required) or host CLI --init-wallet / --import-mnemonic.",
            ),
            "clear_session" => {
                self.session = None;
                Response::ok(id, serde_json::json!({ "cleared": true }))
            }
            "rpc_status" => match self.rpc.status() {
                Ok(status) => match serde_json::to_value(status) {
                    Ok(value) => Response::ok(id, value),
                    Err(error) => Response::err(id, error.to_string()),
                },
                Err(error) => Response::err(id, error.to_string()),
            },
            "sync_status" => match self.rpc.sync_status() {
                Ok(status) => match serde_json::to_value(status) {
                    Ok(value) => Response::ok(id, value),
                    Err(error) => Response::err(id, error.to_string()),
                },
                Err(error) => Response::err(id, error.to_string()),
            },
            "mempool_status" => match self.rpc.mempool_status() {
                Ok(status) => match serde_json::to_value(status) {
                    Ok(value) => Response::ok(id, value),
                    Err(error) => Response::err(id, error.to_string()),
                },
                Err(error) => Response::err(id, error.to_string()),
            },
            "supply" => match self.rpc.supply() {
                Ok(status) => match serde_json::to_value(status) {
                    Ok(value) => Response::ok(id, value),
                    Err(error) => Response::err(id, error.to_string()),
                },
                Err(error) => Response::err(id, error.to_string()),
            },
            "block_latest" => match self.rpc.block_latest() {
                Ok(block) => match serde_json::to_value(block) {
                    Ok(value) => Response::ok(id, value),
                    Err(error) => Response::err(id, error.to_string()),
                },
                Err(error) => Response::err(id, error.to_string()),
            },
            "block_by_height" => {
                let height = request
                    .params
                    .get("height")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| "params.height is required".to_owned());
                match height {
                    Ok(height) => match self.rpc.block_by_height(height) {
                        Ok(block) => match serde_json::to_value(block) {
                            Ok(value) => Response::ok(id, value),
                            Err(error) => Response::err(id, error.to_string()),
                        },
                        Err(error) => Response::err(id, error.to_string()),
                    },
                    Err(error) => Response::err(id, error),
                }
            }
            "block_by_hash" => {
                let hash = request
                    .params
                    .get("hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                if hash.is_empty() {
                    Response::err(id, "params.hash is required")
                } else {
                    match self.rpc.block_by_hash(&hash) {
                        Ok(block) => match serde_json::to_value(block) {
                            Ok(value) => Response::ok(id, value),
                            Err(error) => Response::err(id, error.to_string()),
                        },
                        Err(error) => Response::err(id, error.to_string()),
                    }
                }
            }
            "recent_blocks" => {
                let count = request
                    .params
                    .get("count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as usize;
                match self.rpc.recent_blocks(count) {
                    Ok(blocks) => match serde_json::to_value(blocks) {
                        Ok(value) => Response::ok(id, value),
                        Err(error) => Response::err(id, error.to_string()),
                    },
                    Err(error) => Response::err(id, error.to_string()),
                }
            }
            "transaction_by_hash" => {
                let hash = request
                    .params
                    .get("hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                if hash.is_empty() {
                    Response::err(id, "params.hash is required")
                } else {
                    match self.rpc.transaction(&hash) {
                        Ok(tx) => match serde_json::to_value(tx) {
                            Ok(value) => Response::ok(id, value),
                            Err(error) => Response::err(id, error.to_string()),
                        },
                        Err(error) => Response::err(id, error.to_string()),
                    }
                }
            }
            "indexer_status" => match self.rpc.indexer_status() {
                Ok(status) => match serde_json::to_value(status) {
                    Ok(value) => Response::ok(id, value),
                    Err(error) => Response::err(id, error.to_string()),
                },
                Err(error) => Response::err(id, error.to_string()),
            },
            "indexer_summary" => match self.rpc.indexer_summary() {
                Ok(status) => match serde_json::to_value(status) {
                    Ok(value) => Response::ok(id, value),
                    Err(error) => Response::err(id, error.to_string()),
                },
                Err(error) => Response::err(id, error.to_string()),
            },
            "indexer_block_latest" => match self.rpc.indexer_block_latest() {
                Ok(block) => match serde_json::to_value(block) {
                    Ok(value) => Response::ok(id, value),
                    Err(error) => Response::err(id, error.to_string()),
                },
                Err(error) => Response::err(id, error.to_string()),
            },
            "indexer_address" => {
                let address = request
                    .params
                    .get("address")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                    .or_else(|| {
                        self.session
                            .as_ref()
                            .map(|account| account.address_string())
                    });
                match address {
                    Some(address) if !address.is_empty() => {
                        match self.rpc.indexer_address(&address) {
                            Ok(activity) => match serde_json::to_value(activity) {
                                Ok(value) => Response::ok(id, value),
                                Err(error) => Response::err(id, error.to_string()),
                            },
                            Err(error) => Response::err(id, error.to_string()),
                        }
                    }
                    _ => Response::err(
                        id,
                        "params.address is required (or unlock a session wallet)",
                    ),
                }
            }
            "balance" => self.with_session(id, |state, account| {
                let balance = state
                    .rpc
                    .balance(&account.address_string())
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(balance).map_err(|e| e.to_string())
            }),
            "account" => self.with_session(id, |state, account| {
                let snapshot = state
                    .rpc
                    .account(&account.address_string())
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(snapshot).map_err(|e| e.to_string())
            }),
            "prepare_and_sign" => {
                let params = request.params.clone();
                self.with_session(id, |state, account| {
                    maybe_confirm_send(state, account, &params)?;
                    let signed = prepare_signed(state, account, &params)?;
                    Ok(serde_json::json!({
                        "tx_hash": signed.tx_hash,
                        "transaction": signed.transaction,
                        "network_id": state.config.network_id(),
                        "status_label": state.config.status_label(),
                    }))
                })
            }
            "submit" => {
                // CR-C02 / CR-M22: submitting a signed tx is a fund-moving action — require
                // the same OS confirmation as sign/send. Session unlock is not strictly required
                // (broadcast of already-signed txs), but OS confirm is default-on.
                let tx_value = request
                    .params
                    .get("transaction")
                    .cloned()
                    .ok_or_else(|| "params.transaction is required".to_owned());
                match tx_value {
                    Ok(value) => {
                        match serde_json::from_value::<alvenqis_sdk_rust::Transaction>(value) {
                            Ok(tx) => {
                                let from = tx
                                    .from
                                    .as_deref()
                                    .unwrap_or("(unsigned / missing from)");
                                let summary = format!(
                                    "Confirm Alvenqis SUBMIT of already-signed transaction\n\n\
From: {from}\nTo: {}\nAmount atomic: {}\nMax fee atomic: {}\nNonce: {}\nRPC: {}\n\n\
Press OK to broadcast, Cancel to abort.",
                                    tx.to,
                                    tx.amount.as_atomic(),
                                    tx.max_fee.as_atomic(),
                                    tx.nonce,
                                    self.config.rpc_base_url
                                );
                                if let Err(error) = confirm_send(self.require_os_confirm, &summary)
                                {
                                    return Response::err(id, error);
                                }
                                match self.rpc.submit(&tx) {
                                    Ok(response) => match serde_json::to_value(response) {
                                        Ok(v) => Response::ok(id, v),
                                        Err(e) => Response::err(id, e.to_string()),
                                    },
                                    Err(e) => Response::err(id, e.to_string()),
                                }
                            }
                            Err(e) => Response::err(id, e.to_string()),
                        }
                    }
                    Err(e) => Response::err(id, e),
                }
            }
            "send" => {
                let params = request.params.clone();
                self.with_session(id, |state, account| {
                    maybe_confirm_send(state, account, &params)?;
                    let signed = prepare_signed(state, account, &params)?;
                    let submit = state
                        .rpc
                        .submit(signed.transaction())
                        .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!({
                        "tx_hash": signed.tx_hash,
                        "submit": submit,
                        "network_id": state.config.network_id(),
                        "status_label": state.config.status_label(),
                    }))
                })
            }
            other => Response::err(id, format!("unknown method: {other}")),
        }
    }

    fn with_session(
        &self,
        id: Value,
        f: impl FnOnce(&Self, &WalletAccount) -> Result<Value, String>,
    ) -> Response {
        match &self.session {
            Some(account) => match f(self, account) {
                Ok(result) => Response::ok(id, result),
                Err(error) => Response::err(id, error),
            },
            None => Response::err(id, "session locked; call unlock or create_wallet first"),
        }
    }
}

fn maybe_confirm_send(
    state: &HostState,
    account: &WalletAccount,
    params: &Value,
) -> Result<(), String> {
    let to = params
        .get("to")
        .and_then(|v| v.as_str())
        .unwrap_or("(missing to)");
    let amount = params
        .get("amount_alve")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
        .or_else(|| {
            params
                .get("amount_atomic")
                .and_then(|v| v.as_u64())
                .map(|n| format!("{n} atomic"))
        })
        .unwrap_or_else(|| "(missing amount)".to_owned());
    let summary = format!(
        "Confirm Alvenqis transfer (Mainnet Candidate)\n\nFrom: {}\nTo: {}\nAmount: {}\nRPC: {}\n\nPress OK to sign/submit, Cancel to abort.",
        account.address_string(),
        to,
        amount,
        state.config.rpc_base_url
    );
    confirm_send(state.require_os_confirm, &summary)
}

fn require_passphrase(params: &Value) -> Result<String, String> {
    let passphrase = params
        .get("passphrase")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    if passphrase.is_empty() {
        return Err("params.passphrase is required".to_owned());
    }
    Ok(passphrase)
}

fn prepare_signed(
    state: &HostState,
    account: &WalletAccount,
    params: &Value,
) -> Result<SignedTransfer, String> {
    let to = params
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "params.to is required".to_owned())?;

    let amount = if let Some(atomic) = params.get("amount_atomic").and_then(|v| v.as_u64()) {
        Amount::from_atomic(atomic)
    } else if let Some(amount_str) = params.get("amount_alve").and_then(|v| v.as_str()) {
        Amount::parse_alve(amount_str).map_err(|e| e.to_string())?
    } else {
        return Err("params.amount_atomic or params.amount_alve is required".to_owned());
    };

    let priority_fee =
        if let Some(atomic) = params.get("priority_fee_atomic").and_then(|v| v.as_u64()) {
            Amount::from_atomic(atomic)
        } else if let Some(amount_str) = params.get("priority_fee_alve").and_then(|v| v.as_str()) {
            Amount::parse_alve(amount_str).map_err(|e| e.to_string())?
        } else {
            Amount::from_atomic(1)
        };

    let account_snapshot = state
        .rpc
        .account(&account.address_string())
        .map_err(|e| e.to_string())?;
    let base_fee = Amount::from_atomic(account_snapshot.anticipated_base_fee_atomic.max(1));

    // Optional dry balance check (best-effort).
    let required = amount
        .checked_add(base_fee)
        .map_err(|e| e.to_string())?
        .checked_add(priority_fee)
        .map_err(|e| e.to_string())?;
    if account_snapshot.balance_atomic < required.as_atomic() {
        return Err(format!(
            "insufficient balance: have {} atomic, need at least {} atomic (amount+fees). 1 ALVE = {ATOMIC_UNITS_PER_ALVE} atomic",
            account_snapshot.balance_atomic,
            required.as_atomic()
        ));
    }

    TransferBuilder::new(state.config.network)
        .to(to)
        .map_err(|e| e.to_string())?
        .amount(amount)
        .map_err(|e| e.to_string())?
        .nonce(account_snapshot.next_nonce)
        .fees(base_fee, priority_fee)
        .map_err(|e| e.to_string())?
        .sign(account)
        .map_err(|e| e.to_string())
}

pub fn default_config_from_args(args: &[String]) -> NetworkConfig {
    let mut rpc = None;
    let mut local = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--rpc" => {
                if let Some(url) = args.get(i + 1) {
                    rpc = Some(url.clone());
                    i += 1;
                }
            }
            "--local" => local = true,
            _ => {}
        }
        i += 1;
    }
    if let Some(url) = rpc {
        NetworkConfig::with_rpc(Network::MainnetCandidate, url)
    } else if local {
        NetworkConfig::mainnet_candidate_local()
    } else {
        NetworkConfig::mainnet_candidate()
    }
}
