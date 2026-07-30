# Alvenqis parser fuzz targets

These `cargo-fuzz` targets exercise repository-owned JSON decode paths that
accept untrusted or persisted input:

- `decode_block`: `alvenqis_core::Block` plus non-PoW structural operations;
- `decode_transaction`: `alvenqis_core::Transaction` plus shape, address,
  signature, and canonical-encoding operations;
- `decode_p2p_message`: node sync request/response, handshake, and mining
  presence message types used by libp2p.

Run a bounded local pass from `Blockchain-prototype`:

```bash
cargo fuzz run decode_block fuzz/seeds/decode_block -- -max_total_time=60 -timeout=5 -max_len=1048576
cargo fuzz run decode_transaction fuzz/seeds/decode_transaction -- -max_total_time=60 -timeout=5 -max_len=1048576
cargo fuzz run decode_p2p_message fuzz/seeds/decode_p2p_message -- -max_total_time=60 -timeout=5 -max_len=1048576
```

Use a temporary copy of each seed directory when running locally if generated
corpus entries should not be written into the tracked seed set.
