//! NVIDIA CUDA Driver API (nvcuda) — device discovery without OpenCL ICD.
//!
//! Loads `nvcuda.dll` / `libcuda.so` at runtime so enumeration works with only
//! the NVIDIA display driver installed (CUDA Toolkit optional for discovery).

use super::traits::{BackendId, MiningDevice};
use crate::{MinerError, Result};
use std::ffi::{c_char, c_int, c_uint, c_void, CStr};
use std::sync::OnceLock;

type LibHandle = *mut c_void;

// CUDA Driver API uses __stdcall on Windows (CUDAAPI), cdecl elsewhere.
#[cfg(windows)]
macro_rules! cuda_api {
    (fn($($arg:ty),*) -> $ret:ty) => { unsafe extern "system" fn($($arg),*) -> $ret };
}
#[cfg(not(windows))]
macro_rules! cuda_api {
    (fn($($arg:ty),*) -> $ret:ty) => { unsafe extern "C" fn($($arg),*) -> $ret };
}

type CuInit = cuda_api!(fn(c_uint) -> c_int);
type CuDeviceGetCount = cuda_api!(fn(*mut c_int) -> c_int);
type CuDeviceGet = cuda_api!(fn(*mut c_int, c_int) -> c_int);
type CuDeviceGetName = cuda_api!(fn(*mut c_char, c_int, c_int) -> c_int);
type CuDeviceTotalMem = cuda_api!(fn(*mut usize, c_int) -> c_int);
type CuDeviceGetAttribute = cuda_api!(fn(*mut c_int, c_int, c_int) -> c_int);

// CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT = 16
const CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT: c_int = 16;
const CUDA_SUCCESS: c_int = 0;

struct CudaDriverApi {
    _lib: LibHandle,
    init: CuInit,
    device_get_count: CuDeviceGetCount,
    device_get: CuDeviceGet,
    device_get_name: CuDeviceGetName,
    device_total_mem: CuDeviceTotalMem,
    device_get_attribute: CuDeviceGetAttribute,
}

// SAFETY: CUDA driver symbols are process-global and thread-safe after cuInit.
unsafe impl Send for CudaDriverApi {}
unsafe impl Sync for CudaDriverApi {}

static DRIVER: OnceLock<Option<CudaDriverApi>> = OnceLock::new();

fn load_driver() -> Option<&'static CudaDriverApi> {
    DRIVER.get_or_init(|| unsafe { try_load_driver() }).as_ref()
}

/// SAFETY: `p` must be a valid function pointer of type `T` from the CUDA driver.
/// `T` must be a function-pointer type the same width as `*mut c_void`.
unsafe fn cast_fn<T>(p: *mut c_void) -> T {
    debug_assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<*mut c_void>());
    // Function pointers are not `Sized` in the transmute sense; copy pointer bits.
    std::ptr::read((&p as *const *mut c_void).cast::<T>())
}

#[cfg(windows)]
unsafe fn try_load_driver() -> Option<CudaDriverApi> {
    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryA(name: *const c_char) -> LibHandle;
        fn GetProcAddress(module: LibHandle, name: *const c_char) -> *mut c_void;
    }

    let lib = LoadLibraryA(c"nvcuda.dll".as_ptr());
    if lib.is_null() {
        return None;
    }

    unsafe fn require_sym(lib: LibHandle, name: &CStr) -> Option<*mut c_void> {
        let p = GetProcAddress(lib, name.as_ptr());
        if p.is_null() {
            None
        } else {
            Some(p)
        }
    }

    let init_p = require_sym(lib, c"cuInit")?;
    let count_p = require_sym(lib, c"cuDeviceGetCount")?;
    let get_p = require_sym(lib, c"cuDeviceGet")?;
    let name_p = require_sym(lib, c"cuDeviceGetName")?;
    let attr_p = require_sym(lib, c"cuDeviceGetAttribute")?;
    // Prefer v2 total-mem; fall back to legacy symbol on older drivers.
    let total_p = require_sym(lib, c"cuDeviceTotalMem_v2")
        .or_else(|| require_sym(lib, c"cuDeviceTotalMem"))?;

    let api = CudaDriverApi {
        _lib: lib,
        init: cast_fn(init_p),
        device_get_count: cast_fn(count_p),
        device_get: cast_fn(get_p),
        device_get_name: cast_fn(name_p),
        device_total_mem: cast_fn(total_p),
        device_get_attribute: cast_fn(attr_p),
    };
    let _ = (api.init)(0);
    Some(api)
}

