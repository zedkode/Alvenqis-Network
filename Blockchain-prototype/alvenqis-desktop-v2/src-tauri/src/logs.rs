use crate::error::{AppError, AppResult};
use crate::workspace::{find_workspace_root, local_root};
use std::fs;
use std::path::{Path, PathBuf};

const SERVICES: &[&str] = &["node", "rpc", "miner", "explorer", "miner-activity"];

fn candidate_local_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(workspace) = find_workspace_root() {
        roots.push(local_root(&workspace));
        roots.push(workspace.join(".alvenqis-local"));
    }
    if let Ok(app_data) = std::env::var("LOCALAPPDATA") {
        roots.push(
            PathBuf::from(app_data)
                .join("Alvenqis")
                .join("ControlCenter")
                .join(".alvenqis-local"),
        );
    }
    roots
}

fn freshest_file(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut best: Option<(u64, PathBuf)> = None;
    for path in paths {
        let Ok(meta) = fs::metadata(path) else {
            continue;
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if best.as_ref().map(|(t, _)| mtime >= *t).unwrap_or(true) {
            best = Some((mtime, path.clone()));
        }
    }
    best.map(|(_, p)| p)
}

pub fn recent(service: &str, lines: Option<usize>) -> AppResult<String> {
    if !SERVICES.contains(&service) {
        return Err(AppError::msg("Unsupported log service"));
    }
    // Allow full-session consoles (miner page) without unbounded memory: cap at 50k lines.
    let limit = lines.unwrap_or(160).clamp(1, 50_000);
    let roots = candidate_local_roots();
    if roots.is_empty() {
        return Err(AppError::msg(
            "Cannot locate Control Center data root for logs",
        ));
    }

    // Full structured activity stream written by alvenqis-miner (every action).
    if service == "miner-activity" {
        let paths: Vec<PathBuf> = roots
            .iter()
            .map(|r| r.join("miner").join("activity.log"))
            .collect();
        let activity = freshest_file(&paths).unwrap_or_else(|| paths[0].clone());
        let text = read_tail(&activity, limit);
        if text.trim().is_empty() {
            return Ok(
                "No activity.log yet. Start the miner to stream template fetch, batches, shares and errors."
                    .to_owned(),
            );
        }
        return Ok(text);
    }

    let main_paths: Vec<PathBuf> = roots
        .iter()
        .map(|r| r.join("logs").join(format!("{service}.log")))
        .collect();
    let err_paths: Vec<PathBuf> = roots
        .iter()
        .map(|r| r.join("logs").join(format!("{service}.err.log")))
        .collect();
    let main = freshest_file(&main_paths)
        .map(|p| read_tail(&p, limit))
        .unwrap_or_default();
    let err = freshest_file(&err_paths)
        .map(|p| read_tail(&p, limit))
        .unwrap_or_default();
    let combined = format!("{main}\n{err}").trim().to_string();
    if combined.is_empty() && service == "miner" {
        return Ok(
            "No miner.log yet. Click Start to launch alvenqis-miner; console updates every 1-2s while mining."
                .to_owned(),
        );
    }
    Ok(combined)
}

fn read_tail(path: &Path, lines: usize) -> String {
    match fs::read_to_string(path) {
        Ok(raw) => raw
            .lines()
            .rev()
            .take(lines)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n"),
        Err(_) => String::new(),
    }
}

pub fn export_content(service: &str) -> AppResult<String> {
    recent(service, Some(10_000))
}
