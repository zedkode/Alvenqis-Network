//! Alvenqis Stratum v1 — JSON-RPC over TCP or TLS.
//!
//! This is **not** Bitcoin Stratum. It maps Alvenqis FiroPoW templates through
//! `alvenqis-stratum-v1` methods so GPU miners can use a persistent socket with
//! optional TLS instead of short-lived HTTPS pool calls.
//!
//! Wire format: one JSON object per line (LF-terminated).

use crate::{
    MinerError, MiningSubmitRequest, MiningSubmitResponse, MiningTemplate, Result, SubmitStatus,
    WorkSource, MINING_PROTOCOL_VERSION,
};
use alvenqis_core::{Address, Block};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write;
use std::net::TcpStream;
use std::sync::Mutex;
use std::time::Duration;

pub const STRATUM_PROTOCOL: &str = "alvenqis-stratum-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StratumEndpoint {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub use_tls: bool,
    #[serde(default)]
    pub skip_tls_verify: bool,
    pub worker_name: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_timeout() -> u64 {
    20
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    id: u64,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<Value>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
}

/// Persistent Stratum work source (TCP or TLS).
pub struct StratumWorkSource {
    endpoint: StratumEndpoint,
    miner_address: String,
    next_id: Mutex<u64>,
    stream: Mutex<Option<StreamKind>>,
    last_template: Mutex<Option<MiningTemplate>>,
}

enum StreamKind {
    Plain(TcpStream),
    Tls(native_tls::TlsStream<TcpStream>),
}

impl StreamKind {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.set_read_timeout(timeout),
            Self::Tls(s) => s.get_ref().set_read_timeout(timeout),
        }
    }

    fn set_write_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.set_write_timeout(timeout),
            Self::Tls(s) => s.get_ref().set_write_timeout(timeout),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.write_all(buf),
            Self::Tls(s) => s.write_all(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }

    fn read_line(&mut self) -> std::io::Result<String> {
        use std::io::Read;
        let mut line = Vec::with_capacity(256);
        loop {
            let mut b = [0u8; 1];
            let n = match self {
                Self::Plain(s) => s.read(&mut b)?,
                Self::Tls(s) => s.read(&mut b)?,
            };
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "stratum eof",
                ));
            }
            if b[0] == b'\n' {
                break;
            }
            if b[0] != b'\r' {
                line.push(b[0]);
            }
            if line.len() > 4_000_000 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "stratum line too long",
                ));
            }
        }
        Ok(String::from_utf8_lossy(&line).into_owned())
    }
}

