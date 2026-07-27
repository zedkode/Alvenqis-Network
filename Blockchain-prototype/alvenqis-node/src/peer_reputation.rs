//! Peer scoring and temporary bans for the P2P swarm (audit A-H05).
//!
//! Process-local with optional JSON persistence under the node runtime dir.
//! Scores rise on useful sync contributions and fall on rejected handshakes,
//! invalid blocks and protocol abuse. Peers at or below the ban threshold
//! are refused until `banned_until`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const REPUTATION_FILE_NAME: &str = "peer-reputation.json";

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
    pub last_reason: Option<String>,
    pub good_events: u64,
    pub bad_events: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReputationStore {
    pub peers: BTreeMap<String, PeerScoreRecord>,
}

impl ReputationStore {
    pub fn load(runtime_dir: &Path) -> Self {
        let path = runtime_dir.join(REPUTATION_FILE_NAME);
        fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn persist(&self, runtime_dir: &Path) {
        let _ = fs::create_dir_all(runtime_dir);
        let path = runtime_dir.join(REPUTATION_FILE_NAME);
        if let Ok(raw) = serde_json::to_vec_pretty(self) {
            // Best-effort atomic-ish write: temp + rename when possible.
            let tmp = runtime_dir.join(format!("{REPUTATION_FILE_NAME}.tmp"));
            if fs::write(&tmp, &raw).is_ok() {
                if fs::rename(&tmp, &path).is_err() {
                    let _ = fs::write(&path, raw);
                    let _ = fs::remove_file(&tmp);
                }
            } else {
                let _ = fs::write(path, raw);
            }
        }
    }

    /// Immediately ban a peer for `ban_seconds` (used for severe protocol abuse).
    pub fn ban_now(&mut self, peer_id: &str, reason: &str, ban_seconds: u64) {
        let rec = self.entry(peer_id);
        rec.score = BAN_THRESHOLD;
        rec.bad_events = rec.bad_events.saturating_add(1);
        rec.last_reason = Some(reason.to_owned());
        rec.banned_until_unix = Self::now().saturating_add(ban_seconds.max(1));
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
                last_reason: None,
                good_events: 0,
                bad_events: 0,
            })
    }

    /// True if peer is currently banned.
    pub fn is_banned(&self, peer_id: &str) -> bool {
        let now = Self::now();
        self.peers
            .get(peer_id)
            .is_some_and(|r| r.banned_until_unix > now)
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
    }

    pub fn penalize(&mut self, peer_id: &str, delta: i32, reason: &str, ban_seconds: u64) {
        let rec = self.entry(peer_id);
        rec.score = (rec.score.saturating_sub(delta)).clamp(SCORE_MIN, SCORE_MAX);
        rec.bad_events = rec.bad_events.saturating_add(1);
        rec.last_reason = Some(reason.to_owned());
        if rec.score <= BAN_THRESHOLD {
            let until = Self::now().saturating_add(ban_seconds.max(1));
            rec.banned_until_unix = rec.banned_until_unix.max(until);
        }
    }

    pub fn active_ban_count(&self) -> usize {
        let now = Self::now();
        self.peers
            .values()
            .filter(|r| r.banned_until_unix > now)
            .count()
    }

    /// Drop expired ban markers for hygiene (scores retained).
    pub fn prune_expired_bans(&mut self) {
        let now = Self::now();
        for rec in self.peers.values_mut() {
            if rec.banned_until_unix > 0 && rec.banned_until_unix <= now {
                rec.banned_until_unix = 0;
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
        store.persist(dir.path());
        let loaded = ReputationStore::load(dir.path());
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
}
