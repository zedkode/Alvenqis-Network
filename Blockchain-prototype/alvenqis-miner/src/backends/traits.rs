use crate::Result;
use alvenqis_core::{Block, Hash};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendId {
    GpuCuda,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MiningDevice {
    pub id: String,
    pub backend: BackendId,
    pub name: String,
    pub vendor: String,
    pub index: usize,
    pub compute_units: Option<u32>,
    pub global_memory_bytes: Option<u64>,
    pub selected: bool,
}

#[derive(Clone, Debug)]
pub struct MiningJob {
    pub template_id: String,
    pub block: Block,
    pub difficulty_leading_zero_bits: u8,
    pub start_nonce: u64,
    pub max_nonces: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MiningSolution {
    pub nonce: u64,
    pub final_hash: Hash,
    pub mix_hash: Hash,
    pub attempts: u64,
    pub device_id: String,
    pub backend: BackendId,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BackendMetrics {
    pub hashrate_hs: f64,
    pub hashes_attempted: u64,
    pub accepted_local: u64,
    pub rejected_local: u64,
    pub stale: u64,
    pub active_devices: usize,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub device_id: String,
    pub backend: BackendId,
    pub hashrate_hs: f64,
    pub duration_ms: u64,
    pub hashes: u64,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct BackendConfig {
    pub batch_size: u64,
    pub device_ids: Vec<String>,
    pub intensity: u8,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            batch_size: 8_192,
            device_ids: Vec::new(),
            intensity: 75,
        }
    }
}

pub trait MiningBackend: Send {
    fn backend_id(&self) -> BackendId;
    fn backend_name(&self) -> &str;
    fn available_devices(&mut self) -> Result<Vec<MiningDevice>>;
    fn initialize(&mut self, config: BackendConfig) -> Result<()>;
    fn mine_batch(&mut self, job: &MiningJob) -> Result<Option<MiningSolution>>;
    fn benchmark(&mut self, job: &MiningJob, duration: Duration) -> Result<BenchmarkResult>;
    fn metrics(&self) -> BackendMetrics;
}
