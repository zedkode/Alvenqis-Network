# Component Candidate Releases

Status: Mainnet Candidate / Prototype

Alvenqis candidate tags publish desktop, server-component, and external setup
assets through independent GitHub Actions workflows. Passing artifacts are
available from the repository [Releases page](https://github.com/zedkode/Alvenqis-Network/releases).

## Release asset map

| Component | Linux candidate asset | Build workflow |
|---|---|---|
| Control Center | AppImage, Debian package, RPM | `candidate-linux-release.yml` |
| Control Center for Windows | signed NSIS installer and portable ZIP | `candidate-windows-release.yml` |
| Full node | `alvenqis-node-<version>-linux-x86_64.tar.gz` | `candidate-linux-components-release.yml` |
| RPC gateway | `alvenqis-rpc-gateway-<version>-linux-x86_64.tar.gz` | `candidate-linux-components-release.yml` |
| Indexer | `alvenqis-indexer-<version>-linux-x86_64.tar.gz` | `candidate-linux-components-release.yml` |
| Explorer | `alvenqis-explorer-<version>-linux-x86_64.tar.gz` | `candidate-linux-components-release.yml` |
| Pool coordinator | `alvenqis-mining-pool-<version>-linux-x86_64.tar.gz` | `candidate-linux-components-release.yml` |
| Wallet CLI | `alvenqis-wallet-<version>-linux-x86_64.tar.gz` | `candidate-linux-components-release.yml` |
| CUDA miner | `alvenqis-miner-<version>-linux-x86_64.tar.gz` | `candidate-linux-components-release.yml` |
| External Docker setup | `alvenqis-setup-external.tar.gz` | `candidate-setup-external-release.yml` |

Every component archive has a neighboring SHA-256 file. The repository source
archives attached automatically by GitHub remain the source snapshot for the
tag; binary and web bundles above are the tested component downloads.

## Independent failure rule

The component matrix uses `fail-fast: false`. Each entry runs its own tests,
build, packaging, checksum, workflow-artifact upload, and GitHub Release upload.
A failed entry remains visible as failed and does not cancel successful matrix
entries. Desktop and Setup External workflows are separate for the same reason.

This rule does not turn an unsuccessful component into a successful release:
no archive is uploaded for an entry whose tests or packaging failed.

Setup External container images follow the same isolation rule. The
`setup-external-images.yml` matrix publishes immutable GHCR tags for
`alvenqis-runtime`, `alvenqis-ops`, `alvenqis-backup-scheduler`,
`alvenqis-explorer`, `alvenqis-website`, `alvenqis-gateway`, and
`alvenqis-metrics-exporter`. A failed image build does not cancel passing image
jobs, and an existing immutable image tag is never replaced. Shared stack
validation remains a separate visible job: its failure keeps the workflow red
but does not cancel an image that passed its own build and publication steps.

The shared prerelease notes identify the exact tagged commit and enumerate the
committed changes since the preceding reachable candidate tag. They do not use
the local working tree as changelog input.

## CUDA requirement

The Linux desktop and standalone miner release paths set
`ALVENQIS_REQUIRE_CUDA=1`. Cargo reruns the miner build script when this policy
changes and refuses a release build unless `nvcc` compiles and links the real
FiroPoW CUDA kernel. Diagnostic stub builds are not candidate artifacts.

The 2026-07-30 local Linux build evidence, including component checksums and
CUDA/core parity on a detected NVIDIA GPU, is recorded in
`Blockchain-docs/human/engineering/LINUX_COMPONENT_BUILD_EVIDENCE_2026-07-30.md`.
It is local evidence and does not replace immutable GitHub workflow results.

## Deployment and discovery boundary

`Blockchain-prototype/alvenqis-release/alvenqis-setup-external/` contains the
role overlays, installer, configs, health checks, and application map. Services
inside one role use Docker service-name DNS and health-gated startup.

Automatic cross-host service discovery, diverse PeerId-pinned P2P bootstrap,
and clean-host multi-machine evidence remain G2 work. The presence of release
archives is not evidence that those later-gate criteria are complete.
