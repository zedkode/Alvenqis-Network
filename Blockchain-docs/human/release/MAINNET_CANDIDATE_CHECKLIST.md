# Mainnet Candidate Checklist (G2)

Status: **Draft / Mainnet Candidate / Prototype — not public Mainnet**

> Completing this checklist authorizes **operator rehearsal** only.  
> Public Mainnet requires the G4 criteria in `docs/release/NETWORK_MATURITY.md`.  
> Local software hygiene is G1 (`docs/release/RELEASE_GATE.md`).

Scope of this checklist:
- core chain
- PoW mining
- persistent node
- wallet CLI
- RPC
- indexer
- explorer
- release scripts

Minimum readiness gates:
- deterministic genesis config exists under `configs/genesis.mainnet-candidate.toml`;
- genesis review manifest exists under `docs/release/GENESIS_REVIEW.mainnet-candidate.json`;
- genesis approval record exists under `docs/release/GENESIS_APPROVAL.mainnet-candidate.json`;
- candidate node config exists under `configs/mainnet-candidate.toml`;
- candidate RPC config exists under `configs/rpc.mainnet-candidate.toml`;
- startup requires `allow_mainnet_candidate = true`;
- startup requires a matching pinned genesis approval record;
- candidate startup validates the stored genesis hash against the active config;
- candidate startup validates the stored genesis hash against the active approval record review hash as well;
- candidate chain validation rebuilds full state from genesis before reporting ready;
- reset is refused for Mainnet Candidate;
- Devnet reset requires explicit confirmation and local backup;
- candidate chain data stays under `.alvenqis-mainnet/`;
- mempool capacity is bounded by config and duplicate transaction hashes are rejected;
- secret, repository-hygiene and config-safety scanners pass;
- release gate documentation exists and can be run locally;
- RPC binds to localhost by default unless explicit public opt-in is set;
- wallet material stays outside tracked source folders;
- RPC exposes only read or submit endpoints needed for the local candidate flow;
- RPC application profiles prevent VPS deployments from registering mining and detailed P2P operator routes;
- Ubuntu VPS services run in constrained Docker containers with fixed non-root runtime users, memory and CPU limits;
- public VPS traffic terminates at Caddy or Cloudflare Tunnel while raw RPC and control ports remain private;
- multi-VPS bootstrapping uses explicit libp2p seed multiaddresses and does not create a privileged consensus server;
- explorer reads only RPC and indexer data and keeps Draft / Prototype labels visible;
- release scripts run format, test, clippy and explorer build gates;
- forbidden-file checks block `.env`, keys, seeds, wallet files, runtime data and obvious secret patterns.
- local operator scripts exist for start, stop, status, backup, reset, mining and smoke-test flows under `scripts/local/`;
- local operator data stays under `.alvenqis-local/` for rehearsal mode before any VPS deployment;
- local operator runbook and troubleshooting docs exist under `docs/operator/`.

Still required before any live public launch claim (G4 — detail in `NETWORK_MATURITY.md`):
- independent genesis verification beyond the current repository draft approval;
- public seed deployment and multi-host soak testing of the existing encrypted P2P transport;
- header-first synchronization, fork choice, reorg handling and peer scoring/bans operational maturity;
- production storage review;
- production RPC abuse testing, monitoring and incident response;
- reorg-aware multi-node mempool and indexer behavior;
- external security review;
- explicit go-live decision recorded in project memory and public docs.

**Until G4 is complete, keep all product labels on Mainnet Candidate / Prototype.**
