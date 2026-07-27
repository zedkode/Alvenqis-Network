use crate::Result;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Append-only activity log. TRACE is file-only (never stderr) to avoid panel lag.
pub struct ActivityLog {
    path: Option<PathBuf>,
    inner: Mutex<()>,
}

impl ActivityLog {
    pub fn open(path: Option<PathBuf>) -> Result<Self> {
        if let Some(path) = &path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let _ = OpenOptions::new().create(true).append(true).open(path)?;
        }
        Ok(Self {
            path,
            inner: Mutex::new(()),
        })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn log(&self, level: &str, event: &str, detail: impl AsRef<str>) {
        let ts = unix_millis();
        let detail = detail.as_ref();
        let line = if detail.is_empty() {
            format!("[{ts}] [{level}] {event}")
        } else {
            format!("[{ts}] [{level}] {event} | {detail}")
        };

        // TRACE/DEBUG: file only, no flush every line (avoids I/O thrash + dual-console lag).
        // INFO/WARN/ERROR: mirror to stderr so miner.err.log stays useful and compact.
        let is_trace = level.eq_ignore_ascii_case("TRACE") || level.eq_ignore_ascii_case("DEBUG");
        if !is_trace {
            eprintln!("{line}");
        }

        let _guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(path) = &self.path {
            // Skip writing TRACE to activity.log by default — batches fire thousands/sec.
            if is_trace {
                return;
            }
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(file, "{line}");
                // Flush only important lines.
                if level.eq_ignore_ascii_case("ERROR") || level.eq_ignore_ascii_case("WARN") {
                    let _ = file.flush();
                }
            }
        }
    }

    pub fn info(&self, event: &str, detail: impl AsRef<str>) {
        self.log("INFO", event, detail);
    }

    pub fn warn(&self, event: &str, detail: impl AsRef<str>) {
        self.log("WARN", event, detail);
    }

    pub fn error(&self, event: &str, detail: impl AsRef<str>) {
        self.log("ERROR", event, detail);
    }

    pub fn debug(&self, event: &str, detail: impl AsRef<str>) {
        self.log("DEBUG", event, detail);
    }

    /// No-op for hot path (batch loops). Kept so call sites compile without spam.
    pub fn trace_action(&self, _event: &str, _detail: impl AsRef<str>) {
        // Intentionally empty — batch_begin/batch_end used to flood the UI dual console.
    }
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub fn default_activity_path(metrics_path: Option<&Path>, config_path: &Path) -> PathBuf {
    if let Some(metrics) = metrics_path {
        if let Some(parent) = metrics.parent() {
            return parent.join("activity.log");
        }
    }
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("activity.log")
}
