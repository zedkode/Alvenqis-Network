//! CUDA-only mining coordinator.

use super::traits::{
    BackendConfig, BackendId, BackendMetrics, BenchmarkResult, MiningBackend, MiningDevice,
    MiningJob, MiningSolution,
};
#[cfg(feature = "gpu-cuda")]
use super::CudaGpuBackend;
#[cfg(feature = "gpu-cuda")]
use crate::mining::revalidate_solution;
use crate::{MinerError, Result};
use std::time::Duration;

const GPU_REQUIRED: &str =
    "No working NVIDIA CUDA miner is available. CPU/OpenCL mining and CPU fallback are disabled.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MiningMode {
    /// Compatibility alias; resolves to CUDA-only.
    Gpu,
    Cuda,
    /// Compatibility alias; resolves to CUDA-only.
    Auto,
}

impl MiningMode {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "cuda" | "nvidia" => Ok(Self::Cuda),
            "gpu" | "gpu-only" => Ok(Self::Gpu),
            "auto" => Ok(Self::Auto),
            "cpu" | "cpu-gpu" | "cpu+gpu" | "hybrid" | "both" | "opencl" => {
                Err(MinerError::Config(
                    "only NVIDIA CUDA GPU mining is supported; CPU/OpenCL mining is disabled"
                        .into(),
                ))
            }
            other => Err(MinerError::Config(format!(
                "unknown mining backend mode '{other}' (expected cuda)"
            ))),
        }
    }

    pub const fn as_str(self) -> &'static str {
        "cuda"
    }
}

pub struct CudaMiningCoordinator {
    mode: MiningMode,
    #[cfg(feature = "gpu-cuda")]
    cuda: Option<CudaGpuBackend>,
    metrics: BackendMetrics,
    ready: bool,
    active_name: String,
}

impl Default for CudaMiningCoordinator {
    fn default() -> Self {
        Self {
            mode: MiningMode::Cuda,
            #[cfg(feature = "gpu-cuda")]
            cuda: None,
            metrics: BackendMetrics::default(),
            ready: false,
            active_name: "uninitialized".into(),
        }
    }
}

impl CudaMiningCoordinator {
    pub fn new(mode: MiningMode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    pub fn mode(&self) -> MiningMode {
        self.mode
    }
}

impl MiningBackend for CudaMiningCoordinator {
    fn backend_id(&self) -> BackendId {
        BackendId::GpuCuda
    }

    fn backend_name(&self) -> &str {
        &self.active_name
    }

    fn available_devices(&mut self) -> Result<Vec<MiningDevice>> {
        #[cfg(feature = "gpu-cuda")]
        {
            let mut cuda = CudaGpuBackend::default();
            cuda.available_devices()
        }
        #[cfg(not(feature = "gpu-cuda"))]
        Err(MinerError::Config(GPU_REQUIRED.into()))
    }

    fn initialize(&mut self, config: BackendConfig) -> Result<()> {
        #[cfg(feature = "gpu-cuda")]
        {
            let mut cuda = CudaGpuBackend::default();
            cuda.initialize(config)?;
            self.metrics.active_devices = cuda.metrics().active_devices;
            self.active_name = cuda.backend_name().to_string();
            self.cuda = Some(cuda);
            self.ready = true;
            Ok(())
        }
        #[cfg(not(feature = "gpu-cuda"))]
        {
            let _ = config;
            Err(MinerError::Config(GPU_REQUIRED.into()))
        }
    }

    fn mine_batch(&mut self, _job: &MiningJob) -> Result<Option<MiningSolution>> {
        if !self.ready {
            return Err(MinerError::Config(
                "CUDA coordinator not initialized".into(),
            ));
        }
        #[cfg(feature = "gpu-cuda")]
        if let Some(cuda) = self.cuda.as_mut() {
            let before = cuda.metrics().hashes_attempted;
            let result = cuda.mine_batch(_job)?;
            let cuda_metrics = cuda.metrics();
            self.metrics.hashrate_hs = cuda_metrics.hashrate_hs;
            self.metrics.hashes_attempted = self
                .metrics
                .hashes_attempted
                .saturating_add(cuda_metrics.hashes_attempted.saturating_sub(before));
            self.metrics.active_devices = cuda_metrics.active_devices;
            self.metrics.last_error = cuda_metrics.last_error;

            if let Some(solution) = result {
                if revalidate_solution(
                    &_job.block,
                    solution.nonce,
                    &solution.mix_hash,
                    _job.difficulty_leading_zero_bits,
                )? {
                    self.metrics.accepted_local = self.metrics.accepted_local.saturating_add(1);
                    return Ok(Some(solution));
                }
                self.metrics.rejected_local = self.metrics.rejected_local.saturating_add(1);
                return Err(MinerError::Gpu(
                    "CUDA produced a candidate that failed canonical FiroPoW validation".into(),
                ));
            }
            return Ok(None);
        }
        Err(MinerError::Config(GPU_REQUIRED.into()))
    }

    fn benchmark(&mut self, _job: &MiningJob, _duration: Duration) -> Result<BenchmarkResult> {
        if !self.ready {
            self.initialize(BackendConfig::default())?;
        }
        #[cfg(feature = "gpu-cuda")]
        if let Some(cuda) = self.cuda.as_mut() {
            return cuda.benchmark(_job, _duration);
        }
        Err(MinerError::Config(GPU_REQUIRED.into()))
    }

    fn metrics(&self) -> BackendMetrics {
        self.metrics.clone()
    }
}
