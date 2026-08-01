# Rust dependency hygiene report — 2026-08-01

Scope: the main `Blockchain-prototype` Cargo workspace and its separate
`fuzz` workspace. This is a dated engineering check, not a security
certification.

## Commands and results

| Check | Result |
|---|---|
| `cargo audit` from `Blockchain-prototype/` | Exit 0. The current RustSec database contained 1,177 advisories. One allowed maintenance warning remains for transitive `paste 1.0.15` (`RUSTSEC-2024-0436`). |
| `cargo deny check` from `Blockchain-prototype/` | Exit 0: advisories, bans, licenses, and sources policies completed. Non-failing warnings remain for internal path dependencies without explicit versions, duplicate transitive versions, and one stale unmatched advisory exception. |
| `cargo tree -i event-listener@5.4.2` | `event-listener 5.4.2` is selected through `async-channel` and `libp2p-gossipsub`; the prior `5.4.1` lock entry was removed. |
| `cargo tree -i yamux@0.12.1` | No matching package in the main workspace. |
| `cargo tree -i yamux@0.13.10` | The tracked `third-party/libp2p-yamux` backport resolves only `yamux 0.13.10`. |
| `cargo tree --manifest-path fuzz/Cargo.toml --locked -p alvenqis-fuzz` | Initially exposed both `yamux 0.12.1` and `0.13.10`. After the fuzz workspace inherited the tracked backport, only `yamux 0.13.10` remains. |
| `cargo check --manifest-path fuzz/Cargo.toml --locked --bins` | Exit 0 after the fuzz lockfile correction. |
| `cargo test --locked -p libp2p-yamux -p alvenqis-node` | Exit 0: Yamux malformed-frame regression 1/1; node unit tests 55/55; node devnet tests 40/40. |

## Flagged or changed dependencies

| Dependency | Code area reached | Maintenance / advisory status | Action |
|---|---|---|---|
| `event-listener 5.4.1` | Networking through `async-channel` → `libp2p-gossipsub`; not a direct consensus or cryptographic dependency. | `RUSTSEC-2026-0221` identifies the affected release; maintained `5.4.2` is available. | Lockfile updated to `5.4.2`; targeted node, RPC, admin, and P2P tests plus clippy/check passed. |
| `yamux 0.12.1` | Network stream multiplexing. It was absent from the main and native helper graphs but remained in the separately locked fuzz graph. | GHSA-vxx9-2994-q338 fixes the remotely triggerable panic in `0.13.10`. Published `libp2p-yamux 0.47.0` still carries the old adapter, so the repository uses a compatibility-preserving tracked backport. | The fuzz workspace now applies the same backport and resolves only `yamux 0.13.10`. Keep an explicit dependency-tree assertion because the current RustSec scan did not surface this GitHub advisory. |
| `paste 1.0.15` | Networking through `netlink-packet-core` → `if-watch` → `libp2p-tcp`; not used directly by application consensus or cryptographic code. | `RUSTSEC-2024-0436`: unmaintained. There is no maintained newer `paste` release to bump to. | Remains open pending an upstream network-stack replacement or a separately reviewed substitute. |

No dependency touching application consensus or cryptographic primitives was
flagged by this RustSec run. That statement is limited to the advisory snapshot
and commands above; it does not replace source review or future scans.

## Follow-up

- Track removal of `paste` through upstream `netlink-packet-core` / `if-watch`
  movement.
- Retain the Yamux malformed-frame regression and explicit no-`0.12.1` tree
  check in release verification.
- Clean up non-failing `cargo deny` wildcard/duplicate warnings separately;
  they did not fail the current policy run.
