# Client Platform Direction

Status: Draft / Planned / Prototype

## Goal

Alvenqis should be designed for a broader client surface than one Windows-only product.

The long-term direction should cover:
- Windows desktop;
- Linux desktop;
- Linux CLI;
- Android;
- iOS.

## Product Principle

Alvenqis should expose a unified user experience per platform while keeping core, wallet, node, mining, RPC and indexing responsibilities modular internally.

This means:
- users should not be forced to assemble many disconnected tools for normal workflows;
- platform-specific apps should reuse shared protocol and wallet rules;
- internal modules should still remain separated so failures in one area do not corrupt another.

## Platform Roles

### Windows Desktop

Primary direction:
- one unified desktop app;
- wallet, node control, mining, logs and status in one place;
- suitable for local operator and local mining workflows.

### Linux Desktop

Primary direction:
- one unified desktop app with the same core product shape as Windows where practical;
- wallet, node control, mining, logs and status in one place;
- suitable for users and operators who prefer Linux as a primary workstation.

### Linux CLI

Primary direction:
- first-class CLI support must remain part of the product strategy;
- CLI is important for operators, automation, servers, packaging and troubleshooting;
- Linux CLI should remain viable even after desktop GUIs exist.

### Android

Primary direction:
- wallet visibility;
- balance and transaction history;
- signing and submission where safe and supported;
- viewing node, miner and network status;
- remote control of nodes or miners in other locations once secure operator auth exists.

Current rule:
- Android should not be assumed to mine locally.

### iOS

Primary direction:
- wallet visibility;
- balance and transaction history;
- signing and submission where safe and supported;
- viewing node, miner and network status;
- remote control of nodes or miners in other locations once secure operator auth exists.

Current rule:
- iOS should not be assumed to mine locally.

## Mining Rule By Platform

Expected direction:
- Windows desktop: mining support allowed;
- Linux desktop: mining support allowed;
- Linux CLI: mining support allowed;
- Android: no local mining assumption;
- iOS: no local mining assumption.

Mobile clients may still:
- choose payout wallet;
- view miner state;
- start, stop or monitor remote miners if secure remote-control architecture is implemented later.

## UX Direction

End-user direction should prefer:
- one primary desktop app on desktop platforms;
- one primary mobile app on mobile platforms;
- one coherent account, wallet and status model across devices.

Operator direction should still support:
- CLI for scripting and servers;
- explicit logs and diagnostics;
- safe remote-control boundaries rather than hidden background behavior.

## Security Constraint

Remote control of nodes or miners from Android or iOS must not be assumed safe by default.

Before mobile remote-control features are implemented, Alvenqis will still need:
- authenticated operator access rules;
- session and device trust model;
- clear permission boundaries;
- auditability for remote actions.

## Current Implementation State

The repository now provides:
- the **Tauri** Control Center (Windows + Linux) with wallet, node, miner, indexer and explorer integration;
- Linux packaging for AppImage, Debian/Ubuntu `.deb`, Fedora `.rpm` and Arch `PKGBUILD` via Tauri;
- Linux sidecar/process support with runtime data stored outside the installed application;
- an Android native prototype backed by the same Rust BIP39, SLIP-0010 Ed25519 and address logic as desktop;
- a startup wallet selector and chain-synchronization gate on desktop and Android.

The former Electron desktop tree has been removed; do not reintroduce it.

Current limitations remain:
- Linux packages require native CI or Linux host verification before distribution;
- Android transaction signing/submission is not implemented yet;
- Android and iOS have no local mining support;
- iOS has not been scaffolded;
- secure authenticated remote control for nodes and miners is not implemented;
- P2P v3 includes header-first bounded branch verification and cumulative-work
  reorganization, but durable deep-branch storage, resume, and multi-host soak
  remain incomplete.

Synchronization UX must obey these rules:
- compare local height only with handshake-validated peers;
- show local height, reported network target and remaining blocks when a target exists;
- show an unknown target while peer discovery has not established a trusted height;
- do not unlock the normal panel on a fabricated or RPC-only 100 percent state.

## Implementation Guidance

Future research and implementation should prefer:
- one Windows desktop app for normal Windows usage;
- one Linux desktop app for normal Linux desktop usage;
- continued strong Linux CLI support;
- mobile apps focused on wallet and remote visibility first;
- no mobile mining assumption;
- reuse of existing Rust logic where practical;
- honest status labels until each platform is actually implemented and testable.
