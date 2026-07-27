//! User-approved updates from GitHub Releases.
//!
//! On a packaged (or forced) build the Control Center:
//! 1. Polls `zedkode/Alvenqis-Network` releases
//! 2. Detects newer assets (installer / miner / keystore / node / rpc)
//! 3. Requires an explicit operator action before download and installation
//! 4. Verifies every selected asset against the release `SHA256SUMS`
//!
//! Background checks only notify; they never execute downloaded code.

use crate::error::{AppError, AppResult};
use crate::process;
use crate::settings;
use crate::workspace::{find_workspace_root, resource_root, user_data_dir};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
const GITHUB_REPO: &str = "zedkode/Alvenqis-Network";
const STARTUP_DELAY_SECS: u64 = 8;

#[derive(Debug, Clone, Serialize)]
pub struct UpdateProgress {
    pub percent: f64,
    pub transferred: u64,
    pub total: u64,
    pub bytes_per_second: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateState {
    pub phase: String,
    pub current_version: String,
    pub available_version: Option<String>,
    pub release_name: Option<String>,
    pub release_date: Option<String>,
    pub message: String,
    pub manual: bool,
    pub progress: Option<UpdateProgress>,
    /// Components that will be / were auto-applied (node, rpc, miner, shell, …).
    #[serde(default)]
    pub components: Vec<String>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            phase: "idle".into(),
            current_version: env!("CARGO_PKG_VERSION").into(),
            available_version: None,
            release_name: None,
            release_date: None,
            message: "Update checks use GitHub Releases; installation requires approval and SHA-256 verification."
                .into(),
            manual: false,
            progress: None,
            components: Vec::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct UpdateService {
    inner: Arc<RwLock<UpdateState>>,
    applying: Arc<RwLock<bool>>,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    draft: bool,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize, Clone)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Clone)]
struct PlannedUpdate {
    tag: String,
    release_name: String,
    release_date: Option<String>,
    assets: Vec<PlannedAsset>,
    checksum_asset: GhAsset,
}

#[derive(Debug, Clone)]
struct PlannedAsset {
    role: &'static str,
    asset: GhAsset,
}

impl UpdateService {
    pub fn state(&self) -> UpdateState {
        self.inner.read().clone()
    }

    fn set(&self, app: Option<&AppHandle>, patch: impl FnOnce(&mut UpdateState)) {
        {
            let mut state = self.inner.write();
            patch(&mut state);
        }
        if let Some(app) = app {
            let snapshot = self.state();
            let _ = app.emit("updates:state", snapshot);
        }
    }

    pub fn current(&self, app: &AppHandle) -> UpdateState {
        let state = self.state();
        let _ = app.emit("updates:state", state.clone());
        state
    }

