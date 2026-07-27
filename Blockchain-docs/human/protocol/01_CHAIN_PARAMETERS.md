# Alvenqis Chain Parameters

Status: Implemented / Mainnet Candidate

| Parameter | Current value |
|---|---:|
| Network ID | `alvenqis-mainnet-candidate` |
| Chain magic | `ALMC` (`414c4d43`) |
| Address HRP | `ALVE` |
| RPC / P2P port | `10787` / `20787` |
| State model | Account based |
| PoW | FiroPoW 0.9.4, period length 1 |
| Difficulty target model | Leading-zero bits |
| DAA | LWMA, 60-block window |
| Candidate difficulty range | 16 through 34 leading-zero bits |
| Block target | 60 seconds |
| Maximum future timestamp drift | 7,200 seconds |
| Median-time-past window | 11 blocks |
| Maximum transactions per block | 1,024 including coinbase |
| Maximum transaction wire size | 16,384 bytes |
| Decimals | 8 |
| Atomic units per ALVE | 100,000,000 |
| Maximum-supply cap | 60,000,000 ALVE |
| Halving interval | 1,576,800 blocks |
| Initial block reward | 19.02587519 ALVE |
| Initial/minimum base fee | 1 atomic unit |
| Base-fee max-change denominator | 8 |
| Target non-coinbase transactions per block | 1 |
| Signatures | ed25519 |
| Address encoding | canonical lowercase Bech32m |

Devnet and Testnet constants remain internal test profiles. Changing candidate
genesis, PoW, difficulty history bounds, network identity, or serialized
consensus fields is a fork and requires explicit consensus review.
