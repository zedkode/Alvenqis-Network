//! CUDA-only FiroPoW 0.9.4 product mining.
//!
//! Nodes continue to validate PoW on the host in `alvenqis-core`, but this crate
//! never searches nonces on the CPU and never labels host work as GPU mining.

mod coordinator;
mod traits;

#[cfg(feature = "gpu-cuda")]
mod cuda;
#[cfg(feature = "gpu-cuda")]
mod cuda_driver;

pub use coordinator::{CudaMiningCoordinator, MiningMode};
pub use traits::{
    BackendConfig, BackendId, BackendMetrics, BenchmarkResult, MiningBackend, MiningDevice,
    MiningJob, MiningSolution,
};

#[cfg(feature = "gpu-cuda")]
pub use cuda::CudaGpuBackend;

pub fn available_backends() -> Vec<Box<dyn MiningBackend>> {
    #[cfg(feature = "gpu-cuda")]
    {
        vec![Box::new(CudaGpuBackend::default())]
    }
    #[cfg(not(feature = "gpu-cuda"))]
    {
        Vec::new()
    }
}

pub fn enumerate_all_devices() -> Vec<MiningDevice> {
    enumerate_device_report().devices
}

#[derive(Debug, Clone, Default)]
pub struct DeviceEnumerationReport {
    pub devices: Vec<MiningDevice>,
    pub notes: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GpuVendorClass {
    Nvidia,
    Amd,
    Intel,
    Other,
}

pub fn classify_gpu(vendor: &str, name: &str) -> GpuVendorClass {
    let description = format!("{vendor} {name}").to_ascii_uppercase();
    if description.contains("NVIDIA")
        || description.contains("GEFORCE")
        || description.contains("RTX")
        || description.contains("GTX")
        || description.contains("QUADRO")
        || description.contains("TESLA")
        || description.contains("TITAN")
    {
        GpuVendorClass::Nvidia
    } else if description.contains("AMD")
        || description.contains("ADVANCED MICRO")
        || description.contains("RADEON")
        || description.contains("ATI ")
        || description.contains("ATI/")
    {
        GpuVendorClass::Amd
    } else if description.contains("INTEL") {
        GpuVendorClass::Intel
    } else {
        GpuVendorClass::Other
    }
}

fn physical_key(device: &MiningDevice) -> String {
    let name = device
        .name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    format!("{name}:{}", device.global_memory_bytes.unwrap_or(0))
}

/// De-duplicate CUDA runtime/driver inventory entries for the same physical GPU.
pub fn dedupe_gpu_devices(raw: Vec<MiningDevice>) -> (Vec<MiningDevice>, Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    let mut devices: Vec<MiningDevice> = raw
        .into_iter()
        .filter(|device| device.backend == BackendId::GpuCuda)
        .filter(|device| seen.insert(physical_key(device)))
        .collect();
    for (index, device) in devices.iter_mut().enumerate() {
        device.index = index;
        device.selected = true;
    }

    let mut notes = Vec::new();
    match devices.len() {
        0 => notes
            .push("No NVIDIA CUDA GPU found. CPU and host-emulated mining are disabled.".into()),
        1 => notes.push("Single CUDA GPU detected.".into()),
        count => notes.push(format!("{count} CUDA GPUs detected; multi-GPU is enabled.")),
    }
    (devices, notes)
}

pub fn enumerate_device_report() -> DeviceEnumerationReport {
    let mut raw = Vec::new();
    let mut notes = Vec::new();
    for mut backend in available_backends() {
        let name = backend.backend_name().to_string();
        match backend.available_devices() {
            Ok(devices) => raw.extend(devices),
            Err(error) => notes.push(format!("{name}: {error}")),
        }
    }
    let (devices, mut inventory_notes) = dedupe_gpu_devices(raw);
    notes.append(&mut inventory_notes);
    DeviceEnumerationReport { devices, notes }
}
