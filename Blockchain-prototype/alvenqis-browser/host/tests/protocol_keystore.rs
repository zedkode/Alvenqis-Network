use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn host_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_alvenqis-browser-host"))
}

#[test]
fn jsonl_create_wallet_unlock_lock() {
    let tmp = tempfile::tempdir().expect("temp");
    let keystore = tmp.path().join("ks");
    std::fs::create_dir_all(&keystore).expect("mkdir");

    let mut child = Command::new(host_bin())
        .arg("--jsonl")
        .arg("--local")
        .arg("--no-require-os-confirm")
        .arg("--keystore-dir")
        .arg(&keystore)
        .env("ALVENQIS_HOST_HEADLESS", "1")
        .env("ALVENQIS_HOST_RECOVERY_ACK", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn host");

    let mut stdin = child.stdin.take().expect("stdin");
    let requests = r#"
{"id":1,"method":"keystore_status"}
{"id":2,"method":"create_wallet","params":{"passphrase":"test-pass-123"}}
{"id":3,"method":"export_public"}
{"id":4,"method":"lock"}
{"id":5,"method":"unlock","params":{"passphrase":"test-pass-123"}}
{"id":6,"method":"change_passphrase","params":{"old_passphrase":"test-pass-123","new_passphrase":"test-pass-456"}}
{"id":7,"method":"unlock","params":{"passphrase":"test-pass-456"}}
{"id":8,"method":"session_status"}
"#;
    stdin.write_all(requests.as_bytes()).expect("write");
    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    assert!(
        output.status.success(),
        "host failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"id\":2"), "stdout={stdout}");
    assert!(stdout.contains("\"ok\":true"), "stdout={stdout}");
    assert!(stdout.contains("unlocked"), "stdout={stdout}");
    assert!(
        keystore.join("browser-host-wallet.json").exists(),
        "keystore file missing"
    );
    // CR-C01: recovery phrase must never appear in JSON responses.
    assert!(
        !stdout.to_lowercase().contains("\"phrase\""),
        "must not return recovery phrase field: {stdout}"
    );
    assert!(
        stdout.contains("recovery_acknowledged") || stdout.contains("\"ok\":true"),
        "create_wallet response: {stdout}"
    );
}

#[test]
fn jsonl_create_session_rejected() {
    let mut child = Command::new(host_bin())
        .arg("--jsonl")
        .arg("--local")
        .arg("--no-require-os-confirm")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn host");

    let mut stdin = child.stdin.take().expect("stdin");
    stdin
        .write_all(
            br#"{"id":1,"method":"create_session"}
"#,
        )
        .expect("write");
    drop(stdin);

    let output = child.wait_with_output().expect("wait");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"ok\":false"), "stdout={stdout}");
    assert!(
        stdout.contains("disabled") || stdout.contains("create_wallet"),
        "stdout={stdout}"
    );
}
