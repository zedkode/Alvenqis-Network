# Alvenqis Licensing Policy

Status: Draft / Accepted Repository Policy

Purpose:
- define the current Alvenqis licensing direction explicitly;
- separate protocol-layer licensing from proprietary business-layer licensing;
- avoid accidental public claims that the entire repository already ships under one blanket open-source license.

## Scope

This document sets the current accepted licensing direction for the Alvenqis repository and future package split.

It does not replace:
- legal review;
- final business entity decisions;
- per-package release licensing files;
- exchange, wallet or third-party compliance review.

## Accepted Licensing Direction

### Open-Source Protocol And Infrastructure Components

The following components are intended to be open-source under `Apache-2.0` unless a later explicit decision changes that policy:
- `alvenqis-core`
- `alvenqis-node`
- `alvenqis-miner`
- `alvenqis-indexer`
- `alvenqis-rpc-gateway`
- `alvenqis-sdk-rust`
- `alvenqis-contracts`
- cryptographic and signing parts of `alvenqis-wallet`

Reasoning:
- protocol trust requires public reviewability;
- mining and validation software should be auditable;
- Apache-2.0 supports commercial integration while keeping the protocol open;
- explicit patent language is preferable to an informal permissive-only assumption.

### Proprietary Or Controlled Business-Layer Components

The following components remain proprietary or controlled by project policy unless explicitly relicensed later:
- `alvenqis-website`
- website admin panel and related business CMS layers
- `alvenqis-marketplace` backend and related commercial business logic
- `alvenqis-infra`
- `alvenqis-ops`
- internal operational and deployment details

Reasoning:
- these layers may contain sensitive business workflows, operator logic or commercial differentiators;
- they do not need to be public for protocol verification.

## Current Repository Rule

This repository currently contains a mixed-scope workspace.

Current rule:
- do not treat the full repository as already covered by one single public license;
- do not add a misleading root-level blanket `LICENSE` file until the repository layout and release boundaries are finalized;
- apply final license files at the package or release boundary once the public release structure is ready.

## Public Communication Rule

Until package-level licensing is fully applied:
- do not claim that every folder in this repository is open-source;
- do not claim that proprietary business-layer components are public-domain or Apache-licensed by default;
- describe the current policy as:

```text
Protocol and chain-critical components are intended for Apache-2.0 release, while business and operational layers remain proprietary unless explicitly relicensed.
```

## Required Follow-Up Work

This decision closes the policy question, but further work remains:
- add per-package license files when repository publication boundaries are finalized;
- define contributor-license expectations if outside contributors are accepted;
- review third-party dependency license compatibility;
- confirm the final legal posture with counsel before public production launch.
