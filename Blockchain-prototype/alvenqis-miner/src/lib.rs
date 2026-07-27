mod activity;
mod backends;
mod config;
mod error;
mod mining;
mod nonce;
mod protocol;
mod source;
mod stratum;

#[cfg(feature = "gpu-cuda")]
pub use backends::CudaGpuBackend;
pub use backends::{
    available_backends, classify_gpu, dedupe_gpu_devices, enumerate_all_devices,
    enumerate_device_report, BackendConfig, BackendId, BackendMetrics, BenchmarkResult,
    CudaMiningCoordinator, DeviceEnumerationReport, GpuVendorClass, MiningBackend, MiningDevice,
    MiningJob, MiningMode, MiningSolution,
};

pub use activity::{default_activity_path, ActivityLog};
pub use config::{MinerConfig, WorkSourceConfig, MINER_CONFIG_SCHEMA_VERSION};
pub use error::{MinerError, Result};
pub use nonce::{NonceAllocator, NonceRange, WorkIdentity};
pub use protocol::{
    MiningSubmitRequest, MiningSubmitResponse, MiningTemplate, SubmitStatus,
    MINING_PROTOCOL_VERSION,
};
pub use source::{FileWorkSource, PoolWorkSource, RpcWorkSource, WorkSource};
pub use stratum::{StratumEndpoint, StratumWorkSource, STRATUM_PROTOCOL};
