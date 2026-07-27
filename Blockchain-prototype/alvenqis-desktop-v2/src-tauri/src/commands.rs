use crate::error::AppResult;
use crate::health::{self, RuntimeHealth};
use crate::keystore::{
    self, PreparedTransaction, SubmissionResult, WalletCreateResult, WalletMetadata,
};
use crate::network;
use crate::notify::{self, NotifyState};
use crate::process::{
    self, log_bytes, metrics_present, node_config_present, pid_present, MinerStartOptions,
};
use crate::rpc::{self, NetworkSnapshot};
use crate::settings::{self, AppSettings, DEFAULT_RPC_URL};
use crate::workspace::{
    find_workspace_root, keystore_helper_path, local_root, settings_path, user_data_dir,
};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
pub struct RpcSettings {
    pub rpc_url: String,
    pub default_rpc_url: String,
}

#[derive(Serialize)]
pub struct PathInfo {
    pub workspace: String,
    pub local_root: String,
    pub user_data: String,
    pub settings_file: String,
    pub logs_dir: String,
    pub chain_data_hint: String,
    pub keystore_helper: String,
    pub platform: String,
    pub app_version: String,
    pub packaged: bool,
}

#[derive(Serialize)]
pub struct DiagnosticsInfo {
    pub node_pid_present: bool,
    pub rpc_pid_present: bool,
    pub miner_pid_present: bool,
    pub explorer_pid_present: bool,
    pub node_log_bytes: u64,
    pub rpc_log_bytes: u64,
    pub miner_log_bytes: u64,
    pub explorer_log_bytes: u64,
    pub metrics_present: bool,
    pub node_config_present: bool,
}

fn platform() -> String {
    if cfg!(windows) {
        "windows".into()
    } else if cfg!(target_os = "linux") {
        "linux".into()
    } else {
        "other".into()
    }
}

#[tauri::command]
pub async fn network_snapshot(
    app: AppHandle,
    notify: State<'_, NotifyState>,
) -> AppResult<NetworkSnapshot> {
    let wallet = keystore::metadata().await.ok().flatten();
    let snapshot = rpc::network_snapshot(wallet.clone()).await;
    notify::maybe_notify_mined_blocks(
        &app,
        &notify,
        &snapshot,
        wallet.as_ref().map(|w| w.address.as_str()),
    );
    Ok(snapshot)
}

#[tauri::command]
pub fn network_add_seed(seed: String) -> AppResult<String> {
    network::add_seed(&seed)
}

#[tauri::command]
pub async fn wallet_metadata() -> AppResult<Option<WalletMetadata>> {
    keystore::metadata().await
}

#[tauri::command]
pub async fn wallet_list() -> AppResult<Vec<WalletMetadata>> {
    keystore::list().await
}

#[tauri::command]
pub async fn wallet_select(wallet_id: String) -> AppResult<WalletMetadata> {
    keystore::select(&wallet_id).await
}

#[tauri::command]
pub async fn wallet_create(display_name: String) -> AppResult<WalletCreateResult> {
    keystore::create(&display_name).await
}

/// Import recovery phrase (in-app paste) via parent-token keystore helper.
#[tauri::command]
pub async fn wallet_import(
    display_name: String,
    recovery_phrase: Option<String>,
) -> AppResult<WalletMetadata> {
    if let Some(phrase) = recovery_phrase {
        keystore::import_phrase(&display_name, &phrase).await
    } else {
        keystore::import_native(&display_name).await
    }
}

#[tauri::command]
pub async fn wallet_remove() -> AppResult<()> {
    keystore::remove().await
}

#[tauri::command]
pub async fn tx_prepare(
    recipient: String,
    amount: String,
    tip: String,
) -> AppResult<PreparedTransaction> {
    keystore::prepare(&recipient, &amount, &tip).await
}

