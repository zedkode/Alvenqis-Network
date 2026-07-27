//! TLS-only Alvenqis Stratum v1 server.
//!
//! The wire protocol is newline-delimited JSON-RPC over a persistent TLS socket.
//! Wallet addresses authenticate worker identity; no wallet private key or signature is sent.

use crate::app::{get_work_for_peer, submit_share_for_peer, PoolState};
use crate::config::StratumConfig;
use crate::{PoolError, Result};
use alvenqis_miner::{MiningSubmitRequest, MiningTemplate, SubmitStatus};
use rustls_pemfile::{certs, private_key};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs::File;
use std::io::BufReader as StdBufReader;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt,
    BufReader as TokioBufReader,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Semaphore};
use tokio::time::timeout;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;

pub const STRATUM_PROTOCOL: &str = "alvenqis-stratum-v1";

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(default)]
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Clone, Debug)]
struct WorkerIdentity {
    miner_address: String,
    worker_name: String,
}

struct Session {
    subscribed: bool,
    identity: Option<WorkerIdentity>,
}

struct RpcOutcome {
    response: Value,
    initial_template: Option<MiningTemplate>,
}

pub async fn serve(state: PoolState, config: StratumConfig) -> Result<()> {
    let address: SocketAddr = format!("{}:{}", config.bind_host, config.bind_port)
        .parse()
        .map_err(|error| PoolError::Config(format!("invalid Stratum bind address: {error}")))?;
    let listener = TcpListener::bind(address)
        .await
        .map_err(|error| PoolError::Config(format!("cannot bind Stratum {address}: {error}")))?;
    let acceptor = load_tls_acceptor(&config)?;
    println!(
        "alvenqis-mining-pool Stratum TLS listening on stratum+tls://{address} ({STRATUM_PROTOCOL})"
    );
    let refresh_state = state.clone();
    let refresh_seconds = state.config.job_cache_seconds.max(1);
    tokio::spawn(async move {
        loop {
            if let Err(error) =
                crate::app::ensure_current_job(&refresh_state, crate::app::unix_seconds()).await
            {
                eprintln!("stratum upstream refresh failed: {error}");
            }
            tokio::time::sleep(Duration::from_secs(refresh_seconds)).await;
        }
    });
    serve_listener(listener, state, config, acceptor).await
}

async fn serve_listener(
    listener: TcpListener,
    state: PoolState,
    config: StratumConfig,
    acceptor: TlsAcceptor,
) -> Result<()> {
    let permits = Arc::new(Semaphore::new(config.max_connections));
    loop {
        let (tcp, peer) = listener
            .accept()
            .await
            .map_err(|error| PoolError::Storage(format!("Stratum accept failed: {error}")))?;
        let permit = match Arc::clone(&permits).try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                // Fail closed when the configured concurrency bound is reached.
                drop(tcp);
                continue;
            }
        };
        let acceptor = acceptor.clone();
        let state = state.clone();
        let config = config.clone();
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(error) = serve_connection(acceptor, tcp, peer, state, config).await {
                eprintln!("stratum peer {peer} disconnected: {error}");
            }
        });
    }
}

async fn serve_connection(
    acceptor: TlsAcceptor,
    tcp: TcpStream,
    peer: SocketAddr,
    state: PoolState,
    config: StratumConfig,
) -> Result<()> {
    tcp.set_nodelay(true)
        .map_err(|error| PoolError::Storage(format!("Stratum TCP setup failed: {error}")))?;
    let tls = timeout(
        Duration::from_secs(config.handshake_timeout_seconds),
        acceptor.accept(tcp),
    )
    .await
    .map_err(|_| {
        PoolError::Config(format!(
            "TLS handshake timeout after {}s (client must use stratum+tls and complete TLS before JSON-RPC)",
            config.handshake_timeout_seconds
        ))
    })?
    .map_err(|error| {
        PoolError::Config(format!(
            "TLS handshake failed: {error} (use stratum+tls://HOST:PORT with a valid client TLS stack; plaintext TCP is rejected)"
        ))
    })?;
    run_session(tls, peer.ip(), state, config).await
}

