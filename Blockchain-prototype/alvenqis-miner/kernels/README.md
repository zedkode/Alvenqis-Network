# CUDA FiroPoW kernel

`firopow_cuda.cu` is the only product mining kernel. It implements Alvenqis's
FiroPoW 0.9.4 nonce search and builds the epoch DAG directly in NVIDIA VRAM.

Release builds require the NVIDIA CUDA Toolkit (`nvcc`) and fail when the
kernel cannot be compiled or linked. Diagnostic stub builds may enumerate a
CUDA device, but they cannot search nonces and never fall back to host mining.

Consensus validation remains in `alvenqis-core` and revalidates GPU solutions on
the host. That validation is not a mining backend.