#[cfg(not(windows))]
unsafe fn try_load_driver() -> Option<CudaDriverApi> {
    extern "C" {
        fn dlopen(filename: *const c_char, flag: c_int) -> LibHandle;
        fn dlsym(handle: LibHandle, symbol: *const c_char) -> *mut c_void;
    }
    const RTLD_NOW: c_int = 2;

    let lib = {
        let primary = dlopen(c"libcuda.so.1".as_ptr(), RTLD_NOW);
        if !primary.is_null() {
            primary
        } else {
            let fallback = dlopen(c"libcuda.so".as_ptr(), RTLD_NOW);
            if fallback.is_null() {
                return None;
            }
            fallback
        }
    };

    unsafe fn require_sym(lib: LibHandle, name: &CStr) -> Option<*mut c_void> {
        let p = dlsym(lib, name.as_ptr());
        if p.is_null() {
            None
        } else {
            Some(p)
        }
    }

    let init_p = require_sym(lib, c"cuInit")?;
    let count_p = require_sym(lib, c"cuDeviceGetCount")?;
    let get_p = require_sym(lib, c"cuDeviceGet")?;
    let name_p = require_sym(lib, c"cuDeviceGetName")?;
    let attr_p = require_sym(lib, c"cuDeviceGetAttribute")?;
    let total_p = require_sym(lib, c"cuDeviceTotalMem_v2")
        .or_else(|| require_sym(lib, c"cuDeviceTotalMem"))?;

    let api = CudaDriverApi {
        _lib: lib,
        init: cast_fn(init_p),
        device_get_count: cast_fn(count_p),
        device_get: cast_fn(get_p),
        device_get_name: cast_fn(name_p),
        device_total_mem: cast_fn(total_p),
        device_get_attribute: cast_fn(attr_p),
    };
    let _ = (api.init)(0);
    Some(api)
}

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

/// True when the CUDA driver library loads and reports ≥1 device.
pub fn cuda_driver_available() -> bool {
    load_driver().is_some_and(|api| {
        let mut count: c_int = 0;
        unsafe { (api.device_get_count)(&mut count) == CUDA_SUCCESS && count > 0 }
    })
}

/// Enumerate NVIDIA GPUs via the CUDA Driver API.
pub fn enumerate_cuda_devices() -> Result<Vec<MiningDevice>> {
    let api = load_driver().ok_or_else(|| {
        MinerError::Gpu("CUDA driver not available (install NVIDIA display driver / nvcuda)".into())
    })?;
    let mut count: c_int = 0;
    if unsafe { (api.device_get_count)(&mut count) } != CUDA_SUCCESS || count <= 0 {
        return Ok(Vec::new());
    }

    let mut out = Vec::with_capacity(count as usize);
    for index in 0..count {
        let mut dev: c_int = 0;
        if unsafe { (api.device_get)(&mut dev, index) } != CUDA_SUCCESS {
            continue;
        }
        // c_char is i8 on Windows, u8 on Linux — do not hardcode i8.
        let mut name_buf = [0 as c_char; 256];
        if unsafe { (api.device_get_name)(name_buf.as_mut_ptr(), 256, dev) } != CUDA_SUCCESS {
            continue;
        }
        let name = unsafe { CStr::from_ptr(name_buf.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let mut total_mem: usize = 0;
        let _ = unsafe { (api.device_total_mem)(&mut total_mem, dev) };
        let mut mp: c_int = 0;
        let _ = unsafe {
            (api.device_get_attribute)(&mut mp, CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT, dev)
        };

        out.push(MiningDevice {
            id: format!("cuda:{index}:{}", slug(&name)),
            backend: BackendId::GpuCuda,
            name,
            vendor: "NVIDIA".into(),
            index: index as usize,
            compute_units: if mp > 0 { Some(mp as u32) } else { None },
            global_memory_bytes: if total_mem > 0 {
                Some(total_mem as u64)
            } else {
                None
            },
            selected: true,
        });
    }
    Ok(out)
}