async fn run_session(
    tls: TlsStream<TcpStream>,
    peer_ip: IpAddr,
    state: PoolState,
    config: StratumConfig,
) -> Result<()> {
    let (read_half, mut write_half) = tokio::io::split(tls);
    let mut reader = TokioBufReader::new(read_half);
    let mut updates = state.subscribe_jobs();
    let mut session = Session {
        subscribed: false,
        identity: None,
    };
    let idle = Duration::from_secs(config.idle_timeout_seconds);

    loop {
        tokio::select! {
            line = timeout(idle, read_line_limited(&mut reader, config.max_line_bytes)) => {
                let line = line
                    .map_err(|_| {
                        PoolError::Config(format!(
                            "idle timeout after {}s with no JSON-RPC line (send mining.subscribe then mining.authorize as wallet.worker)",
                            config.idle_timeout_seconds
                        ))
                    })?
                    .map_err(|error| {
                        let msg = error.to_string();
                        // Client aborts without TLS close_notify are common on reconnect; not a share crime.
                        if msg.contains("close_notify")
                            || msg.contains("Connection reset")
                            || msg.contains("Broken pipe")
                            || msg.contains("early eof")
                            || msg.contains("UnexpectedEof")
                        {
                            PoolError::Config(format!("peer closed TLS session: {msg}"))
                        } else {
                            PoolError::InvalidShare(format!("invalid Stratum frame: {msg}"))
                        }
                    })?;
                let Some(line) = line else {
                    return Ok(());
                };
                let request: JsonRpcRequest = match serde_json::from_slice(&line) {
                    Ok(request) => request,
                    Err(error) => {
                        write_json(&mut write_half, &rpc_error(Value::Null, -32700, format!("parse error: {error}"))).await?;
                        continue;
                    }
                };
                let outcome = handle_request(&state, peer_ip, &mut session, request).await;
                write_json(&mut write_half, &outcome.response).await?;
                if let Some(template) = outcome.initial_template {
                    write_notify(&mut write_half, template, true).await?;
                }
            }
            update = updates.recv(), if session.identity.is_some() => {
                match update {
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {
                        if let Some(identity) = session.identity.as_ref() {
                            match get_work_for_peer(
                                &state,
                                peer_ip,
                                &identity.miner_address,
                                &identity.worker_name,
                            ).await {
                                Ok(template) => write_notify(&mut write_half, template, true).await?,
                                Err(error) => {
                                    write_json(
                                        &mut write_half,
                                        &json!({
                                            "id": null,
                                            "method": "client.show_message",
                                            "params": [format!("work refresh failed: {error}")]
                                        }),
                                    ).await?;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                }
            }
        }
    }
}

async fn handle_request(
    state: &PoolState,
    peer_ip: IpAddr,
    session: &mut Session,
    request: JsonRpcRequest,
) -> RpcOutcome {
    let id = request.id.clone();
    let result = match request.method.as_str() {
        "mining.subscribe" => {
            session.subscribed = true;
            Ok((
                json!([
                    [["mining.notify", STRATUM_PROTOCOL]],
                    STRATUM_PROTOCOL,
                    STRATUM_PROTOCOL
                ]),
                None,
            ))
        }
        "mining.authorize" => {
            if !session.subscribed {
                Err((-32001, "mining.subscribe is required first".to_owned()))
            } else {
                match parse_authorize(&request.params) {
                    Ok(identity) => {
                        // Always accept a well-formed wallet.worker login.
                        // Work is best-effort so a temporary upstream glitch does not reject auth
                        // and force a reconnect storm (which looked like "unauthorized" TLS churn).
                        match get_work_for_peer(
                            state,
                            peer_ip,
                            &identity.miner_address,
                            &identity.worker_name,
                        )
                        .await
                        {
                            Ok(template) => {
                                session.identity = Some(identity);
                                Ok((json!(true), Some(template)))
                            }
                            Err(error) => {
                                eprintln!(
                                    "stratum authorize ok for {}.{} but initial work failed: {error}",
                                    identity.miner_address, identity.worker_name
                                );
                                session.identity = Some(identity);
                                Ok((json!(true), None))
                            }
                        }
                    }
                    Err(error) => Err((-32602, error)),
                }
            }
        }
        "mining.get_work" => match session.identity.as_ref() {
            Some(identity) => match get_work_for_peer(
                state,
                peer_ip,
                &identity.miner_address,
                &identity.worker_name,
            )
            .await
            {
                Ok(template) => Ok((
                    json!({
                        "protocol": STRATUM_PROTOCOL,
                        "job_id": template.template_id,
                        "template": template,
                        "clean_jobs": false
                    }),
                    None,
                )),
                Err(error) => Err((-32002, error.to_string())),
            },
            None => Err((-32001, "worker is not authorized".to_owned())),
        },
        "mining.submit" => match session.identity.as_ref() {
            Some(identity) => match parse_submit(&request.params, identity) {
                Ok(submission) => match submit_share_for_peer(
                    state,
                    peer_ip,
                    &submission,
                    &identity.miner_address,
                    &identity.worker_name,
                )
                .await
                {
                    Ok(response) => {
                        let accepted = matches!(
                            response.status,
                            SubmitStatus::Accepted | SubmitStatus::PendingLocal
                        );
                        Ok((
                            json!({
                                "accepted": accepted,
                                "status": response.status,
                                "template_id": response.template_id,
                                "block_hash": response.block_hash,
                                "height": response.height,
                                "reason": response.reason
                            }),
                            None,
                        ))
                    }
                    Err(error) => Err((-32003, error.to_string())),
                },
                Err(error) => Err((-32602, error)),
            },
            None => Err((-32001, "worker is not authorized".to_owned())),
        },
        "client.get_version" => Ok((json!(env!("CARGO_PKG_VERSION")), None)),
        _ => Err((-32601, format!("method not found: {}", request.method))),
    };

    match result {
        Ok((value, initial_template)) => RpcOutcome {
            response: json!({"id": id, "result": value, "error": null}),
            initial_template,
        },
        Err((code, message)) => RpcOutcome {
            response: rpc_error(id, code, message),
            initial_template: None,
        },
    }
}

fn parse_authorize(params: &Value) -> std::result::Result<WorkerIdentity, String> {
    let user = params
        .as_array()
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "authorize requires [\"alve1….worker\", \"password\"] — not Bitcoin Stratum user formats"
                .to_owned()
        })?;
    // Prefer the last '.' so wallet bech32 stays intact if a worker name has dots.
    let (miner_address, worker_name) = match user.rsplit_once('.') {
        Some(parts) => parts,
        None => {
            // Convenience: bare wallet → worker "default"
            (user, "default")
        }
    };
    if miner_address.is_empty() || worker_name.is_empty() {
        return Err("worker login must be wallet.worker (example: alve1qq…desktop-01)".to_owned());
    }
    if !miner_address.starts_with("alve1") {
        return Err(format!(
            "miner address must be an Alvenqis alve1… bech32 wallet, got '{}'",
            &miner_address[..miner_address.len().min(24)]
        ));
    }
    Ok(WorkerIdentity {
        miner_address: miner_address.to_owned(),
        worker_name: worker_name.to_owned(),
    })
}

fn parse_submit(
    params: &Value,
    identity: &WorkerIdentity,
) -> std::result::Result<MiningSubmitRequest, String> {
    let values = params
        .as_array()
        .ok_or_else(|| "submit params must be an array".to_owned())?;
    let user = values
        .first()
        .and_then(Value::as_str)
        .ok_or_else(|| "submit is missing wallet.worker".to_owned())?;
    let expected = format!("{}.{}", identity.miner_address, identity.worker_name);
    if user != expected {
        return Err("submit identity does not match authorized worker".to_owned());
    }
    let mut request: MiningSubmitRequest = serde_json::from_value(
        values
            .get(4)
            .cloned()
            .ok_or_else(|| "submit is missing the solution payload".to_owned())?,
    )
    .map_err(|error| format!("invalid solution payload: {error}"))?;
    request.miner_address = Some(identity.miner_address.clone());
    request.worker_name = Some(identity.worker_name.clone());
    Ok(request)
}

async fn write_notify<W>(writer: &mut W, template: MiningTemplate, clean_jobs: bool) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    write_json(
        writer,
        &json!({
            "id": null,
            "method": "mining.notify",
            "params": {
                "job_id": template.template_id,
                "template": template,
                "clean_jobs": clean_jobs
            }
        }),
    )
    .await
}

async fn write_json<W>(writer: &mut W, value: &Value) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut frame = serde_json::to_vec(value)
        .map_err(|error| PoolError::Storage(format!("cannot encode Stratum response: {error}")))?;
    frame.push(b'\n');
    writer
        .write_all(&frame)
        .await
        .map_err(|error| PoolError::Storage(format!("cannot write Stratum response: {error}")))?;
    writer
        .flush()
        .await
        .map_err(|error| PoolError::Storage(format!("cannot flush Stratum response: {error}")))
}

