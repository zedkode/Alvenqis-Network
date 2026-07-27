# Candidate chain reset — FiroPoW 0.10.0

> **Historical record — not current operator guidance.** This reset record is
> preserved as chain-history evidence. Current protocol and genesis sources are
> `../protocol/06_CONSENSUS_POW.md` and `GENESIS.md`.

**Reason:** Consensus PoW changed from Blake3 leading-zero-bits to **FiroPoW 0.9.4** (AlvenqisPoW v1). All prior blocks, balances, and pool state are invalid.

## New genesis pin

| Field | Value |
|-------|--------|
| Deterministic genesis hash | `0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5` |
| Review hash | `751e18b949e408119505cee9150739ce8f35db179d73a9e17b06c4df0e3cbe08` |
| Difficulty at genesis | 16 leading-zero bits on FiroPoW final hash |
| Recipient | `alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q` |

## Operator steps

1. Deploy Linux release binaries (`alvenqis-node`, `alvenqis-rpc-gateway`, `alvenqis-indexer`, `alvenqis-mining-pool`) built with FiroPoW native linkage.
2. Deploy updated genesis approval/review JSON under `/etc/alvenqis` and `/opt/alvenqis/docs/release`.
3. **Preferred (no re-mine on VPS):** import pre-mined genesis from a dev machine:

```bash
# Dev (once):
alvenqis-node --config configs/mainnet-candidate.toml export-genesis-block \
  --output docs/release/genesis.mainnet-candidate.block.json

# VPS:
systemctl stop alvenqis-node alvenqis-rpc alvenqis-mining-pool
# from /opt/alvenqis working dir, with approval files in place:
alvenqis-node --config /etc/alvenqis/node.toml \
  --data-dir /var/lib/alvenqis/.alvenqis-mainnet/chain \
  import-genesis-block --genesis-file /path/to/genesis.mainnet-candidate.block.json --force
systemctl start alvenqis-node alvenqis-rpc alvenqis-mining-pool
```

4. Confirm:

```bash
curl -sS https://rpcnode.dohotstudio.com/status
# height 0, tip_hash = 0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5
```

5. Mine only on **GPU PCs** (`auto` / `cuda` / `gpu`). VPS has no GPU miner.

**Note:** Live VPS was already initialized with the FiroPoW genesis above (one-shot force-genesis). Further resets should prefer **import-genesis-block**.

## Local wipe (Windows)

```
%LOCALAPPDATA%\Alvenqis\ControlCenter\.alvenqis-local\{chain,mempool,indexer,node}
```

Wallet addresses can be kept; balances restart at zero on the new chain.
