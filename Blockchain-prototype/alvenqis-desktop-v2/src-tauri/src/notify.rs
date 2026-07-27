use crate::rpc::NetworkSnapshot;
use crate::settings;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

#[derive(Default)]
pub struct NotifyState {
    last_height: Mutex<Option<u64>>,
    baseline_ready: Mutex<bool>,
}

pub fn maybe_notify_mined_blocks(
    app: &AppHandle,
    state: &NotifyState,
    snapshot: &NetworkSnapshot,
    wallet_address: Option<&str>,
) {
    let settings = settings::get();
    if !settings.notify_block_mined {
        return;
    }
    let Some(current) = snapshot.height else {
        return;
    };
    let Some(address) = wallet_address else {
        *state.last_height.lock() = Some(current);
        *state.baseline_ready.lock() = true;
        return;
    };

    let mut last = state.last_height.lock();
    let mut ready = state.baseline_ready.lock();
    if !*ready {
        *last = Some(current);
        *ready = true;
        return;
    }
    let previous = last.unwrap_or(current);
    if current <= previous {
        *last = Some(current);
        return;
    }

    let mined: Vec<&serde_json::Value> = snapshot
        .recent_blocks
        .iter()
        .filter(|block| {
            let height = block.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
            let miner = block
                .get("miner_address")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            height > previous && miner == address
        })
        .collect();

    for block in mined {
        let height = block.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
        let reward = block
            .get("miner_reward_atomic")
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| v.to_string())
            })
            .unwrap_or_else(|| "0".into());
        let body = format!(
            "Block {height} was accepted. Reward: {} ALVE.",
            format_atomic(&reward)
        );
        let _ = app
            .notification()
            .builder()
            .title("Alvenqis block mined")
            .body(&body)
            .show();
        let _ = app.emit(
            "alvenqis:block-mined",
            serde_json::json!({ "height": height, "reward_atomic": reward }),
        );
        if settings.notify_sound {
            play_alert_sound();
        }
    }

    *last = Some(current);
}

fn format_atomic(value: &str) -> String {
    let Ok(atomic) = value.parse::<u128>() else {
        return value.to_string();
    };
    let whole = atomic / 100_000_000;
    let fraction = atomic % 100_000_000;
    format!("{whole}.{fraction:08}")
}

fn play_alert_sound() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[console]::beep(880,160)",
            ])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .status();
    }

    #[cfg(target_os = "linux")]
    {
        // Best-effort desktop sound; missing tools are silent (non-fatal).
        if std::process::Command::new("paplay")
            .arg("/usr/share/sounds/freedesktop/stereo/complete.oga")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return;
        }
        if std::process::Command::new("canberra-gtk-play")
            .args(["-i", "complete"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return;
        }
        let _ = std::process::Command::new("printf").arg("\x07").status();
    }
}