impl StratumWorkSource {
    pub fn new(endpoint: StratumEndpoint, miner_address: String) -> Result<Self> {
        Address::parse(&miner_address).map_err(|e| MinerError::Config(e.to_string()))?;
        if endpoint.host.trim().is_empty() {
            return Err(MinerError::Config("stratum host is empty".into()));
        }
        if endpoint.port == 0 {
            return Err(MinerError::Config("stratum port must be non-zero".into()));
        }
        let local = matches!(
            endpoint.host.trim().to_ascii_lowercase().as_str(),
            "127.0.0.1" | "localhost" | "::1" | "[::1]"
        );
        if !endpoint.use_tls && !local {
            return Err(MinerError::Config(
                "remote Stratum requires TLS; plaintext is limited to loopback".into(),
            ));
        }
        if endpoint.skip_tls_verify && !local {
            return Err(MinerError::Config(
                "remote Stratum certificate verification cannot be disabled".into(),
            ));
        }
        Ok(Self {
            endpoint,
            miner_address,
            next_id: Mutex::new(1),
            stream: Mutex::new(None),
            last_template: Mutex::new(None),
        })
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.endpoint.timeout_seconds.max(5))
    }

    fn connect_locked(&self, slot: &mut Option<StreamKind>) -> Result<()> {
        if slot.is_some() {
            return Ok(());
        }
        let addr = format!("{}:{}", self.endpoint.host, self.endpoint.port);
        let sock_addrs: Vec<std::net::SocketAddr> = std::net::ToSocketAddrs::to_socket_addrs(&addr)
            .map_err(|e| MinerError::Rpc(format!("stratum resolve {addr}: {e}")))?
            .collect();
        if sock_addrs.is_empty() {
            return Err(MinerError::Rpc(format!(
                "stratum resolve {addr}: no addresses"
            )));
        }
        let mut last_error = None;
        let mut connected = None;
        for target in sock_addrs {
            match TcpStream::connect_timeout(&target, self.timeout()) {
                Ok(stream) => {
                    connected = Some(stream);
                    break;
                }
                Err(error) => last_error = Some(error),
            }
        }
        let tcp = connected.ok_or_else(|| {
            MinerError::Rpc(format!(
                "stratum connect {addr}: {}",
                last_error
                    .map(|error| error.to_string())
                    .unwrap_or_else(|| "all resolved addresses failed".to_owned())
            ))
        })?;
        tcp.set_nodelay(true).ok();
        let stream = if self.endpoint.use_tls {
            let connector = if self.endpoint.skip_tls_verify {
                native_tls::TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .danger_accept_invalid_hostnames(true)
                    .build()
            } else {
                native_tls::TlsConnector::new()
            }
            .map_err(|e| MinerError::Rpc(format!("stratum tls connector: {e}")))?;
            let tls = connector
                .connect(&self.endpoint.host, tcp)
                .map_err(|e| MinerError::Rpc(format!("stratum tls handshake: {e}")))?;
            StreamKind::Tls(tls)
        } else {
            StreamKind::Plain(tcp)
        };
        stream
            .set_read_timeout(Some(self.timeout()))
            .map_err(|e| MinerError::Rpc(e.to_string()))?;
        stream
            .set_write_timeout(Some(self.timeout()))
            .map_err(|e| MinerError::Rpc(e.to_string()))?;
        *slot = Some(stream);
        if let Err(error) = self.handshake(slot) {
            *slot = None;
            return Err(error);
        }
        Ok(())
    }

    fn handshake(&self, slot: &mut Option<StreamKind>) -> Result<()> {
        let sub = self.rpc(
            slot,
            "mining.subscribe",
            json!([
                format!("alvenqis-miner/{STRATUM_PROTOCOL}"),
                STRATUM_PROTOCOL
            ]),
        )?;
        if sub.get("error").and_then(|e| e.as_object()).is_some() {
            return Err(MinerError::Rpc(format!(
                "stratum subscribe rejected: {sub}"
            )));
        }
        let user = format!("{}.{}", self.miner_address, self.endpoint.worker_name);
        let auth = self.rpc(
            slot,
            "mining.authorize",
            json!([user, self.endpoint.password]),
        )?;
        let ok = auth
            .get("result")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !ok {
            return Err(MinerError::Rpc(format!(
                "stratum authorize failed for {user}: {auth}"
            )));
        }
        Ok(())
    }

    fn next_id(&self) -> u64 {
        let mut g = self.next_id.lock().expect("id lock");
        let id = *g;
        *g = g.saturating_add(1);
        id
    }

    fn rpc(&self, slot: &mut Option<StreamKind>, method: &str, params: Value) -> Result<Value> {
        self.connect_locked(slot)?;
        let id = self.next_id();
        let req = JsonRpcRequest {
            id,
            method: method.to_owned(),
            params,
        };
        let mut line = serde_json::to_string(&req).map_err(|e| MinerError::Rpc(e.to_string()))?;
        line.push('\n');
        let write_result = {
            let stream = slot
                .as_mut()
                .ok_or_else(|| MinerError::Rpc("stratum not connected".into()))?;
            stream
                .write_all(line.as_bytes())
                .and_then(|_| stream.flush())
        };
        if let Err(error) = write_result {
            *slot = None;
            return Err(MinerError::Rpc(format!("stratum write: {error}")));
        }
        // Read until matching id or mining.notify side-channel.
        for _ in 0..32 {
            let raw = self.read_line(slot)?;
            let msg: JsonRpcResponse = serde_json::from_str(&raw)
                .map_err(|e| MinerError::Rpc(format!("stratum decode: {e}; line={raw}")))?;
            if let Some(method) = msg.method.as_deref() {
                if method == "mining.notify" {
                    if let Some(params) = msg.params {
                        self.ingest_notify(params)?;
                    }
                    continue;
                }
            }
            if msg.id == Some(id) {
                if let Some(err) = msg.error {
                    return Err(MinerError::Rpc(format!("stratum error: {err}")));
                }
                return Ok(json!({ "result": msg.result }));
            }
        }
        Err(MinerError::Rpc(
            "stratum timeout waiting for JSON-RPC response".into(),
        ))
    }

    fn read_line(&self, slot: &mut Option<StreamKind>) -> Result<String> {
        let stream = slot
            .as_mut()
            .ok_or_else(|| MinerError::Rpc("stratum not connected".into()))?;
        match stream.read_line() {
            Ok(line) => Ok(line),
            Err(e) => {
                *slot = None;
                Err(MinerError::Rpc(format!("stratum read: {e}")))
            }
        }
    }

    fn ingest_notify(&self, params: Value) -> Result<()> {
        // Prefer full template object in params.template or params[1]
        let template_val = params
            .get("template")
            .cloned()
            .or_else(|| params.as_array().and_then(|a| a.get(1).cloned()))
            .ok_or_else(|| {
                MinerError::Rpc("stratum mining.notify missing template payload".into())
            })?;
        let template: MiningTemplate = serde_json::from_value(template_val)
            .map_err(|e| MinerError::Rpc(format!("stratum template parse: {e}")))?;
        *self.last_template.lock().expect("tpl lock") = Some(template);
        Ok(())
    }

    fn ensure_template(&self) -> Result<MiningTemplate> {
        let mut slot = self.stream.lock().expect("stream lock");
        // Prefer get_work (pull) so clients work without push notifies.
        let resp = self.rpc(
            &mut slot,
            "mining.get_work",
            json!([
                self.miner_address,
                self.endpoint.worker_name,
                MINING_PROTOCOL_VERSION
            ]),
        );
        match resp {
            Ok(body) => {
                if let Some(result) = body.get("result") {
                    if let Some(tpl) = result.get("template").cloned().or_else(|| {
                        if result.get("protocol").is_some() {
                            Some(result.clone())
                        } else {
                            None
                        }
                    }) {
                        let template: MiningTemplate = serde_json::from_value(tpl)
                            .map_err(|e| MinerError::Rpc(format!("stratum get_work: {e}")))?;
                        *self.last_template.lock().expect("tpl lock") = Some(template.clone());
                        return Ok(template);
                    }
                }
            }
            Err(err) => {
                // Drop connection so next call reconnects.
                *slot = None;
                // Fall through to cached template if any.
                if let Some(cached) = self.last_template.lock().expect("tpl lock").clone() {
                    return Ok(cached);
                }
                return Err(err);
            }
        }
        if let Some(cached) = self.last_template.lock().expect("tpl lock").clone() {
            return Ok(cached);
        }
        Err(MinerError::Rpc(
            "stratum server did not return a mining template (implement mining.get_work or mining.notify with alvenqis-mining-v1 template)".into(),
        ))
    }
}

