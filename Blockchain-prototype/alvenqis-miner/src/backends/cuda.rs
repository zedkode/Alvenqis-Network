//! Real NVIDIA CUDA path for FiroPoW 0.9.4.
//!
//! ## Discovery
//! - Primary: CUDA Driver API (`nvcuda.dll` / `libcuda.so`) — **not OpenCL**.
//! - Secondary (when device kernels linked): CUDA Runtime via compiled kernel lib.
//!
//! ## Mining
//! - Builds the epoch DAG directly in VRAM from the small Ethash light cache.
//! - Runs FiroPoW search exclusively on CUDA devices; there is no CPU fallback.
//! - Solutions are always re-validated by alvenqis-core before submit.

use super::cuda_driver;
use super::traits::{
    BackendConfig, BackendId, BackendMetrics, BenchmarkResult, MiningBackend, MiningDevice,
    MiningJob, MiningSolution,
};
use crate::{MinerError, Result};
#[cfg(alvenqis_cuda_linked)]
use alvenqis_core::firopow;
#[cfg(alvenqis_cuda_linked)]
use alvenqis_core::firopow::mining_header_hash;
#[cfg(alvenqis_cuda_linked)]
use alvenqis_core::Hash;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Optional linked CUDA device-kernel FFI (firopow_cuda.cu)
// ---------------------------------------------------------------------------
#[cfg(alvenqis_cuda_linked)]
#[repr(C)]
struct AlvenqisCudaDeviceInfo {
    index: i32,
    name: [u8; 256],
    total_mem: usize,
    multi_processor_count: i32,
    major: i32,
    minor: i32,
}

#[cfg(alvenqis_cuda_linked)]
#[repr(C)]
struct AlvenqisCudaMiner {
    _private: [u8; 0],
}

#[cfg(alvenqis_cuda_linked)]
extern "C" {
    fn alvenqis_cuda_available() -> i32;
    fn alvenqis_cuda_device_info(index: i32, out: *mut AlvenqisCudaDeviceInfo) -> i32;
    fn alvenqis_cuda_miner_create(device_index: i32) -> *mut AlvenqisCudaMiner;
    fn alvenqis_cuda_miner_destroy(m: *mut AlvenqisCudaMiner);
    fn alvenqis_cuda_miner_build_dag(
        m: *mut AlvenqisCudaMiner,
        block_number: i32,
        light_host: *const u32,
        light_items: u32,
        l1_host: *const u32,
        l1_words: u32,
        full_dataset_num_items_1024: i32,
    ) -> i32;
    fn alvenqis_cuda_mine_firopow(
        m: *mut AlvenqisCudaMiner,
        block_number: i32,
        header_hash: *const u8,
        boundary: *const u8,
        start_nonce: u64,
        max_jobs: u32,
        nonce_out: *mut u64,
        final_hash_out: *mut u8,
        mix_hash_out: *mut u8,
        found_out: *mut i32,
        hashes_done_out: *mut u64,
    ) -> i32;
    fn alvenqis_cuda_miner_copy_dag_item(
        m: *mut AlvenqisCudaMiner,
        index: u32,
        out: *mut u8,
    ) -> i32;
    fn alvenqis_cuda_device_kernels_linked() -> i32;
}

pub struct CudaGpuBackend {
    config: BackendConfig,
    metrics: BackendMetrics,
    devices: Vec<MiningDevice>,
    ready: bool,
    cancel: AtomicBool,
    /// Opaque CUDA miner handles per selected device (device-kernel path only).
    #[cfg(alvenqis_cuda_linked)]
    miners: Vec<*mut AlvenqisCudaMiner>,
    #[cfg(alvenqis_cuda_linked)]
    dag_height: Option<u64>,
}

// SAFETY: miner handles used from single mining thread in product path.
#[cfg(alvenqis_cuda_linked)]
unsafe impl Send for CudaGpuBackend {}

