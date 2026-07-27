# Mining Pool Protocol

Status: Draft / Mainnet Candidate / Prototype

Alvenqis pooled mining is an off-chain coordination protocol. It does not modify FiroPoW 0.9.4, block headers, network difficulty, coinbase validity, supply, fees or fork choice.

## Work Flow

1. The pool requests a canonical `alvenqis-mining-v1` template paying a dedicated pool address.
2. A miner requests that job with its payout address and worker name.
3. The pool returns the canonical template plus a worker-specific `share_difficulty_leading_zero_bits`.
4. The miner searches the same nonce space and submits hashes meeting the lower share target.
5. The pool recomputes the hash and validates identity, job lifetime, duplicate status and share difficulty.
6. Only a share satisfying the real network difficulty is forwarded to `/mining/submit`.
7. Node consensus validation remains the final authority for block acceptance.

Lower share difficulty proves contributed work but cannot create a valid block. Miner addresses receive accounting credit; the block coinbase pays the pool address.

## Variable Difficulty And Admission

- VarDiff adjusts one leading-zero bit at a time from recent accepted-share timing.
- Minimum, maximum, target interval and observation window are operator-configured and bounded by network difficulty.
- Every difficulty issued to a worker remains valid until that job expires, preventing valid in-flight work from being rejected during an adjustment.
- The pool credits the highest issued difficulty actually satisfied by the hash.
- Worker count, work requests, share requests and invalid-share bans are bounded in the application. When deployed behind the supplied loopback reverse proxy, the immediate trusted proxy supplies the original client IP.

## PPLNS Accounting

- Scheme: Pay Per Last N Shares.
- All calculations use integer atomic units and `u128` intermediates.
- Shares are weighted by proven work (`2^leading_zero_bits`), so different VarDiff targets receive proportional PPLNS credit.
- Pool fee is configured in basis points and deducted once.
- Integer remainder is assigned deterministically in canonical address order.
- Allocations become mature only after the configured confirmation count and canonical-hash verification.
- Orphaned blocks remove immature allocations.

## Payout Boundary

The coordinator prepares payout batches and moves balances from mature to pending. An unsigned prepared batch can be cancelled to restore mature balances. A separate wallet or HSM must sign and submit payout transactions. The coordinator records unique transaction hashes only after operator confirmation. No pool private key is accepted by the public service.

Current implementation is a single-coordinator prototype with atomic JSON persistence and process-local admission controls. Public operation still requires a production database, reorg-safe chain APIs, distributed rate controls, DDoS protection, signer isolation, audit and multi-host soak tests.