    /// Background loop only detects new releases. It never downloads or installs.
    pub fn spawn_auto_loop(self, app: AppHandle) {
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(STARTUP_DELAY_SECS)).await;
            loop {
                let auto = settings::get().auto_update;
                if auto {
                    let _ = self.check_for_update(&app, false).await;
                }
                let secs = settings::get().auto_update_interval_secs.max(60);
                tokio::time::sleep(Duration::from_secs(secs)).await;
            }
        });
    }

    pub async fn check(&self, app: &AppHandle, manual: bool) -> UpdateState {
        self.check_for_update(app, manual).await
    }

    async fn check_for_update(&self, app: &AppHandle, manual: bool) -> UpdateState {
        if *self.applying.read() {
            return self.state();
        }

        let auto = settings::get().auto_update;
        if !auto && !manual && !settings::get().notify_updates {
            return self.state();
        }

        self.set(Some(app), |state| {
            state.phase = "checking".into();
            state.manual = manual;
            state.message = format!(
                "Checking GitHub Releases ({GITHUB_REPO}) for verified application updates..."
            );
            state.progress = None;
            state.components.clear();
        });

        let planned = match fetch_latest_update().await {
            Ok(Some(plan)) => plan,
            Ok(None) => {
                self.set(Some(app), |state| {
                    state.phase = "idle".into();
                    state.available_version = None;
                    state.release_name = None;
                    state.message =
                        "Control Center is current - no newer GitHub release assets detected."
                            .into();
                    state.progress = None;
                    state.components.clear();
                });
                return self.state();
            }
            Err(err) => {
                self.set(Some(app), |state| {
                    state.phase = "error".into();
                    state.message = format!("GitHub update check failed: {err}");
                    state.progress = None;
                });
                return self.state();
            }
        };

        let roles: Vec<String> = planned.assets.iter().map(|a| a.role.to_string()).collect();
        self.set(Some(app), |state| {
            state.phase = "available".into();
            state.available_version = Some(planned.tag.clone());
            state.release_name = Some(planned.release_name.clone());
            state.release_date = planned.release_date.clone();
            state.components = roles.clone();
            state.message = format!(
                "Update {} detected ({}). Review and approve download to install.",
                planned.tag,
                roles.join(", ")
            );
        });

        self.state()
    }

    pub async fn download(&self, app: &AppHandle) -> AppResult<()> {
        let plan = fetch_latest_update()
            .await?
            .ok_or_else(|| AppError::msg("no newer verified release is available"))?;
        self.apply(app, &plan).await
    }

    pub async fn install(&self, app: &AppHandle, restart: bool) -> AppResult<()> {
        // Installation happens during the explicitly approved download action.
        let phase = self.state().phase;
        if phase != "downloaded" && phase != "idle" && phase != "available" {
            // Allow re-check
            let _ = self.check_for_update(app, true).await;
        }
        if restart {
            let _ = process::run_operator("stop", None, None).await;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.close();
            }
        }
        Ok(())
    }

    async fn apply(&self, app: &AppHandle, plan: &PlannedUpdate) -> AppResult<()> {
        {
            let mut guard = self.applying.write();
            if *guard {
                return Ok(());
            }
            *guard = true;
        }
        let result = self.apply_inner(app, plan).await;
        *self.applying.write() = false;
        result
    }

    async fn apply_inner(&self, app: &AppHandle, plan: &PlannedUpdate) -> AppResult<()> {
        let stage = update_stage_dir()?.join(safe_component(&plan.tag)?);
        std::fs::create_dir_all(&stage)?;

        let client = reqwest::Client::builder()
            .user_agent(format!(
                "alvenqis-control-center-auto-update/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(Duration::from_secs(600))
            .build()
            .map_err(|e| AppError::msg(e.to_string()))?;

        let total_bytes: u64 = plan
            .assets
            .iter()
            .map(|a| a.asset.size.max(1))
            .sum::<u64>()
            .saturating_add(plan.checksum_asset.size.max(1));
        let mut transferred: u64 = 0;
        let start = std::time::Instant::now();

        self.set(Some(app), |state| {
            state.phase = "downloading".into();
            state.message = format!("Downloading {} and its SHA256SUMS from GitHub...", plan.tag);
            state.progress = Some(UpdateProgress {
                percent: 0.0,
                transferred: 0,
                total: total_bytes,
                bytes_per_second: 0.0,
            });
        });

        let checksum_dest = stage.join(safe_component(&plan.checksum_asset.name)?);
        download_asset(&client, &plan.checksum_asset, &checksum_dest, |chunk| {
            transferred += chunk;
        })
        .await?;
        let checksum_text = tokio::fs::read_to_string(&checksum_dest)
            .await
            .map_err(|e| AppError::msg(format!("cannot read SHA256SUMS: {e}")))?;
        let checksums = parse_sha256sums(&checksum_text)?;

        let mut local_files: Vec<(&str, PathBuf)> = Vec::new();

        for planned in &plan.assets {
            let dest = stage.join(safe_component(&planned.asset.name)?);
            download_asset(&client, &planned.asset, &dest, |chunk| {
                transferred += chunk;
                let elapsed = start.elapsed().as_secs_f64().max(0.001);
                let bps = transferred as f64 / elapsed;
                let percent = (transferred as f64 / total_bytes as f64 * 100.0).min(100.0);
                self.set(Some(app), |state| {
                    state.progress = Some(UpdateProgress {
                        percent,
                        transferred,
                        total: total_bytes,
                        bytes_per_second: bps,
                    });
                });
            })
            .await?;
            verify_asset_checksum(&dest, &planned.asset.name, &checksums)?;
            local_files.push((planned.role, dest));
        }

        self.set(Some(app), |state| {
            state.phase = "installing".into();
            state.message = "SHA-256 verified. Applying the approved update...".into();
            state.progress = None;
        });

        // Stop all managed processes before replacing their binaries.
        let _ = process::run_operator("stop", None, None).await;

        let mut install_errors: Vec<String> = Vec::new();
        for (role, path) in &local_files {
            let result = match *role {
                "miner" | "keystore" | "node" | "rpc" | "indexer" => install_sidecar(role, path),
                "control_center_setup" => run_silent_setup(path).await,
                "control_center_msi" => run_silent_msi(path).await,
                _ => Ok(()),
            };
            if let Err(err) = result {
                install_errors.push(format!("{role}: {err}"));
            }
        }

        if !install_errors.is_empty() {
            self.set(Some(app), |state| {
                state.phase = "error".into();
                state.message = format!(
                    "Update {} was verified but installation failed: {}",
                    plan.tag,
                    install_errors.join("; ")
                );
                state.progress = None;
            });
            return Err(AppError::msg(install_errors.join("; ")));
        }

        persist_applied_tag(&plan.tag)?;

        self.set(Some(app), |state| {
            state.phase = "idle".into();
            state.current_version = env!("CARGO_PKG_VERSION").into();
            state.available_version = None;
            state.message = format!(
                "Approved update {} was verified and installed. Restart Control Center if the shell package changed.",
                plan.tag
            );
            state.progress = None;
        });

        // Best-effort OS notification
        #[cfg(not(target_os = "android"))]
        {
            use tauri_plugin_notification::NotificationExt;
            let _ = app
                .notification()
                .builder()
                .title("Alvenqis update installed")
                .body(format!("{} verified and installed from GitHub", plan.tag))
                .show();
        }

        Ok(())
    }
}

async fn fetch_latest_update() -> AppResult<Option<PlannedUpdate>> {
    let client = reqwest::Client::builder()
        .user_agent(format!(
            "alvenqis-control-center-auto-update/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::msg(e.to_string()))?;

    let mut req = client
        .get(format!(
            "https://api.github.com/repos/{GITHUB_REPO}/releases?per_page=30"
        ))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            req = req.bearer_auth(token.trim());
        }
    }

    let releases: Vec<GhRelease> = req
        .send()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?
        .error_for_status()
        .map_err(|e| AppError::msg(e.to_string()))?
        .json()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;

    let applied = load_applied_tag().unwrap_or_default();
    let current = env!("CARGO_PKG_VERSION");
    let Some(current_ver) = parse_app_version(current) else {
        return Ok(None);
    };
    let applied_norm = normalize_tag(&applied);
    let applied_ver = if applied_norm.is_empty() {
        None
    } else {
        parse_app_version(&applied_norm)
    };

    // Walk newest → oldest; pick the first release that is *strictly newer* than the
    // running package (and not already recorded as applied). Equal / older / same-tag
    // releases must never re-download — that was locking operators out of the app.
    for rel in releases {
        if rel.draft {
            continue;
        }
        if is_non_desktop_release_tag(&rel.tag_name) {
            continue;
        }
        let assets = plan_assets(&rel.assets);
        if assets.is_empty() {
            continue;
        }
        let checksum_asset = rel
            .assets
            .iter()
            .find(|asset| asset.name.eq_ignore_ascii_case("SHA256SUMS"))
            .cloned()
            .ok_or_else(|| {
                AppError::msg(format!(
                    "release {} has executable assets but no SHA256SUMS; refusing update",
                    rel.tag_name
                ))
            })?;

        let tag_norm = normalize_tag(&rel.tag_name);
        if !applied_norm.is_empty() && tags_equal(&tag_norm, &applied_norm) {
            // Already applied this exact tag (installer may have restarted before).
            return Ok(None);
        }

        let Some(remote_ver) = parse_app_version(&tag_norm) else {
            continue;
        };

        // Never re-apply equal or older product versions (e.g. installed 0.10.2 vs
        // GitHub tag v0.10.2-candidate.1 which is not strictly newer).
        if !is_strictly_newer(&remote_ver, &current_ver) {
            continue;
        }
        if let Some(ref applied_v) = applied_ver {
            if !is_strictly_newer(&remote_ver, applied_v) {
                continue;
            }
        }

        return Ok(Some(PlannedUpdate {
            tag: rel.tag_name.clone(),
            release_name: rel.name.unwrap_or_else(|| rel.tag_name.clone()),
            release_date: rel.published_at.clone(),
            assets,
            checksum_asset,
        }));
    }
    Ok(None)
}

/// Strip a single leading `v` only when it prefixes a digit (`v0.10.2` → `0.10.2`).
/// Do **not** strip the `v` in `vps-control-…` or similar non-product tags.
fn normalize_tag(tag: &str) -> String {
    let s = tag.trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && (bytes[0] == b'v' || bytes[0] == b'V') && bytes[1].is_ascii_digit() {
        return s[1..].trim().to_string();
    }
    s.to_string()
}

fn tags_equal(a: &str, b: &str) -> bool {
    normalize_tag(a).eq_ignore_ascii_case(&normalize_tag(b))
}

fn is_non_desktop_release_tag(tag: &str) -> bool {
    let t = tag.trim().to_ascii_lowercase();
    t.starts_with("vps-control") || t.starts_with("vps-") || t.contains("vps-control")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppVersion {
    major: u64,
    minor: u64,
    patch: u64,
    /// Pre-release label without leading `-` (e.g. `candidate.1`, `rc.7`).
    pre: Option<String>,
}

/// Parse product versions from tags / Cargo versions.
/// Accepts: `0.10.2`, `v0.10.2`, `0.10.2-candidate.1`, `desktop-v0.10.3`.
fn parse_app_version(raw: &str) -> Option<AppVersion> {
    if raw.trim().is_empty() || is_non_desktop_release_tag(raw) {
        return None;
    }
    let s = normalize_tag(raw);
    if s.is_empty() {
        return None;
    }
    // Find first `major.minor.patch` sequence in the string.
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let rest = &s[i..];
            if let Some(ver) = parse_version_at(rest) {
                return Some(ver);
            }
        }
        i += 1;
    }
    None
}