impl Default for CudaGpuBackend {
    fn default() -> Self {
        Self {
            config: BackendConfig::default(),
            metrics: BackendMetrics::default(),
            devices: Vec::new(),
            ready: false,
            cancel: AtomicBool::new(false),
            #[cfg(alvenqis_cuda_linked)]
            miners: Vec::new(),
            #[cfg(alvenqis_cuda_linked)]
            dag_height: None,
        }
    }
}

impl Drop for CudaGpuBackend {
    fn drop(&mut self) {
        #[cfg(alvenqis_cuda_linked)]
        {
            for m in self.miners.drain(..) {
                if !m.is_null() {
                    unsafe { alvenqis_cuda_miner_destroy(m) };
                }
            }
        }
    }
}

impl CudaGpuBackend {
    fn device_kernels_available() -> bool {
        #[cfg(alvenqis_cuda_linked)]
        {
            unsafe { alvenqis_cuda_device_kernels_linked() != 0 && alvenqis_cuda_available() > 0 }
        }
        #[cfg(not(alvenqis_cuda_linked))]
        {
            false
        }
    }

    fn enumerate_devices() -> Result<Vec<MiningDevice>> {
        // 1) Prefer CUDA Runtime enum when kernels are linked (same stack as mine).
        #[cfg(alvenqis_cuda_linked)]
        {
            let n = unsafe { alvenqis_cuda_available() };
            if n > 0 {
                let mut out = Vec::with_capacity(n as usize);
                for index in 0..n {
                    let mut info = AlvenqisCudaDeviceInfo {
                        index: 0,
                        name: [0; 256],
                        total_mem: 0,
                        multi_processor_count: 0,
                        major: 0,
                        minor: 0,
                    };
                    if unsafe { alvenqis_cuda_device_info(index, &mut info) } != 0 {
                        continue;
                    }
                    let name = {
                        let end = info
                            .name
                            .iter()
                            .position(|&b| b == 0)
                            .unwrap_or(info.name.len());
                        String::from_utf8_lossy(&info.name[..end]).into_owned()
                    };
                    out.push(MiningDevice {
                        id: format!("cuda:{index}:{}", slug(&name)),
                        backend: BackendId::GpuCuda,
                        name,
                        vendor: "NVIDIA".into(),
                        index: index as usize,
                        compute_units: if info.multi_processor_count > 0 {
                            Some(info.multi_processor_count as u32)
                        } else {
                            None
                        },
                        global_memory_bytes: if info.total_mem > 0 {
                            Some(info.total_mem as u64)
                        } else {
                            None
                        },
                        selected: true,
                    });
                }
                if !out.is_empty() {
                    return Ok(out);
                }
            }
        }

        // 2) CUDA Driver API (nvcuda) — real CUDA driver, never OpenCL.
        cuda_driver::enumerate_cuda_devices()
    }

    #[cfg(alvenqis_cuda_linked)]
    fn ensure_device_miners(&mut self) -> Result<()> {
        if !self.miners.is_empty() {
            return Ok(());
        }
        for d in &self.devices {
            let m = unsafe { alvenqis_cuda_miner_create(d.index as i32) };
            if m.is_null() {
                return Err(MinerError::Gpu(format!(
                    "cudaSetDevice/create failed for {}",
                    d.name
                )));
            }
            self.miners.push(m);
        }
        Ok(())
    }

