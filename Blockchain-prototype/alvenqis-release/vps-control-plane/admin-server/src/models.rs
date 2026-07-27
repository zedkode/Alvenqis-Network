use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ServiceStates {
    pub node: String,
    pub rpc: String,
    pub indexer_timer: String,
    pub admin: String,
    pub stratum: String,
    pub explorer: String,
    pub validator: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProbeResult {
    pub configured: bool,
    pub healthy: bool,
    pub checked_at_unix_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ProbeResult {
    pub fn not_configured(detail: impl Into<String>, now: u64) -> Self {
        Self {
            configured: false,
            healthy: false,
            checked_at_unix_seconds: now,
            latency_ms: None,
            detail: detail.into(),
            error: None,
        }
    }
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
    #[serde(default)]
    pub stratum: Value,
    #[serde(default)]
    pub probes: BTreeMap<String, ProbeResult>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvitationRequest {
    #[serde(default = "default_enrollment_role")]
    pub role: DeploymentRole,
    pub node_name: String,
    pub advertise_host: String,
    #[serde(default)]
    pub admin_domain: Option<String>,
    pub acme_email: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct InvitationResponse {
    pub operation_id: String,
    pub replayed: bool,
    pub invitation_id: String,
    pub enrollment_token: String,
    pub role: String,
    pub expires_at_unix_seconds: u64,
    pub install_command: String,
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct ReportRequest {
    pub node_id: String,
    pub report: NodeReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct HealthRating {
    pub window_seconds: u64,
    pub sample_count: usize,
    pub expected_sample_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<u8>,
    pub grade: String,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FleetNodeView {
    pub node_id: String,
    pub node_name: String,
    pub advertise_host: String,
    pub p2p_multiaddr: String,
    pub last_seen_unix_seconds: u64,
    pub online: bool,
    pub banned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ban_reason: Option<String>,
    pub peer_id: Option<String>,
    pub height: Option<u64>,
    pub connected_peers: u64,
    pub validating_peers: u64,
    pub mining_peers: u64,
    pub observed_hashrate_hs: u64,
    pub indexer_lag_blocks: Option<u64>,
    pub roles: Vec<String>,
    pub services: ServiceStates,
    pub health: HealthRating,
}

#[derive(Clone, Debug, Serialize)]
pub struct FleetTopology {
    pub mode: &'static str,
    pub network_id: String,
    pub generated_at_unix_seconds: u64,
    pub registered_node_count: usize,
    pub online_node_count: usize,
    pub banned_node_count: usize,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BanRequest {
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct MutationResponse {
    pub operation_id: String,
    pub action: String,
    pub target: String,
    pub status: String,
    pub replayed: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeploymentRole {
    Node,
    Validator,
    Indexer,
    Explorer,
    Stratum,
    FullStack,
}

impl DeploymentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Validator => "validator",
            Self::Indexer => "indexer",
            Self::Explorer => "explorer",
            Self::Stratum => "stratum",
            Self::FullStack => "full-stack",
        }
    }
}

fn default_enrollment_role() -> DeploymentRole {
    DeploymentRole::Indexer
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapManifestRequest {
    pub role: DeploymentRole,
    pub node_name: String,
    pub advertise_host: String,
    pub acme_email: String,
    #[serde(default)]
    pub admin_domain: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BootstrapPort {
    pub port: u16,
    pub transport: &'static str,
    pub purpose: &'static str,
    pub public: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct BootstrapManifest {
    pub schema_version: &'static str,
    pub network_id: String,
    pub role: String,
    pub node_name: String,
    pub advertise_host: String,
    pub controller_url: String,
    pub immutable_release_bundle_url: String,
    pub components: Vec<&'static str>,
    pub required_ports: Vec<BootstrapPort>,
    pub environment: BTreeMap<String, String>,
    pub apt_packages: Vec<&'static str>,
    pub install_command_template: String,
    pub secrets_required: Vec<&'static str>,
    pub contains_secrets: bool,
    pub post_install_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuditEntry {
    pub operation_id: String,
    pub occurred_at_unix_seconds: u64,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub request_sha256: String,
    pub outcome: String,
    pub idempotency_key_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceInventoryItem {
    pub node_id: String,
    pub node_name: String,
    pub service: String,
    pub state: String,
    pub healthy: Option<bool>,
    pub evidence: String,
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

pub fn value_optional_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|item| {
            item.as_u64()
                .or_else(|| item.as_i64().and_then(|number| u64::try_from(number).ok()))
                .or_else(|| item.as_str().and_then(|raw| raw.parse::<u64>().ok()))
        })
    })
}

pub fn value_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}
