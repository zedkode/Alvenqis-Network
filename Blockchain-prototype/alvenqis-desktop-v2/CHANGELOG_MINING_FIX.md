# Mining / Stratum fix + UX pass — changelog

Root cause investigation was done against the full monorepo
(`github.com/zedkode/Alvenqis-Network`), which contains the
`alvenqis-miner` sidecar source (CUDA backend + stratum client) that isn't
part of this desktop repo. That confirmed the miner binary itself already
implements solo/pool/stratum correctly — the bugs were all in this desktop
shell's settings persistence and stat widgets, not in the mining protocol.

## Root causes found

1. **Stratum could never be saved as the default mode.**
   - `shared/types.ts` typed `default_miner_mode` as `"solo" | "pool"` only.
   - `src-tauri/src/settings.rs::update()` silently forced any other value
     back to `"solo"`.
   - `src/pages/Mining.tsx` actively converted `mode === "stratum" ? "solo" : mode`
     before persisting, and the initial-state fallback on a fresh session
     only ever resolved to `"solo"` or `"pool"`.
   - Net effect: you could mine over Stratum for the current session, but it
     always reverted to Solo/Pool on the next app launch, and the Settings
     page didn't even expose Stratum as a profile option.

2. **Stratum was invisible in the mining stat widgets even while running.**
   - "Accepted" showed `acceptedBlocks` (effectively always ~0) instead of
     `acceptedShares` for Stratum, same as it correctly did for Pool.
   - "Miners online" and "Network H/s" never folded in your own local
     hashrate/activity when `mode === "stratum"`, only for `"pool"`.
   - Net effect: Stratum mining could be fully functional under the hood
     and still look dead in the UI.

3. **Solo/"local" mining friction.** The Mining RPC URL used for Solo mode
   (`mining_rpc_url`) is a separate setting from the general node RPC URL,
   and only the general RPC field had a "use local" quick button — the one
   that actually matters for solo mining had none.

## Fixes

- `src-tauri/src/settings.rs`: `default_miner_mode` now validates
  `solo | pool | stratum`.
- `shared/types.ts`, `shared/settingsDefaults.ts`: widened/annotated as a
  first-class persisted mode.
- `src/pages/Settings.tsx`:
  - Mining profile selector is now a real three-way Solo / Pool / Stratum
    control (with matching Wallet / Share2 / Plug icons), not a
    Solo/Pool toggle with Stratum missing.
  - Added a durable "Default Stratum endpoint" panel (host, port, TLS,
    skip-verify, password) — previously stratum config only lived in
    per-session storage.
  - Added a "Use local 127.0.0.1:10787" quick button for the **Mining**
    RPC URL specifically (solo work source), next to an explanation that
    it's separate from the general RPC endpoint above it.
- `src/pages/Mining.tsx`:
  - Removed the `stratum → solo` persistence workaround; mode now saves
    exactly as selected.
  - `accepted`, "miners online", and "network H/s" now treat Stratum the
    same as Pool (share-based, folds in local hashrate/activity) instead
    of falling through to the Solo/block-based branch.
  - Mode tabs now carry the same Wallet / Share2 / Plug icons as Settings.
- `src/shared/errorPrediction.ts`: minor copy fix (Stratum framed as an
  equal option, not a stopgap until "Pool mode" — worded as a fallback
  between equals).
- Small shared CSS additions: `.field-stack` / `.field-stack-title` (used
  for the new Stratum defaults card and to consistently style Mining.tsx's
  existing Stratum panel, which had no dedicated styling before), smoother
  hover/active transitions on the mode/segmented selectors.

## Verified

- `npx tsc --noEmit` — clean.
- `npm run build` (`tsc && vite build`) — clean production build.
- Rust side reviewed manually against `src-tauri/src/settings.rs`,
  `process.rs`, and the upstream `alvenqis-miner` source; no Rust
  toolchain was available in this environment to run `cargo check`, so
  please run it once on your machine before shipping.

## Suggested next pass

The existing design system (tokens.css, glass panels, cyan/gold glow
accents, MiningCore matrix-rain hero) is already a deliberate, mature
visual identity — not a generic template. This pass focused on making
Solo/Pool/Stratum genuinely equal citizens end-to-end and fixing the
widgets that were misreporting Stratum activity, plus consistency touches
on the shared mode-selector and stratum-panel styling.

If you want a deeper visual pass next (specific pages, a new color
direction, bespoke layouts for Overview/Wallet/Analytics, etc.), point me
at which pages/widgets matter most and I'll go deep on those rather than
spreading thin across all ~15 pages at once.
