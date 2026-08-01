use crate::workspace::{
    find_workspace_root, keystore_helper_path, local_root, settings_path, user_data_dir,
};
use serde::Serialize;
use std::fs;
use std::net::TcpListener;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct RuntimeHealth {
    pub ok: bool,
    pub packaged: bool,
    pub workspace: String,
    pub workspace_ok: bool,
    pub local_root: String,
    pub local_root_writable: bool,
    pub local_root_name_ok: bool,
    pub user_data: String,
    pub user_data_writable: bool,
    pub settings_file: String,
    pub settings_writable: bool,
    pub keystore_helper: String,
    pub keystore_helper_ok: bool,
    pub operator_script: String,
    pub operator_script_ok: bool,
    pub local_operator_script_ok: bool,
    pub bundled_node_ok: bool,
    pub bundled_rpc_ok: bool,
    pub bundled_miner_ok: bool,
    pub bundled_indexer_ok: bool,
    pub configs_ok: bool,
    pub rpc_port_free: bool,
    pub p2p_port_free: bool,
    pub issues: Vec<String>,
}

fn writable_dir(path: &Path) -> bool {
    if fs::create_dir_all(path).is_err() {
        return false;
    }
    let probe = path.join(".alvenqis-write-probe");
    match fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

fn bin_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn port_free(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Chain storage must live under a directory named `.alvenqis-local` or `.alvenqis-mainnet`.
pub fn local_root_name_ok(local: &Path) -> bool {
    local.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s == ".alvenqis-local" || s == ".alvenqis-mainnet"
    })
}

pub fn ensure_runtime_dirs() {
    let user = user_data_dir();
    let _ = fs::create_dir_all(&user);
    if let Ok(workspace) = find_workspace_root() {
        let local = local_root(&workspace);
        let _ = fs::create_dir_all(local.join("logs"));
        let _ = fs::create_dir_all(local.join("chain"));
        let _ = fs::create_dir_all(local.join("mempool"));
        let _ = fs::create_dir_all(local.join("indexer"));
        let _ = fs::create_dir_all(local.join("miner"));
        let _ = fs::create_dir_all(local.join("backups"));
    }
}

fn empty_failed(packaged: bool, issues: Vec<String>) -> RuntimeHealth {
    RuntimeHealth {
        ok: false,
        packaged,
        workspace: String::new(),
        workspace_ok: false,
        local_root: String::new(),
        local_root_writable: false,
        local_root_name_ok: false,
        user_data: user_data_dir().to_string_lossy().into_owned(),
        user_data_writable: writable_dir(&user_data_dir()),
        settings_file: settings_path().to_string_lossy().into_owned(),
        settings_writable: false,
        keystore_helper: String::new(),
        keystore_helper_ok: false,
        operator_script: String::new(),
        operator_script_ok: false,
        local_operator_script_ok: false,
        bundled_node_ok: false,
        bundled_rpc_ok: false,
        bundled_miner_ok: false,
        bundled_indexer_ok: false,
        configs_ok: false,
        rpc_port_free: port_free(10787),
        p2p_port_free: port_free(20787),
        issues,
    }
}

