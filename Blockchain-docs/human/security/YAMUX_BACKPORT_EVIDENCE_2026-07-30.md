# Yamux Compatibility Backport Evidence — 2026-07-30

Status: Local remediation verified / immutable GitHub verification pending

This report records the defensive networking change for `ALV-NET-003`. It is
not a general security verdict and does not close other P2P findings.

## Finding

Published `libp2p-yamux 0.47.0` switched from its Yamux 0.13 adapter to Yamux
0.12 when any configuration setter was called. Alvenqis calls
`set_max_num_streams(32)` to bound streams per connection, which made the
affected `yamux 0.12.1` path in GHSA-vxx9-2994-q338 / CVE-2026-32314 active.

Removing the stream cap was not accepted as a remediation because it would
discard an independent resource bound.

## Implemented compatibility backport

Commit `80dc72b5e54a0ac61fd5dd64b4222082f645e1da`:

- patches `libp2p-yamux 0.47.0` in the main, native-helper, and standalone fuzz
  workspaces;
- uses the single-Yamux adapter structure from upstream development commit
  `343f1491126c599b84ebcc17862bfa54c140b9f1`;
- remains compatible with the current `libp2p-core 0.43` family;
- pins the only Yamux implementation to fixed `=0.13.10`;
- preserves `set_max_num_streams(32)`;
- documents source provenance, license, and removal criteria in
  `Blockchain-prototype/third-party/libp2p-yamux/UPSTREAM.md`.

The backport is temporary. It must be removed after a reviewed published
rust-libp2p family upgrade provides equivalent fixed behavior.

## Local verification

Commands were run on 2026-07-30/31 against the commit above.

| Check | Result |
|---|---|
| `cargo tree -i yamux@0.12.1 --workspace --locked` in `Blockchain-prototype/` | No matching package; only `yamux 0.13.10` resolves. |
| `cargo tree -i yamux@0.12.1 --locked` in the native keystore helper | No matching package; only `yamux 0.13.10` resolves. |
| `cargo tree -i yamux@0.12.1 --locked` in the standalone fuzz workspace | No matching package; only `yamux 0.13.10` resolves. |
| `cargo check --bins --locked` in the standalone fuzz workspace | Exit 0. |
| Bounded `cargo fuzz` runs for block, transaction, and P2P message decoding | Completed without a panic, hang, or crash artifact. |
| `cargo test -p libp2p-yamux oversized_first_data_syn_is_rejected_without_panic` | Passed; the advisory-shaped oversized first `Data|SYN` frame was rejected without panic. |
| `cargo test -p alvenqis-node p2p::tests::two_nodes_connect_and_sync_a_direct_chain_extension` | Passed. |
| `cargo test -p alvenqis-node p2p::tests::two_divergent_nodes_reorg_to_the_higher_work_branch` | Passed. |
| `cargo test --workspace --locked` | 307 passed, 0 failed, 1 live-RPC test ignored. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0. |
| `cargo test --locked` in the native keystore helper | 10 passed, 0 failed. |
| `cargo check --locked` in the Tauri workspace | Exit 0. |
| `cargo audit` and `cargo deny check` in the main, helper, and fuzz workspaces | Exit 0; the documented `paste 1.0.15`, duplicate-version, wildcard-path, and unused-policy warnings remain. |

## Remaining evidence

- Complete the GitHub checks on the immutable commit containing this report.
- Re-run the Dependabot security update after the report is published and
  record whether GHSA-vxx9-2994-q338 is cleared.
- Retain the regression test and 32-stream bound during future libp2p upgrades.
