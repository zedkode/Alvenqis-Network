# GitHub Candidate Release Guide

## Purpose

Alvenqis candidate releases use independent GitHub Actions workflows for Windows, Linux, Alvenqis Setup External, and quality verification. A successful platform workflow may publish its own artifacts without hiding a failure on another platform.

Passing a candidate workflow is not a public Mainnet approval. Network maturity remains governed by `Blockchain-docs/human/release/NETWORK_MATURITY.md`.

## Prerequisites

- Git is installed and available in `PATH`.
- GitHub CLI is installed and authenticated with repository and workflow access.
- The repository remote named `origin` points to `zedkode/Alvenqis-Network`.
- Windows signing secrets are configured before running a Windows candidate release.

Authenticate GitHub CLI when required:

```powershell
gh auth login
gh auth status
```

## Interactive Release Manager

From the repository root, run:

```powershell
.\Blockchain-scripts\release\alvenqis-release.cmd
```

The manager reads the Control Center V2 version, creates an annotated `desktop-vX.Y.Z-candidate.N` tag, and starts the independent candidate workflows. Candidate builds are never skipped through tag-message markers.

## Candidate Workflows

- `Candidate Quality Checks` verifies repository safety, documentation, formatting, tests, RocksDB support, and Clippy for core and desktop candidate tags.
- `Candidate Windows Release` builds and signs Tauri Windows packages for `desktop-v*-candidate.*` tags.
- `Candidate Linux Release` builds Tauri Linux packages for `desktop-v*-candidate.*` tags.
- `Candidate Setup External Release` validates and packages the independent
  deployment bundle for core and desktop candidate tags.
- `Candidate Linux Components Release` tests, packages, and publishes node,
  RPC, indexer, explorer, pool, wallet CLI, and CUDA miner artifacts as
  independent matrix entries. Matrix fail-fast is disabled: a failed component
  remains failed while passing components may still publish.
- `Release Gate` runs documentation, security, Rust, web, and Setup External
  jobs on release-relevant changes.

## Existing Tags

To upload a new local artifact to an existing candidate tag, use the interactive manager and select the existing-tag upload operation. Existing asset names are not overwritten; publish a new candidate tag when an artifact must be replaced.

To restart a platform build, select the workflow dispatch operation and choose the platform. The workflow always rebuilds and validates that platform.

## Verification

Before treating a candidate artifact as usable:

1. Confirm every required GitHub Actions run completed.
2. Download the platform checksum file.
3. Verify SHA-256 checksums locally.
4. Confirm Windows Authenticode signing for Windows installers.
5. Record any failed or skipped platform separately.

## Required Repository Settings

The repository owner must configure these GitHub settings before relying on the workflows as enforcement:

- protect `main` with pull requests, conversation resolution, and the final `Release Gate` status check;
- block force pushes and branch deletion;
- enable GitHub Actions full-SHA pinning;
- restrict allowed third-party actions to the repositories pinned in `.github/workflows/`;
- configure `WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PASSWORD` as Actions secrets;
- use a protected release environment with a required reviewer for candidate publishing.
