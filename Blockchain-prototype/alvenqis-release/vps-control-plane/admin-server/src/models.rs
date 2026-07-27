use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ServiceStates {
    pub node: String,
    pub rpc: String,
    pub indexer_timer: String,
    pub admin: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NodeReport {
    pub network_id: String,
    pub node_name: String,
    pub advertise_host: String,
    pub p2p_multiaddr: String,
    pub reported_at_unix_seconds: u64,
    pub services: ServiceStates,
    pub status: Value,
    pub sync: Value,
    pub p2p: Value,
    pub mempool: Value,
    pub indexer: Value,
}

#[derive(Clone, Debug, Deserialize)]
pub struct InvitationRequest {
    pub node_name: String,
    pub advertise_host: String,
    #[serde(default)]
    pub admin_domain: Option<String>,
    pub acme_email: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct InvitationResponse {
    pub invitation_id: String,
    pub expires_at_unix_seconds: u64,
    pub install_command: String,
    /// Multi-step operator guide for the Add Node wizard.
    pub steps: Vec<EnrollmentStep>,
    pub seed_multiaddr: String,
    pub controller_url: String,
    pub node_name: String,
    pub advertise_host: String,
    pub ttl_seconds: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct EnrollmentStep {
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct InvitationView {
    pub invitation_id: String,
    pub node_name: String,
    pub advertise_host: String,
    pub expires_at_unix_seconds: u64,
    pub used: bool,
    pub expired: bool,
    pub status: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct NodeDetailView {
    pub node: FleetNodeView,
    pub report: NodeReport,
    pub is_local_controller: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnrollmentRequest {
    pub invitation_token: String,
    pub report: NodeReport,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnrollmentResponse {
    pub node_id: String,
    pub node_token: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReportRequest {
    pub node_id: String,
    pub report: NodeReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct FleetNodeView {
    pub node_id: String,
    pub node_name: String,
    pub advertise_host: String,
    pub p2p_multiaddr: String,
    pub last_seen_unix_seconds: u64,
    pub online: bool,
    pub peer_id: Option<String>,
    pub height: Option<u64>,
    pub connected_peers: u64,
    pub validating_peers: u64,
    pub mining_peers: u64,
    pub observed_hashrate_hs: u64,
    pub services: ServiceStates,
}

#[derive(Clone, Debug, Serialize)]
pub struct FleetTopology {
    pub mode: &'static str,
    pub network_id: String,
    pub generated_at_unix_seconds: u64,
    pub registered_node_count: usize,
    pub online_node_count: usize,
    pub direct_validated_connections: u64,
    pub observed_miner_count: u64,
    pub observed_hashrate_hs: u64,
    pub nodes: Vec<FleetNodeView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminOverview {
    pub mode: &'static str,
    pub status_label: String,
    pub local: NodeReport,
    pub topology: FleetTopology,
}

pub fn value_u64(value: &Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(|item| {
            item.as_u64()
                .or_else(|| item.as_f64().map(|number| number.max(0.0) as u64))
                .or_else(|| item.as_i64().map(|number| number.max(0) as u64))
                .or_else(|| {
                    item.as_str()
                        .and_then(|raw| raw.parse::<f64>().ok())
                        .map(|number| number.max(0.0) as u64)
                })
        })
        .unwrap_or(0)
}

pub fn value_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

pub type EndpointErrors = BTreeMap<String, String>;
