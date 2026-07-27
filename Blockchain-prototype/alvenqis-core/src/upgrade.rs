use crate::network::Network;
use serde::Serialize;

pub const UPGRADE_ACTIVATION_POLICY_ID: &str = "alvenqis-flag-day-upgrade-v1";
pub const UPGRADE_ACTIVATION_MODE: &str = "fixed-height-flag-day";
pub const LAUNCH_PROTOCOL_VERSION: u32 = 1;
pub const LAUNCH_BLOCK_VERSION: u32 = 1;
pub const LAUNCH_UPGRADE_MIGRATION_PATH: &str =
    "Launch uses fixed-height flag-day activation only. Any later negotiated or governance-driven activation path remains a separate follow-up freeze.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ScheduledProtocolUpgrade {
    pub protocol_version: u32,
    pub block_version: u32,
    pub activation_height: u64,
    pub status_label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UpgradeActivationPolicy {
    pub policy_id: &'static str,
    pub activation_mode: &'static str,
    pub network_id: &'static str,
    pub launch_protocol_version: u32,
    pub launch_block_version: u32,
    pub current_protocol_version: u32,
    pub current_block_version: u32,
    pub scheduled_upgrades: Vec<ScheduledProtocolUpgrade>,
    pub migration_path: &'static str,
}

const DEVNET_UPGRADES: [ScheduledProtocolUpgrade; 0] = [];
const TESTNET_UPGRADES: [ScheduledProtocolUpgrade; 0] = [];
const MAINNET_CANDIDATE_UPGRADES: [ScheduledProtocolUpgrade; 0] = [];

pub fn scheduled_protocol_upgrades(network: Network) -> &'static [ScheduledProtocolUpgrade] {
    match network {
        Network::Devnet => &DEVNET_UPGRADES,
        Network::Testnet => &TESTNET_UPGRADES,
        Network::MainnetCandidate => &MAINNET_CANDIDATE_UPGRADES,
    }
}

pub fn protocol_version_at_height(network: Network, height: u64) -> u32 {
    let mut current = LAUNCH_PROTOCOL_VERSION;
    for upgrade in scheduled_protocol_upgrades(network) {
        if height >= upgrade.activation_height {
            current = upgrade.protocol_version;
        }
    }
    current
}

pub fn expected_block_version(network: Network, height: u64) -> u32 {
    let mut current = LAUNCH_BLOCK_VERSION;
    for upgrade in scheduled_protocol_upgrades(network) {
        if height >= upgrade.activation_height {
            current = upgrade.block_version;
        }
    }
    current
}

pub fn launch_upgrade_policy(network: Network) -> UpgradeActivationPolicy {
    UpgradeActivationPolicy {
        policy_id: UPGRADE_ACTIVATION_POLICY_ID,
        activation_mode: UPGRADE_ACTIVATION_MODE,
        network_id: network.network_id(),
        launch_protocol_version: LAUNCH_PROTOCOL_VERSION,
        launch_block_version: LAUNCH_BLOCK_VERSION,
        current_protocol_version: protocol_version_at_height(network, 0),
        current_block_version: expected_block_version(network, 0),
        scheduled_upgrades: scheduled_protocol_upgrades(network).to_vec(),
        migration_path: LAUNCH_UPGRADE_MIGRATION_PATH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn protocol_version_from_schedule(
        launch_version: u32,
        schedule: &[ScheduledProtocolUpgrade],
        height: u64,
    ) -> u32 {
        let mut current = launch_version;
        for upgrade in schedule {
            if height >= upgrade.activation_height {
                current = upgrade.protocol_version;
            }
        }
        current
    }

    fn block_version_from_schedule(
        launch_version: u32,
        schedule: &[ScheduledProtocolUpgrade],
        height: u64,
    ) -> u32 {
        let mut current = launch_version;
        for upgrade in schedule {
            if height >= upgrade.activation_height {
                current = upgrade.block_version;
            }
        }
        current
    }

    #[test]
    fn launch_policy_has_no_scheduled_upgrades_yet() {
        let policy = launch_upgrade_policy(Network::MainnetCandidate);

        assert_eq!(policy.policy_id, UPGRADE_ACTIVATION_POLICY_ID);
        assert_eq!(policy.activation_mode, UPGRADE_ACTIVATION_MODE);
        assert_eq!(policy.launch_protocol_version, 1);
        assert_eq!(policy.launch_block_version, 1);
        assert!(policy.scheduled_upgrades.is_empty());
    }

    #[test]
    fn fixed_height_schedule_changes_versions_at_activation_height() {
        let schedule = [
            ScheduledProtocolUpgrade {
                protocol_version: 2,
                block_version: 2,
                activation_height: 100,
                status_label: "Planned",
            },
            ScheduledProtocolUpgrade {
                protocol_version: 3,
                block_version: 4,
                activation_height: 250,
                status_label: "Planned",
            },
        ];

        assert_eq!(protocol_version_from_schedule(1, &schedule, 99), 1);
        assert_eq!(protocol_version_from_schedule(1, &schedule, 100), 2);
        assert_eq!(protocol_version_from_schedule(1, &schedule, 251), 3);
        assert_eq!(block_version_from_schedule(1, &schedule, 99), 1);
        assert_eq!(block_version_from_schedule(1, &schedule, 100), 2);
        assert_eq!(block_version_from_schedule(1, &schedule, 251), 4);
    }
}