fn parse_version_at(s: &str) -> Option<AppVersion> {
    let (numeric, pre_part) = match s.split_once('-') {
        Some((num, pre)) => (num, Some(pre)),
        None => {
            // Also allow `+build` metadata without pre
            let num = s.split_once('+').map(|(n, _)| n).unwrap_or(s);
            (num, None)
        }
    };
    // numeric may still include trailing junk after patch — take only digits/dots prefix
    let numeric = numeric
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .next()
        .unwrap_or(numeric);
    let mut parts = numeric.split('.');
    let major: u64 = parts.next()?.parse().ok()?;
    let minor: u64 = parts.next()?.parse().ok()?;
    let patch: u64 = parts.next()?.parse().ok()?;
    // Require at least major.minor.patch; reject if first segment wasn't a clean triple start
    // (e.g. lone "10" mid-string without three components).
    let pre = pre_part.map(|p| {
        let p = p.split_once('+').map(|(a, _)| a).unwrap_or(p);
        p.trim().to_ascii_lowercase()
    });
    if let Some(ref p) = pre {
        if p.is_empty() {
            return Some(AppVersion {
                major,
                minor,
                patch,
                pre: None,
            });
        }
    }
    Some(AppVersion {
        major,
        minor,
        patch,
        pre,
    })
}

