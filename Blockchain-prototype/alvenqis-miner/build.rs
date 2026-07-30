//! Build CUDA FiroPoW library when `gpu-cuda` is enabled and `nvcc` + MSVC (`cl.exe`) are available.
//!
//! Release builds set `ALVENQIS_REQUIRE_CUDA=1` and fail unless the real CUDA
//! FiroPoW kernel can be compiled and linked. Stub builds are diagnostic only;
//! they can enumerate devices but cannot mine.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=kernels/firopow_cuda.cu");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CUDA_PATH");
    println!("cargo:rerun-if-env-changed=NVCC");
    println!("cargo:rerun-if-env-changed=ALVENQIS_FORCE_CUDA");
    println!("cargo:rerun-if-env-changed=ALVENQIS_REQUIRE_CUDA");
    println!("cargo:rerun-if-env-changed=ALVENQIS_CUDA_ARCH");
    println!("cargo:rustc-check-cfg=cfg(alvenqis_cuda_linked)");
    println!("cargo:rustc-check-cfg=cfg(alvenqis_cuda_stub)");

    let cuda_feature = env::var("CARGO_FEATURE_GPU_CUDA").is_ok();
    if !cuda_feature {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cu = manifest_dir.join("kernels").join("firopow_cuda.cu");
    if !cu.is_file() {
        require_cuda_or_continue("CUDA kernel source is missing");
        println!("cargo:warning=CUDA source missing at {}", cu.display());
        println!("cargo:rustc-cfg=alvenqis_cuda_stub");
        return;
    }

    // Ensure MSVC host compiler is visible to nvcc (required on Windows).
    ensure_msvc_on_path();

    let nvcc = find_nvcc();
    let Some(nvcc) = nvcc else {
        require_cuda_or_continue("nvcc was not found; install the CUDA Toolkit");
        println!("cargo:warning=nvcc not found - CUDA mining is unavailable (install CUDA Toolkit). Device enumeration may still work.");
        println!("cargo:rustc-cfg=alvenqis_cuda_stub");
        return;
    };

    if cfg!(windows) && which("cl").is_none() {
        require_cuda_or_continue("cl.exe was not found; nvcc needs the MSVC C++ toolchain");
        println!(
            "cargo:warning=cl.exe (MSVC) not on PATH - nvcc cannot compile device kernels. Open 'x64 Native Tools Command Prompt for VS' or install Visual Studio C++ build tools, then rebuild."
        );
        println!("cargo:rustc-cfg=alvenqis_cuda_stub");
        return;
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let lib_name = if target_os == "windows" {
        "alvenqis_pow_cuda.lib"
    } else {
        "libalvenqis_pow_cuda.a"
    };
    let obj_ext = if target_os == "windows" { "obj" } else { "o" };
    let obj = out_dir.join(format!("firopow_cuda.{obj_ext}"));
    let lib = out_dir.join(lib_name);

    let mut cmd = Command::new(&nvcc);
    cmd.arg("-O3")
        .arg("--compiler-options")
        .arg(if target_os == "windows" {
            "/MD"
        } else {
            "-fPIC"
        })
        .arg("-c")
        .arg(&cu)
        .arg("-o")
        .arg(&obj);

    // RTX 20xx = sm_75, RTX 30xx = sm_86, RTX 40xx = sm_89; PTX for forward compat.
    if let Ok(arch) = env::var("ALVENQIS_CUDA_ARCH") {
        cmd.arg(format!("-arch={arch}"));
    } else {
        cmd.arg("-gencode=arch=compute_75,code=sm_75");
        cmd.arg("-gencode=arch=compute_86,code=sm_86");
        cmd.arg("-gencode=arch=compute_89,code=sm_89");
        cmd.arg("-gencode=arch=compute_75,code=compute_75");
    }

    // Surface nvcc stderr on failure.
    let output = cmd.output();
    match output {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let tail = format!("{stdout}\n{stderr}");
            let short = tail.chars().rev().take(800).collect::<String>();
            let short: String = short.chars().rev().collect();
            require_cuda_or_continue(&format!("nvcc failed: {}", out.status));
            println!(
                "cargo:warning=nvcc failed ({}) - CUDA mining is unavailable. {}",
                out.status,
                short.replace('\n', " | ")
            );
            println!("cargo:rustc-cfg=alvenqis_cuda_stub");
            return;
        }
        Err(e) => {
            require_cuda_or_continue(&format!("failed to launch nvcc: {e}"));
            println!("cargo:warning=failed to run nvcc ({e}) - CUDA mining is unavailable");
            println!("cargo:rustc-cfg=alvenqis_cuda_stub");
            return;
        }
    }

    if target_os == "windows" {
        if let Ok(lib_exe) = find_lib_exe() {
            let _ = Command::new(lib_exe)
                .arg(format!("/OUT:{}", lib.display()))
                .arg(&obj)
                .status();
            if lib.is_file() {
                println!("cargo:rustc-link-search=native={}", out_dir.display());
                println!("cargo:rustc-link-lib=static=alvenqis_pow_cuda");
            } else {
                println!("cargo:rustc-link-arg={}", obj.display());
            }
        } else {
            println!("cargo:rustc-link-arg={}", obj.display());
        }
        println!("cargo:rustc-link-lib=cudart");
        if let Some(lib_dir) = find_cuda_lib_dir() {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
        }
    } else {
        let ar = env::var("AR").unwrap_or_else(|_| "ar".into());
        let _ = Command::new(ar).arg("crs").arg(&lib).arg(&obj).status();
        println!("cargo:rustc-link-search=native={}", out_dir.display());
        println!("cargo:rustc-link-lib=static=alvenqis_pow_cuda");
        // Keep the Linux miner portable: the NVIDIA display driver does not
        // necessarily install the CUDA runtime shared object used at build time.
        println!("cargo:rustc-link-lib=static=cudart_static");
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=rt");
        println!("cargo:rustc-link-lib=dylib=pthread");
        if let Some(lib_dir) = find_cuda_lib_dir() {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
        }
    }

    println!("cargo:rustc-cfg=alvenqis_cuda_linked");
    println!("cargo:warning=CUDA FiroPoW device kernels linked successfully");
}

