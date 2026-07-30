# Dependency Security Review — 2026-07-30

Status: Dated Rust dependency review / open findings retained

This report records the commands, advisory snapshot, remediation, and remaining
maintenance risks observed on 2026-07-30. It is not an external audit or a
general security verdict. A later advisory-database update can change these
results without any repository change.

## Scope and tooling

Reviewed Cargo lockfiles:

- `Blockchain-prototype/Cargo.lock`;
- `Blockchain-prototype/alvenqis-desktop-v2/src-tauri/Cargo.lock`;
- `Blockchain-prototype/alvenqis-desktop-v2/native/keystore-helper/Cargo.lock`.

Tools and advisory data:

- `cargo-audit 0.22.2`;
- `cargo-deny 0.20.2`;
- RustSec database: 1,174 advisories, commit
  `bd347a52f842c6c696ffcaed55f7ad3568a58cb5`, updated
  `2026-07-30T21:13:51+02:00`.

## Routine workspace hygiene rerun

Commands run from `Blockchain-prototype/` on 2026-07-30:

```text
cargo audit
cargo deny check
```

Both commands exited 0. No dependency was upgraded during this routine rerun.

| Flag | Code area | Maintained newer version | Maintenance action |
|---|---|---|---|
| `paste 1.0.15`, RUSTSEC-2024-0436, unmaintained | Networking only in this graph: `netlink-packet-core` → `if-watch` → `libp2p-tcp`; not used by consensus or cryptographic application code. | No maintained newer `paste` release; replacement requires upstream movement or a reviewed substitute. | Track upstream; no direct version bump is available. |
| Duplicate `yamux 0.12.1` / `0.13.10` | Networking; the node's configured stream limit selects the `0.12.1` adapter path. | Yes: maintained/fixed `0.13.10` and newer `0.14` exist, but published `libp2p-yamux 0.47.0` cannot independently resolve its legacy adapter to them. | Version/backport work is needed under `ALV-NET-003`; intentionally not applied in this pass. |
| Duplicate `cpufeatures`, `getrandom`, `rand`, `rand_chacha`, and `rand_core` lines | Cryptographic/key-generation dependencies, with `cpufeatures` also below consensus hashing; the warning is version multiplicity, not a reported vulnerability. | Newer maintained lines already coexist in the lockfile. | Consolidate only through reviewed parent-crate upgrades; do not force versions across incompatible semver lines. |
| Duplicate `foldhash`, `hashbrown`, `hashlink`, `itertools`, `shlex`, `thiserror`, `thiserror-impl`, `unsigned-varint`, and `windows-sys` lines | General persistence, build, error-handling, platform, and networking support; no direct consensus or cryptographic implementation finding was reported by these checks. | Newer maintained lines already coexist in the lockfile. | Treat as transitive size/policy cleanup, not an emergency bump. |

`cargo deny` additionally emitted eight internal path-dependency wildcard
warnings, three unused license-allowance warnings, and one
`advisory-not-detected` warning because the Tauri-only glib exception does not
match the main workspace lockfile. These are configuration-hygiene warnings,
not newly flagged third-party vulnerabilities.

Every final `cargo audit` and `cargo deny check` command listed below exited
with code 0. Warnings and the scoped advisory exception remain documented
instead of being described as absent.

The RustSec-backed tools are not the only advisory source. A post-gate
cross-check of the GitHub Advisory Database and the repository's Dependabot
security-update jobs found one additional High-severity active networking
finding, `ALV-NET-003`, which is not present in the reviewed RustSec snapshot.
The exit-code-zero Cargo results therefore do not close the dependency review.

## Results by lockfile

