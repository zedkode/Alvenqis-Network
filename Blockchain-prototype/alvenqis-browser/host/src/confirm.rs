//! Human confirmation before signing/submitting and before discarding recovery material.
//!
//! Security policy (audit CR-C01 / CR-C02):
//! - Recovery phrase is shown exactly once at OS/host level before any spendable keystore
//!   is written; it is never returned over native-messaging JSON.
//! - Sign / send / submit require OS (or env) confirmation by default.

/// Confirm a sign/send/submit action.
///
/// - On Windows: MessageBox OK/Cancel when `require_os_confirm` is true.
/// - Elsewhere: prints to stderr and requires env `ALVENQIS_HOST_CONFIRM=1` for non-interactive
///   JSONL. When OS confirm is disabled (`--no-require-os-confirm`), auto-allows (dev/test only).
pub fn confirm_send(require_os_confirm: bool, summary: &str) -> Result<(), String> {
    if !require_os_confirm {
        return Ok(());
    }

    // Non-Windows and headless Windows: never pop MessageBox; require explicit env ack.
    let use_env_gate = headless_mode() || !cfg!(windows);
    if use_env_gate {
        eprintln!("alvenqis-browser-host confirm required:\n{summary}");
        eprintln!("Set ALVENQIS_HOST_CONFIRM=1 to allow this action in non-GUI environments.");
        if env_truthy("ALVENQIS_HOST_CONFIRM") {
            return Ok(());
        }
        return Err(
            "send/sign/submit blocked: OS confirm enabled and ALVENQIS_HOST_CONFIRM is not set"
                .to_owned(),
        );
    }

    // Interactive Windows desktop (native messaging host under the browser).
    #[cfg(windows)]
    {
        windows_confirm(
            "Alvenqis browser host — confirm transfer",
            summary,
            "user cancelled send/sign confirmation",
        )
    }

    #[cfg(not(windows))]
    {
        Err("send/sign/submit blocked: OS confirm required".to_owned())
    }
}

fn env_truthy(name: &str) -> bool {
    std::env::var(name)
        .map(|v| {
            v == "1"
                || v.eq_ignore_ascii_case("true")
                || v.eq_ignore_ascii_case("yes")
                || v.eq_ignore_ascii_case("on")
        })
        .unwrap_or(false)
}

/// True when the host must not open GUI dialogs (CI, JSONL automation, servers).
/// Set `ALVENQIS_HOST_HEADLESS=1` to force this even on Windows.
fn headless_mode() -> bool {
    env_truthy("ALVENQIS_HOST_HEADLESS")
}

/// Require the operator to acknowledge a recovery phrase **before** the keystore is written.
///
/// The mnemonic is never returned over native messaging JSON. It is shown only via:
/// - Windows MessageBox (operator must click OK after writing it down), or
/// - stderr + `ALVENQIS_HOST_RECOVERY_ACK=1` for headless/jsonl tests and non-Windows hosts.
///
/// If acknowledgement fails, the caller must not persist the keystore and must drop the
/// mnemonic immediately.
pub fn confirm_recovery_backup(mnemonic: &str) -> Result<(), String> {
    let summary = format!(
        "CRITICAL: write down this recovery phrase NOW.\n\
It will NOT be shown again and is NEVER returned to the browser extension.\n\n\
{mnemonic}\n\n\
Click OK only after you have backed it up offline on paper.\n\
Cancel aborts wallet creation (no keystore is written)."
    );

    // Headless / CI path: phrase on stderr, explicit ack env required.
    if env_truthy("ALVENQIS_HOST_RECOVERY_ACK") {
        eprintln!("alvenqis-browser-host RECOVERY PHRASE (write down offline):\n{mnemonic}");
        eprintln!("ALVENQIS_HOST_RECOVERY_ACK accepted; proceeding with keystore create.");
        return Ok(());
    }

    // Force refuse without GUI (tests / servers). Never silently create a wallet.
    if headless_mode() {
        eprintln!("alvenqis-browser-host RECOVERY PHRASE (write down offline):\n{mnemonic}");
        eprintln!(
            "Headless mode: set ALVENQIS_HOST_RECOVERY_ACK=1 after writing the phrase down, \
or use host CLI --init-wallet."
        );
        return Err(
            "create_wallet blocked: recovery backup not acknowledged in headless mode \
(set ALVENQIS_HOST_RECOVERY_ACK=1 or use host CLI --init-wallet)"
                .to_owned(),
        );
    }

    #[cfg(windows)]
    {
        windows_confirm(
            "Alvenqis recovery phrase — backup required",
            &summary,
            "create_wallet cancelled: recovery phrase was not acknowledged; no keystore written",
        )
    }

    #[cfg(not(windows))]
    {
        eprintln!("alvenqis-browser-host RECOVERY PHRASE (write down offline):\n{mnemonic}");
        eprintln!(
            "Set ALVENQIS_HOST_RECOVERY_ACK=1 after writing the phrase down to allow create_wallet, \
or use host CLI --init-wallet for an offline backup flow."
        );
        Err("create_wallet blocked: recovery backup not acknowledged \
(set ALVENQIS_HOST_RECOVERY_ACK=1 or use host CLI --init-wallet)"
            .to_owned())
    }
}

#[cfg(windows)]
fn windows_confirm(title: &str, body: &str, cancel_message: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    const MB_OKCANCEL: u32 = 0x0000_0001;
    const MB_ICONWARNING: u32 = 0x0000_0030;
    const MB_SETFOREGROUND: u32 = 0x0001_0000;
    const MB_TOPMOST: u32 = 0x0004_0000;
    const IDOK: i32 = 1;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            hwnd: *mut core::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            flags: u32,
        ) -> i32;
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let text = wide(body);
    let caption = wide(title);
    // SAFETY: MessageBoxW with null HWND and NUL-terminated UTF-16 strings.
    let result = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OKCANCEL | MB_ICONWARNING | MB_SETFOREGROUND | MB_TOPMOST,
        )
    };
    if result == IDOK {
        Ok(())
    } else {
        Err(cancel_message.to_owned())
    }
}