    #[cfg(alvenqis_cuda_linked)]
    fn ensure_dag_on_devices(&mut self, height: u64) -> Result<()> {
        self.ensure_device_miners()?;
        // Rebuild only when epoch may change (or first use).
        let same_epoch = matches!(self.dag_height, Some(h) if h / 1300 == height / 1300);
        if same_epoch {
            return Ok(());
        }
        self.metrics.last_error = Some(format!(
            "building FiroPoW epoch DAG directly on CUDA devices for height {height}"
        ));
        eprintln!("building FiroPoW DAG in VRAM for height {height}…");
        let view = firopow::firopow_export_light_cache(height).map_err(|e| {
            MinerError::Gpu(format!("FiroPoW light-cache export for CUDA failed: {e}"))
        })?;
        let handles: Vec<usize> = self.miners.iter().map(|miner| *miner as usize).collect();
        let light_cache_ptr = view.light_cache_ptr as usize;
        let l1_ptr = view.l1_ptr as usize;
        let results = std::thread::scope(|scope| {
            let mut joins = Vec::with_capacity(handles.len());
            for (index, handle) in handles.into_iter().enumerate() {
                joins.push(scope.spawn(move || {
                    let rc = unsafe {
                        alvenqis_cuda_miner_build_dag(
                            handle as *mut AlvenqisCudaMiner,
                            height as i32,
                            light_cache_ptr as *const u32,
                            view.light_cache_items,
                            l1_ptr as *const u32,
                            view.l1_words,
                            view.full_dataset_num_items,
                        )
                    };
                    (index, rc)
                }));
            }
            joins
                .into_iter()
                .map(|join| join.join().expect("CUDA DAG worker panicked"))
                .collect::<Vec<_>>()
        });
        for (index, rc) in results {
            if rc != 0 {
                let device = &self.devices[index];
                return Err(MinerError::Gpu(format!(
                    "CUDA DAG build failed (rc={rc}) on {}. Check driver and VRAM capacity.",
                    device.name
                )));
            }
        }
        // Validate representative GPU-built DAG entries against canonical Ethash.
        // This is validation only (three items per epoch), never CPU mining.
        let last = view.full_dataset_num_items.saturating_sub(1) as u32;
        for sample in [0, view.full_dataset_num_items as u32 / 2, last] {
            let expected = firopow::firopow_dataset_item_1024(height, sample).map_err(|e| {
                MinerError::Gpu(format!("canonical DAG sample {sample} failed: {e}"))
            })?;
            for (index, miner) in self.miners.iter().enumerate() {
                let mut actual = [0u8; 128];
                let rc = unsafe {
                    alvenqis_cuda_miner_copy_dag_item(*miner, sample, actual.as_mut_ptr())
                };
                if rc != 0 || actual != expected {
                    return Err(MinerError::Gpu(format!(
                        "CUDA DAG parity failed on {} at item {sample} (rc={rc})",
                        self.devices[index].name
                    )));
                }
            }
        }
        self.dag_height = Some(height);
        self.metrics.last_error = None;
        let dag_mib = firopow::firopow_full_dataset_bytes(height) / (1024 * 1024);
        eprintln!("FiroPoW DAG ready ({dag_mib} MiB per GPU) — CUDA mining starting");
        Ok(())
    }

