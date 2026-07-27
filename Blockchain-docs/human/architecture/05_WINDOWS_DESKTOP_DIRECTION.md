# Windows Desktop Direction

Status: Draft / Planned / Prototype

Related direction:
- broader cross-platform planning is tracked in `docs/architecture/06_CLIENT_PLATFORM_DIRECTION.md`.

## Goal

Alvenqis should aim for a single Windows desktop application for end users instead of forcing them to operate multiple separate tools by hand.

## Product Rule

User-facing direction:
- one Alvenqis desktop app;
- one primary Windows entrypoint;
- one consistent interface for wallet, node, mining and local chain visibility.

Internal engineering direction:
- wallet, miner, node, RPC and indexing concerns stay modular internally;
- the desktop app orchestrates those modules instead of merging all logic into one fragile code path;
- failures in one module should degrade gracefully without corrupting wallet state or chain state.

## Expected Desktop Areas

The future Windows desktop app should eventually expose:
- Overview;
- Wallet;
- Mining;
- Node Status;
- Transactions;
- Logs;
- Settings.

## User Experience Requirement

Windows users should not need:
- many separate terminal windows;
- manual low-level command knowledge for normal use;
- multiple unrelated apps just to create a wallet and mine locally.

Instead, the desktop app should provide:
- wallet creation and import;
- address and balance visibility;
- node start and stop controls;
- mining start and stop controls;
- mine-to-active-wallet behavior;
- recent rewards, recent transactions and logs;
- clear network labels such as Draft, Private Devnet, Testnet Candidate or Mainnet Candidate.

## Architecture Rule

The correct long-term shape is:
- unified UX for users;
- separated modules internally.

This means:
- the wallet UI is not the miner engine;
- the miner engine is not the node state store;
- the desktop shell coordinates those components through stable interfaces.

## Current Limitation

This direction is accepted as a product requirement, but the current repository still provides:
- CLI wallet;
- CLI node;
- local RPC;
- local indexer;
- local explorer prototype.

There is not yet a production-style Windows GUI wallet or integrated desktop miner application.

## Implementation Guidance

Future research and implementation should prefer:
- one Windows desktop app as the public user surface;
- modular internal services and domain boundaries;
- reuse of existing Rust core, node and wallet logic where possible;
- honest status labels until the desktop app is implemented and testable.
