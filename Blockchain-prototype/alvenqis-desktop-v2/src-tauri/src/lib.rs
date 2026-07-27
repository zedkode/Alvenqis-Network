mod availability;
mod commands;
mod error;
mod health;
mod keystore;
mod logs;
mod network;
mod notify;
mod pool;
mod process;
mod rpc;
mod settings;
mod workspace;

use notify::NotifyState;
use tauri::Manager;
use workspace::{migrate_legacy_user_data, set_resource_root};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(NotifyState::default())
        .setup(|app| {
            if let Ok(resource_dir) = app.path().resource_dir() {
                set_resource_root(resource_dir);
            }
            migrate_legacy_user_data()?;
            health::ensure_runtime_dirs();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::network_snapshot,
            commands::network_add_seed,
            commands::wallet_metadata,
            commands::wallet_list,
            commands::wallet_select,
            commands::wallet_create,
            commands::wallet_import,
            commands::wallet_remove,
            commands::tx_prepare,
            commands::tx_sign_submit,
            commands::operator_run,
            commands::logs_recent,
            commands::logs_export,
            commands::miner_devices,
            commands::miner_run_command,
            commands::explorer_open,
            commands::explorer_lookup,
            commands::pool_snapshot,
            commands::pool_catalog,
            commands::settings_rpc,
            commands::settings_set_rpc_url,
            commands::settings_get,
            commands::settings_update,
            commands::settings_reset,
            commands::settings_defaults,
            commands::settings_paths,
            commands::settings_diagnostics,
            commands::settings_open_path,
            commands::runtime_health,
            commands::app_workspace,
            commands::app_version,
            commands::app_minimize,
            commands::app_maximize,
            commands::app_close,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            // Avoid an uncaught panic at the process boundary; surface a clear fatal exit.
            eprintln!("Alvenqis Control Center failed to start: {error}");
            std::process::exit(1);
        });
}