    #[cfg(alvenqis_cuda_linked)]
    fn mine_device_batch(&mut self, job: &MiningJob) -> Result<Option<MiningSolution>> {
        self.ensure_dag_on_devices(job.block.header.height)?;
        let header = mining_header_hash(&job.block);
        let boundary = firopow::boundary_from_leading_zero_bits(job.difficulty_leading_zero_bits);
        // batch_size is already intensity-scaled by Control Center / config.
        // Intensity only fills in when batch_size is tiny (CLI defaults).
        let intensity = self.config.intensity.max(1) as u64;
        let configured = self.config.batch_size.max(1024);
        let batch = if configured >= 16_384 {
            configured
        } else {
            (configured * intensity / 10).max(configured)
        }
        .clamp(2_048, 1_048_576);
        let max = job.max_nonces.min(batch).min(u32::MAX as u64) as u32;
        if max == 0 {
            return Ok(None);
        }
        let n_dev = self.devices.len().max(1);
        let started = Instant::now();
        let header_bytes = *header.as_bytes();
        let handles: Vec<usize> = self.miners.iter().map(|miner| *miner as usize).collect();
        let ranges = partition_nonce_range(job.start_nonce, max, n_dev);
        let results = std::thread::scope(|scope| {
            let mut joins = Vec::new();
            for (index, ((start, count), handle)) in ranges.into_iter().zip(handles).enumerate() {
                if count == 0 {
                    continue;
                }
                joins.push(scope.spawn(move || {
                    let mut nonce = 0u64;
                    let mut final_hash = [0u8; 32];
                    let mut mix_hash = [0u8; 32];
                    let mut found = 0i32;
                    let mut hashes_done = 0u64;
                    let rc = unsafe {
                        alvenqis_cuda_mine_firopow(
                            handle as *mut AlvenqisCudaMiner,
                            job.block.header.height as i32,
                            header_bytes.as_ptr(),
                            boundary.as_ptr(),
                            start,
                            count,
                            &mut nonce,
                            final_hash.as_mut_ptr(),
                            mix_hash.as_mut_ptr(),
                            &mut found,
                            &mut hashes_done,
                        )
                    };
                    // Prefer kernel counter; fall back to launched range so hashrate never reads 0.
                    if rc == 0 && hashes_done == 0 {
                        hashes_done = u64::from(count);
                    }
                    (
                        index,
                        rc,
                        found,
                        nonce,
                        final_hash,
                        mix_hash,
                        hashes_done,
                        count,
                    )
                }));
            }
            joins
                .into_iter()
                .map(|join| join.join().expect("CUDA mining worker panicked"))
                .collect::<Vec<_>>()
        });

        let mut attempts = 0u64;
        let mut solution = None;
        for (index, rc, found, nonce, final_hash, mix_hash, hashes_done, count) in results {
            let device = &self.devices[index];
            if rc != 0 {
                self.metrics.last_error = Some(format!(
                    "CUDA FiroPoW kernel error rc={rc} on {}",
                    device.name
                ));
                return Err(MinerError::Gpu(format!(
                    "CUDA FiroPoW kernel error rc={rc} on {}",
                    device.name
                )));
            }
            // Never under-report below launched work when kernel completed cleanly.
            let counted = hashes_done.max(1).min(u64::from(count).max(1));
            attempts = attempts.saturating_add(counted);
            if found != 0 && solution.is_none() {
                solution = Some(MiningSolution {
                    nonce,
                    final_hash: Hash::from_bytes(final_hash),
                    mix_hash: Hash::from_bytes(mix_hash),
                    attempts,
                    device_id: device.id.clone(),
                    backend: BackendId::GpuCuda,
                });
            }
        }

        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        // If every device returned empty, still count the dispatch lease so telemetry moves.
        if attempts == 0 {
            attempts = u64::from(max).max(1);
        }
        self.metrics.hashes_attempted = self.metrics.hashes_attempted.saturating_add(attempts);
        self.metrics.hashrate_hs = attempts as f64 / elapsed;
        self.metrics.last_error = None;
        if solution.is_some() {
            self.metrics.accepted_local = self.metrics.accepted_local.saturating_add(1);
        }
        Ok(solution)
    }
}

#[cfg(any(alvenqis_cuda_linked, test))]
fn partition_nonce_range(start_nonce: u64, total: u32, devices: usize) -> Vec<(u64, u32)> {
    let devices = devices.max(1);
    let base = total / devices as u32;
    let remainder = total % devices as u32;
    let mut offset = 0u64;
    (0..devices)
        .map(|index| {
            let count = base + u32::from((index as u32) < remainder);
            let start = start_nonce.wrapping_add(offset);
            offset = offset.wrapping_add(count as u64);
            (start, count)
        })
        .collect()
}

impl MiningBackend for CudaGpuBackend {
    fn backend_id(&self) -> BackendId {
        BackendId::GpuCuda
    }

    fn backend_name(&self) -> &str {
        "CUDA FiroPoW 0.9.4 (GPU kernel + GPU-built VRAM DAG)"
    }

    fn available_devices(&mut self) -> Result<Vec<MiningDevice>> {
        Self::enumerate_devices()
    }