impl WorkSource for StratumWorkSource {
    fn fetch_template(&self, miner_address: &str) -> Result<MiningTemplate> {
        if miner_address != self.miner_address {
            return Err(MinerError::InvalidTemplate(
                "stratum source is bound to a fixed miner_address".into(),
            ));
        }
        self.ensure_template()
    }

    fn submit(&self, request: &MiningSubmitRequest) -> Result<MiningSubmitResponse> {
        let mut slot = self.stream.lock().expect("stream lock");
        let user = format!("{}.{}", self.miner_address, self.endpoint.worker_name);
        let body = match self.rpc(
            &mut slot,
            "mining.submit",
            json!([
                user,
                request.template_id,
                request.nonce,
                request.block_hash,
                request
            ]),
        ) {
            Ok(body) => body,
            Err(error) => {
                *slot = None;
                return Err(error);
            }
        };
        let result = body.get("result").unwrap_or(&Value::Null);
        let accepted = result
            .as_bool()
            .or_else(|| result.get("accepted").and_then(Value::as_bool))
            .unwrap_or(false);
        let status = result
            .get("status")
            .and_then(Value::as_str)
            .and_then(|value| match value {
                "accepted" => Some(SubmitStatus::Accepted),
                "pending_local" => Some(SubmitStatus::PendingLocal),
                "stale" => Some(SubmitStatus::Stale),
                "rejected" => Some(SubmitStatus::Rejected),
                _ => None,
            })
            .unwrap_or(if accepted {
                SubmitStatus::Accepted
            } else {
                SubmitStatus::Rejected
            });
        let reason = result
            .get("reason")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let height = result.get("height").and_then(Value::as_u64);
        Ok(MiningSubmitResponse {
            protocol: MINING_PROTOCOL_VERSION.to_owned(),
            status,
            template_id: request.template_id.clone(),
            block_hash: request.block_hash.clone(),
            height,
            reason: if accepted || status == SubmitStatus::Stale {
                reason
            } else {
                reason.or_else(|| Some(format!("stratum rejected: {body}")))
            },
        })
    }

    fn description(&self) -> String {
        format!(
            "{}://{}:{} worker={}",
            if self.endpoint.use_tls {
                "stratum+tls"
            } else {
                "stratum+tcp"
            },
            self.endpoint.host,
            self.endpoint.port,
            self.endpoint.worker_name
        )
    }

    fn validate_and_build(&self, template: &MiningTemplate, _miner_address: &str) -> Result<Block> {
        let reward_address = template
            .transactions
            .first()
            .map(|transaction| transaction.to.as_str())
            .ok_or_else(|| MinerError::InvalidTemplate("coinbase is missing".to_owned()))?;
        Address::parse(reward_address)?;
        template.validate_and_build(reward_address)
    }
}
