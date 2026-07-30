# Linux Component and Desktop Build Evidence — 2026-07-30

Status: Local G1 packaging evidence / `TM-1110` remains In Progress

## Scope

This report records local Linux build, test, and packaging evidence for the
independently gated candidate-component work. It does not record a GitHub
Release, Windows signing, a clean-host installation, VPS deployment, or
cross-host discovery.

Repository base commit:
`1e964a24a335bd97d83c3d73a711568cc3b59a29`, plus the working-tree fixes
listed below. The generated archives are ignored local build output, so this is
not same-SHA immutable GitHub Actions evidence.

Environment:

- Linux `7.1.3-2-cachyos`, x86_64;
- Rust/Cargo `1.97.1`;
- CUDA toolkit `13.3`, `nvcc` `13.3.73`;
- NVIDIA GeForce RTX 4070 Laptop GPU, driver `610.43.03`;
- Node.js `24.13.0`, npm `11.6.2`;
- WebKitGTK `4.1` development files available.

## Component archives

The following components passed their component-specific tests, release build,
archive creation, and SHA-256 verification:

| Component | Local artifact | Relevant evidence |
|---|---|---|
| Full node | `alvenqis-node-1.0.0-linux-x86_64.tar.gz` | RocksDB feature build; 59 library and 40 Devnet tests passed after the tests were corrected to load the active chain-backed mempool |
| RPC gateway | `alvenqis-rpc-gateway-1.0.0-linux-x86_64.tar.gz` | 23 RPC tests passed, including HTTP 410 for public-submit `/mining/*` |
| Indexer | `alvenqis-indexer-1.0.0-linux-x86_64.tar.gz` | 2 unit and 12 integration tests passed |
| Explorer | `alvenqis-explorer-1.0.0-linux-x86_64.tar.gz` | `npm ci`, lint, and production build passed |
| Mining pool | `alvenqis-mining-pool-1.0.0-linux-x86_64.tar.gz` | 22 tests passed |
| Wallet CLI | `alvenqis-wallet-1.0.0-linux-x86_64.tar.gz` | 14 tests passed |
| CUDA miner | `alvenqis-miner-1.0.0-linux-x86_64.tar.gz` | 5 unit, 4 miner, and 6 parity tests passed; GPU/core parity test ran on the detected RTX 4070 |

All seven neighboring `.sha256` files passed `sha256sum --check`.

## Control Center packages

Command:

```text
PATH="/opt/cuda/bin:$PATH" CUDA_PATH=/opt/cuda NVCC_CCBIN=/usr/bin/g++-15 \
  bash Blockchain-scripts/release/build-linux-desktop.sh
```

The build compiled and linked the real CUDA FiroPoW kernels, staged the node,
RPC, indexer, miner, native keystore helper, candidate configs, operator
scripts, and explorer assets, then produced:

| Package | Bytes | SHA-256 |
|---|---:|---|
| `Alvenqis Control Center V2_2.0.1_amd64.AppImage` | 130,357,752 | `8eedbd89c65d57bce48f4fb261fd335dfc15c0b57ac030f0f31e9770f8f51e9e` |
| `Alvenqis Control Center V2_2.0.1_amd64.deb` | 28,701,002 | `4767999fe21e70e64f879df76a289eb841147f9b3dd2cd9fec31fca8463e597a` |
| `Alvenqis Control Center V2-2.0.1-1.x86_64.rpm` | 28,702,548 | `b61e4e37c482241ffed1f5594ddd95409ad3d395a34d46a135e0b4a50491416f` |

`sha256sum --check SHA256SUMS` passed for all three. The DEB and RPM content
listings confirmed that the expected sidecars and resources are installed with
the desktop application. The AppImage was identified as an x86-64 ELF
executable and passed its recorded checksum; its SquashFS content was not
independently listed because `unsquashfs` was unavailable.

The desktop frontend separately passed 5 test files / 19 tests, lint,
TypeScript compilation, and its Vite production build.

## Workspace verification

The following commands passed with CUDA available:

```text
cargo fmt --manifest-path Blockchain-prototype/Cargo.toml --all --check
cargo test --manifest-path Blockchain-prototype/Cargo.toml --workspace --locked
cargo clippy --manifest-path Blockchain-prototype/Cargo.toml \
  --workspace --all-targets --locked -- -D warnings
```

The workspace test run recorded 306 passed tests and one explicitly ignored
live-RPC smoke test. Clippy completed with warnings denied.

Fresh `cargo audit` and `cargo deny check` runs also exited 0 for the main
workspace, Tauri desktop, and native keystore lockfiles. Their allowed
maintenance warnings and open findings remain documented in
`Blockchain-docs/human/security/DEPENDENCY_AUDIT_2026-07-30.md`; in particular,
the separate High-severity `ALV-NET-003` Yamux path remains a G1 blocker.

## Working-tree fixes found by the build

- `alvenqis-miner/build.rs` now tells Cargo to rerun when
  `ALVENQIS_REQUIRE_CUDA` changes. This prevents a cached diagnostic stub from
  satisfying a later release-required CUDA build.
- RocksDB Devnet tests now read pending transactions through the same
  chain-backed mempool API used by that feature configuration.
- the Linux desktop packaging script restores the tracked source manifest after
  packaging, including on failure, so platform-specific timestamps do not leave
  unrelated source changes.

## Remaining evidence

`TM-1110` remains In Progress. Completion still requires:

- passing GitHub workflows and uploaded component assets from one immutable
  candidate tag and commit;
- Windows installer and portable archive evidence, including configured signing;
- publisher signature verification in addition to SHA-256 files;
- Docker-backed Setup External build/render evidence on a suitable host;
- resolution of the G1 dependency and release blockers tracked elsewhere.

Automatic cross-host discovery and clean-host multi-machine installation remain
G2 work and were not started.
