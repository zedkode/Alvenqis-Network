use crate::models::{FleetNodeView, NodeReport};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct FleetData {
    invitations: Vec<Invitation>,
    nodes: Vec<FleetNode>,
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
struct FleetNode {
    id: String,
    token_hash: String,
    report: NodeReport,
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
        let token = random_hex(32);
        let id = random_hex(12);
        let created = CreatedInvitation {
            id: id.clone(),
            token: token.clone(),
            expires_at_unix_seconds: now.saturating_add(ttl),
        };
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
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
        self.save(&data)?;
        Ok(created)
    }

    pub fn enroll(
        &self,
        invitation_token: &str,
        report: NodeReport,
        now: u64,
    ) -> Result<(String, String), String> {
        let hash = token_hash(invitation_token);
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let invitation = data
            .invitations
            .iter_mut()
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
        invitation.used = true;
        let node_id = random_hex(12);
        let node_token = random_hex(32);
        data.nodes.push(FleetNode {
            id: node_id.clone(),
            token_hash: token_hash(&node_token),
            report,
        });
        self.save(&data)?;
        Ok((node_id, node_token))
    }

    pub fn update_report(
        &self,
        node_id: &str,
        node_token: &str,
        report: NodeReport,
    ) -> Result<(), String> {
        let hash = token_hash(node_token);
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let node = data
            .nodes
            .iter_mut()
            .find(|item| item.id == node_id && item.token_hash == hash)
            .ok_or_else(|| "invalid node credentials".to_owned())?;
        node.report = report;
        self.save(&data)
    }

    pub fn node_views(&self, now: u64) -> Result<Vec<FleetNodeView>, String> {
        let data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        Ok(data
            .nodes
            .iter()
            .map(|node| node_view(&node.id, &node.report, now))
            .collect())
    }

    pub fn invitation_views(&self, now: u64) -> Result<Vec<crate::models::InvitationView>, String> {
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
                crate::models::InvitationView {
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

    pub fn revoke_invitation(&self, invitation_id: &str) -> Result<(), String> {
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let before = data.invitations.len();
        data.invitations.retain(|item| item.id != invitation_id);
        if data.invitations.len() == before {
            return Err("invitation not found".to_owned());
        }
        self.save(&data)
    }

    pub fn remove_node(&self, node_id: &str) -> Result<(), String> {
        if node_id == "local-controller" {
            return Err("cannot remove the local controller from inventory".to_owned());
        }
        let mut data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        let before = data.nodes.len();
        data.nodes.retain(|node| node.id != node_id);
        if data.nodes.len() == before {
            return Err("node not found".to_owned());
        }
        self.save(&data)
    }

    pub fn get_node_report(&self, node_id: &str) -> Result<Option<NodeReport>, String> {
        let data = self.inner.lock().map_err(|_| "fleet lock poisoned")?;
        Ok(data
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .map(|node| node.report.clone()))
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

pub fn node_view(node_id: &str, report: &NodeReport, now: u64) -> FleetNodeView {
    FleetNodeView {
        node_id: node_id.to_owned(),
        node_name: report.node_name.clone(),
        advertise_host: report.advertise_host.clone(),
        p2p_multiaddr: report.p2p_multiaddr.clone(),
        last_seen_unix_seconds: report.reported_at_unix_seconds,
        online: now.saturating_sub(report.reported_at_unix_seconds) <= 45,
        peer_id: crate::models::value_string(&report.p2p, "local_peer_id"),
        height: report
            .status
            .get("height")
            .and_then(serde_json::Value::as_u64),
        connected_peers: crate::models::value_u64(&report.p2p, "connected_peer_count"),
        validating_peers: crate::models::value_u64(&report.p2p, "validating_peer_count"),
        mining_peers: crate::models::value_u64(&report.p2p, "mining_peer_count"),
        observed_hashrate_hs: crate::models::value_u64(&report.p2p, "observed_network_hashrate_hs"),
        services: report.services.clone(),
    }
}

fn random_hex(bytes: usize) -> String {
    let mut value = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut value);
    hex::encode(value)
}

fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ServiceStates;
    use serde_json::json;

    fn report() -> NodeReport {
        NodeReport {
            network_id: "alvenqis-mainnet-candidate".to_owned(),
            node_name: "peer-2".to_owned(),
            advertise_host: "peer2.example.org".to_owned(),
            p2p_multiaddr: "/dns4/peer2.example.org/tcp/20787".to_owned(),
            reported_at_unix_seconds: 100,
            services: ServiceStates::default(),
            status: json!({"height": 12}),
            sync: json!({}),
            p2p: json!({"connected_peer_count": 2}),
            mempool: json!({}),
            indexer: json!({}),
        }
    }

    #[test]
    fn invitation_is_single_use_and_report_credentials_are_required() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        let invite = store
            .create_invitation("peer-2".to_owned(), "peer2.example.org".to_owned(), 90, 60)
            .expect("invite");
        let (id, token) = store.enroll(&invite.token, report(), 100).expect("enroll");
        assert!(store.enroll(&invite.token, report(), 101).is_err());
        assert!(store.update_report(&id, "wrong", report()).is_err());
        store.update_report(&id, &token, report()).expect("report");
    }

    #[test]
    fn revoke_invitation_and_remove_node() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FleetStore::load(dir.path().to_path_buf()).expect("store");
        let invite = store
            .create_invitation("peer-3".to_owned(), "peer3.example.org".to_owned(), 90, 60)
            .expect("invite");
        assert_eq!(store.invitation_views(100).expect("views").len(), 1);
        store.revoke_invitation(&invite.id).expect("revoke");
        assert!(store.invitation_views(100).expect("views").is_empty());

        let invite2 = store
            .create_invitation("peer-2".to_owned(), "peer2.example.org".to_owned(), 90, 60)
            .expect("invite2");
        let (id, _token) = store.enroll(&invite2.token, report(), 100).expect("enroll");
        store.remove_node(&id).expect("remove");
        assert!(store.node_views(100).expect("nodes").is_empty());
        assert!(store.remove_node("local-controller").is_err());
    }
}
