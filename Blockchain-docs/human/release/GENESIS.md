# Mainnet Candidate Genesis

Status: Draft / Mainnet Candidate / Prototype — **Alvenqis Network rebrand freeze**

Canonical files:
- node config: `Blockchain-prototype/configs/mainnet-candidate.toml`
- genesis config: `Blockchain-prototype/configs/genesis.mainnet-candidate.toml`
- genesis review: `Blockchain-docs/human/release/GENESIS_REVIEW.mainnet-candidate.json`
- genesis approval: `Blockchain-docs/human/release/GENESIS_APPROVAL.mainnet-candidate.json`
- genesis block: `Blockchain-docs/human/release/genesis.mainnet-candidate.block.json`

## Deterministic genesis inputs

| Field | Value |
|---|---|
| network ID | `alvenqis-mainnet-candidate` |
| human name | `Alvenqis Mainnet Candidate` |
| address prefix (HRP) | `alve` |
| timestamp | `1720000000` |
| difficulty leading zero bits | `16` |
| recipient strategy | `default_miner_address` (fixed seed key `[7; 32]`) |
| PoW | FiroPoW 0.9.4 (AlvenqisPoW v1) |
| chain magic | `ALMC` (`414c4d43`) |

## Frozen outputs (rebrand genesis)

Resolved deterministic recipient address:
- `alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q`

**Important:** this is the bech32 encoding for the default miner public key (`PrivateKey::from_bytes([7; 32])`) under the **`alve`** HRP. It is **not** a string-rename of a legacy `alve1…` address.

Deterministic genesis hash (two independent `print-genesis-hash` runs matched):
- `0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5`

Pinned review hash:
- `3b245dfb0004603200d804996e24375728b12cf209a88ce61ced23e38e5a602c`

Approval note:
- `approved_by`: `alvenqis-rebrand-genesis-v1`
- repository artifact for Mainnet Candidate rehearsal only
- does **not** claim public mainnet launch or external independent verification

## Retired hashes (do not use)

| Hash | Why retired |
|---|---|
| `0000a26d…` | Pre-FiroPoW / Blake3 era |

## Safety rules

- Node refuses accidental Mainnet Candidate chain-root regeneration unless `--force-genesis` is passed explicitly.
- Node refuses startup when active genesis hash does not match the pinned approval record.
- Changing `Blockchain-prototype/configs/mainnet-candidate.toml` or
  `Blockchain-prototype/configs/genesis.mainnet-candidate.toml` requires
  regenerating review, approval, and block JSON.
- The fixed seed key `[7; 32]` is a deterministic development helper and is
  publicly reconstructible. Its candidate reward is not a final production
  allocation or custody design.
- The config currently retains a legacy `docs/release/...` approval path. That
  path must be reconciled and tested before the candidate startup workflow can
  be treated as release evidence.
- Legacy pre-Alvenqis chain data is archive-only; no balance migration.

## Operator workflow

```powershell
cd Blockchain-prototype
cargo run -p alvenqis-node --release -- --config configs/mainnet-candidate.toml print-genesis-hash
cargo run -p alvenqis-node --release -- --config configs/mainnet-candidate.toml export-genesis-review --output ../Blockchain-docs/human/release/GENESIS_REVIEW.mainnet-candidate.json
cargo run -p alvenqis-node --release -- --config configs/mainnet-candidate.toml approve-genesis --review-file ../Blockchain-docs/human/release/GENESIS_REVIEW.mainnet-candidate.json --approved-by <name> --output ../Blockchain-docs/human/release/GENESIS_APPROVAL.mainnet-candidate.json
cargo run -p alvenqis-node --release -- --config configs/mainnet-candidate.toml export-genesis-block --output ../Blockchain-docs/human/release/genesis.mainnet-candidate.block.json
cargo run -p alvenqis-node --release -- --config configs/mainnet-candidate.toml genesis-approval-status
```
