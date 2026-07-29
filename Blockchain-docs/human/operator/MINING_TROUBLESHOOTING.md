# Mining troubleshooting (Mainnet Candidate)

Alvenqis 1.0.0 mines FiroPoW 0.9.4 on supported NVIDIA CUDA GPUs only. The
product miner has no CPU, OpenCL, or host-emulated fallback.

## Startup before the first hashrate sample

The miner must construct and validate the epoch DAG in GPU memory before it can
search nonces. On the reference RTX 2060 this makes the first solo or pool
hashrate sample appear after roughly 15-20 seconds. During this phase the log
must progress through `building FiroPoW DAG in VRAM` and `FiroPoW DAG ready`.

If startup never completes:

1. Run `alvenqis-miner devices` and confirm the intended NVIDIA device appears.
2. Confirm the NVIDIA display driver is installed and the GPU has enough free
   VRAM for the reported DAG size.
3. Check that the release log says the CUDA FiroPoW kernels were linked. A
   release build must fail rather than create a stub miner.
4. Close other GPU-heavy applications, then restart the miner.

## Solo versus pool progress

Solo mining accepts only a full network-difficulty block. A healthy miner can
therefore run for a long time without finding a block. Pool mode supplies an
easier share target and reports accepted shares between blocks.

Recommended product setup:

1. Mode: `pool` for visible share progress **only when the VPS pool profile is
   intentionally enabled** (`ENABLE_POOL=true` + approved `POOL_ADDRESS`), or
   `solo` against a **local loopback** RPC.
2. **Solo against public RPC host alone will fail** while the public profile
   returns HTTP 410 for mining methods. Use local loopback RPC
   (`http://127.0.0.1:10787`) or remote Stratum TLS.
3. Pool endpoint when independently configured:
   `stratum+tls://<operator-stratum-host>:3333`.
4. Backend: `cuda`.
5. Restart mining after changing the source or selected CUDA devices.

The current control-plane gateway/RPC/public-smoke policy is internally
inconsistent. Do not expose public HTTP mining or treat it as supported until
`../security/KNOWN_LIMITATIONS.md` records closure evidence.

Never publish the internal RPC container directly or bypass the gateway boundary.

The Control Center writes the pool source as:

```toml
[source]
kind = "stratum"
host = "stratum.operator.example"
port = 3333
use_tls = true
skip_tls_verify = false
worker_name = "desktop-01"
```

Solo source (local loopback only):

```toml
[source]
kind = "rpc"
url = "http://127.0.0.1:10787"
```

## Metrics to watch

- `hashrate_hs` becomes greater than zero after DAG initialization.
- `work_source` identifies `pool` or `rpc`.
- `accepted_shares` increases in pool mode.
- `accepted_blocks` increases only for full network solutions.
- `share_difficulty_leading_zero_bits` is lower than network difficulty in
  normal pool operation.
- `last_error` remains empty.

## Consensus safety

Do not change difficulty-adjustment bounds to make an existing candidate chain
easier. Historical headers are validated using the frozen consensus rules; an
incompatible change can make stored chain data fail validation.

## Update checklist

1. Verify the downloaded release against `SHA256SUMS`.
2. Install the matching 1.0.0 Control Center and VPS control-plane artifacts.
3. Confirm `alvenqis-miner devices` reports CUDA and no CPU/OpenCL backend.
4. Start pool mining and confirm accepted shares and a non-zero hashrate.