fn require_cuda_or_continue(reason: &str) {
    if env::var("ALVENQIS_REQUIRE_CUDA").as_deref() == Ok("1") {
        panic!("CUDA release build requirement failed: {reason}");
    }
}

/// Prepend Visual Studio `cl.exe` directory to PATH when missing (Windows).
fn ensure_msvc_on_path() {
    if which("cl").is_some() {
        return;
    }
    if let Some(cl_dir) = find_msvc_bin_dir() {
        let path = env::var_os("PATH").unwrap_or_default();
        let mut prefix = vec![cl_dir.clone()];
        if let Some(parent) = cl_dir.parent() {
            prefix.push(parent.to_path_buf());
        }
        if let Ok(joined) = env::join_paths(prefix.into_iter().chain(env::split_paths(&path))) {
            env::set_var("PATH", joined);
        }
        if which("cl").is_some() {
            println!(
                "cargo:warning=Located MSVC cl.exe for nvcc at {}",
                cl_dir.display()
            );
        }
    }
}

fn find_msvc_bin_dir() -> Option<PathBuf> {
    // vswhere (VS 2017+)
    let vswhere =
        PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    if vswhere.is_file() {
        let out = Command::new(&vswhere)
            .args([
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-find",
                r"VC\Tools\MSVC\*\bin\Hostx64\x64\cl.exe",
            ])
            .output()
            .ok()?;
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let p = PathBuf::from(line.trim());
                if p.is_file() {
                    return p.parent().map(|d| d.to_path_buf());
                }
            }
        }
    }

    // Common install roots (newest first-ish).
    let roots = [
        r"C:\Program Files\Microsoft Visual Studio\2022\BuildTools",
        r"C:\Program Files\Microsoft Visual Studio\2022\Community",
        r"C:\Program Files\Microsoft Visual Studio\2022\Professional",
        r"C:\Program Files\Microsoft Visual Studio\2022\Enterprise",
        r"C:\Program Files\Microsoft Visual Studio\2019\BuildTools",
        r"C:\Program Files\Microsoft Visual Studio\2019\Community",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Community",
    ];
    for root in roots {
        let msvc = PathBuf::from(root).join(r"VC\Tools\MSVC");
        if let Ok(entries) = std::fs::read_dir(&msvc) {
            let mut versions: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            versions.sort();
            versions.reverse();
            for ver in versions {
                let cl = ver.join(r"bin\Hostx64\x64\cl.exe");
                if cl.is_file() {
                    return cl.parent().map(|d| d.to_path_buf());
                }
            }
        }
    }
    None
}