/// Semver-like: higher major/minor/patch wins. A release *without* pre-release is newer
/// than the same numbers *with* pre-release (`0.10.2` > `0.10.2-candidate.1`).
fn is_strictly_newer(remote: &AppVersion, local: &AppVersion) -> bool {
    if remote.major != local.major {
        return remote.major > local.major;
    }
    if remote.minor != local.minor {
        return remote.minor > local.minor;
    }
    if remote.patch != local.patch {
        return remote.patch > local.patch;
    }
    match (&remote.pre, &local.pre) {
        // same numbers, both stable → not newer
        (None, None) => false,
        // stable remote vs local pre → remote is newer
        (None, Some(_)) => true,
        // pre remote vs stable local → remote is older channel, not an upgrade
        (Some(_), None) => false,
        (Some(a), Some(b)) => compare_prerelease(a, b) == Ordering::Greater,
    }
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a_part), Some(b_part)) => {
                let order = match (a_part.parse::<u64>(), b_part.parse::<u64>()) {
                    (Ok(a_num), Ok(b_num)) => a_num.cmp(&b_num),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => a_part.cmp(b_part),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

fn plan_assets(assets: &[GhAsset]) -> Vec<PlannedAsset> {
    let mut out = Vec::new();
    let is_windows = cfg!(windows);
    let is_linux = cfg!(target_os = "linux");

    let mut push = |role: &'static str, pred: &dyn Fn(&str) -> bool| {
        if out.iter().any(|a: &PlannedAsset| a.role == role) {
            return;
        }
        if let Some(asset) = assets.iter().find(|a| pred(&a.name)) {
            out.push(PlannedAsset {
                role,
                asset: asset.clone(),
            });
        }
    };

    if is_windows {
        push("control_center_setup", &|n| {
            let l = n.to_ascii_lowercase();
            (l.contains("control") && l.contains("setup") && l.ends_with(".exe"))
                || l.contains("alvenqis-control-center") && l.ends_with(".exe") && l.contains("win")
        });
        push("miner", &|n| {
            let l = n.to_ascii_lowercase();
            l.contains("alvenqis-miner")
                && (l.contains("windows")
                    || l.contains("msvc")
                    || l.ends_with("alvenqis-miner.exe"))
        });
        push("keystore", &|n| {
            let l = n.to_ascii_lowercase();
            l.contains("keystore")
                && (l.contains("windows") || l.contains("msvc") || l.ends_with(".exe"))
        });
        push("node", &|n| {
            let l = n.to_ascii_lowercase();
            l.contains("alvenqis-node")
                && (l.contains("windows") || l.contains("msvc") || l.ends_with("alvenqis-node.exe"))
        });
        push("rpc", &|n| {
            let l = n.to_ascii_lowercase();
            (l.contains("rpc-gateway") || l.contains("alvenqis-rpc"))
                && (l.contains("windows") || l.contains("msvc") || l.ends_with(".exe"))
        });
    }

    if is_linux {
        push("control_center_setup", &|n| {
            let l = n.to_ascii_lowercase();
            l.contains("control") && l.ends_with(".appimage")
        });
        push("control_center_setup", &|n| {
            let l = n.to_ascii_lowercase();
            l.contains("control") && l.ends_with(".deb")
        });
        push("miner", &|n| {
            let l = n.to_ascii_lowercase();
            l.contains("alvenqis-miner")
                && (l.contains("linux") || !l.contains("windows"))
                && !l.ends_with(".exe")
        });
        push("keystore", &|n| {
            let l = n.to_ascii_lowercase();
            l.contains("keystore")
                && (l.contains("linux") || !l.contains("windows"))
                && !l.ends_with(".exe")
        });
        push("node", &|n| {
            let l = n.to_ascii_lowercase();
            l.contains("alvenqis-node") && !l.ends_with(".exe") && !l.contains("windows")
        });
        push("rpc", &|n| {
            let l = n.to_ascii_lowercase();
            (l.contains("rpc-gateway") || l.contains("alvenqis-rpc"))
                && !l.ends_with(".exe")
                && !l.contains("windows")
        });
    }

    out
}

async fn download_asset<F>(
    client: &reqwest::Client,
    asset: &GhAsset,
    dest: &Path,
    mut on_chunk: F,
) -> AppResult<()>
where
    F: FnMut(u64),
{
    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?
        .error_for_status()
        .map_err(|e| AppError::msg(e.to_string()))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    if asset.size > 0 && bytes.len() as u64 != asset.size {
        return Err(AppError::msg(format!(
            "downloaded size mismatch for {}: expected {}, got {}",
            asset.name,
            asset.size,
            bytes.len()
        )));
    }
    tokio::fs::write(dest, &bytes)
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    on_chunk(bytes.len() as u64);
    Ok(())
}

fn safe_component(name: &str) -> AppResult<&str> {
    let path = Path::new(name);
    if name.is_empty()
        || path.components().count() != 1
        || path.file_name().and_then(|part| part.to_str()) != Some(name)
    {
        return Err(AppError::msg(format!("unsafe release asset name: {name}")));
    }
    Ok(name)
}

fn parse_sha256sums(input: &str) -> AppResult<HashMap<String, String>> {
    let mut sums = HashMap::new();
    for (line_number, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let digest = fields.next().unwrap_or_default().to_ascii_lowercase();
        let name = fields
            .next()
            .unwrap_or_default()
            .trim_start_matches('*')
            .to_string();
        if digest.len() != 64
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || name.is_empty()
            || fields.next().is_some()
        {
            return Err(AppError::msg(format!(
                "invalid SHA256SUMS entry at line {}",
                line_number + 1
            )));
        }
        safe_component(&name)?;
        if sums.insert(name.clone(), digest).is_some() {
            return Err(AppError::msg(format!(
                "duplicate SHA256SUMS entry for {name}"
            )));
        }
    }
    if sums.is_empty() {
        return Err(AppError::msg("SHA256SUMS is empty"));
    }
    Ok(sums)
}

fn verify_asset_checksum(
    path: &Path,
    asset_name: &str,
    checksums: &HashMap<String, String>,
) -> AppResult<()> {
    // Integrity only (same-channel checksum). Authenticity is NOT proven: a
    // compromised GitHub release can republish both the binary and SHA256SUMS.
    //
    // TODO(security/CR-H13): add a second verification layer shared with the
    // VPS control-plane enrollment path (minisign/GPG detached signature
    // against a public key embedded at build time, OR GitHub release
    // attestations). Do not mark auto-update as production-ready until this
    // ships. Tracking: open issue "desktop+vps release authenticity (sig)".
    let expected = checksums
        .get(asset_name)
        .ok_or_else(|| AppError::msg(format!("SHA256SUMS has no entry for {asset_name}")))?;
    let bytes = std::fs::read(path)?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(AppError::msg(format!(
            "SHA-256 mismatch for {asset_name}: expected {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn install_sidecar(role: &str, downloaded: &Path) -> AppResult<()> {
    let file_name = match role {
        "miner" => {
            if cfg!(windows) {
                "alvenqis-miner.exe"
            } else {
                "alvenqis-miner"
            }
        }
        "keystore" => {
            if cfg!(windows) {
                "alvenqis-keystore-helper.exe"
            } else {
                "alvenqis-keystore-helper"
            }
        }
        "node" => {
            if cfg!(windows) {
                "alvenqis-node.exe"
            } else {
                "alvenqis-node"
            }
        }
        "rpc" => {
            if cfg!(windows) {
                "alvenqis-rpc-gateway.exe"
            } else {
                "alvenqis-rpc-gateway"
            }
        }
        "indexer" => {
            if cfg!(windows) {
                "alvenqis-indexer.exe"
            } else {
                "alvenqis-indexer"
            }
        }
        _ => return Ok(()),
    };

    let mut targets: Vec<PathBuf> = Vec::new();
    if let Some(res) = resource_root() {
        targets.push(res.join("bin").join(file_name));
        if role == "keystore" {
            let triple = if cfg!(windows) {
                "alvenqis-keystore-helper-x86_64-pc-windows-msvc.exe"
            } else {
                "alvenqis-keystore-helper-x86_64-unknown-linux-gnu"
            };
            let parent = res.parent().unwrap_or(res.as_path());
            targets.push(parent.join("binaries").join(triple));
        }
    }
    if let Ok(ws) = find_workspace_root() {
        targets.push(ws.join("target").join("release").join(file_name));
        targets.push(
            ws.join("alvenqis-desktop-tauri")
                .join("src-tauri")
                .join("resources")
                .join("bin")
                .join(file_name),
        );
        if role == "keystore" {
            targets.push(
                ws.join("alvenqis-desktop-tauri")
                    .join("src-tauri")
                    .join("binaries")
                    .join(if cfg!(windows) {
                        "alvenqis-keystore-helper-x86_64-pc-windows-msvc.exe"
                    } else {
                        "alvenqis-keystore-helper-x86_64-unknown-linux-gnu"
                    }),
            );
        }
    }

    let mut installed = 0usize;
    let mut failures = Vec::new();
    for target in targets {
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Best-effort replace; skip if locked
        match std::fs::copy(downloaded, &target) {
            Ok(_) => {
                installed += 1;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ =
                        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
                }
            }
            Err(error) => failures.push(format!("{}: {error}", target.display())),
        }
    }
    if installed == 0 {
        return Err(AppError::msg(format!(
            "could not install {role} sidecar: {}",
            failures.join("; ")
        )));
    }
    Ok(())
}

async fn run_silent_setup(path: &Path) -> AppResult<()> {
    let mut cmd = tokio::process::Command::new(path);
    if cfg!(windows) {
        cmd.arg("/S");
        #[cfg(windows)]
        {
            cmd.creation_flags(0x0800_0000);
        }
    } else {
        // AppImage/deb: best-effort
        let s = path.to_string_lossy();
        if s.ends_with(".deb") {
            cmd = tokio::process::Command::new("pkexec");
            cmd.args(["dpkg", "-i", path.to_str().unwrap_or("")]);
        } else {
            cmd.arg("--appimage-extract-and-run");
        }
    }
    let status = cmd
        .status()
        .await
        .map_err(|e| AppError::msg(format!("failed to launch installer: {e}")))?;
    if !status.success() {
        return Err(AppError::msg(format!(
            "installer exited with status {status}"
        )));
    }
    Ok(())
}

async fn run_silent_msi(path: &Path) -> AppResult<()> {
    let status = tokio::process::Command::new("msiexec.exe")
        .args(["/i", path.to_str().unwrap_or(""), "/qn", "/norestart"])
        .status()
        .await
        .map_err(|e| AppError::msg(format!("msiexec failed: {e}")))?;
    if !status.success() {
        return Err(AppError::msg(format!("msiexec status {status}")));
    }
    Ok(())
}

fn update_stage_dir() -> AppResult<PathBuf> {
    let dir = user_data_dir().join("auto-update").join("stage");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn applied_tag_path() -> PathBuf {
    user_data_dir().join("auto-update").join("applied-tag.txt")
}

fn load_applied_tag() -> AppResult<String> {
    let path = applied_tag_path();
    Ok(std::fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_string())
}

fn persist_applied_tag(tag: &str) -> AppResult<()> {
    let path = applied_tag_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, tag)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_product_tags() {
        let v = parse_app_version("v0.10.2").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 10);
        assert_eq!(v.patch, 2);
        assert!(v.pre.is_none());

        let c = parse_app_version("v0.10.2-candidate.1").unwrap();
        assert_eq!(c.pre.as_deref(), Some("candidate.1"));

        let d = parse_app_version("desktop-v0.11.0").unwrap();
        assert_eq!((d.major, d.minor, d.patch), (0, 11, 0));
    }

    #[test]
    fn rejects_vps_only_tags() {
        assert!(parse_app_version("vps-control-v0.1.0-rc.7").is_none());
        assert!(is_non_desktop_release_tag("vps-control-v0.1.0-rc.7"));
    }

    #[test]
    fn does_not_treat_current_or_older_as_upgrade() {
        let local = parse_app_version("0.10.2").unwrap();
        let same = parse_app_version("v0.10.2").unwrap();
        let candidate = parse_app_version("v0.10.2-candidate.1").unwrap();
        let older = parse_app_version("0.7.1-candidate.3").unwrap();
        let newer = parse_app_version("0.10.3").unwrap();
        let newer_pre = parse_app_version("0.10.3-candidate.1").unwrap();

        assert!(!is_strictly_newer(&same, &local));
        // Installed stable 0.10.2 must NOT re-download candidate of the same numbers.
        assert!(!is_strictly_newer(&candidate, &local));
        assert!(!is_strictly_newer(&older, &local));
        assert!(is_strictly_newer(&newer, &local));
        assert!(is_strictly_newer(&newer_pre, &local));
    }

    #[test]
    fn pre_to_stable_and_pre_bump() {
        let local_pre = parse_app_version("0.10.2-candidate.1").unwrap();
        let stable = parse_app_version("0.10.2").unwrap();
        let pre2 = parse_app_version("0.10.2-candidate.2").unwrap();
        assert!(is_strictly_newer(&stable, &local_pre));
        assert!(is_strictly_newer(&pre2, &local_pre));
        assert!(!is_strictly_newer(&local_pre, &stable));

        let pre10 = parse_app_version("0.10.2-candidate.10").unwrap();
        assert!(is_strictly_newer(&pre10, &pre2));
    }

    #[test]
    fn tag_normalize_equality() {
        assert!(tags_equal("v0.10.2-candidate.1", "0.10.2-candidate.1"));
        assert!(tags_equal("V0.10.2", "v0.10.2"));
    }

    #[test]
    fn parses_strict_sha256sums() {
        let digest = "a".repeat(64);
        let sums = parse_sha256sums(&format!("{digest}  alvenqis-miner.exe\n")).unwrap();
        assert_eq!(sums.get("alvenqis-miner.exe"), Some(&digest));
        assert!(parse_sha256sums(&format!("{digest}  ../escape.exe\n")).is_err());
        assert!(parse_sha256sums("not-a-digest file.exe\n").is_err());
    }
}
