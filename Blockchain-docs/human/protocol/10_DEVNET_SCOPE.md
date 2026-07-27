# Internal Devnet and Test Profiles

Status: Internal test profiles / not product networks

Devnet and Testnet remain available for automated tests, low-difficulty protocol
experiments, reset/recovery exercises, and incompatible consensus development.
Normal product/operator flows use Mainnet Candidate.

## Isolation

| Profile | Network ID | HRP | Data root | RPC | P2P | Resettable |
|---|---|---|---|---:|---:|---:|
| Devnet | `alvenqis-devnet` | `dalve` | `.alvenqis-dev` | 8787 | 18787 | Yes |
| Testnet fixture | `alvenqis-testnet` | `talve` | `.alvenqis-testnet` | 9787 | 19787 | No product reset workflow |
| Mainnet Candidate | `alvenqis-mainnet-candidate` | `alve` | `.alvenqis-mainnet` | 10787 | 20787 | No |

Cross-network blocks, addresses, P2P handshakes, and storage roots must be
rejected. Devnet reset is destructive and requires explicit confirmation plus
the documented backup behavior.

The older phase-by-phase devnet narrative is preserved only in project memory.
It must not be used to claim that P2P, RPC, wallet, indexer, or explorer remain
unimplemented in the current workspace.