fn find_nvcc() -> Option<PathBuf> {
    if let Ok(p) = env::var("NVCC") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(cuda) = env::var("CUDA_PATH") {
        let cand =
            PathBuf::from(cuda)
                .join("bin")
                .join(if cfg!(windows) { "nvcc.exe" } else { "nvcc" });
        if cand.is_file() {
            return Some(cand);
        }
    }
    #[cfg(windows)]
    {
        let root = PathBuf::from(r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA");
        // Prefer highest version directory (v13.3 > v12.6 > …).
        if let Ok(entries) = std::fs::read_dir(&root) {
            let mut versions: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            versions.sort_by(|a, b| {
                let ka = version_key(a);
                let kb = version_key(b);
                kb.cmp(&ka)
            });
            for dir in versions {
                let cand = dir.join("bin").join("nvcc.exe");
                if cand.is_file() {
                    // Also set CUDA_PATH for lib discovery if unset.
                    if env::var_os("CUDA_PATH").is_none() {
                        env::set_var("CUDA_PATH", &dir);
                    }
                    return Some(cand);
                }
            }
        }
        // Explicit fallbacks
        for ver in [
            "v13.3", "v13.2", "v13.1", "v13.0", "v12.8", "v12.6", "v12.5", "v12.4", "v12.3",
            "v12.2", "v12.1", "v12.0", "v11.8",
        ] {
            let cand = root.join(ver).join("bin").join("nvcc.exe");
            if cand.is_file() {
                if env::var_os("CUDA_PATH").is_none() {
                    env::set_var("CUDA_PATH", root.join(ver));
                }
                return Some(cand);
            }
        }
    }
    let nvcc = which("nvcc")?;
    if env::var_os("CUDA_PATH").is_none() {
        if let Some(cuda_root) = nvcc.parent().and_then(Path::parent) {
            env::set_var("CUDA_PATH", cuda_root);
        }
    }
    Some(nvcc)
}

/// Sort key for CUDA dirs like `v13.3` / `v12.6`.
#[cfg(windows)]
fn version_key(path: &Path) -> (u32, u32) {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let name = name.trim_start_matches('v');
    let mut parts = name.split('.');
    let major = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor)
}

fn find_cuda_lib_dir() -> Option<PathBuf> {
    if let Ok(cuda) = env::var("CUDA_PATH") {
        let root = PathBuf::from(cuda);
        let candidates = if cfg!(windows) {
            vec![root.join("lib").join("x64")]
        } else {
            vec![
                root.join("targets").join("x86_64-linux").join("lib"),
                root.join("lib64"),
            ]
        };
        for candidate in candidates {
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }
    #[cfg(windows)]
    {
        let root = PathBuf::from(r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA");
        if let Ok(entries) = std::fs::read_dir(root) {
            let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            dirs.sort_by_key(|b| std::cmp::Reverse(version_key(b)));
            for e in dirs {
                let lib64 = e.join("lib").join("x64");
                if lib64.is_dir() {
                    return Some(lib64);
                }
            }
        }
    }
    None
}

fn find_lib_exe() -> Result<PathBuf, ()> {
    which("lib.exe").or_else(|| which("lib")).ok_or(())
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let cand = dir.join(bin);
        if cand.is_file() {
            return Some(cand);
        }
        #[cfg(windows)]
        {
            let cand = dir.join(format!("{bin}.exe"));
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}
