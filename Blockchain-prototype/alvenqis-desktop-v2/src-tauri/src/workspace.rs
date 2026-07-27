use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static RESOURCE_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Called once at app startup so packaged resource paths resolve correctly.
pub fn set_resource_root(path: PathBuf) {
    let _ = RESOURCE_ROOT.set(path);
}

/// Packaged Tauri resource directory, if registered at startup.
pub fn resource_root() -> Option<PathBuf> {
    RESOURCE_ROOT.get().cloned()
}

pub fn find_workspace_root() -> AppResult<PathBuf> {
    if let Ok(configured) = std::env::var("ALVENQIS_WORKSPACE_ROOT") {
        let path = PathBuf::from(configured);
        if is_workspace(&path) || has_operator_script(&path) {
            return Ok(path);
        }
    }

    // Dev preference: monorepo root (cargo workspace) wins over empty resource dirs.
    if cfg!(debug_assertions) {
        if let Some(monorepo) = monorepo_from_manifest() {
            return Ok(monorepo);
        }
    }

    if let Some(resource) = RESOURCE_ROOT.get() {
        for candidate in resource_candidates(resource) {
            if is_workspace(&candidate)
                || has_operator_script(&candidate)
                || has_bundled_node(&candidate)
                || has_bundled_miner(&candidate)
            {
                return Ok(candidate);
            }
        }
        // Last resort for NSIS installs: use the resource dir even if checks are partial.
        return Ok(resource.clone());
    }

    let mut current = std::env::current_dir()?;
    for _ in 0..12 {
        if is_workspace(&current) || has_operator_script(&current) {
            return Ok(current);
        }
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    if let Some(monorepo) = monorepo_from_manifest() {
        return Ok(monorepo);
    }

    Err(AppError::msg(
        "Alvenqis workspace could not be located. Set ALVENQIS_WORKSPACE_ROOT or run from the monorepo / packaged resources.",
    ))
}

fn monorepo_from_manifest() -> Option<PathBuf> {
    let from_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // alvenqis-desktop-v2/src-tauri -> alvenqis-desktop-v2 -> monorepo
    if let Some(parent) = from_crate.parent() {
        if is_workspace(parent) {
            return Some(parent.to_path_buf());
        }
        if let Some(grand) = parent.parent() {
            if is_workspace(grand) {
                return Some(grand.to_path_buf());
            }
        }
    }
    None
}

fn resource_candidates(resource: &Path) -> Vec<PathBuf> {
    let mut out = vec![resource.to_path_buf()];
    let nested = resource.join("resources");
    if nested.exists() {
        out.push(nested);
    }
    out
}

pub fn local_root(workspace: &Path) -> PathBuf {
    // Packaged builds keep miner metrics/logs outside the install directory.
    let packaged = has_bundled_node(workspace)
        || has_bundled_miner(workspace)
        || RESOURCE_ROOT.get().is_some();
    if packaged {
        if cfg!(windows) {
            let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
                dirs::home_dir()
                    .map(|home| {
                        home.join("AppData")
                            .join("Local")
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|| ".".into())
            });
            return PathBuf::from(local_app_data)
                .join("Alvenqis")
                .join("ControlCenter")
                .join(".alvenqis-local");
        }
        return user_data_dir().join(".alvenqis-local");
    }
    workspace.join(".alvenqis-local")
}

pub fn user_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Alvenqis")
        .join("ControlCenter")
}

/// Prior Control Center brand folders (Veiron → Vireon → Alvenqis).
/// Must stay DIFFERENT from the current `Alvenqis/ControlCenter` path.
const LEGACY_CONTROL_CENTER_BRANDS: &[&str] = &["Vireon", "Veiron"];
/// Prior monorepo local-stack directories under the workspace root.
const LEGACY_LOCAL_DIRS: &[&str] = &[".vireon-local", ".veiron-local"];