pub fn check() -> RuntimeHealth {
    let packaged = cfg!(not(debug_assertions));
    let mut issues = Vec::new();

    let workspace = match find_workspace_root() {
        Ok(path) => path,
        Err(err) => return empty_failed(packaged, vec![format!("workspace: {err}")]),
    };

    let local = local_root(&workspace);
    let user_data = user_data_dir();
    let settings = settings_path();
    let helper = keystore_helper_path(&workspace);
    let operator = if cfg!(windows) {
        workspace.join("alvenqis.ps1")
    } else {
        workspace.join("alvenqis.sh")
    };
    let local_operator = if cfg!(windows) {
        workspace
            .join("scripts")
            .join("local")
            .join("alvenqis-local.ps1")
    } else {
        workspace.join("scripts").join("local").join("start-all.sh")
    };

    let bin = workspace.join("bin");
    let bundled_node_ok = bin.join(bin_name("alvenqis-node")).exists();
    let bundled_rpc_ok = bin.join(bin_name("alvenqis-rpc-gateway")).exists();
    let bundled_miner_ok = bin.join(bin_name("alvenqis-miner")).exists();
    let bundled_indexer_ok = bin.join(bin_name("alvenqis-indexer")).exists();
    let configs_ok = workspace
        .join("configs")
        .join("mainnet-candidate.toml")
        .exists()
        || workspace.join("configs").join("local.toml").exists();

    let local_root_writable = writable_dir(&local);
    let local_root_name_ok = local_root_name_ok(&local);
    let user_data_writable = writable_dir(&user_data);
    let settings_writable = settings.parent().map(writable_dir).unwrap_or(false);
    let keystore_helper_ok = helper.exists();
    let operator_script_ok = operator.exists();
    let local_operator_script_ok = local_operator.exists();
    let rpc_port_free = port_free(10787);
    let p2p_port_free = port_free(20787);

    if !keystore_helper_ok {
        issues.push(format!(
            "keystore helper missing at {} (run: npm run prepare:native)",
            helper.display()
        ));
    }
    if !operator_script_ok {
        issues.push(format!("operator script missing at {}", operator.display()));
    }
    if !local_operator_script_ok {
        issues.push(format!(
            "local operator script missing at {}",
            local_operator.display()
        ));
    }
    if !local_root_writable {
        issues.push(format!("local root not writable: {}", local.display()));
    }
    if !local_root_name_ok {
        issues.push(format!(
            "local root must include .alvenqis-local or .alvenqis-mainnet path segment: {}",
            local.display()
        ));
    }
    if !user_data_writable {
        issues.push(format!("user data not writable: {}", user_data.display()));
    }
    if !settings_writable {
        issues.push(format!(
            "settings directory not writable for {}",
            settings.display()
        ));
    }
    if packaged {
        if !bundled_node_ok {
            issues.push("packaged build missing bin/alvenqis-node".into());
        }
        if !bundled_rpc_ok {
            issues.push("packaged build missing bin/alvenqis-rpc-gateway".into());
        }
        if !bundled_miner_ok {
            issues.push("packaged build missing bin/alvenqis-miner".into());
        }
        if !bundled_indexer_ok {
            issues.push("packaged build missing bin/alvenqis-indexer".into());
        }
        if !configs_ok {
            issues.push("packaged build missing configs".into());
        }
    }
    // Port conflicts are warnings for health.ok in dev (external install may be running).
    // They still surface so operators know start will fail.
    if !p2p_port_free {
        issues.push(
            "P2P port 20787 is already in use (another Alvenqis node is running). Stop it before start."
                .into(),
        );
    }
    if !rpc_port_free {
        issues.push(
            "RPC port 10787 is already in use (another RPC gateway is running). Stop it before start."
                .into(),
        );
    }

    // VPS-first Control Center: local stack operator scripts are optional.
    // Required: keystore helper, writable data dirs, and (when packaged) the miner binary.
    let core_ok = keystore_helper_ok
        && local_root_writable
        && local_root_name_ok
        && user_data_writable
        && settings_writable
        && (!packaged || (bundled_miner_ok && keystore_helper_ok));

    RuntimeHealth {
        ok: core_ok,
        packaged,
        workspace: workspace.to_string_lossy().into_owned(),
        workspace_ok: true,
        local_root: local.to_string_lossy().into_owned(),
        local_root_writable,
        local_root_name_ok,
        user_data: user_data.to_string_lossy().into_owned(),
        user_data_writable,
        settings_file: settings.to_string_lossy().into_owned(),
        settings_writable,
        keystore_helper: helper.to_string_lossy().into_owned(),
        keystore_helper_ok,
        operator_script: operator.to_string_lossy().into_owned(),
        operator_script_ok,
        local_operator_script_ok,
        bundled_node_ok,
        bundled_rpc_ok,
        bundled_miner_ok,
        bundled_indexer_ok,
        configs_ok,
        rpc_port_free,
        p2p_port_free,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::local_root_name_ok;
    use std::path::PathBuf;

    #[test]
    fn accepts_dot_alvenqis_local_segment() {
        let path = PathBuf::from("test-root")
            .join("Alvenqis")
            .join("ControlCenter")
            .join(".alvenqis-local");
        assert!(local_root_name_ok(&path));
    }

    #[cfg(windows)]
    #[test]
    fn accepts_windows_absolute_dot_alvenqis_local_segment() {
        let path =
            PathBuf::from(r"C:\Users\x\AppData\Local\Alvenqis\ControlCenter\.alvenqis-local");
        assert!(local_root_name_ok(&path));
    }

    #[test]
    fn rejects_temp_without_required_segment() {
        let path = PathBuf::from("test-root")
            .join("Temp")
            .join("alvenqis-critical-smoke-local");
        assert!(!local_root_name_ok(&path));
    }
}
