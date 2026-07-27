use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Network {
    Devnet,
    Testnet,
    #[default]
    MainnetCandidate,
}

impl Network {
    pub const fn network_id(self) -> &'static str {
        match self {
            Self::Devnet => "alvenqis-devnet",
            Self::Testnet => "alvenqis-testnet",
            Self::MainnetCandidate => "alvenqis-mainnet-candidate",
        }
    }

    pub const fn human_name(self) -> &'static str {
        match self {
            Self::Devnet => "Alvenqis Devnet",
            Self::Testnet => "Alvenqis Testnet",
            Self::MainnetCandidate => "Alvenqis Mainnet Candidate",
        }
    }

    pub const fn address_prefix(self) -> &'static str {
        match self {
            Self::Devnet => "dalve",
            Self::Testnet => "talve",
            Self::MainnetCandidate => "alve",
        }
    }

    pub const fn default_data_root(self) -> &'static str {
        match self {
            Self::Devnet => ".alvenqis-dev",
            Self::Testnet => ".alvenqis-testnet",
            Self::MainnetCandidate => ".alvenqis-mainnet",
        }
    }

    pub const fn default_rpc_port(self) -> u16 {
        match self {
            Self::Devnet => 8787,
            Self::Testnet => 9787,
            Self::MainnetCandidate => 10787,
        }
    }

    pub const fn default_p2p_port(self) -> u16 {
        match self {
            Self::Devnet => 18787,
            Self::Testnet => 19787,
            Self::MainnetCandidate => 20787,
        }
    }

    pub const fn genesis_config_path(self) -> &'static str {
        match self {
            Self::Devnet => "alvenqis-devnet/config/genesis-devnet.json",
            Self::Testnet => "alvenqis-devnet/config/genesis-testnet.json",
            Self::MainnetCandidate => "configs/genesis.mainnet-candidate.toml",
        }
    }

    pub const fn chain_magic_bytes(self) -> [u8; 4] {
        match self {
            Self::Devnet => *b"ALDV",
            Self::Testnet => *b"ALTS",
            Self::MainnetCandidate => *b"ALMC",
        }
    }

    pub const fn status_label(self) -> &'static str {
        match self {
            Self::Devnet => "Draft / Private Devnet",
            Self::Testnet => "Planned / Public Testnet Candidate",
            Self::MainnetCandidate => "Planned / Mainnet Candidate",
        }
    }

    pub const fn is_resettable(self) -> bool {
        matches!(self, Self::Devnet)
    }

    pub const fn is_local_private_only(self) -> bool {
        matches!(self, Self::Devnet)
    }

    pub const fn allows_low_difficulty_defaults(self) -> bool {
        matches!(self, Self::Devnet)
    }

    pub const fn requires_explicit_allow(self) -> bool {
        matches!(self, Self::MainnetCandidate)
    }

    pub const fn difficulty_adjustment_window(self) -> usize {
        match self {
            Self::Devnet | Self::Testnet | Self::MainnetCandidate => 60,
        }
    }

    pub const fn minimum_difficulty_leading_zero_bits(self) -> u8 {
        match self {
            Self::Devnet => 4,
            Self::Testnet => 12,
            // Frozen for Mainnet Candidate chain history (changing breaks historical DAA replay).
            Self::MainnetCandidate => 16,
        }
    }

    pub const fn maximum_difficulty_leading_zero_bits(self) -> u8 {
        match self {
            Self::Devnet => 24,
            Self::Testnet => 32,
            // After candidate chain reset (0.9.0): cap so a single ~100-200 MH/s GPU
            // can hold ~60s blocks (difficulty approximately 33-34). Full historical wipe required when changing this.
            Self::MainnetCandidate => 34,
        }
    }

    pub fn from_network_id(value: &str) -> Option<Self> {
        match value {
            "alvenqis-devnet" => Some(Self::Devnet),
            "alvenqis-testnet" => Some(Self::Testnet),
            "alvenqis-mainnet-candidate" => Some(Self::MainnetCandidate),
            _ => None,
        }
    }

    pub fn from_address_prefix(value: &str) -> Option<Self> {
        match value {
            "dalve" => Some(Self::Devnet),
            "talve" => Some(Self::Testnet),
            "alve" => Some(Self::MainnetCandidate),
            _ => None,
        }
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.network_id())
    }
}

impl FromStr for Network {
    type Err = crate::protocol::errors::AlvenqisError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "devnet" | "alvenqis-devnet" => Ok(Self::Devnet),
            "testnet" | "alvenqis-testnet" => Ok(Self::Testnet),
            "mainnet-candidate" | "alvenqis-mainnet-candidate" => Ok(Self::MainnetCandidate),
            _ => Err(crate::protocol::errors::AlvenqisError::InvalidNetwork {
                expected: "mainnet-candidate (devnet and testnet are internal test profiles)"
                    .to_owned(),
                actual: value.to_owned(),
            }),
        }
    }
}
