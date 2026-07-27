//! Peer scoring and temporary bans for the P2P swarm (audit A-H05).
//!
//! Durable JSON persistence under the node runtime dir.
//! Scores rise on useful sync contributions and fall on rejected handshakes,
//! invalid blocks and protocol abuse. Peers at or below the ban threshold
//! are refused until `banned_until`.

use crate::error::{NodeError, NodeResult};
use atomic_write_file::AtomicWriteFile;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const REPUTATION_FILE_NAME: &str = "peer-reputation.json";
pub const PEER_ADMIN_QUEUE_FILE_NAME: &str = "peer-admin-queue.json";
const PEER_ADMIN_QUEUE_LOCK_FILE_NAME: &str = "peer-admin-queue.lock";
const MAX_ADMIN_QUEUE_ENTRIES: usize = 1_024;

/// Starting score for a newly observed peer.
pub const DEFAULT_SCORE: i32 = 50;
/// Ban when score drops to this value or below.
pub const BAN_THRESHOLD: i32 = 0;
/// Score floor / ceiling.
pub const SCORE_MIN: i32 = -100;
pub const SCORE_MAX: i32 = 100;
/// Default ban duration after threshold.
pub const DEFAULT_BAN_SECONDS: u64 = 600;
/// Longer ban for forged PoW / header-body mismatch (protocol abuse).
pub const SEVERE_BAN_SECONDS: u64 = 3_600;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerScoreRecord {
    pub peer_id: String,
    pub score: i32,
    pub banned_until_unix: u64,
    #[serde(default)]
    pub manually_banned_until_unix: u64,
    pub last_reason: Option<String>,
    pub good_events: u64,
    pub bad_events: u64,
    #[serde(default)]
    pub first_observed_at_unix: u64,
    #[serde(default)]
    pub last_observed_at_unix: u64,
    #[serde(default)]
    pub connected_since_unix: u64,
    #[serde(default)]
    pub observed_uptime_seconds: u64,
    #[serde(default)]
    pub successful_connections: u64,
    #[serde(default)]
    pub failed_connections: u64,
    #[serde(default)]
    pub validated_handshakes: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReputationStore {
    pub peers: BTreeMap<String, PeerScoreRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PeerAdminAction {
    Ban {
        reason: String,
        duration_seconds: u64,
    },
    Unban,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerAdminRequest {
    pub request_id: String,
    pub peer_id: String,
    pub created_at_unix: u64,
    pub action: PeerAdminAction,
}

impl ReputationStore {
    pub fn load(runtime_dir: &Path) -> NodeResult<Self> {
        let path = runtime_dir.join(REPUTATION_FILE_NAME);
        if !path.exists() {
            return Ok(Self::default());
        }
        let mut store: Self =
            serde_json::from_str(&fs::read_to_string(&path)?).map_err(|error| {
                NodeError::P2p(format!(
                    "invalid persisted peer reputation at {}: {error}",
                    path.display()
                ))
            })?;
        for record in store.peers.values_mut() {
            record.connected_since_unix = 0;
        }
        Ok(store)
    }

    pub fn persist(&self, runtime_dir: &Path) -> NodeResult<()> {
        fs::create_dir_all(runtime_dir)?;
        let path = runtime_dir.join(REPUTATION_FILE_NAME);
        let mut file = AtomicWriteFile::open(path)?;
        serde_json::to_writer_pretty(&mut file, self)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        file.commit()?;
        Ok(())
    }

    /// Immediately ban a peer for `ban_seconds` (used for severe protocol abuse).
    pub fn ban_now(&mut self, peer_id: &str, reason: &str, ban_seconds: u64) {
        let rec = self.entry(peer_id);
        rec.score = BAN_THRESHOLD;
        rec.bad_events = rec.bad_events.saturating_add(1);
        rec.last_reason = Some(reason.to_owned());
        rec.banned_until_unix = Self::now().saturating_add(ban_seconds.max(1));
        rec.last_observed_at_unix = Self::now();
    }

    pub fn path(runtime_dir: &Path) -> PathBuf {
        runtime_dir.join(REPUTATION_FILE_NAME)
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn entry(&mut self, peer_id: &str) -> &mut PeerScoreRecord {
        self.peers
            .entry(peer_id.to_owned())
            .or_insert_with(|| PeerScoreRecord {
                peer_id: peer_id.to_owned(),
                score: DEFAULT_SCORE,
                banned_until_unix: 0,
                manually_banned_until_unix: 0,
                last_reason: None,
                good_events: 0,
                bad_events: 0,
                first_observed_at_unix: 0,
                last_observed_at_unix: 0,
                connected_since_unix: 0,
                observed_uptime_seconds: 0,
                successful_connections: 0,
                failed_connections: 0,
                validated_handshakes: 0,
            })
    }

    /// True if peer is currently banned.
    pub fn is_banned(&self, peer_id: &str) -> bool {
        let now = Self::now();
        self.peers.get(peer_id).is_some_and(|record| {
            record.banned_until_unix > now || record.manually_banned_until_unix > now
        })
    }

    pub fn score_of(&self, peer_id: &str) -> i32 {
        self.peers
            .get(peer_id)
            .map(|r| r.score)
            .unwrap_or(DEFAULT_SCORE)
    }

    pub fn reward(&mut self, peer_id: &str, delta: i32, reason: &str) {
        let rec = self.entry(peer_id);
        rec.score = (rec.score.saturating_add(delta)).clamp(SCORE_MIN, SCORE_MAX);
        rec.good_events = rec.good_events.saturating_add(1);
        rec.last_reason = Some(reason.to_owned());
        observe_record(rec, Self::now());
    }

    pub fn penalize(&mut self, peer_id: &str, delta: i32, reason: &str, ban_seconds: u64) {
        let rec = self.entry(peer_id);
        rec.score = (rec.score.saturating_sub(delta)).clamp(SCORE_MIN, SCORE_MAX);
        rec.bad_events = rec.bad_events.saturating_add(1);
        rec.last_reason = Some(reason.to_owned());
        observe_record(rec, Self::now());
        if rec.score <= BAN_THRESHOLD {
            let until = Self::now().saturating_add(ban_seconds.max(1));
            rec.banned_until_unix = rec.banned_until_unix.max(until);
        }
    }

    pub fn active_ban_count(&self) -> usize {
        let now = Self::now();
        self.peers
            .values()
            .filter(|record| {
                record.banned_until_unix > now || record.manually_banned_until_unix > now
            })
            .count()
    }

    /// Drop expired ban markers for hygiene (scores retained).
    pub fn prune_expired_bans(&mut self) {
        let now = Self::now();
        for rec in self.peers.values_mut() {
            if rec.banned_until_unix > 0 && rec.banned_until_unix <= now {
                rec.banned_until_unix = 0;
            }
            if rec.manually_banned_until_unix > 0
                && rec.manually_banned_until_unix != u64::MAX
                && rec.manually_banned_until_unix <= now
            {
                rec.manually_banned_until_unix = 0;
            }
        }
    }

    pub fn observe_connection(&mut self, peer_id: &str) {
        self.observe_connection_at(peer_id, Self::now());
    }

    pub fn observe_validation(&mut self, peer_id: &str) {
        let now = Self::now();
        let record = self.entry(peer_id);
        observe_record(record, now);
        record.validated_handshakes = record.validated_handshakes.saturating_add(1);
    }

    pub fn observe_disconnect(&mut self, peer_id: &str) {
        self.observe_disconnect_at(peer_id, Self::now());
    }

    pub fn observe_failed_connection(&mut self, peer_id: &str) {
        let now = Self::now();
        let record = self.entry(peer_id);
        observe_record(record, now);
        record.failed_connections = record.failed_connections.saturating_add(1);
    }

    pub fn observed_uptime_seconds(&self, peer_id: &str) -> u64 {
        self.observed_uptime_seconds_at(peer_id, Self::now())
    }

    pub fn record(&self, peer_id: &str) -> Option<&PeerScoreRecord> {
        self.peers.get(peer_id)
    }

    pub fn apply_admin_request(&mut self, request: &PeerAdminRequest) {
        let now = Self::now();
        let record = self.entry(&request.peer_id);
        observe_record(record, now);
        match &request.action {
            PeerAdminAction::Ban {
                reason,
                duration_seconds,
            } => {
                record.manually_banned_until_unix = if *duration_seconds == 0 {
                    u64::MAX
                } else {
                    now.saturating_add(*duration_seconds)
                };
                record.last_reason = Some(format!("operator ban: {reason}"));
            }
            PeerAdminAction::Unban => {
                record.banned_until_unix = 0;
                record.manually_banned_until_unix = 0;
                record.last_reason = Some("operator unban".to_owned());
            }
        }
    }

    /// Classify a sync/protocol error and apply the matching penalty or hard ban.
    pub fn apply_protocol_fault(&mut self, peer_id: &str, message: &str) {
        let lower = message.to_ascii_lowercase();
        let severe = lower.contains("pow")
            || lower.contains("mix_hash")
            || lower.contains("mix hash")
            || lower.contains("does not match verified header")
            || lower.contains("header chain")
            || lower.contains("work commitment")
            || lower.contains("invalid checkpoint")
            || lower.contains("invalid difficulty")
            || lower.contains("firopow");
        if severe {
            // Forged work or header-body mismatch → immediate ban.
            self.ban_now(peer_id, message, SEVERE_BAN_SECONDS);
            return;
        }
        let delta = if lower.contains("handshake") || lower.contains("genesis") {
            40
        } else if lower.contains("empty") || lower.contains("unsolicited") {
            20
        } else {
            25
        };
        self.penalize(peer_id, delta, message, DEFAULT_BAN_SECONDS);
    }

    fn observe_connection_at(&mut self, peer_id: &str, now: u64) {
        let record = self.entry(peer_id);
        observe_record(record, now);
        if record.connected_since_unix == 0 {
            record.connected_since_unix = now.max(1);
            record.successful_connections = record.successful_connections.saturating_add(1);
        }
    }

    fn observe_disconnect_at(&mut self, peer_id: &str, now: u64) {
        let record = self.entry(peer_id);
        observe_record(record, now);
        if record.connected_since_unix > 0 {
            record.observed_uptime_seconds = record
                .observed_uptime_seconds
                .saturating_add(now.saturating_sub(record.connected_since_unix));
            record.connected_since_unix = 0;
        }
    }

    fn observed_uptime_seconds_at(&self, peer_id: &str, now: u64) -> u64 {
        self.peers.get(peer_id).map_or(0, |record| {
            let active_interval = if record.connected_since_unix > 0 {
                now.saturating_sub(record.connected_since_unix)
            } else {
                0
            };
            record
                .observed_uptime_seconds
                .saturating_add(active_interval)
        })
    }
}

fn observe_record(record: &mut PeerScoreRecord, now: u64) {
    if record.first_observed_at_unix == 0 {
        record.first_observed_at_unix = now;
    }
    record.last_observed_at_unix = now;
}

pub fn enqueue_admin_request(
    runtime_dir: &Path,
    peer_id: String,
    action: PeerAdminAction,
) -> NodeResult<PeerAdminRequest> {
    let created_at_unix = ReputationStore::now();
    let request = PeerAdminRequest {
        request_id: format!(
            "{}-{}-{}",
            created_at_unix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        ),
        peer_id,
        created_at_unix,
        action,
    };
    with_admin_queue_lock(runtime_dir, |path| {
        let mut requests = load_admin_queue(path)?;
        if requests.len() >= MAX_ADMIN_QUEUE_ENTRIES {
            return Err(NodeError::P2p(format!(
                "peer admin queue limit of {MAX_ADMIN_QUEUE_ENTRIES} entries reached"
            )));
        }
        requests.push(request.clone());
        persist_admin_queue(path, &requests)
    })?;
    Ok(request)
}

pub fn load_admin_requests(runtime_dir: &Path) -> NodeResult<Vec<PeerAdminRequest>> {
    with_admin_queue_lock(runtime_dir, load_admin_queue)
}

pub fn acknowledge_admin_requests(runtime_dir: &Path, request_ids: &[String]) -> NodeResult<()> {
    let acknowledged = request_ids
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    with_admin_queue_lock(runtime_dir, |path| {
        let mut requests = load_admin_queue(path)?;
        requests.retain(|request| !acknowledged.contains(&request.request_id));
        persist_admin_queue(path, &requests)
    })
}

fn with_admin_queue_lock<T>(
    runtime_dir: &Path,
    operation: impl FnOnce(&Path) -> NodeResult<T>,
) -> NodeResult<T> {
    fs::create_dir_all(runtime_dir)?;
    let lock_path = runtime_dir.join(PEER_ADMIN_QUEUE_LOCK_FILE_NAME);
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.lock_exclusive()?;
    let result = operation(&runtime_dir.join(PEER_ADMIN_QUEUE_FILE_NAME));
    FileExt::unlock(&lock)?;
    result
}

fn load_admin_queue(path: &Path) -> NodeResult<Vec<PeerAdminRequest>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&fs::read_to_string(path)?).map_err(|error| {
        NodeError::P2p(format!(
            "invalid persisted peer admin queue at {}: {error}",
            path.display()
        ))
    })
}

fn persist_admin_queue(path: &Path, requests: &[PeerAdminRequest]) -> NodeResult<()> {
    let mut file = AtomicWriteFile::open(path)?;
    serde_json::to_writer_pretty(&mut file, requests)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    file.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn ban_triggers_after_penalties() {
        let mut store = ReputationStore::default();
        store.penalize("peer-a", 30, "bad handshake", 60);
        store.penalize("peer-a", 30, "invalid block", 60);
        assert!(store.score_of("peer-a") <= BAN_THRESHOLD);
        assert!(store.is_banned("peer-a"));
        assert_eq!(store.active_ban_count(), 1);
    }

    #[test]
    fn reward_raises_score() {
        let mut store = ReputationStore::default();
        store.reward("peer-b", 10, "useful headers");
        assert!(store.score_of("peer-b") > DEFAULT_SCORE);
        assert!(!store.is_banned("peer-b"));
    }

    #[test]
    fn persists_roundtrip() {
        let dir = tempdir().expect("temp");
        let mut store = ReputationStore::default();
        store.penalize("peer-c", 100, "spam", 120);
        store.persist(dir.path()).expect("persist");
        let loaded = ReputationStore::load(dir.path()).expect("load");
        assert!(loaded.is_banned("peer-c"));
        // DEFAULT_SCORE 50 − 100 = −50 (clamped to SCORE_MIN only if lower).
        assert_eq!(loaded.score_of("peer-c"), -50);
        assert!(loaded.score_of("peer-c") <= BAN_THRESHOLD);
    }

    #[test]
    fn protocol_fault_hard_bans_forged_work() {
        let mut store = ReputationStore::default();
        store.apply_protocol_fault("peer-pow", "invalid FiroPoW mix_hash for height 12");
        assert!(store.is_banned("peer-pow"));
        assert!(store.score_of("peer-pow") <= BAN_THRESHOLD);
    }

    #[test]
    fn protocol_fault_soft_penalizes_generic_errors() {
        let mut store = ReputationStore::default();
        store.apply_protocol_fault("peer-soft", "temporary timeout waiting for headers");
        assert!(!store.is_banned("peer-soft"));
        assert!(store.score_of("peer-soft") < DEFAULT_SCORE);
    }

    #[test]
    fn ban_now_forces_threshold() {
        let mut store = ReputationStore::default();
        store.ban_now("peer-d", "severe abuse", 30);
        assert!(store.is_banned("peer-d"));
        assert_eq!(store.score_of("peer-d"), BAN_THRESHOLD);
    }

    #[test]
    fn corrupt_reputation_is_rejected_instead_of_losing_bans() {
        let dir = tempdir().expect("temp");
        fs::write(dir.path().join(REPUTATION_FILE_NAME), "{broken").expect("write corrupt file");
        assert!(ReputationStore::load(dir.path()).is_err());
    }

    #[test]
    fn uptime_uses_only_observed_connected_intervals() {
        let mut store = ReputationStore::default();
        store.observe_connection_at("peer-up", 100);
        assert_eq!(store.observed_uptime_seconds_at("peer-up", 130), 30);
        store.observe_disconnect_at("peer-up", 145);
        assert_eq!(store.observed_uptime_seconds_at("peer-up", 1_000), 45);
    }

    #[test]
    fn operator_ban_does_not_manufacture_rating_events() {
        let mut store = ReputationStore::default();
        let request = PeerAdminRequest {
            request_id: "request-1".to_owned(),
            peer_id: "peer-admin".to_owned(),
            created_at_unix: 1,
            action: PeerAdminAction::Ban {
                reason: "maintenance".to_owned(),
                duration_seconds: 60,
            },
        };
        store.apply_admin_request(&request);
        let record = store.record("peer-admin").expect("record");
        assert_eq!(record.score, DEFAULT_SCORE);
        assert_eq!(record.good_events, 0);
        assert_eq!(record.bad_events, 0);
        assert!(store.is_banned("peer-admin"));
    }

    #[test]
    fn admin_queue_acknowledges_only_processed_requests() {
        let dir = tempdir().expect("temp");
        let first =
            enqueue_admin_request(dir.path(), "peer-one".to_owned(), PeerAdminAction::Unban)
                .expect("first");
        let second =
            enqueue_admin_request(dir.path(), "peer-two".to_owned(), PeerAdminAction::Unban)
                .expect("second");
        acknowledge_admin_requests(dir.path(), &[first.request_id]).expect("ack");
        assert_eq!(load_admin_requests(dir.path()).expect("load"), vec![second]);
    }
}
