use crate::models::{
    value_optional_u64, value_string, AuditEntry, FleetNodeView, HealthRating, InvitationView,
    MutationResponse, NodeReport,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const HEALTH_WINDOW_SECONDS: u64 = 86_400;
const MAX_OBSERVATIONS: usize = 17_280;
const MAX_IDEMPOTENCY_RECORDS: usize = 4_096;
const MAX_AUDIT_ENTRIES: usize = 20_000;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct FleetData {
    invitations: Vec<Invitation>,
    nodes: Vec<FleetNode>,
    local_observations: Vec<Observation>,
    idempotency: Vec<IdempotencyRecord>,
    audit: Vec<AuditEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Invitation {
    id: String,
    token_hash: String,
    expires_at_unix_seconds: u64,
    used: bool,
    node_name: String,
    advertise_host: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct FleetNode {
    id: String,
    token_hash: String,
    certificate_fingerprint_sha1: Option<String>,
    #[serde(default)]
    pending_certificate_fingerprint_sha1: Option<String>,
    report: NodeReport,
    created_at_unix_seconds: u64,
    banned: bool,
    ban_reason: Option<String>,
    banned_at_unix_seconds: Option<u64>,
    observations: Vec<Observation>,
}

impl Default for FleetNode {
    fn default() -> Self {
        Self {
            id: String::new(),
            token_hash: String::new(),
            certificate_fingerprint_sha1: None,
            pending_certificate_fingerprint_sha1: None,
            report: empty_report(),
            created_at_unix_seconds: 0,
            banned: false,
            ban_reason: None,
            banned_at_unix_seconds: None,
            observations: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Observation {
    bucket: u64,
    observed_at_unix_seconds: u64,
    healthy: bool,
    connected_peers: u64,
    indexer_healthy: bool,
    stratum_healthy: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct IdempotencyRecord {
    key_sha256: String,
    action: String,
    request_sha256: String,
    operation_id: String,
    target: String,
    created_at_unix_seconds: u64,
}

#[derive(Clone)]
pub struct FleetStore {
    path: PathBuf,
    inner: Arc<Mutex<FleetData>>,
}

pub struct CreatedInvitation {
    pub id: String,
    pub token: String,
    pub expires_at_unix_seconds: u64,
}

pub struct MutationContext<'a> {
    pub actor: &'a str,
    pub idempotency_key: &'a str,
    pub request_sha256: &'a str,
    pub now: u64,
}

pub enum ControlledMutation<T> {
    Applied {
        value: T,
        response: MutationResponse,
    },
    Replayed(MutationResponse),
}

impl FleetStore {
    pub fn load(state_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&state_dir).map_err(|error| error.to_string())?;
        let path = state_dir.join("fleet.json");
        let data = if path.exists() {
            serde_json::from_str(&fs::read_to_string(&path).map_err(|error| error.to_string())?)
                .map_err(|error| format!("invalid fleet store {}: {error}", path.display()))?
        } else {
            FleetData::default()
        };
        Ok(Self {
            path,
            inner: Arc::new(Mutex::new(data)),
        })
    }

    pub fn create_invitation(
        &self,
        node_name: String,
        advertise_host: String,
        now: u64,
        ttl: u64,
    ) -> Result<CreatedInvitation, String> {
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let created = create_invitation_locked(&mut data, node_name, advertise_host, now, ttl);
        self.save(&data)?;
        Ok(created)
    }

    pub fn create_invitation_controlled(
        &self,
        node_name: String,
        advertise_host: String,
        ttl: u64,
        context: MutationContext<'_>,
    ) -> Result<ControlledMutation<CreatedInvitation>, String> {
        let action = "invitation.create";
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        if let Some(response) = replay(&data, action, &context)? {
            return Ok(ControlledMutation::Replayed(response));
        }
        let created =
            create_invitation_locked(&mut data, node_name, advertise_host, context.now, ttl);
        let response = commit_mutation(&mut data, action, &created.id, "created", context);
        self.save(&data)?;
        Ok(ControlledMutation::Applied {
            value: created,
            response,
        })
    }

    pub fn enroll(
        &self,
        invitation_token: &str,
        certificate_fingerprint_sha1: String,
        report: NodeReport,
        now: u64,
        report_interval_seconds: u64,
    ) -> Result<(String, String), String> {
        let hash = token_hash(invitation_token);
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let invitation_index = data
            .invitations
            .iter()
            .position(|item| item.token_hash == hash && !item.used)
            .ok_or_else(|| "invalid or already used enrollment token".to_owned())?;
        let invitation = &data.invitations[invitation_index];
        if invitation.expires_at_unix_seconds <= now {
            return Err("enrollment token expired".to_owned());
        }
        if invitation.node_name != report.node_name
            || invitation.advertise_host != report.advertise_host
        {
            return Err("node identity does not match invitation".to_owned());
        }
        if data.nodes.iter().any(|node| {
            node.report.node_name == report.node_name
                || node.report.advertise_host == report.advertise_host
        }) {
            return Err("node identity is already registered".to_owned());
        }
        data.invitations[invitation_index].used = true;
        let node_id = random_hex(12);
        let node_token = random_hex(32);
        let observation = observation_from_report(&report, report_interval_seconds);
        data.nodes.push(FleetNode {
            id: node_id.clone(),
            token_hash: token_hash(&node_token),
            certificate_fingerprint_sha1: Some(certificate_fingerprint_sha1),
            pending_certificate_fingerprint_sha1: None,
            report,
            created_at_unix_seconds: now,
            banned: false,
            ban_reason: None,
            banned_at_unix_seconds: None,
            observations: vec![observation],
        });
        self.save(&data)?;
        Ok((node_id, node_token))
    }

    pub fn validate_enrollment(
        &self,
        invitation_token: &str,
        report: &NodeReport,
        now: u64,
    ) -> Result<(), String> {
        let hash = token_hash(invitation_token);
        let data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let invitation = data
            .invitations
            .iter()
            .find(|item| item.token_hash == hash && !item.used)
            .ok_or_else(|| "invalid or already used enrollment token".to_owned())?;
        if invitation.expires_at_unix_seconds <= now {
            return Err("enrollment token expired".to_owned());
        }
        if invitation.node_name != report.node_name
            || invitation.advertise_host != report.advertise_host
        {
            return Err("node identity does not match invitation".to_owned());
        }
        if data.nodes.iter().any(|node| {
            node.report.node_name == report.node_name
                || node.report.advertise_host == report.advertise_host
        }) {
            return Err("node identity is already registered".to_owned());
        }
        Ok(())
    }

    pub fn validate_node_certificate_credentials(
        &self,
        node_id: &str,
        node_token: &str,
        certificate_fingerprint_sha1: &str,
    ) -> Result<(), String> {
        let hash = token_hash(node_token);
        let data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let node = data
            .nodes
            .iter()
            .find(|item| {
                item.id == node_id
                    && item.token_hash == hash
                    && certificate_matches(item, certificate_fingerprint_sha1)
            })
            .ok_or_else(|| "invalid node credentials".to_owned())?;
        if node.banned {
            return Err("node is banned from fleet operations".to_owned());
        }
        Ok(())
    }

    pub fn update_report(
        &self,
        node_id: &str,
        node_token: &str,
        certificate_fingerprint_sha1: &str,
        report: NodeReport,
        report_interval_seconds: u64,
    ) -> Result<(), String> {
        let hash = token_hash(node_token);
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let node = data
            .nodes
            .iter_mut()
            .find(|item| {
                item.id == node_id
                    && item.token_hash == hash
                    && certificate_matches(item, certificate_fingerprint_sha1)
            })
            .ok_or_else(|| "invalid node credentials".to_owned())?;
        if node.banned {
            return Err("node is banned from fleet reporting".to_owned());
        }
        if node.report.node_name != report.node_name
            || node.report.advertise_host != report.advertise_host
        {
            return Err("node identity cannot change after enrollment".to_owned());
        }
        if node.pending_certificate_fingerprint_sha1.as_deref()
            == Some(certificate_fingerprint_sha1)
        {
            node.certificate_fingerprint_sha1 = node.pending_certificate_fingerprint_sha1.take();
        }
        record_observation(
            &mut node.observations,
            observation_from_report(&report, report_interval_seconds),
        );
        node.report = report;
        self.save(&data)
    }

    pub fn stage_certificate_rotation(
        &self,
        node_id: &str,
        node_token: &str,
        current_fingerprint_sha1: &str,
        new_fingerprint_sha1: String,
    ) -> Result<(), String> {
        let hash = token_hash(node_token);
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let node = data
            .nodes
            .iter_mut()
            .find(|item| {
                item.id == node_id
                    && item.token_hash == hash
                    && certificate_matches(item, current_fingerprint_sha1)
            })
            .ok_or_else(|| "invalid node credentials".to_owned())?;
        if node.banned {
            return Err("node is banned from certificate rotation".to_owned());
        }
        if node.pending_certificate_fingerprint_sha1.as_deref() == Some(current_fingerprint_sha1) {
            node.certificate_fingerprint_sha1 = node.pending_certificate_fingerprint_sha1.take();
        }
        node.pending_certificate_fingerprint_sha1 = Some(new_fingerprint_sha1);
        self.save(&data)
    }

    pub fn revoke_node_certificate_controlled(
        &self,
        node_id: &str,
        context: MutationContext<'_>,
    ) -> Result<ControlledMutation<()>, String> {
        let action = "node.certificate.revoke";
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        if let Some(response) = replay(&data, action, &context)? {
            return Ok(ControlledMutation::Replayed(response));
        }
        let node = data
            .nodes
            .iter_mut()
            .find(|node| node.id == node_id)
            .ok_or_else(|| "node not found".to_owned())?;
        let current = node.certificate_fingerprint_sha1.take();
        let pending = node.pending_certificate_fingerprint_sha1.take();
        if current.is_none() && pending.is_none() {
            return Err("node certificate is already revoked".to_owned());
        }
        let response = commit_mutation(&mut data, action, node_id, "revoked", context);
        self.save(&data)?;
        Ok(ControlledMutation::Applied {
            value: (),
            response,
        })
    }

    pub fn record_local_observation(
        &self,
        report: &NodeReport,
        report_interval_seconds: u64,
    ) -> Result<(), String> {
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        record_observation(
            &mut data.local_observations,
            observation_from_report(report, report_interval_seconds),
        );
        self.save(&data)
    }

    pub fn node_views(
        &self,
        now: u64,
        report_interval_seconds: u64,
    ) -> Result<Vec<FleetNodeView>, String> {
        let data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        Ok(data
            .nodes
            .iter()
            .map(|node| node_view(node, now, report_interval_seconds))
            .collect())
    }

    pub fn local_node_view(
        &self,
        report: &NodeReport,
        now: u64,
        report_interval_seconds: u64,
    ) -> Result<FleetNodeView, String> {
        let data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let first_observation = data
            .local_observations
            .first()
            .map(|observation| observation.observed_at_unix_seconds)
            .unwrap_or(now);
        Ok(node_view_parts(
            "local-controller",
            report,
            first_observation,
            false,
            None,
            &data.local_observations,
            now,
            report_interval_seconds,
        ))
    }

    pub fn invitation_views(&self, now: u64) -> Result<Vec<InvitationView>, String> {
        let data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        Ok(data
            .invitations
            .iter()
            .map(|item| {
                let expired = item.expires_at_unix_seconds <= now;
                let status = if item.used {
                    "used"
                } else if expired {
                    "expired"
                } else {
                    "pending"
                };
                InvitationView {
                    invitation_id: item.id.clone(),
                    node_name: item.node_name.clone(),
                    advertise_host: item.advertise_host.clone(),
                    expires_at_unix_seconds: item.expires_at_unix_seconds,
                    used: item.used,
                    expired,
                    status: status.to_owned(),
                }
            })
            .collect())
    }

    pub fn revoke_invitation_controlled(
        &self,
        invitation_id: &str,
        context: MutationContext<'_>,
    ) -> Result<ControlledMutation<()>, String> {
        let action = "invitation.revoke";
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        if let Some(response) = replay(&data, action, &context)? {
            return Ok(ControlledMutation::Replayed(response));
        }
        let before = data.invitations.len();
        data.invitations.retain(|item| item.id != invitation_id);
        if data.invitations.len() == before {
            return Err("invitation not found".to_owned());
        }
        let response = commit_mutation(&mut data, action, invitation_id, "revoked", context);
        self.save(&data)?;
        Ok(ControlledMutation::Applied {
            value: (),
            response,
        })
    }

    pub fn remove_node_controlled(
        &self,
        node_id: &str,
        context: MutationContext<'_>,
    ) -> Result<ControlledMutation<()>, String> {
        if node_id == "local-controller" {
            return Err("cannot remove the local controller from inventory".to_owned());
        }
        let action = "node.remove";
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        if let Some(response) = replay(&data, action, &context)? {
            return Ok(ControlledMutation::Replayed(response));
        }
        let before = data.nodes.len();
        data.nodes.retain(|node| node.id != node_id);
        if data.nodes.len() == before {
            return Err("node not found".to_owned());
        }
        let response = commit_mutation(&mut data, action, node_id, "removed", context);
        self.save(&data)?;
        Ok(ControlledMutation::Applied {
            value: (),
            response,
        })
    }

    pub fn set_node_ban_controlled(
        &self,
        node_id: &str,
        reason: Option<String>,
        context: MutationContext<'_>,
    ) -> Result<ControlledMutation<()>, String> {
        if node_id == "local-controller" {
            return Err("cannot ban the local controller".to_owned());
        }
        let (action, status) = if reason.is_some() {
            ("node.ban", "banned")
        } else {
            ("node.unban", "active")
        };
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        if let Some(response) = replay(&data, action, &context)? {
            return Ok(ControlledMutation::Replayed(response));
        }
        let node = data
            .nodes
            .iter_mut()
            .find(|node| node.id == node_id)
            .ok_or_else(|| "node not found".to_owned())?;
        node.banned = reason.is_some();
        node.ban_reason = reason;
        node.banned_at_unix_seconds = node.banned.then_some(context.now);
        let response = commit_mutation(&mut data, action, node_id, status, context);
        self.save(&data)?;
        Ok(ControlledMutation::Applied {
            value: (),
            response,
        })
    }

    pub fn get_node_report(&self, node_id: &str) -> Result<Option<NodeReport>, String> {
        let data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        Ok(data
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .map(|node| node.report.clone()))
    }

    pub fn audit_entries(&self, limit: usize) -> Result<Vec<AuditEntry>, String> {
        let data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        Ok(data
            .audit
            .iter()
            .rev()
            .take(limit.min(1_000))
            .cloned()
            .collect())
    }

    fn save(&self, data: &FleetData) -> Result<(), String> {
        let temp = self.path.with_extension("json.tmp");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(data).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        fs::rename(temp, &self.path).map_err(|error| error.to_string())
    }
}

fn create_invitation_locked(
    data: &mut FleetData,
    node_name: String,
    advertise_host: String,
    now: u64,
    ttl: u64,
) -> CreatedInvitation {
    let token = random_hex(32);
    let id = random_hex(12);
    let created = CreatedInvitation {
        id: id.clone(),
        token: token.clone(),
        expires_at_unix_seconds: now.saturating_add(ttl),
    };
    data.invitations
        .retain(|item| item.expires_at_unix_seconds > now && !item.used);
    data.invitations.push(Invitation {
        id,
        token_hash: token_hash(&token),
        expires_at_unix_seconds: created.expires_at_unix_seconds,
        used: false,
        node_name,
        advertise_host,
    });
    created
}

fn certificate_matches(node: &FleetNode, fingerprint_sha1: &str) -> bool {
    node.certificate_fingerprint_sha1.as_deref() == Some(fingerprint_sha1)
        || node.pending_certificate_fingerprint_sha1.as_deref() == Some(fingerprint_sha1)
}

fn replay(
    data: &FleetData,
    action: &str,
    context: &MutationContext<'_>,
) -> Result<Option<MutationResponse>, String> {
    let key_sha256 = token_hash(context.idempotency_key);
    let Some(existing) = data
        .idempotency
        .iter()
        .find(|record| record.key_sha256 == key_sha256)
    else {
        return Ok(None);
    };
    if existing.action != action || existing.request_sha256 != context.request_sha256 {
        return Err("idempotency key was already used for a different request".to_owned());
    }
    Ok(Some(MutationResponse {
        operation_id: existing.operation_id.clone(),
        action: existing.action.clone(),
        target: existing.target.clone(),
        status: "already-applied".to_owned(),
        replayed: true,
    }))
}

fn commit_mutation(
    data: &mut FleetData,
    action: &str,
    target: &str,
    outcome: &str,
    context: MutationContext<'_>,
) -> MutationResponse {
    let operation_id = random_hex(12);
    let key_sha256 = token_hash(context.idempotency_key);
    data.idempotency.push(IdempotencyRecord {
        key_sha256: key_sha256.clone(),
        action: action.to_owned(),
        request_sha256: context.request_sha256.to_owned(),
        operation_id: operation_id.clone(),
        target: target.to_owned(),
        created_at_unix_seconds: context.now,
    });
    trim_front(&mut data.idempotency, MAX_IDEMPOTENCY_RECORDS);
    data.audit.push(AuditEntry {
        operation_id: operation_id.clone(),
        occurred_at_unix_seconds: context.now,
        actor: context.actor.to_owned(),
        action: action.to_owned(),
        target: target.to_owned(),
        request_sha256: context.request_sha256.to_owned(),
        outcome: outcome.to_owned(),
        idempotency_key_sha256: key_sha256,
    });
    trim_front(&mut data.audit, MAX_AUDIT_ENTRIES);
    MutationResponse {
        operation_id,
        action: action.to_owned(),
        target: target.to_owned(),
        status: outcome.to_owned(),
        replayed: false,
    }
}

fn record_observation(observations: &mut Vec<Observation>, observation: Observation) {
    if let Some(existing) = observations
        .iter_mut()
        .find(|item| item.bucket == observation.bucket)
    {
        *existing = observation;
    } else {
        observations.push(observation);
    }
    observations.sort_by_key(|item| item.bucket);
    trim_front(observations, MAX_OBSERVATIONS);
}

fn observation_from_report(report: &NodeReport, interval: u64) -> Observation {
    let status_healthy = probe_or_payload_healthy(report, "status", &report.status);
    let p2p_healthy = probe_or_payload_healthy(report, "p2p", &report.p2p);
    let indexer_healthy = probe_or_payload_healthy(report, "indexer", &report.indexer);
    let stratum_healthy = report
        .probes
        .get("stratum_tls")
        .filter(|probe| probe.configured)
        .map(|probe| probe.healthy);
    Observation {
        bucket: report.reported_at_unix_seconds / interval.max(1),
        observed_at_unix_seconds: report.reported_at_unix_seconds,
        healthy: status_healthy && p2p_healthy && indexer_healthy,
        connected_peers: crate::models::value_u64(&report.p2p, "connected_peer_count"),
        indexer_healthy,
        stratum_healthy,
    }
}

fn probe_or_payload_healthy(report: &NodeReport, name: &str, payload: &serde_json::Value) -> bool {
    report
        .probes
        .get(name)
        .map(|probe| probe.configured && probe.healthy)
        .unwrap_or_else(|| payload.get("error").is_none() && !payload.is_null())
}

fn node_view(node: &FleetNode, now: u64, interval: u64) -> FleetNodeView {
    node_view_parts(
        &node.id,
        &node.report,
        node.created_at_unix_seconds,
        node.banned,
        node.ban_reason.clone(),
        &node.observations,
        now,
        interval,
    )
}

fn node_view_parts(
    node_id: &str,
    report: &NodeReport,
    created_at: u64,
    banned: bool,
    ban_reason: Option<String>,
    observations: &[Observation],
    now: u64,
    interval: u64,
) -> FleetNodeView {
    let online_threshold = interval.saturating_mul(3).max(45);
    FleetNodeView {
        node_id: node_id.to_owned(),
        node_name: report.node_name.clone(),
        advertise_host: report.advertise_host.clone(),
        p2p_multiaddr: report.p2p_multiaddr.clone(),
        last_seen_unix_seconds: report.reported_at_unix_seconds,
        online: !banned && now.saturating_sub(report.reported_at_unix_seconds) <= online_threshold,
        banned,
        ban_reason,
        peer_id: value_string(&report.p2p, "local_peer_id"),
        height: value_optional_u64(&report.status, &["height", "chain_height"]),
        connected_peers: crate::models::value_u64(&report.p2p, "connected_peer_count"),
        validating_peers: crate::models::value_u64(&report.p2p, "validating_peer_count"),
        mining_peers: crate::models::value_u64(&report.p2p, "mining_peer_count"),
        observed_hashrate_hs: crate::models::value_u64(&report.p2p, "observed_network_hashrate_hs"),
        indexer_lag_blocks: value_optional_u64(
            &report.indexer,
            &["lag_blocks", "indexer_lag_blocks", "lag"],
        ),
        roles: infer_roles(report),
        services: report.services.clone(),
        health: health_rating(created_at, observations, report, now, interval),
    }
}

fn health_rating(
    created_at: u64,
    observations: &[Observation],
    report: &NodeReport,
    now: u64,
    interval: u64,
) -> HealthRating {
    let interval = interval.max(5);
    let window_start = created_at.max(now.saturating_sub(HEALTH_WINDOW_SECONDS));
    let expected = now.saturating_sub(window_start) / interval + 1;
    let relevant: Vec<&Observation> = observations
        .iter()
        .filter(|item| item.observed_at_unix_seconds >= window_start)
        .collect();
    let healthy = relevant.iter().filter(|item| item.healthy).count() as u64;
    let uptime = (!relevant.is_empty())
        .then_some(((healthy as f64 / expected.max(1) as f64) * 10_000.0).round() / 100.0);
    let mut reasons = Vec::new();
    if relevant.len() < 3 {
        reasons.push("at least three real probe samples are required for a rating".to_owned());
    }
    let latest = relevant.last().copied();
    let score = if relevant.len() >= 3 {
        let availability = uptime.unwrap_or(0.0);
        let mut value = (availability * 0.75).round() as u8;
        if latest.is_some_and(|sample| sample.connected_peers > 0) {
            value = value.saturating_add(15);
        } else {
            reasons.push("no connected P2P peer in the latest sample".to_owned());
        }
        if latest.is_some_and(|sample| sample.indexer_healthy) {
            value = value.saturating_add(10);
        } else {
            reasons.push("indexer probe is not healthy".to_owned());
        }
        if latest.and_then(|sample| sample.stratum_healthy) == Some(false) {
            reasons.push("configured Stratum TLS probe is failing".to_owned());
        }
        Some(value.min(100))
    } else {
        None
    };
    if report
        .probes
        .values()
        .any(|probe| probe.configured && !probe.healthy)
    {
        reasons.push("one or more current configured probes are failing".to_owned());
    }
    let grade = match score {
        Some(95..=100) => "A",
        Some(85..=94) => "B",
        Some(70..=84) => "C",
        Some(50..=69) => "D",
        Some(_) => "F",
        None => "insufficient-data",
    }
    .to_owned();
    HealthRating {
        window_seconds: HEALTH_WINDOW_SECONDS,
        sample_count: relevant.len(),
        expected_sample_count: expected,
        uptime_percent: uptime,
        score,
        grade,
        reasons,
    }
}

fn infer_roles(report: &NodeReport) -> Vec<String> {
    let mut roles = vec!["node".to_owned()];
    if report
        .status
        .get("validator")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        || report
            .status
            .get("role")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|role| role.eq_ignore_ascii_case("validator"))
    {
        roles.push("validator".to_owned());
    }
    if report.indexer.get("error").is_none() && !report.indexer.is_null() {
        roles.push("indexer".to_owned());
    }
    if report
        .probes
        .get("stratum_tls")
        .is_some_and(|probe| probe.configured)
    {
        roles.push("stratum".to_owned());
    }
    roles
}

fn trim_front<T>(values: &mut Vec<T>, max: usize) {
    if values.len() > max {
        values.drain(..values.len() - max);
    }
}

fn random_hex(bytes: usize) -> String {
    let mut value = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut value);
    hex::encode(value)
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn empty_report() -> NodeReport {
    NodeReport {
        network_id: String::new(),
        node_name: String::new(),
        advertise_host: String::new(),
        p2p_multiaddr: String::new(),
        reported_at_unix_seconds: 0,
        services: Default::default(),
        status: serde_json::Value::Null,
        sync: serde_json::Value::Null,
        p2p: serde_json::Value::Null,
        mempool: serde_json::Value::Null,
        indexer: serde_json::Value::Null,
        stratum: serde_json::Value::Null,
        probes: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ServiceStates;
    use serde_json::json;
    use std::collections::BTreeMap;

    const CERTIFICATE: &str = "0123456789ABCDEF0123456789ABCDEF01234567";

    fn report(at: u64) -> NodeReport {
        NodeReport {
            network_id: "alvenqis-mainnet-candidate".to_owned(),
            node_name: "peer-2".to_owned(),
            advertise_host: "peer2.example.org".to_owned(),
            p2p_multiaddr: "/dns4/peer2.example.org/tcp/20787".to_owned(),
            reported_at_unix_seconds: at,
            services: ServiceStates::default(),
            status: json!({"height": 12}),
            sync: json!({}),
            p2p: json!({"connected_peer_count": 2}),
            mempool: json!({}),
            indexer: json!({"lag_blocks": 0}),
            stratum: json!({}),
            probes: BTreeMap::new(),
        }
    }

    #[test]
    fn invitation_is_single_use_and_report_credentials_are_required() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        let invite = store
            .create_invitation("peer-2".to_owned(), "peer2.example.org".to_owned(), 90, 60)
            .expect("invite");
        let (id, token) = store
            .enroll(&invite.token, CERTIFICATE.to_owned(), report(100), 100, 15)
            .expect("enroll");
        assert!(store
            .enroll(&invite.token, CERTIFICATE.to_owned(), report(101), 101, 15)
            .is_err());
        assert!(store
            .update_report(&id, "wrong", CERTIFICATE, report(115), 15)
            .is_err());
        store
            .update_report(&id, &token, CERTIFICATE, report(115), 15)
            .expect("report");
        assert!(store
            .update_report(
                &id,
                &token,
                "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
                report(130),
                15,
            )
            .is_err());
    }

    #[test]
    fn certificate_rotation_keeps_current_identity_until_pending_identity_reports() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        let invite = store
            .create_invitation("peer-2".to_owned(), "peer2.example.org".to_owned(), 90, 60)
            .expect("invite");
        let (id, token) = store
            .enroll(&invite.token, CERTIFICATE.to_owned(), report(100), 100, 15)
            .expect("enroll");
        let lost_response_certificate = "89ABCDEF0123456789ABCDEF0123456789ABCDEF";
        store
            .stage_certificate_rotation(
                &id,
                &token,
                CERTIFICATE,
                lost_response_certificate.to_owned(),
            )
            .expect("stage first rotation");
        drop(store);
        let store =
            FleetStore::load(dir.path().to_path_buf()).expect("reload after staged rotation");

        // A lost response must not strand the agent: its current certificate
        // remains valid and can authenticate a retry with a new CSR.
        store
            .update_report(&id, &token, CERTIFICATE, report(115), 15)
            .expect("current certificate remains valid");
        let installed_certificate = "FEDCBA9876543210FEDCBA9876543210FEDCBA98";
        store
            .stage_certificate_rotation(&id, &token, CERTIFICATE, installed_certificate.to_owned())
            .expect("retry rotation with current certificate");
        assert!(store
            .update_report(&id, &token, lost_response_certificate, report(130), 15,)
            .is_err());
        store
            .update_report(&id, &token, CERTIFICATE, report(130), 15)
            .expect("current certificate remains valid until promotion");
        store
            .update_report(&id, &token, installed_certificate, report(145), 15)
            .expect("pending certificate promotes after authenticated report");
        assert!(store
            .update_report(&id, &token, CERTIFICATE, report(160), 15)
            .is_err());
        store
            .update_report(&id, &token, installed_certificate, report(160), 15)
            .expect("promoted certificate remains valid");
    }

    #[test]
    fn revocation_clears_current_and_pending_certificate_identities() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        let invite = store
            .create_invitation("peer-2".to_owned(), "peer2.example.org".to_owned(), 90, 60)
            .expect("invite");
        let (id, token) = store
            .enroll(&invite.token, CERTIFICATE.to_owned(), report(100), 100, 15)
            .expect("enroll");
        let pending = "89ABCDEF0123456789ABCDEF0123456789ABCDEF";
        store
            .stage_certificate_rotation(&id, &token, CERTIFICATE, pending.to_owned())
            .expect("stage rotation");
        store
            .revoke_node_certificate_controlled(
                &id,
                MutationContext {
                    actor: "operator",
                    idempotency_key: "certificate-revoke-0001",
                    request_sha256: "def",
                    now: 120,
                },
            )
            .expect("revoke");
        assert!(store
            .update_report(&id, &token, CERTIFICATE, report(115), 15)
            .is_err());
        assert!(store
            .update_report(&id, &token, pending, report(115), 15)
            .is_err());
    }

    #[test]
    fn legacy_fleet_node_without_pending_certificate_field_deserializes() {
        let legacy: FleetNode = serde_json::from_value(json!({
            "id": "legacy-node",
            "token_hash": "legacy-token-hash",
            "certificate_fingerprint_sha1": CERTIFICATE,
        }))
        .expect("legacy fleet node");

        assert!(legacy.pending_certificate_fingerprint_sha1.is_none());
    }

    #[test]
    fn controlled_ban_is_idempotent_and_audited() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        let invite = store
            .create_invitation("peer-2".to_owned(), "peer2.example.org".to_owned(), 90, 60)
            .expect("invite");
        let (id, token) = store
            .enroll(&invite.token, CERTIFICATE.to_owned(), report(100), 100, 15)
            .expect("enroll");
        let context = || MutationContext {
            actor: "operator",
            idempotency_key: "ban-request-0001",
            request_sha256: "abc",
            now: 110,
        };
        assert!(matches!(
            store
                .set_node_ban_controlled(&id, Some("abuse".to_owned()), context())
                .expect("ban"),
            ControlledMutation::Applied { .. }
        ));
        assert!(matches!(
            store
                .set_node_ban_controlled(&id, Some("abuse".to_owned()), context())
                .expect("replay"),
            ControlledMutation::Replayed(_)
        ));
        assert!(store
            .update_report(&id, &token, CERTIFICATE, report(115), 15)
            .is_err());
        assert_eq!(store.audit_entries(10).expect("audit").len(), 1);
    }

    #[test]
    fn rating_requires_real_samples_and_accounts_for_missing_intervals() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        let invite = store
            .create_invitation("peer-2".to_owned(), "peer2.example.org".to_owned(), 90, 60)
            .expect("invite");
        let (id, token) = store
            .enroll(&invite.token, CERTIFICATE.to_owned(), report(100), 100, 15)
            .expect("enroll");
        let first = store.node_views(100, 15).expect("views").remove(0);
        assert_eq!(first.health.grade, "insufficient-data");
        store
            .update_report(&id, &token, CERTIFICATE, report(115), 15)
            .expect("report 2");
        store
            .update_report(&id, &token, CERTIFICATE, report(130), 15)
            .expect("report 3");
        let rated = store.node_views(145, 15).expect("views").remove(0);
        assert!(rated.health.score.is_some());
        assert!(rated.health.uptime_percent.expect("uptime") < 100.0);
    }
}
