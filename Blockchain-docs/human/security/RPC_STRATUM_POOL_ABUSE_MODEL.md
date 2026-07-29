# RPC, Stratum, and Pool Abuse Model

Status: Draft / open security work

## Public RPC

Threats include oversized or expensive queries, cache amplification, write
spam, unauthenticated capability exposure, forged forwarding headers, detailed
peer-data leakage, stale index reads, and upstream failure masked as healthy.

Required controls:

- explicit route allowlist per profile;
- fail-closed write/mining authentication;
- body, query, concurrency, and rate bounds;
- trusted-proxy handling;
- aggregate-only public P2P telemetry;
- SQLite-aware index freshness;
- clear degraded health and no hidden fallback;
- load, abuse, and project-endpoint outage tests.

## Stratum TLS

Threats include plaintext downgrade, invalid certificate acceptance, worker
identity spoofing, job replay, duplicate shares, low-difficulty floods, stale
jobs, connection exhaustion, and certificate-renewal failure.

Required controls:

- verified TLS for remote clients;
- no silent plaintext fallback;
- bounded handshakes, connections, frames, jobs, and share rate;
- job identity, expiry, duplicate detection, and difficulty validation;
- provider-neutral certificate input and renewal monitoring.

## Pool and payouts

Threats include share-accounting manipulation, reorged rewards, payout
double-confirmation, operator key compromise, worker/IP Sybil admission,
single-coordinator failure, and unauthorized admin mutation.

Required controls:

- core recomputation of network-valid work;
- integer weighted PPLNS with deterministic largest-remainder allocation;
- canonical reorg re-check and clawback before payment;
- transactional production persistence;
- idempotent payout states and on-chain confirmation;
- offline/HSM signing and no private key in the coordinator;
- multi-instance admission, distributed abuse controls, and independent
  operation evidence.

Current controls do not satisfy the complete production abuse model.
