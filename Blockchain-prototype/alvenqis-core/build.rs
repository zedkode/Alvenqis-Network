//! Compile vendored FiroPoW 0.9.4 (ProgPoW period=1) C/C++ sources.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let native = manifest.join("native");
    let pp = native.join("crypto").join("progpow");

    println!("cargo:rerun-if-changed=native/crypto/progpow");
    println!("cargo:rustc-check-cfg=cfg(alvenqis_firopow_native)");

    // C sources (must not be compiled as C++ — noexcept mismatch).
    let mut c = cc::Build::new();
    c.include(&native)
        .include(pp.join("include"))
        .include(pp.join("lib"))
        .file(pp.join("lib/ethash/primes.c"))
        .file(pp.join("lib/keccak/keccak.c"))
        .file(pp.join("lib/keccak/keccakf1600.c"))
        .file(pp.join("lib/keccak/keccakf800.c"))
        .warnings(false);
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        c.flag("-O3");
    }
    c.compile("alvenqis_firopow_c");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .include(&native)
        .include(pp.join("include"))
        .include(pp.join("lib"))
        .file(pp.join("lib/ethash/ethash.cpp"))
        .file(pp.join("lib/ethash/progpow.cpp"))
        .file(pp.join("alvenqis_firopow_ffi.cpp"))
        .warnings(false);

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let is_android = target_os == "android";
    let is_msvc = target_env == "msvc";

    if is_msvc {
        build.flag("/EHsc");
        build.flag("/O2");
    } else {
        build.flag("-O3");
        build.flag("-fexceptions");
        // Android Bionic has no separate libpthread; -pthread would pull -lpthread.
        if !is_android {
            build.flag("-pthread");
        }
    }

    build.compile("alvenqis_firopow");
    println!("cargo:rustc-cfg=alvenqis_firopow_native");
    println!("cargo:rustc-link-lib=static=alvenqis_firopow");
    println!("cargo:rustc-link-lib=static=alvenqis_firopow_c");
    if !is_msvc {
        if is_android {
            // NDK C++ runtime (libc++). Bionic has no libpthread / libstdc++.
            println!("cargo:rustc-link-lib=dylib=c++_shared");
        } else {
            println!("cargo:rustc-link-lib=dylib=stdc++");
            println!("cargo:rustc-link-lib=dylib=pthread");
        }
    }
}