/// Preserve previous-brand profiles as rollback sources while copying missing
/// files into the Alvenqis location on first launch. Existing Alvenqis files always win.
pub fn migrate_legacy_user_data() -> AppResult<()> {
    let Some(base) = dirs::data_dir() else {
        return Ok(());
    };
    let current = user_data_dir();
    for brand in LEGACY_CONTROL_CENTER_BRANDS {
        let legacy = base.join(brand).join("ControlCenter");
        if legacy.exists() {
            copy_missing_tree(&legacy, &current)?;
        }
    }

    if let Ok(workspace) = find_workspace_root() {
        let new_local = workspace.join(".alvenqis-local");
        for old_name in LEGACY_LOCAL_DIRS {
            let old_local = workspace.join(old_name);
            if old_local.exists() {
                copy_missing_tree(&old_local, &new_local)?;
            }
        }
    }
    Ok(())
}

/// True when both paths refer to the same filesystem location.
fn same_path(source: &Path, destination: &Path) -> bool {
    if source == destination {
        return true;
    }
    match (source.canonicalize(), destination.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Copy files from `source` into `destination` without overwriting existing files.
/// No-op when source and destination are the same path (rebrand safety net).
fn copy_missing_tree(source: &Path, destination: &Path) -> AppResult<()> {
    if same_path(source, destination) {
        return Ok(());
    }
    if source.is_file() {
        if !destination.exists() {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(source, destination)?;
        }
        return Ok(());
    }
    if !source.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        copy_missing_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn legacy_control_center_brands_differ_from_current() {
        let current_brand = "Alvenqis";
        for brand in LEGACY_CONTROL_CENTER_BRANDS {
            assert_ne!(
                *brand, current_brand,
                "legacy ControlCenter brand must differ from Alvenqis"
            );
        }
        for name in LEGACY_LOCAL_DIRS {
            assert_ne!(
                *name, ".alvenqis-local",
                "legacy local dir must differ from .alvenqis-local"
            );
        }
    }

    #[test]
    fn copy_missing_tree_noop_on_same_path() {
        let dir = temp_dir("ws-same");
        fs::create_dir_all(dir.join("a")).unwrap();
        fs::write(dir.join("a").join("f.txt"), b"x").unwrap();
        copy_missing_tree(&dir, &dir).expect("self-copy must be Ok");
        let canon = dir.canonicalize().unwrap();
        copy_missing_tree(&canon, &canon).expect("canonical self-copy must be Ok");
        assert_eq!(
            fs::read_to_string(dir.join("a").join("f.txt")).unwrap(),
            "x"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_missing_tree_migrates_missing_only() {
        let root = temp_dir("ws-migrate");
        let legacy = root.join("Vireon").join("ControlCenter");
        let current = root.join("Alvenqis").join("ControlCenter");
        fs::create_dir_all(legacy.join("nested")).unwrap();
        fs::create_dir_all(current.join("nested")).unwrap();
        fs::write(legacy.join("nested").join("from-legacy.txt"), b"L").unwrap();
        fs::write(legacy.join("nested").join("shared.txt"), b"legacy").unwrap();
        fs::write(current.join("nested").join("shared.txt"), b"current").unwrap();

        copy_missing_tree(&legacy, &current).unwrap();

        assert_eq!(
            fs::read_to_string(current.join("nested").join("from-legacy.txt")).unwrap(),
            "L"
        );
        assert_eq!(
            fs::read_to_string(current.join("nested").join("shared.txt")).unwrap(),
            "current"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// Linux dirs::data_dir() → $XDG_DATA_HOME or ~/.local/share.
    /// Confirm self-referential Alvenqis/ControlCenter under that layout is a no-op.
    #[test]
    fn linux_xdg_data_home_control_center_self_copy_is_noop() {
        let home = temp_dir("xdg-cc-home");
        let xdg_data = home.join(".local").join("share");
        let current = xdg_data.join("Alvenqis").join("ControlCenter");
        fs::create_dir_all(current.join("settings")).unwrap();
        fs::write(current.join("settings").join("ui.json"), b"keep").unwrap();

        let legacy_broken = xdg_data.join("Alvenqis").join("ControlCenter");
        assert_eq!(legacy_broken, current);
        copy_missing_tree(&legacy_broken, &current).expect("XDG ControlCenter self-copy");
        assert_eq!(
            fs::read_to_string(current.join("settings").join("ui.json")).unwrap(),
            "keep"
        );
        let _ = fs::remove_dir_all(&home);
    }

    /// AREA 7a: mock HOME + XDG_DATA_HOME and call migrate_legacy_user_data() so
    /// dirs::data_dir() points at a temp Linux XDG tree (not Windows LocalAppData).
    #[test]
    #[cfg(target_os = "linux")]
    fn migrate_legacy_user_data_mocked_xdg_is_noop_when_only_alvenqis_present() {
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _guard = ENV_LOCK.lock().expect("env lock");

        let home = temp_dir("xdg-migrate-env");
        let xdg_data = home.join("share");
        let current = xdg_data.join("Alvenqis").join("ControlCenter");
        fs::create_dir_all(current.join("settings")).unwrap();
        fs::write(current.join("settings").join("ui.json"), b"keep-cc").unwrap();

        let prev_home = std::env::var_os("HOME");
        let prev_xdg = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_DATA_HOME", &xdg_data);

        assert_eq!(
            user_data_dir().canonicalize().unwrap(),
            current.canonicalize().unwrap()
        );
        migrate_legacy_user_data().expect("migrate with only Alvenqis present");
        assert_eq!(
            fs::read_to_string(current.join("settings").join("ui.json")).unwrap(),
            "keep-cc"
        );

        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match prev_xdg {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn linux_xdg_data_home_migrates_vireon_control_center() {
        let home = temp_dir("xdg-cc-migrate");
        let xdg_data = home.join(".local").join("share");
        let legacy = xdg_data.join("Vireon").join("ControlCenter");
        let current = xdg_data.join("Alvenqis").join("ControlCenter");
        fs::create_dir_all(legacy.join("nested")).unwrap();
        fs::create_dir_all(&current).unwrap();
        fs::write(legacy.join("nested").join("from-vireon.txt"), b"v").unwrap();

        assert!(LEGACY_CONTROL_CENTER_BRANDS.contains(&"Vireon"));
        copy_missing_tree(&legacy, &current).unwrap();
        assert_eq!(
            fs::read_to_string(current.join("nested").join("from-vireon.txt")).unwrap(),
            "v"
        );
        let _ = fs::remove_dir_all(&home);
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "alvenqis-workspace-test-{}-{}-{}",
            label,
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}

pub fn settings_path() -> PathBuf {
    user_data_dir().join("settings.json")
}

fn is_workspace(candidate: &Path) -> bool {
    has_bundled_node(candidate)
        || candidate
            .join("scripts")
            .join("local")
            .join("alvenqis-local.ps1")
            .exists()
        || candidate
            .join("scripts")
            .join("local")
            .join("start-all.sh")
            .exists()
        || candidate.join("alvenqis-core").join("Cargo.toml").exists()
}

fn has_operator_script(candidate: &Path) -> bool {
    candidate.join("alvenqis.ps1").exists() || candidate.join("alvenqis.sh").exists()
}

fn has_bundled_node(candidate: &Path) -> bool {
    let binary = if cfg!(windows) {
        "alvenqis-node.exe"
    } else {
        "alvenqis-node"
    };
    candidate.join("bin").join(binary).exists()
}

fn has_bundled_miner(candidate: &Path) -> bool {
    let binary = if cfg!(windows) {
        "alvenqis-miner.exe"
    } else {
        "alvenqis-miner"
    };
    candidate.join("bin").join(binary).exists()
}

pub fn keystore_helper_path(_workspace: &Path) -> PathBuf {
    let binary = if cfg!(windows) {
        "alvenqis-keystore-helper.exe"
    } else {
        "alvenqis-keystore-helper"
    };

    // 1) Next to the running executable (externalBin install layout)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidates = [
                parent.join(binary),
                parent.join("bin").join(binary),
                parent.join("resources").join("bin").join(binary),
            ];
            for path in candidates {
                if path.exists() {
                    return path;
                }
            }
        }
    }

    // 2) Resource root staged layout
    if let Some(resource) = RESOURCE_ROOT.get() {
        for root in resource_candidates(resource) {
            for path in [root.join("bin").join(binary), root.join(binary)] {
                if path.exists() {
                    return path;
                }
            }
        }
    }

    // 3) Tauri project binaries (dev after prepare-native)
    let tauri_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(binary);
    if tauri_bin.exists() {
        return tauri_bin;
    }

    // 4) Local native crate release build
    let native = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("native")
        .join("keystore-helper")
        .join("target")
        .join("release")
        .join(binary);
    if native.exists() {
        return native;
    }

    tauri_bin
}