    fn initialize(&mut self, config: BackendConfig) -> Result<()> {
        let mut devices = self.available_devices()?;
        if !config.device_ids.is_empty() {
            devices.retain(|d| {
                config.device_ids.iter().any(|sel| {
                    sel == &d.id
                        || sel == &d.index.to_string()
                        || d.id.contains(sel.as_str())
                        || sel.eq_ignore_ascii_case("cuda")
                        || sel.starts_with("cuda:")
                })
            });
            if devices.is_empty() {
                return Err(MinerError::Gpu(
                    "requested CUDA/NVIDIA devices not found via CUDA driver".into(),
                ));
            }
        }
        if devices.is_empty() {
            return Err(MinerError::Gpu("no NVIDIA CUDA GPU detected".into()));
        }
        if !Self::device_kernels_available() {
            return Err(MinerError::Gpu(
                "this miner was built without CUDA device kernels; CPU fallback is disabled".into(),
            ));
        }
        self.config = config;
        self.devices = devices;
        self.ready = true;
        self.metrics.active_devices = self.devices.len();
        self.cancel.store(false, Ordering::SeqCst);

        #[cfg(alvenqis_cuda_linked)]
        self.ensure_device_miners()?;
        Ok(())
    }

    fn mine_batch(&mut self, _job: &MiningJob) -> Result<Option<MiningSolution>> {
        if !self.ready {
            return Err(MinerError::Gpu("CUDA backend not initialized".into()));
        }
        self.cancel.store(false, Ordering::SeqCst);

        #[cfg(alvenqis_cuda_linked)]
        return self.mine_device_batch(_job);
        #[cfg(not(alvenqis_cuda_linked))]
        Err(MinerError::Gpu(
            "CUDA device kernels are not linked; CPU fallback is disabled".into(),
        ))
    }

    fn benchmark(&mut self, job: &MiningJob, duration: Duration) -> Result<BenchmarkResult> {
        if !self.ready {
            self.initialize(BackendConfig::default())?;
        }
        #[cfg(alvenqis_cuda_linked)]
        self.ensure_dag_on_devices(job.block.header.height)?;
        let started = Instant::now();
        let mut hashes = 0u64;
        let mut cursor = job.start_nonce;
        while started.elapsed() < duration {
            let batch = MiningJob {
                start_nonce: cursor,
                max_nonces: 4_096,
                ..job.clone()
            };
            let before = self.metrics.hashes_attempted;
            let _ = self.mine_batch(&batch)?;
            hashes = hashes.saturating_add(self.metrics.hashes_attempted.saturating_sub(before));
            cursor = cursor.wrapping_add(4_096);
        }
        let secs = started.elapsed().as_secs_f64().max(0.001);
        Ok(BenchmarkResult {
            device_id: self
                .devices
                .first()
                .map(|d| d.id.clone())
                .unwrap_or_else(|| "cuda".into()),
            backend: BackendId::GpuCuda,
            hashrate_hs: hashes as f64 / secs,
            duration_ms: started.elapsed().as_millis() as u64,
            hashes,
            errors: Vec::new(),
        })
    }

    fn metrics(&self) -> BackendMetrics {
        self.metrics.clone()
    }
}

#[cfg(alvenqis_cuda_linked)]
fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// True when CUDA driver reports at least one NVIDIA GPU.
#[allow(dead_code)]
#[cfg(alvenqis_cuda_linked)]
pub fn cuda_runtime_present() -> bool {
    cuda_driver::cuda_driver_available() || unsafe { alvenqis_cuda_available() > 0 }
}

#[allow(dead_code)]
#[cfg(not(alvenqis_cuda_linked))]
pub fn cuda_runtime_present() -> bool {
    cuda_driver::cuda_driver_available()
}

#[cfg(test)]
mod tests {
    use super::partition_nonce_range;

    #[test]
    fn nonce_partitions_are_exact_and_contiguous() {
        assert_eq!(partition_nonce_range(50, 0, 2), vec![(50, 0), (50, 0)]);
        assert_eq!(partition_nonce_range(50, 1, 2), vec![(50, 1), (51, 0)]);
        assert_eq!(
            partition_nonce_range(1_000, 10, 3),
            vec![(1_000, 4), (1_004, 3), (1_007, 3)]
        );
        let ranges = partition_nonce_range(u64::MAX - 1, 4, 2);
        assert_eq!(ranges, vec![(u64::MAX - 1, 2), (0, 2)]);
    }
}