async fn read_line_limited<R>(
    reader: &mut R,
    max_line_bytes: usize,
) -> std::io::Result<Option<Vec<u8>>>
where
    R: AsyncBufRead + Unpin,
{
    let mut frame = Vec::with_capacity(512);
    let mut limited = (&mut *reader).take((max_line_bytes + 1) as u64);
    let read = limited.read_until(b'\n', &mut frame).await?;
    if read == 0 {
        return Ok(None);
    }
    if frame.len() > max_line_bytes || frame.last() != Some(&b'\n') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame exceeds configured line limit",
        ));
    }
    while matches!(frame.last(), Some(b'\n' | b'\r')) {
        frame.pop();
    }
    Ok(Some(frame))
}

fn rpc_error(id: Value, code: i64, message: String) -> Value {
    json!({
        "id": id,
        "result": null,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn load_tls_acceptor(config: &StratumConfig) -> Result<TlsAcceptor> {
    // The workspace also enables rustls through reqwest, so both crypto providers can
    // otherwise be present. Select one explicitly before constructing any TLS config.
    let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default();
    let cert_file = File::open(&config.tls_cert_file).map_err(|error| {
        PoolError::Config(format!(
            "cannot open Stratum TLS certificate {}: {error}",
            config.tls_cert_file.display()
        ))
    })?;
    let key_file = File::open(&config.tls_key_file).map_err(|error| {
        PoolError::Config(format!(
            "cannot open Stratum TLS private key {}: {error}",
            config.tls_key_file.display()
        ))
    })?;
    let certificate_chain = certs(&mut StdBufReader::new(cert_file))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| PoolError::Config(format!("invalid Stratum certificate: {error}")))?;
    if certificate_chain.is_empty() {
        return Err(PoolError::Config(
            "Stratum TLS certificate chain is empty".to_owned(),
        ));
    }
    let key = private_key(&mut StdBufReader::new(key_file))
        .map_err(|error| PoolError::Config(format!("invalid Stratum private key: {error}")))?
        .ok_or_else(|| PoolError::Config("Stratum TLS private key is missing".to_owned()))?;
    let tls = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificate_chain, key)
        .map_err(|error| PoolError::Config(format!("invalid Stratum TLS identity: {error}")))?;
    Ok(TlsAcceptor::from(Arc::new(tls)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PoolConfig;
    use crate::PoolStore;
    use alvenqis_core::{Address, Network, PrivateKey};
    use rcgen::{generate_simple_self_signed, CertifiedKey};
    use std::fs;
    use std::path::PathBuf;
    use tokio_rustls::rustls::pki_types::ServerName;
    use tokio_rustls::rustls::{ClientConfig, RootCertStore};
    use tokio_rustls::TlsConnector;

    fn test_pool_config(
        data_dir: PathBuf,
        pool_address: String,
        stratum: StratumConfig,
    ) -> PoolConfig {
        PoolConfig {
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 0,
            network_id: Network::MainnetCandidate.network_id().to_owned(),
            status_label: "Mainnet Candidate / Stratum TLS test".to_owned(),
            pool_name: "TLS Test Pool".to_owned(),
            pool_address,
            upstream_rpc_url: "http://127.0.0.1:1".to_owned(),
            public_url: "https://pool.example.org".to_owned(),
            data_dir,
            admin_token_file: PathBuf::from("unused.token"),
            share_difficulty_leading_zero_bits: 0,
            vardiff_enabled: true,
            min_share_difficulty_leading_zero_bits: 0,
            max_share_difficulty_leading_zero_bits: 8,
            share_network_gap_bits: 4,
            target_share_seconds: 15,
            vardiff_window_shares: 4,
            pool_fee_basis_points: 100,
            pplns_window_shares: 10,
            block_maturity_confirmations: 12,
            minimum_payout_atomic: 1,
            job_cache_seconds: 3,
            hashrate_window_seconds: 60,
            worker_timeout_seconds: 120,
            max_stored_shares: 100,
            max_workers_per_address: 64,
            max_work_requests_per_minute: 240,
            max_share_requests_per_minute: 600,
            invalid_share_ban_threshold: 20,
            ban_seconds: 600,
            admin_token_max_age_seconds: 90 * 24 * 60 * 60,
            cors_allowed_origins: Vec::new(),
            allow_public_pool_prototype: false,
            stratum: Some(stratum),
        }
    }

    #[test]
    fn worker_login_requires_wallet_and_worker() {
        let parsed = parse_authorize(&json!(["alve1example.gpu-1", "x"])).expect("login");
        assert_eq!(parsed.miner_address, "alve1example");
        assert_eq!(parsed.worker_name, "gpu-1");
        // Bare wallet is accepted as worker "default"
        let bare = parse_authorize(&json!(["alve1example", "x"])).expect("bare wallet");
        assert_eq!(bare.worker_name, "default");
        assert!(parse_authorize(&json!([".gpu", "x"])).is_err());
        assert!(parse_authorize(&json!(["btc1notalve.worker", "x"])).is_err());
    }

    #[tokio::test]
    async fn line_reader_rejects_oversized_frames() {
        let bytes = vec![b'a'; 32];
        let mut reader = TokioBufReader::new(bytes.as_slice());
        let error = read_line_limited(&mut reader, 16)
            .await
            .expect_err("oversized frame");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn tls_listener_negotiates_and_answers_json_rpc() {
        let dir = tempfile::tempdir().expect("tempdir");
        let CertifiedKey { cert, key_pair } =
            generate_simple_self_signed(vec!["localhost".to_owned()]).expect("certificate");
        let cert_path = dir.path().join("fullchain.pem");
        let key_path = dir.path().join("privkey.pem");
        fs::write(&cert_path, cert.pem()).expect("write certificate");
        fs::write(&key_path, key_pair.serialize_pem()).expect("write key");
        let stratum = StratumConfig {
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 3333,
            tls_cert_file: cert_path,
            tls_key_file: key_path,
            max_connections: 8,
            max_line_bytes: 64 * 1024,
            handshake_timeout_seconds: 5,
            idle_timeout_seconds: 30,
        };
        let pool_address = Address::from_public_key_for_network(
            &PrivateKey::generate().public_key(),
            Network::MainnetCandidate,
        )
        .to_string();
        let config = test_pool_config(dir.path().join("data"), pool_address, stratum.clone());
        let store = PoolStore::load(config.data_dir.clone(), 100).expect("store");
        let state = PoolState::new(config, store).expect("state");
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("listener address");
        let acceptor = load_tls_acceptor(&stratum).expect("TLS acceptor");
        let server = tokio::spawn(serve_listener(listener, state, stratum, acceptor));

        let mut roots = RootCertStore::empty();
        roots.add(cert.der().clone()).expect("trusted test root");
        let client = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(client));
        let tcp = TcpStream::connect(address).await.expect("TCP connect");
        let server_name = ServerName::try_from("localhost")
            .expect("server name")
            .to_owned();
        let tls = connector
            .connect(server_name, tcp)
            .await
            .expect("verified TLS handshake");
        let (read_half, mut write_half) = tokio::io::split(tls);
        let mut reader = TokioBufReader::new(read_half);
        write_half
            .write_all(b"{\"id\":1,\"method\":\"mining.subscribe\",\"params\":[]}\n")
            .await
            .expect("subscribe write");
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("subscribe response");
        let response: Value = serde_json::from_str(&line).expect("subscribe JSON");
        assert_eq!(response["id"], 1);
        assert_eq!(response["error"], Value::Null);
        assert_eq!(response["result"][1], STRATUM_PROTOCOL);

        server.abort();
    }
}
