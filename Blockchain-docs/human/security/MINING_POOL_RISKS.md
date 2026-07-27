# Mining Pool Security Risks

Status: Draft / Mainnet Candidate / Prototype

Implemented baseline controls include strict address/worker validation, duplicate and stale-share rejection, canonical FiroPoW hash recomputation, bounded request bodies, per-worker and trusted-proxy client request limits, active worker caps, temporary invalid-share bans and external-file admin authentication.

The admission state is process-local. Multiple coordinators require a shared limiter and ban store before they can safely operate behind one public endpoint. Reverse-proxy rate limiting and DDoS protection remain mandatory.

Primary risks include forged or replayed shares, flooding, payout rounding defects, chain reorganizations, signing-key compromise, storage rollback, upstream template compromise, block withholding, pool concentration and DDoS.

Implemented mitigations include hash recomputation, address validation, bounded request bodies, duplicate rejection, expiring jobs, checked integer accounting, maturity delay, canonical block-hash checks, atomic persistence and separation of payout signing from the public service.

Production remains blocked until multi-instance shared admission, full transactional storage (beyond atomic JSON), backup drills, HSM/offline signer integration, TLS/DDoS testing and independent review are complete.

## Prototype guards (2026-07-17)

- Non-loopback bind requires `allow_public_pool_prototype = true` plus HTTPS `public_url`.
- Immature blocks are orphaned early when the upstream canonical hash at that height diverges.
- Tip rewind voids immature pool blocks above the new tip (`void_immature_above_tip`).
- Pool state JSON is written with atomic replace + fsync.
- Admission bans remain process-local (still not multi-coordinator safe).
