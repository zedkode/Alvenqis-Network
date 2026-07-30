# Alvenqis FiroPoW CUDA mining

Status: **Implemented / Mainnet Candidate**

## Runtime policy

- `cuda` is the only product backend.
- Legacy configuration aliases `auto` and `gpu` migrate to `cuda`.
- `cpu`, `cpu-gpu`, `hybrid`, and `opencl` are rejected.
- No failure path searches nonces on the host.
- Nodes still verify every submitted solution with canonical host FiroPoW.

## CUDA implementation

| Layer | Implementation |
|---|---|
| Device discovery | CUDA runtime plus CUDA Driver API |
| DAG construction | CUDA kernel, directly in VRAM from the canonical light cache |
| Nonce search | `kernels/firopow_cuda.cu` |
| Multi-GPU | Exact contiguous nonce partitions, one concurrent dispatch per selected GPU |
| Metrics | Actual completed hashes returned by the CUDA kernel |
| Startup validation | GPU DAG items 0, middle, and last compared with canonical core output |
| Solution validation | `alvenqis-core` recomputes `mix_hash` and `final_hash` before submission |

Work buffers and the VRAM DAG persist across batches. They are not allocated for
every dispatch. Startup reports `building_dag` so the Control Center does not
misrepresent DAG initialization as a zero hashrate mining stall.

## Build and runtime requirements

Windows release builds require:

- NVIDIA CUDA Toolkit with `nvcc`;
- MSVC C++ build tools (`cl.exe`);
- an NVIDIA driver compatible with the linked CUDA runtime.

Linux release builds require:

- NVIDIA CUDA Toolkit with `nvcc`;
- a C++ compiler and static archive tools;
- NVIDIA driver support on the mining machine;
- Tauri desktop packaging dependencies for the desktop bundles.

The current DAG is approximately 1.5 GiB and grows by epoch rules. Available
VRAM must also cover the L1 cache and work buffers.

## Solo and pool behavior

Solo mode requests canonical templates from the RPC gateway and submits complete
FiroPoW solutions. Pool mode uses the pool share target and submits shares with
the same canonical nonce, mix hash, and final hash fields. Both modes use the
same CUDA engine; neither has a CPU mining path.

The Alvenqis Setup External runs node, RPC, indexer, and pool services. It does not
run the CUDA desktop miner unless the host explicitly has a supported NVIDIA
GPU, which is not part of the standard VPS bundle.
