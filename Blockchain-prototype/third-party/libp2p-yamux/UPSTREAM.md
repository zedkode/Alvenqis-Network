# libp2p-yamux security backport

Status: temporary reviewed dependency backport

This package keeps the public `libp2p-yamux 0.47.0` interface used by the
Alvenqis node while backporting the single-Yamux adapter structure from
upstream `libp2p-yamux 0.48.0` development commit
`343f1491126c599b84ebcc17862bfa54c140b9f1`.

The crates.io `0.47.0` adapter defaults to `yamux 0.13`, but every
configuration setter switches to its compatibility dependency on
`yamux 0.12.1`. Alvenqis must call `set_max_num_streams(32)` as a
defense-in-depth limit, so the vulnerable compatibility path is active.

This backport:

- depends only on patched `yamux = 0.13.10`;
- exposes `set_max_num_streams` against that implementation;
- remains compatible with the current `libp2p-core 0.43` dependency family;
- includes a regression test for the oversized first `Data|SYN` frame in
  GHSA-vxx9-2994-q338 / CVE-2026-32314;
- retains the upstream MIT license and notices.

Remove this package and the workspace `[patch.crates-io]` entry after a
reviewed published rust-libp2p family upgrade provides the same fixed behavior.
