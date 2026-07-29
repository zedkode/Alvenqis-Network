# Client Platform Direction

Status: Draft / Planned / Prototype

## Goal

Alvenqis currently targets a focused, verifiable client surface:

- Windows desktop;
- Linux desktop;
- Linux CLI;
- browser native-host prototype.

There is no active mobile client, mobile FFI crate, mobile workflow, or mobile
release artifact in this repository. Any future mobile work requires a new
architecture and security review rather than extending a retired prototype.

## Product Principle

Alvenqis should expose a unified user experience per supported platform while
keeping core, wallet, node, mining, RPC, and indexing responsibilities modular.

This means:

- users should not assemble disconnected tools for normal workflows;
- platform clients reuse canonical protocol and wallet rules;
- internal modules remain separated so one failure cannot corrupt another;
- unsupported platforms are not represented by dormant production code.

## Platform Roles

### Windows Desktop

- one Tauri Control Center;
- wallet, node control, mining, logs, and status in one place;
- local operator and local mining workflows.

### Linux Desktop

- the same Tauri product shape where practical;
- wallet, node control, mining, logs, and status in one place;
- native Linux packaging and sidecar handling.

### Linux CLI

- first-class operator, automation, server, packaging, and troubleshooting
  support;
- remains viable even as desktop interfaces mature.

### Browser Prototype

- native-host boundary for narrowly scoped local capabilities;
- no direct secret, filesystem, process, or unauthenticated operator access;
- not store-ready and not a replacement for desktop or CLI.

## Mining Rule

Local mining is supported only through the Windows/Linux miner and Control
Center paths. Public HTTP mining is retired; remote pool mining uses verified
Stratum TLS.

## Security Constraint

Any future mobile or remote-control client must first define:

- authenticated operator access;
- device and session trust;
- explicit permissions;
- auditable remote actions;
- key-custody and recovery boundaries;
- a clean implementation and release gate.

## Current Implementation State

The repository provides:

- the Tauri Control Center for Windows and Linux;
- Linux AppImage, Debian/Ubuntu, Fedora, and Arch packaging definitions;
- Linux sidecar/process support with runtime data outside the installed app;
- a browser native-host prototype with constrained capabilities.

Current limitations include:

- native packages still require same-commit CI and signing evidence;
- secure authenticated remote control is not implemented;
- browser distribution and external review remain open;
- multi-host P2P and operational maturity remain incomplete.

Synchronization UX must:

- compare local height only with handshake-validated peers;
- show local height, reported network target, and remaining blocks when known;
- show an unknown target until discovery establishes a trusted height;
- never unlock a normal panel using a fabricated or RPC-only 100 percent state.