#[tauri::command]
pub async fn tx_sign_submit(
    prepared: PreparedTransaction,
    confirmed: bool,
) -> AppResult<SubmissionResult> {
    if !confirmed {
        return Err(crate::error::AppError::msg(
            "Confirm the transaction details before signing.",
        ));
    }
    keystore::sign_submit(prepared).await
}

#[tauri::command]
pub async fn operator_run(
    command: String,
    miner_options: Option<MinerStartOptions>,
) -> AppResult<String> {
    let wallet = keystore::metadata().await.ok().flatten();
    process::run_operator(
        &command,
        wallet.as_ref().map(|w| w.address.as_str()),
        miner_options,
    )
    .await
}

#[tauri::command]
pub fn logs_recent(service: String, lines: Option<usize>) -> AppResult<String> {
    crate::logs::recent(&service, lines)
}

#[tauri::command]
pub async fn miner_devices() -> AppResult<serde_json::Value> {
    process::list_mining_devices().await
}

#[tauri::command]
pub async fn miner_run_command(line: String) -> AppResult<String> {
    process::run_miner_cli(&line).await
}

#[tauri::command]
pub async fn logs_export(app: AppHandle, service: String) -> AppResult<Option<String>> {
    let content = crate::logs::export_content(&service)?;
    let file_path = app
        .dialog()
        .file()
        .set_title(format!("Export {service} logs"))
        .set_file_name(format!("alvenqis-{service}.log"))
        .add_filter("Log files", &["log", "txt"])
        .blocking_save_file();
    let Some(path) = file_path else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|err| crate::error::AppError::msg(err.to_string()))?;
    std::fs::write(&path, content)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn explorer_open(path: String) -> AppResult<()> {
    let url = explorer_url(&settings::get_explorer_url(), &path)?;
    open::that(url.as_str()).map_err(|err| crate::error::AppError::msg(err.to_string()))?;
    Ok(())
}

fn explorer_url(explorer: &str, path: &str) -> AppResult<url::Url> {
    let clean = path.trim().trim_start_matches('/');
    let base = format!("{}/", explorer.trim_end_matches('/'));
    let url = if clean.is_empty() {
        url::Url::parse(&explorer)?
    } else {
        url::Url::parse(&base)?.join(clean)?
    };
    if url.origin().ascii_serialization()
        != url::Url::parse(&explorer)?.origin().ascii_serialization()
    {
        return Err(crate::error::AppError::msg(
            "Explorer path escapes the configured public Explorer origin",
        ));
    }
    Ok(url)
}

/// In-app safe lookup: height, hash, alve1 address, peer id, pool worker (public data only).
#[tauri::command]
pub async fn explorer_lookup(query: String) -> AppResult<serde_json::Value> {
    rpc::explorer_lookup(&query).await
}

/// Full public snapshot for one mining pool (status + history + optional miner row).
#[tauri::command]
pub async fn pool_snapshot(
    pool_url: Option<String>,
    miner_address: Option<String>,
) -> AppResult<serde_json::Value> {
    crate::pool::pool_snapshot(pool_url, miner_address).await
}

/// Probe all configured pool URLs for multi-pool selection UI.
#[tauri::command]
pub async fn pool_catalog() -> AppResult<serde_json::Value> {
    crate::pool::pool_catalog().await
}

#[tauri::command]
pub fn settings_rpc() -> RpcSettings {
    RpcSettings {
        rpc_url: settings::get_rpc_url(),
        default_rpc_url: DEFAULT_RPC_URL.into(),
    }
}

#[tauri::command]
pub fn settings_set_rpc_url(value: String) -> AppResult<String> {
    settings::set_rpc_url(&value)
}

#[tauri::command]
pub fn settings_get() -> AppSettings {
    settings::get()
}

#[tauri::command]
pub fn settings_update(patch: serde_json::Value) -> AppResult<AppSettings> {
    settings::update(patch)
}

#[tauri::command]
pub fn settings_reset() -> AppResult<AppSettings> {
    settings::reset()
}