| Lockfile | `cargo audit` | `cargo deny check` | Material result |
|---|---|---|---|
| Main Rust workspace | Exit 0; one unmaintained transitive warning (`paste 1.0.15`) | Exit 0; advisories, bans, licenses, and sources checks completed | The prior direct `rustls-pemfile 2.2.0` maintenance error was removed. GitHub's advisory database additionally identifies the active `yamux 0.12.1` path described by `ALV-NET-003`. Duplicate-version, internal path-version, and unmatched-license warnings remain policy cleanup work. |
| Tauri Control Center | Exit 0; 16 unmaintained warnings and one unsoundness warning | Exit 0 with one narrow, reasoned exception for RUSTSEC-2024-0429 | Tauri 2.11.5 still brings GTK3/glib 0.18 on Linux. `ALV-DESKTOP-001` remains open. |
| Native keystore helper | Exit 0; one unmaintained transitive warning (`paste 1.0.15`) | Exit 0; advisories, bans, licenses, and sources checks completed | The warning arrives through the node/libp2p Linux network-interface dependency path. Dependabot also reports that the inherited `yamux 0.12.1` path cannot be updated independently of `libp2p-yamux`. |

### Fixed during this review

`alvenqis-mining-pool` directly depended on the archived
`rustls-pemfile 2.2.0` and caused `cargo deny` to fail on
RUSTSEC-2025-0134. The Stratum TLS loader now uses the maintained
`rustls-pki-types::pem::PemObject` API directly. The existing TLS integration
test negotiates a verified connection and receives a JSON-RPC response after
the migration.

Evidence:

```text
cargo test -p alvenqis-mining-pool
result: 22 passed, 0 failed

cargo clippy -p alvenqis-mining-pool --all-targets -- -D warnings
result: exit 0

cargo deny --manifest-path Cargo.toml --config deny.toml check
result: advisories ok, bans ok, licenses ok, sources ok
```

