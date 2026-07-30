# Unsafe Code Inventory — 2026-07-30

Status: Point-in-time source inventory

Scope: repository-owned Rust source in the 10 members of the primary Cargo
workspace at `Blockchain-prototype/Cargo.toml`. Build output, generated code,
third-party dependencies, and the separately managed Tauri and keystore-helper
workspaces are excluded.

Method: `cargo metadata --format-version 1 --no-deps` established workspace
membership, followed by source searches for explicit `unsafe { ... }` blocks,
`unsafe impl`, `unsafe fn`, and `unsafe extern` function-pointer declarations.
Each result was reviewed with its surrounding source. Configuration-gated
Windows and non-Windows definitions are counted separately because both are
repository-owned safety boundaries.

Summary: 34 explicit unsafe blocks and 14 additional unsafe constructs were
found. Of the 48 total entries, 8 have an adequate explanatory safety comment,
3 have a partial comment, and 37 have no local safety justification. An
explanatory comment is considered adequate only when it records both the reason
unsafe is necessary and the invariants that make the operation sound.

| File/line | Kind | What it does | Justification comment |
|---|---|---|---|
| `Blockchain-prototype/alvenqis-browser/host/src/confirm.rs:157` | Block | Calls the Windows `MessageBoxW` FFI with a null owner and pointers to temporary NUL-terminated UTF-16 buffers. | **Yes** — line 156 identifies the FFI contract, null owner, and NUL-terminated inputs. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:98` | `unsafe impl Send` | Permits moving a backend containing raw CUDA miner handles between threads. | **Partial** — line 96 says handles are used by one mining thread in the product path, but does not establish that all safe API use preserves handle ownership, lifetime, and CUDA thread-affinity requirements. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:122` | Block | Destroys each non-null opaque CUDA miner handle during `Drop`. | **No** — no local comment states handle validity, unique ownership, or why destruction is called exactly once. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:133` | Block | Calls CUDA FFI to check whether device kernels are linked and a CUDA device is available. | **No** — no local safety justification for the FFI calls. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:145` | Block | Calls CUDA FFI to obtain the device count. | **No** — no local safety justification for the FFI call. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:157` | Block | Passes a mutable device-info record to CUDA FFI for initialization. | **No** — no local comment records layout compatibility, pointer validity, or initialization guarantees. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:203` | Block | Creates an opaque CUDA miner handle for a selected device. | **No** — no local comment records the returned-pointer ownership and lifetime contract. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:237` | Block | Calls CUDA FFI on a scoped worker to build a GPU DAG from process-owned light-cache and L1 pointers. | **No** — no local comment explains pointer lifetimes, per-device handle exclusivity, or cross-thread validity. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:274` | Block | Copies one DAG item from a CUDA miner into a 128-byte Rust output buffer. | **No** — no local comment records required output length, handle validity, or write bounds. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:328` | Block | Invokes the CUDA mining kernel with input pointers and multiple mutable output pointers from a scoped worker. | **No** — no local comment records input/output sizes, lifetimes, non-aliasing, or per-handle concurrency invariants. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda.rs:546` | Block | Calls CUDA FFI to check runtime device availability. | **No** — no local safety justification for the FFI call. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:16` | `unsafe extern` function-pointer type | Defines the Windows CUDA Driver API function-pointer ABI as `extern "system"`. | **Partial** — line 13 explains the ABI selection, but not the validity and call-safety contract for dynamically loaded symbols. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:20` | `unsafe extern` function-pointer type | Defines the non-Windows CUDA Driver API function-pointer ABI as `extern "C"`. | **Partial** — line 13 explains the ABI selection, but not the validity and call-safety contract for dynamically loaded symbols. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:45` | `unsafe impl Send` | Permits moving the loaded library handle and CUDA function pointers between threads. | **Yes** — line 44 states that the symbols are process-global and thread-safe after `cuInit`; the API struct retains the library handle. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:46` | `unsafe impl Sync` | Permits sharing the loaded library handle and CUDA function pointers between threads. | **Yes** — line 44 states that the symbols are process-global and thread-safe after `cuInit`; the API struct retains the library handle. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:51` | Block | Calls the unsafe dynamic-loader routine once and stores its result in `OnceLock`. | **No** — no local comment states how the loader's symbol, ABI, and library-lifetime requirements are satisfied. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:56` | `unsafe fn` | Reinterprets a raw dynamic-library symbol pointer as a requested function-pointer type by copying its bits. | **Yes** — lines 54–59 state the valid-symbol/type/width preconditions and why a pointer-bit copy is used. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:63` | `unsafe fn` | On Windows, loads `nvcuda.dll`, resolves required CUDA symbols, casts them to typed function pointers, and calls `cuInit`. | **No** — no function-level safety contract explains library lifetime, exact symbol signatures, ABI matching, or caller obligations. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:75` | `unsafe fn` | On Windows, calls `GetProcAddress` and converts a null result into `None`. | **No** — no safety contract states the validity requirements for the module handle and symbol-name pointer. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:107` | `unsafe fn` | On non-Windows systems, loads `libcuda`, resolves required CUDA symbols, casts them to typed function pointers, and calls `cuInit`. | **No** — no function-level safety contract explains library lifetime, exact symbol signatures, ABI matching, or caller obligations. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:127` | `unsafe fn` | On non-Windows systems, calls `dlsym` and converts a null result into `None`. | **No** — no safety contract states the validity requirements for the library handle and symbol-name pointer. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:173` | Block | Calls the dynamically loaded `cuDeviceGetCount` pointer for an availability check. | **No** — no local comment states symbol validity, initialization state, or output-pointer validity. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:183` | Block | Calls the dynamically loaded `cuDeviceGetCount` pointer before enumeration. | **No** — no local comment states symbol validity, initialization state, or output-pointer validity. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:190` | Block | Calls `cuDeviceGet` through a loaded function pointer to obtain a device handle. | **No** — no local safety justification for the dynamic call or output pointer. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:195` | Block | Calls `cuDeviceGetName` to write at most 256 bytes into a stack buffer. | **No** — no local comment records the buffer size contract or termination guarantee. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:198` | Block | Interprets the CUDA-populated name buffer as a NUL-terminated C string. | **No** — the nearby `c_char` portability note does not establish that a NUL occurs within the 256-byte buffer. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:202` | Block | Calls `cuDeviceTotalMem` through a loaded function pointer to populate a `usize`. | **No** — no local safety justification for the symbol signature or output pointer. |
| `Blockchain-prototype/alvenqis-miner/src/backends/cuda_driver.rs:204` | Block | Calls `cuDeviceGetAttribute` through a loaded function pointer to obtain multiprocessor count. | **No** — no local safety justification for the symbol signature or output pointer. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:143` | Block | Calls vendored native Keccak-256 with a Rust slice pointer and a 32-byte output buffer. | **No** — no local comment records the native length/write contract and buffer validity. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:152` | Block | Calls native FiroPoW epoch-number calculation with a converted block height. | **No** — no local safety justification for the FFI call. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:157` | Block | Calls native FiroPoW to read the epoch length. | **No** — no local safety justification for the FFI call. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:245` | Block | Calls native FiroPoW hashing with a 32-byte header pointer and mutable `NativeResult` output. | **No** — no local comment records C layout, pointer lengths, output initialization, or write bounds. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:272` | Block | Calls native FiroPoW verification with header, mix-hash, and boundary pointers. | **No** — no local comment records the three required 32-byte input lengths and lifetimes. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:296` | Block | Calls native single-threaded light search with input pointers and mutable nonce/result outputs. | **No** — no local comment records pointer lengths, output validity, or native write guarantees. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:343` | Block | Calls native multithreaded search with hash/boundary pointers, optional cancellation pointer, and mutable outputs. | **No** — no local comment explains the `AtomicI32`/`c_int` pointer representation assumption, lifetimes, or concurrent native access contract. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:376` | Block | Calls native code to prewarm the full epoch DAG. | **No** — no local safety justification for the FFI call or native global-state behavior. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:413` | `unsafe impl Send` | Permits moving a view containing raw light-cache and L1 pointers between threads. | **Yes** — line 412 states that the referenced native epoch context is process-global and immutable after creation. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:414` | `unsafe impl Sync` | Permits sharing a view containing raw light-cache and L1 pointers between threads. | **Yes** — line 412 states that the referenced native epoch context is process-global and immutable after creation. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:423` | Block | Calls native code to export process-owned light-cache and L1 pointers plus their sizes. | **No** — later null/size checks validate results, but no local comment records lifetime, alignment, initialization, and native ownership guarantees. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:457` | Block | Calls native code to write one 128-byte DAG item into a Rust array. | **No** — no local comment records the fixed output-size/write-bounds contract. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:471` | `unsafe impl Send` | Permits moving a view containing raw full-DAG and L1 pointers between threads. | **Yes** — lines 387–389 and 470 state that the native epoch context is process-static and immutable after materialization. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:472` | `unsafe impl Sync` | Permits sharing a view containing raw full-DAG and L1 pointers between threads. | **Yes** — lines 387–389 and 470 state that the native epoch context is process-static and immutable after materialization. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:481` | Block | Calls native code to materialize a full DAG and export process-owned DAG/L1 pointers and sizes. | **No** — later null/size checks validate results, but no local comment records lifetime, alignment, initialization, and native ownership guarantees. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:508` | Block | Calls native code to query the epoch's full-dataset byte size. | **No** — no local safety justification for the FFI call. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:633` | Block | Calls native code to write a revision string into a 16-byte `c_char` buffer. | **No** — the nearby `c_char` portability note does not record the native write bound or termination contract. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:635` | Block | Calls native code to read the FiroPoW period length during availability validation. | **No** — no local safety justification for the FFI call. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:677` | Test block | Calls native code to assert the compiled FiroPoW period length. | **No** — no local safety justification for the test FFI call. |
| `Blockchain-prototype/alvenqis-core/src/firopow.rs:678` | Test block | Calls native code to assert the compiled FiroPoW epoch length. | **No** — no local safety justification for the test FFI call. |

Every row marked **No** lacks the requested justification comment. Rows marked
**Partial** also need stronger documented invariants before they satisfy the
stated standard. This report does not assert that an operation is unsound merely
because its justification is undocumented; it records the documentation gap for
follow-up review.

The two separately managed Rust workspaces were sanity-scanned but are not
included in the counts above. They contain additional unsafe constructs at
`alvenqis-desktop-v2/native/keystore-helper/src/windows_dialog.rs:48`,
`:83`, and `:202`;
`alvenqis-desktop-v2/native/keystore-helper/src/main.rs:1173` and `:1263`;
and `alvenqis-desktop-v2/src-tauri/src/process.rs:107`. Those require a separate
workspace inventory if their scope is added later.
