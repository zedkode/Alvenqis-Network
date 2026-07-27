use std::path::PathBuf;
use std::process::{Command, Output};

fn host_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_alvenqis-browser-host"))
}

/// Live public-RPC tests should not hard-fail CI when the candidate gateway is down.
fn skip_if_public_rpc_unavailable(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let blob = format!("{stderr}\n{stdout}").to_ascii_lowercase();
    let unavailable = blob.contains("502")
        || blob.contains("503")
        || blob.contains("504")
        || blob.contains("500")
        || blob.contains("bad gateway")
        || blob.contains("internal server error")
        || blob.contains("connection refused")
        || blob.contains("timed out")
        || blob.contains("error sending request")
        || blob.contains("could not reach")
        || blob.contains("dns error")
        || blob.contains("failed to lookup")
        || blob.contains("rpc returned http");
    if unavailable {
        eprintln!("skip: public candidate RPC unavailable (live network)\n{stderr}{stdout}");
    }
    unavailable
}

#[test]
fn print_tip_against_public_candidate() {
    let output = Command::new(host_bin())
        .arg("--print-tip")
        .output()
        .expect("spawn host");
    if skip_if_public_rpc_unavailable(&output) {
        return;
    }
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("height="), "stdout={stdout}");
    assert!(stdout.contains("hash="), "stdout={stdout}");
}

#[test]
fn print_chain_json_has_network_id() {
    let output = Command::new(host_bin())
        .arg("--print-chain")
        .output()
        .expect("spawn host");
    if skip_if_public_rpc_unavailable(&output) {
        return;
    }
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("alvenqis-mainnet-candidate"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("Mainnet Candidate"), "stdout={stdout}");
}

#[test]
fn print_tip_json_is_object() {
    let output = Command::new(host_bin())
        .args(["--print-tip", "--json"])
        .output()
        .expect("spawn host");
    if skip_if_public_rpc_unavailable(&output) {
        return;
    }
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("tip json parses");
    assert!(value.get("height").and_then(|v| v.as_u64()).is_some());
    assert!(value.get("hash").and_then(|v| v.as_str()).is_some());
}

#[test]
fn check_health_public_candidate_ok() {
    let output = Command::new(host_bin())
        .args(["--check-health", "--json"])
        .output()
        .expect("spawn host");
    if skip_if_public_rpc_unavailable(&output) {
        return;
    }
    // Public candidate should be chain-ready (code 0) when RPC is up.
    assert!(
        output.status.success(),
        "stderr={} stdout={} code={:?}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
        output.status.code()
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("health json parses");
    assert_eq!(value.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert!(value.get("height").and_then(|v| v.as_u64()).is_some());
}

#[test]
fn check_health_max_indexer_lag_zero_strictish() {
    let output = Command::new(host_bin())
        .args(["--check-health", "--json", "--max-indexer-lag", "0"])
        .output()
        .expect("spawn host");
    if skip_if_public_rpc_unavailable(&output) {
        return;
    }
    // lag allowed=0: healthy (0) or indexer lag (3). Gateway down already skipped above.
    let code = output.status.code();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        code == Some(0) || code == Some(3),
        "expected exit 0 or 3; got {code:?} stderr={stderr} stdout={stdout}"
    );
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("health json parses");
    assert_eq!(
        value.get("max_indexer_lag").and_then(|v| v.as_u64()),
        Some(0)
    );
    if code == Some(0) {
        assert_eq!(value.get("ok").and_then(|v| v.as_bool()), Some(true));
    } else {
        assert_eq!(value.get("ok").and_then(|v| v.as_bool()), Some(false));
    }
}

#[test]
fn print_info_json_has_service() {
    let output = Command::new(host_bin())
        .args(["--print-info", "--json"])
        .output()
        .expect("spawn host");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("info json parses");
    assert_eq!(
        value.get("service").and_then(|v| v.as_str()),
        Some("alvenqis-browser-host")
    );
    assert_eq!(
        value.get("network_id").and_then(|v| v.as_str()),
        Some("alvenqis-mainnet-candidate")
    );
}
