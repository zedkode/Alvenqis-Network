# Rust dependency hygiene report — 2026-08-08

Scope: the complete `Blockchain-prototype` Cargo workspace after adding the
project-operated Pingora edge gateway. This is a dated engineering snapshot,
not a security certification.

## Checks performed

| Command | Evidence |
|---|---|
| `cargo audit` | Exit 0 after scanning 520 locked crates against 1,190 RustSec advisories. It reported two explicitly allowed unmaintained warnings and no disallowed advisory. |
| `cargo deny check` | Exit 0: `advisories ok, bans ok, licenses ok, sources ok`. Non-failing duplicate, internal path wildcard, unmatched license, and stale unmatched advisory-exception warnings remain. |
| `cargo tree -i derivative@2.2.0` | Confirms the warning is introduced only through pinned `pingora-core`. |
| `cargo tree -i paste@1.0.15` | Confirms the existing warning remains in the networking path `netlink-packet-core -> if-watch -> libp2p-tcp`. |
| `cargo tree -p alvenqis-pingora-gateway --depth 2` | Confirms the gateway uses the exact official Pingora revision `402acae52ff29c4183b9eca55ffa3f77814a5ee0`, `prometheus 0.14.0`, and maintained `futures-util 0.3.32`; the rejected `protobuf 2.28.0` graph is absent. |

## Findings and maintenance decisions

| Dependency | Sensitive code area reached | Maintenance / known-advisory status | Maintained newer version and action |
|---|---|---|---|
| `derivative 2.2.0` | Networking/project edge through `pingora-core`; it is not used by Alvenqis consensus or cryptographic primitives. | `RUSTSEC-2024-0388`, unmaintained. The warning is allowed and visible, not treated as resolved. | No maintained release in the same crate line. Track upstream Pingora removal/replacement; do not silently patch the pinned source. |
| `paste 1.0.15` | P2P networking through `netlink-packet-core -> if-watch -> libp2p-tcp`; not a direct consensus or cryptographic dependency. | `RUSTSEC-2024-0436`, unmaintained. | No maintained newer `paste` release exists. Track the upstream network-stack path. |
| official Pingora Git revision `402acae...` | Project edge routing, TLS/mTLS, limits, DNS, health and metrics. | Exact immutable source is allowed in `deny.toml`; crates.io `0.8.1` was not selected because its earlier Prometheus/protobuf graph triggered `RUSTSEC-2024-0437`. | A newer source may exist, but no version bump is authorized in this maintenance pass. Re-audit any proposed revision before changing it. |
| `futures-util 0.3.32` | Concurrent bounded upstream health probes in the Pingora gateway; no consensus or cryptographic use. | Maintained release, no finding in this scan. | No bump needed. |

## Open maintenance work

- Keep both allowed unmaintained warnings visible in every release audit.
- Review a future released Pingora version only after its resolved graph passes
  the same advisory, license and source-policy checks.
- Clean up non-failing duplicate and internal path wildcard warnings as a
  separate dependency-maintenance task; they did not fail current policy.