#[tauri::command]
pub fn settings_defaults() -> AppSettings {
    settings::defaults()
}

#[tauri::command]
pub fn settings_paths() -> AppResult<PathInfo> {
    let workspace = find_workspace_root()?;
    let local = local_root(&workspace);
    Ok(PathInfo {
        workspace: workspace.to_string_lossy().into_owned(),
        local_root: local.to_string_lossy().into_owned(),
        user_data: user_data_dir().to_string_lossy().into_owned(),
        settings_file: settings_path().to_string_lossy().into_owned(),
        logs_dir: local.join("logs").to_string_lossy().into_owned(),
        chain_data_hint: local.join("chain").to_string_lossy().into_owned(),
        keystore_helper: keystore_helper_path(&workspace)
            .to_string_lossy()
            .into_owned(),
        platform: platform(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        packaged: cfg!(not(debug_assertions)),
    })
}

#[tauri::command]
pub fn settings_diagnostics() -> DiagnosticsInfo {
    DiagnosticsInfo {
        node_pid_present: pid_present("node"),
        rpc_pid_present: pid_present("rpc"),
        miner_pid_present: pid_present("miner"),
        explorer_pid_present: pid_present("explorer"),
        node_log_bytes: log_bytes("node"),
        rpc_log_bytes: log_bytes("rpc"),
        miner_log_bytes: log_bytes("miner"),
        explorer_log_bytes: log_bytes("explorer"),
        metrics_present: metrics_present(),
        node_config_present: node_config_present(),
    }
}

#[tauri::command]
pub fn runtime_health() -> RuntimeHealth {
    health::check()
}

#[tauri::command]
pub async fn settings_open_path(app: AppHandle, kind: String) -> AppResult<()> {
    // Allowlist of path *kinds* only — the frontend never supplies filesystem paths.
    let workspace = find_workspace_root()?;
    let local = local_root(&workspace);
    let path = match kind.as_str() {
        "workspace" => workspace.clone(),
        "local_root" => local.clone(),
        "logs" => local.join("logs"),
        "user_data" => user_data_dir(),
        "settings_file" => settings_path(),
        _ => return Err(crate::error::AppError::msg(
            "Unknown path kind (allowed: workspace, local_root, logs, user_data, settings_file)",
        )),
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(if path.is_file() { parent } else { &path });
    }
    // Ensure we only open directories/files under workspace, local data, or user data.
    let roots = [workspace, local, user_data_dir()];
    let path_canon = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    let allowed = roots.iter().any(|root| {
        let root_c = std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
        path_canon.starts_with(&root_c) || path.starts_with(root)
    });
    if !allowed {
        return Err(crate::error::AppError::msg(
            "Refusing to open path outside Alvenqis data roots",
        ));
    }
    app.opener()
        .open_path(path.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|err| crate::error::AppError::msg(err.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn app_workspace() -> AppResult<String> {
    Ok(find_workspace_root()?.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

#[tauri::command]
pub fn app_minimize(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .minimize()
            .map_err(|err| crate::error::AppError::msg(err.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn app_maximize(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("main") {
        if window
            .is_maximized()
            .map_err(|err| crate::error::AppError::msg(err.to_string()))?
        {
            window
                .unmaximize()
                .map_err(|err| crate::error::AppError::msg(err.to_string()))?;
        } else {
            window
                .maximize()
                .map_err(|err| crate::error::AppError::msg(err.to_string()))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn app_close(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .close()
            .map_err(|err| crate::error::AppError::msg(err.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::explorer_url;

    #[test]
    fn explorer_url_keeps_routes_on_configured_origin() {
        let url = explorer_url("https://dohotstudio.com/explorer", "blocks/42").unwrap();
        assert_eq!(url.as_str(), "https://dohotstudio.com/explorer/blocks/42");
    }

    #[test]
    fn explorer_url_rejects_origin_escape() {
        assert!(explorer_url("https://dohotstudio.com/explorer", "https://example.com").is_err());
    }
}
