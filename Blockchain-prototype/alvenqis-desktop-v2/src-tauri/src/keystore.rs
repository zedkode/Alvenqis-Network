use crate::error::{AppError, AppResult};
use crate::settings::get_rpc_url;
use crate::workspace::{find_workspace_root, keystore_helper_path};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub wallet_id: String,
    pub display_name: String,
    pub schema: String,
    pub network_id: String,
    pub address: String,
    pub public_key_hex: String,
    pub key_origin: String,
    pub derivation_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletCreateResult {
    pub metadata: WalletMetadata,
    pub recovery_confirmed: bool,
    /// One-time BIP39 phrase for in-app backup UI (never persisted by the shell).
    #[serde(default)]
    pub recovery_phrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedTransaction {
    pub recipient: String,
    pub amount_atomic: String,
    pub tip_atomic: String,
    pub base_fee_atomic: String,
    pub total_atomic: String,
    pub available_atomic: String,
    pub nonce: u64,
    pub chain_tip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResult {
    pub tx_hash: String,
    pub lifecycle_status: String,
    pub mempool_size: u64,
}

/// Spawn keystore helper with a one-shot parent token so local processes cannot
/// invoke the staged binary without the token (audit A-H08).
fn parent_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // 128-bit-ish token from time + random-ish process bits
    format!(
        "{:x}-{:x}-{:x}",
        nanos,
        std::process::id(),
        nanos.wrapping_mul(0x9e37_79b9_7f4a_7c15)
    )
}

async fn invoke<T: DeserializeOwned>(mut request: serde_json::Value) -> AppResult<T> {
    let workspace = find_workspace_root()?;
    let helper = keystore_helper_path(&workspace);
    if !helper.exists() {
        return Err(AppError::msg(format!(
            "Keystore helper not found at {}. Build it with: npm run prepare:native",
            helper.display()
        )));
    }

    let token = parent_token();
    if let Some(obj) = request.as_object_mut() {
        obj.insert("parent_token".into(), json!(token));
    }

    let mut cmd = Command::new(&helper);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("ALVENQIS_RPC_URL", get_rpc_url())
        .env("ALVENQIS_KEYSTORE_PARENT_TOKEN", &token)
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|err| {
        AppError::msg(format!(
            "Failed to spawn keystore helper at {}: {err}",
            helper.display()
        ))
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload = serde_json::to_vec(&request)?;
        stdin.write_all(&payload).await?;
        stdin.shutdown().await?;
    }

    let output = child.wait_with_output().await?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Err(AppError::msg(if stderr.is_empty() {
            format!(
                "Keystore helper failed with exit code {:?}",
                output.status.code()
            )
        } else {
            stderr
        }));
    }
    if output.stdout.is_empty() || output.stdout == b"null" {
        return serde_json::from_value(json!(null)).map_err(Into::into);
    }
    serde_json::from_slice(&output.stdout).map_err(|err| {
        AppError::msg(format!(
            "Keystore helper returned invalid data: {err}; stderr={stderr}"
        ))
    })
}

pub async fn metadata() -> AppResult<Option<WalletMetadata>> {
    invoke(json!({ "command": "metadata" })).await
}

pub async fn list() -> AppResult<Vec<WalletMetadata>> {
    invoke(json!({ "command": "list" })).await
}

pub async fn select(wallet_id: &str) -> AppResult<WalletMetadata> {
    invoke(json!({ "command": "select", "wallet_id": wallet_id })).await
}

pub async fn create(display_name: &str) -> AppResult<WalletCreateResult> {
    invoke(json!({ "command": "create", "display_name": display_name })).await
}

/// Native OS dialog import (legacy path).
pub async fn import_native(display_name: &str) -> AppResult<WalletMetadata> {
    invoke(json!({
        "command": "import",
        "display_name": display_name
    }))
    .await
}

/// In-app paste import — phrase travels only through the parent-token keystore helper.
pub async fn import_phrase(display_name: &str, recovery_phrase: &str) -> AppResult<WalletMetadata> {
    invoke(json!({
        "command": "import_phrase",
        "display_name": display_name,
        "recovery_phrase": recovery_phrase
    }))
    .await
}

pub async fn remove() -> AppResult<()> {
    let _: Option<WalletMetadata> = invoke(json!({ "command": "remove" })).await?;
    Ok(())
}

pub async fn prepare(recipient: &str, amount: &str, tip: &str) -> AppResult<PreparedTransaction> {
    let workspace = find_workspace_root()?;
    invoke(json!({
        "command": "prepare",
        "workspace": workspace,
        "recipient": recipient,
        "amount": amount,
        "tip": tip
    }))
    .await
}

pub async fn sign_submit(prepared: PreparedTransaction) -> AppResult<SubmissionResult> {
    let workspace = find_workspace_root()?;
    invoke(json!({
        "command": "sign_submit",
        "workspace": workspace,
        "prepared": prepared
    }))
    .await
}
