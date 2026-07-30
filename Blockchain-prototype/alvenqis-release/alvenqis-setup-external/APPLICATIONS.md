# Setup External Application Map

Status: Mainnet Candidate / Prototype

This directory packages the external-host applications, their role overlays,
health checks, configuration templates, and operator scripts. The legacy
`alvenqis-release/vps/` tree is not part of this package.

## Applications and release artifacts

| Application | Compose service or role | Candidate release artifact |
|---|---|---|
| Full validation node | `alvenqis-node`; role `node` | `alvenqis-node-<version>-linux-x86_64.tar.gz` |
| RPC gateway | `alvenqis-rpc`; role `rpc` | `alvenqis-rpc-gateway-<version>-linux-x86_64.tar.gz` |
| Indexer | `alvenqis-indexer`; role `indexer` | `alvenqis-indexer-<version>-linux-x86_64.tar.gz` |
| Explorer | `alvenqis-explorer`; role `explorer` | `alvenqis-explorer-<version>-linux-x86_64.tar.gz` |
| Pool coordinator | `alvenqis-pool`; role `pool` | `alvenqis-mining-pool-<version>-linux-x86_64.tar.gz` |
| Wallet CLI | not installed on VPS by default | `alvenqis-wallet-<version>-linux-x86_64.tar.gz` |
| CUDA miner | desktop/client only | `alvenqis-miner-<version>-linux-x86_64.tar.gz` |
| External Docker installer | all role overlays | `alvenqis-setup-external.tar.gz` |

Every artifact has a neighboring `.sha256` file. Candidate component builds
run as independent matrix entries with `fail-fast: false`; only a component
whose own tests and packaging succeed reaches the GitHub prerelease.

## Role installation

From this directory:

```bash
./scripts/install-docker-stack.sh --role node
./scripts/install-docker-stack.sh --role rpc
./scripts/install-docker-stack.sh --role indexer
./scripts/install-docker-stack.sh --role explorer
./scripts/install-docker-stack.sh --role pool
```

The current role graph includes local dependencies. For example, the `rpc`
role also starts its node, and the `explorer` role starts node, RPC, and
indexer. This produces a self-contained host role; it is not evidence that
every process can already be placed on a different host.

## Service connection and discovery boundary

Within one selected role, Docker Compose supplies service-name DNS on the
private `alvenqis-internal` network and health-gated startup ordering.
Cross-host endpoints and P2P seeds remain explicit configuration inputs.

Automatic cross-host service discovery, PeerId-pinned diverse P2P bootstrap,
and clean-host multi-machine proof are G2 work under `TM-409` and `TM-1211`.
They are not implemented or claimed by the G1 packaging and release changes.

## Source and deployment inputs

- `compose/roles.json`: allowed roles, overlays, and required services;
- `compose/*.yaml`: base and role overlays;
- `configs/` and `docker/templates/`: application configuration;
- `scripts/install-docker-stack.sh`: role installer;
- `scripts/compose.sh`: consistent Compose frontend;
- `scripts/validate-stack.sh`: static and Docker rendering checks;
- `DOCKER_DEPLOYMENT.md`: architecture and trust boundaries;
- `INSTALL_AND_UNINSTALL.md`: lifecycle commands;
- `MANUAL_UPGRADE.md`: immutable, checksum-verified upgrade process.
