# Difficulty Adjustment

Status: Implemented / Mainnet Candidate

Alvenqis uses an LWMA-style next-difficulty rule over a 60-block window and a
60-second solve-time target. Solve times are bounded before weighting to limit
extreme timestamp effects. Each block must encode the exact next difficulty
derived from validated history.

The current target representation remains leading-zero bits. Mainnet Candidate
difficulty is bounded from 16 through 34 bits; changing these history-sensitive
bounds requires a reviewed chain reset or fork plan.

Validation also enforces an 11-block median-time-past floor, monotonic context,
and a 7,200-second maximum future drift. Explorer and miner estimates are
observational; core history determines the accepted next target.

The filename retains `_DRAFT` for link stability, not because the DAA is still
an unselected recommendation.
