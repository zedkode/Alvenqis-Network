# Execution Layer

Status: Planned / Research / not implemented

## Scope

The Execution Layer is intended to support:
- smart contracts;
- dApps;
- VRC standards;
- NFT logic;
- game and app logic;
- software license workflows;
- indexed contract events.

## Fixed Direction

The source-info set fixes only the broad direction:
- Rust/WASM-oriented contract model;
- low-fee application direction;
- deterministic execution requirement;
- bounded resource usage requirement.

## Draft Recommendation

The recommendation set currently suggests:
- WASM runtime: wasmtime;
- resource metering: fuel-based deterministic metering;
- reuse of the implemented base-fee-plus-tip transfer model where appropriate,
  with separate deterministic gas rules for execution.

These remain draft until explicitly accepted.

## Non-Claims

This architecture document does not claim:
- a live contract runtime;
- production contract tooling;
- finalized VRC implementations;
- deployed contract standards.

## Downstream Impact

- Core must expose deterministic execution hooks once the VM direction is finalized.
- Wallet and SDK need ABI, signing and transaction composition rules.
- Explorer and Indexer need contract event and receipt indexing.
- RPC must expose execution results without overpromising unsupported methods.
