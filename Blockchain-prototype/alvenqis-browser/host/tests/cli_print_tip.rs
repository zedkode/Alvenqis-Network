use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

fn host_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_alvenqis-browser-host"))
}

fn spawn_rpc_fixture(expected_requests: usize) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind RPC fixture");
    listener
        .set_nonblocking(true)
        .expect("set fixture nonblocking");
    let address = listener.local_addr().expect("fixture address");
    let handle = thread::spawn(move || {
        let started = Instant::now();
        let mut served = 0;
        while served < expected_requests {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .expect("fixture read timeout");
                    let mut request_line = String::new();
                    BufReader::new(stream.try_clone().expect("clone fixture stream"))
                        .read_line(&mut request_line)
                        .expect("read fixture request line");
                    let path = request_line
                        .split_whitespace()
                        .nth(1)
                        .expect("request path");
                    let body = fixture_body(path);
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .expect("write fixture response");
                    served += 1;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        started.elapsed() < Duration::from_secs(10),
                        "fixture received {served}/{expected_requests} requests"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("fixture accept failed: {error}"),
            }
        }
    });
    (format!("http://{address}"), handle)
}

fn fixture_body(path: &str) -> &'static str {
    match path {
        "/status" => {
            r#"{"network_id":"alvenqis-mainnet-candidate","network_name":"Alvenqis Mainnet Candidate","status_label":"Mainnet Candidate","initialized":true,"block_count":8,"height":7,"tip_hash":"fixture-tip","emitted_supply_atomic":100}"#
        }
        "/chain/tip" => r#"{"height":7,"hash":"fixture-tip"}"#,
        "/sync/status" => {
            r#"{"network_id":"alvenqis-mainnet-candidate","sync_state":"synced","local_height":7,"network_height":7,"remaining_blocks":0,"progress_percent":100.0,"connected_peer_count":1,"validated_peer_count":1}"#
        }
        "/mempool/status" => {
            r#"{"status":"ready","pending_count":0,"anticipated_base_fee_atomic":1}"#
        }
        "/supply" => {
            r#"{"emitted_supply_atomic":100,"max_supply_atomic":6000000000000000,"remaining_supply_atomic":5999999999999900}"#
        }
        "/indexer/status" => {
            r#"{"mode":"sqlite","network_id":"alvenqis-mainnet-candidate","status_label":"Mainnet Candidate","initialized":true,"indexed_height":7,"indexed_block_count":8,"transaction_count":1,"address_count":1,"tip_hash":"fixture-tip","chain_height":7,"chain_tip_hash":"fixture-tip","in_sync":true,"lag_blocks":0}"#
        }
        _ => panic!("unexpected fixture path: {path}"),
    }
}

#[test]
fn print_tip_against_rpc_fixture() {
    let (rpc_url, fixture) = spawn_rpc_fixture(1);
    let output = Command::new(host_bin())
        .args(["--print-tip", "--rpc", &rpc_url])
        .output()
        .expect("spawn host");
    fixture.join().expect("RPC fixture");
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
    let (rpc_url, fixture) = spawn_rpc_fixture(6);
    let output = Command::new(host_bin())
        .args(["--print-chain", "--rpc", &rpc_url])
        .output()
        .expect("spawn host");
    fixture.join().expect("RPC fixture");
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
    let (rpc_url, fixture) = spawn_rpc_fixture(1);
    let output = Command::new(host_bin())
        .args(["--print-tip", "--json", "--rpc", &rpc_url])
        .output()
        .expect("spawn host");
    fixture.join().expect("RPC fixture");
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
fn check_health_fixture_ok() {
    let (rpc_url, fixture) = spawn_rpc_fixture(3);
    let output = Command::new(host_bin())
        .args(["--check-health", "--json", "--rpc", &rpc_url])
        .output()
        .expect("spawn host");
    fixture.join().expect("RPC fixture");
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
fn check_health_max_indexer_lag_zero_passes_in_sync_fixture() {
    let (rpc_url, fixture) = spawn_rpc_fixture(3);
    let output = Command::new(host_bin())
        .args([
            "--check-health",
            "--json",
            "--max-indexer-lag",
            "0",
            "--rpc",
            &rpc_url,
        ])
        .output()
        .expect("spawn host");
    fixture.join().expect("RPC fixture");
    let code = output.status.code();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        code,
        Some(0),
        "expected healthy fixture; stderr={stderr} stdout={stdout}"
    );
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("health json parses");
    assert_eq!(
        value.get("max_indexer_lag").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(value.get("ok").and_then(|v| v.as_bool()), Some(true));
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
