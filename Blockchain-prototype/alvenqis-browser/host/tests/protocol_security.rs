//! Regression tests for CR-C01 (recovery backup) and CR-C02 (OS confirm default).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn host_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_alvenqis-browser-host"))
}

fn run_jsonl(args: &[&str], env: &[(&str, &str)], requests: &str) -> (bool, String, String) {
    let mut cmd = Command::new(host_bin());
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Isolate from parent env; tests set only what they need.
    cmd.env_remove("ALVENQIS_HOST_CONFIRM");
    cmd.env_remove("ALVENQIS_HOST_RECOVERY_ACK");
    cmd.env_remove("ALVENQIS_HOST_HEADLESS");
    cmd.env_remove("ALVENQIS_HOST_REQUIRE_OS_CONFIRM");
    for (k, v) in env {
        cmd.env(k, v);
    }

    let mut child = cmd.spawn().expect("spawn host");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(requests.as_bytes()).expect("write");
    }
    let output = child.wait_with_output().expect("wait");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn default_ping_reports_require_os_confirm_true() {
    let (ok, stdout, stderr) = run_jsonl(
        &["--jsonl", "--local"],
        &[("ALVENQIS_HOST_HEADLESS", "1")],
        r#"{"id":1,"method":"ping"}
"#,
    );
    assert!(ok, "host failed: {stderr}");
    assert!(
        stdout.contains("\"require_os_confirm\":true"),
        "default must require OS confirm; stdout={stdout}"
    );
}

#[test]
fn create_wallet_without_recovery_ack_is_refused() {
    let tmp = tempfile::tempdir().expect("temp");
    let keystore = tmp.path().join("ks");
    std::fs::create_dir_all(&keystore).expect("mkdir");
    let keystore_s = keystore.to_string_lossy().into_owned();

    let (ok, stdout, stderr) = run_jsonl(
        &[
            "--jsonl",
            "--local",
            "--no-require-os-confirm",
            "--keystore-dir",
            &keystore_s,
        ],
        // Headless forces refuse without RECOVERY_ACK on all platforms (no MessageBox hang).
        &[("ALVENQIS_HOST_HEADLESS", "1")],
        r#"{"id":1,"method":"create_wallet","params":{"passphrase":"test-pass-123456"}}
"#,
    );
    assert!(ok, "host process should exit 0: {stderr}");
    assert!(
        stdout.contains("\"ok\":false"),
        "create_wallet without ack must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("recovery")
            || stdout.contains("blocked")
            || stdout.contains("not acknowledged")
            || stdout.contains("headless"),
        "error should mention recovery/headless: {stdout}"
    );
    assert!(
        !keystore.join("browser-host-wallet.json").exists(),
        "keystore must not be written without recovery ack"
    );
    // Phrase may be printed on stderr for the operator to copy, but never in JSON.
    assert!(
        !stdout.to_lowercase().contains("\"phrase\""),
        "must not return recovery phrase field in JSON: {stdout}"
    );
}

#[test]
fn create_wallet_with_recovery_ack_succeeds_and_never_returns_phrase() {
    let tmp = tempfile::tempdir().expect("temp");
    let keystore = tmp.path().join("ks");
    std::fs::create_dir_all(&keystore).expect("mkdir");
    let keystore_s = keystore.to_string_lossy().into_owned();

    let (ok, stdout, stderr) = run_jsonl(
        &[
            "--jsonl",
            "--local",
            "--no-require-os-confirm",
            "--keystore-dir",
            &keystore_s,
        ],
        &[
            ("ALVENQIS_HOST_HEADLESS", "1"),
            ("ALVENQIS_HOST_RECOVERY_ACK", "1"),
        ],
        r#"{"id":1,"method":"create_wallet","params":{"passphrase":"test-pass-123456"}}
{"id":2,"method":"keystore_status"}
"#,
    );
    assert!(ok, "host failed: {stderr}");
    assert!(stdout.contains("\"id\":1"), "stdout={stdout}");
    assert!(
        stdout.contains("\"ok\":true"),
        "create should succeed with ack: {stdout}"
    );
    assert!(
        stdout.contains("\"mnemonic_returned\":false")
            || stdout.contains("\"recovery_acknowledged\":true"),
        "must mark recovery path and never return phrase: {stdout}"
    );
    assert!(
        !stdout.to_lowercase().contains("\"phrase\""),
        "must not return phrase field: {stdout}"
    );
    assert!(
        keystore.join("browser-host-wallet.json").exists(),
        "keystore should exist after ack'd create"
    );
    assert!(
        stderr.contains("RECOVERY PHRASE") || stderr.contains("recovery"),
        "stderr should surface recovery path: {stderr}"
    );
}

#[test]
fn create_session_is_disabled() {
    let (ok, stdout, stderr) = run_jsonl(
        &["--jsonl", "--local", "--no-require-os-confirm"],
        &[("ALVENQIS_HOST_HEADLESS", "1")],
        r#"{"id":1,"method":"create_session"}
"#,
    );
    assert!(ok, "host failed: {stderr}");
    assert!(
        stdout.contains("\"ok\":false"),
        "create_session must be refused: {stdout}"
    );
    assert!(
        stdout.contains("disabled")
            || stdout.contains("create_wallet")
            || stdout.contains("init-wallet"),
        "error should point to safe create path: {stdout}"
    );
}

#[test]
fn send_without_confirm_env_is_blocked_when_os_confirm_on() {
    let tmp = tempfile::tempdir().expect("temp");
    let keystore = tmp.path().join("ks");
    std::fs::create_dir_all(&keystore).expect("mkdir");
    let keystore_s = keystore.to_string_lossy().into_owned();

    // create_wallet with recovery ack, then send WITHOUT ALVENQIS_HOST_CONFIRM while
    // require_os_confirm stays at default true (no --no-require-os-confirm).
    let requests = r#"{"id":1,"method":"create_wallet","params":{"passphrase":"test-pass-123456"}}
{"id":2,"method":"send","params":{"to":"alve1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq","amount_alve":"0.001"}}
"#;
    let (ok, stdout, stderr) = run_jsonl(
        &["--jsonl", "--local", "--keystore-dir", &keystore_s],
        &[
            ("ALVENQIS_HOST_HEADLESS", "1"),
            ("ALVENQIS_HOST_RECOVERY_ACK", "1"),
            // deliberately NO ALVENQIS_HOST_CONFIRM
        ],
        requests,
    );
    assert!(ok, "host failed: {stderr}");
    assert!(
        stdout.contains("blocked")
            || stdout.contains("confirm")
            || (stdout.contains("\"id\":2") && stdout.contains("\"ok\":false")),
        "send without confirm must fail: stdout={stdout} stderr={stderr}"
    );
}