The replacement API follows the rustls-pki-types
[`PemObject`](https://docs.rs/rustls-pki-types/latest/rustls_pki_types/pem/trait.PemObject.html)
documentation and the migration guidance in
[RUSTSEC-2025-0134](https://rustsec.org/advisories/RUSTSEC-2025-0134.html).

## Manual review of chain-critical dependency groups

| Area | Resolved packages reviewed | Maintenance and advisory observation |
|---|---|---|
| Signatures and secret handling | `ed25519-dalek 2.2.0`, `curve25519-dalek 4.1.3`, `subtle 2.6.1`, `zeroize 1.9.0` | The active ed25519 implementation moved into the maintained curve25519-dalek repository rather than being abandoned. The reviewed RustSec snapshot produced no error for these resolved versions. Secret-dependent comparison helpers remain provided by `subtle`; application-level usage still requires the separate cryptographic code review. |
| Hashes, derivation, and authentication | `blake3 1.8.5`, `sha2 0.10.9`, `hmac 0.12.1`, `bip39 2.2.2` | BLAKE3 1.8.5 is the current reviewed release in the [official implementation repository](https://github.com/BLAKE3-team/BLAKE3). The reviewed RustSec snapshot produced no error for these versions. Dependency status does not validate Alvenqis protocol composition or key-generation behavior. |
| P2P transport and identity | `libp2p-core 0.43.2`, `libp2p-identity 0.2.14`, `libp2p-gossipsub 0.49.4`, `libp2p-noise 0.46.1`, `libp2p-swarm 0.47.1`, `libp2p-tcp 0.44.1`, `libp2p-yamux 0.47.0`, `yamux 0.12.1` and `0.13.10`, `snow 0.9.6`, `x25519-dalek 2.0.1` | The [rust-libp2p project](https://github.com/libp2p/rust-libp2p/releases) remains actively released. RustSec produced no error, but GitHub Advisory Database cross-checking found the active High-severity `yamux 0.12.1` path in `ALV-NET-003`. `paste 1.0.15` remains an unmaintained transitive macro through `netlink-packet-core`; replacement depends on upstream network-stack movement. |
| Persistence | `rusqlite 0.39.0`, bundled SQLite through `libsqlite3-sys 0.37.0`, optional `rocksdb 0.24.0` | The reviewed RustSec snapshot produced no error for these versions. Node tests separately verify the minimum bundled SQLite version and WAL behavior; dependency review does not replace corruption, restore, or disk-failure tests. |
| TLS | `rustls 0.23.42`, `tokio-rustls 0.26.4`, `rustls-pki-types 1.15.0`, `aws-lc-rs 1.17.3` | The [rustls project](https://github.com/rustls/rustls) remains actively maintained. Direct PEM loading now uses rustls-pki-types. TLS configuration and certificate lifecycle remain separate operational review surfaces. |

## Open findings

### ALV-NET-003 — High

The resolved graph contains both `yamux 0.12.1` and `yamux 0.13.10` through
`libp2p-yamux 0.47.0`. GitHub
[GHSA-vxx9-2994-q338](https://github.com/advisories/GHSA-vxx9-2994-q338)
reports a remotely triggerable panic for `yamux < 0.13.10`.

This is an active code path, not only a stale lockfile entry:

- `alvenqis-node/src/p2p.rs` starts with `libp2p_yamux::Config::default()` and
  then calls `set_max_num_streams(MAX_STREAMS_PER_CONNECTION)`;
- `libp2p-yamux 0.47.0` defaults to its `yamux 0.13` adapter, but every
  configuration setter switches the adapter to `yamux 0.12`;
- `cargo tree -i yamux@0.12.1` traces the package through
  `libp2p-yamux 0.47.0` into the node and the native keystore helper;
- Dependabot update job `30536896042` reports latest resolvable `0.12.1` and
  earliest fixed `0.13.10`.

Closure requires a reviewed `libp2p-yamux` upgrade or compatible backport that
keeps the per-connection stream limit on a fixed Yamux implementation, a
malformed-frame regression test, fresh dependency scans, and multi-node P2P
tests. Removing the stream limit merely to select the newer default is not an
equivalent defense-in-depth fix.

### ALV-DESKTOP-001 — Medium

The Linux Tauri dependency graph includes `glib 0.18.5`; RustSec
[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)
identifies undefined behavior in `VariantStrIter`.

Current evidence and handling:

- Tauri is already resolved at 2.11.5, but its Linux WebKit/GTK stack still
  depends on GTK3/glib 0.18;
- repository and resolved Tauri, Tao, Wry, GTK, WebKitGTK, and Muda source
  searches found no call to `VariantStrIter` or `array_iter_str`; the symbol
  appears only in the glib crate's definition;
- `deny.toml` contains one reasoned advisory exception, following cargo-deny's
  [documented exception mechanism](https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html);
- the exception is not a technical fix and must be removed when an
  upstream-compatible GTK3 exit or compatible fix is available.

### Transitive maintenance warnings — Low

- `paste 1.0.15` is unmaintained and enters the main workspace and keystore
  helper through `netlink-packet-core` / `if-watch` / libp2p TCP.
- The Tauri Linux graph includes the archived GTK3 binding family plus
  `proc-macro-error` and several `unic-*` crates.
- Multiple-version and wildcard warnings from `cargo deny` are retained for
  follow-up. Internal path dependencies are source-local rather than registry
  wildcards, but explicit workspace versions would make policy intent clearer.

## Reproduction commands

From `Blockchain-prototype/`:

```text
cargo audit
cargo deny --manifest-path Cargo.toml --config deny.toml check

cd alvenqis-desktop-v2/src-tauri
cargo audit
cargo deny --manifest-path Cargo.toml --config ../../deny.toml check

cd ../native/keystore-helper
cargo audit
cargo deny --manifest-path Cargo.toml --config ../../../deny.toml check
```

## Follow-up

1. Remediate `ALV-NET-003` without dropping the bounded-stream control, and add
   GitHub Advisory Database coverage alongside RustSec in the dependency gate.
2. Add the three-lockfile audit matrix to CI with the scoped exception visible
   in logs.
3. Re-check the Tauri GTK3 dependency on every Tauri/Wry release and remove the
   exception as soon as a supported path exists.
4. Track the upstream removal of `paste` from the libp2p network-interface
   dependency path.
5. Complete the separate cryptographic implementation review, unsafe inventory,
   fuzzing pass, Node package audit, and artifact-signing verification.
