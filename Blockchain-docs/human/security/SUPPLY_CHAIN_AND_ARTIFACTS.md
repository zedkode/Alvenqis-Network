# Supply Chain, SBOM, and Artifact Verification

Status: Draft / candidate checksums exist / publisher signing incomplete

## Required release identity

Every reviewed release must bind:

- immutable Git commit and tag;
- Rust and Node lockfiles;
- compiler, linker, CUDA, Node, package-manager, OS, and container versions;
- workflow/action commit pins;
- source archive and generated-code provenance;
- binary/package/container hashes;
- SBOM and dependency vulnerability results;
- publisher signature and independently verifiable update metadata.

## Current controls

- repository hygiene and secret scanners;
- Cargo and Node lockfiles;
- workflow-pinning checks;
- candidate `SHA256SUMS`;
- user approval before desktop update application;
- explicit control-plane versions and no automatic mutable-source updates.

Checksums prove integrity against a supplied manifest; they do not prove who
published that manifest. Native artifacts and update metadata are not yet
cryptographically signed.

## Audit requirements

- reproducible or independently comparable builds on clean Windows and Linux;
- SBOM for Rust, Node, native CUDA, and container layers;
- license and provenance review;
- dependency-vulnerability gate with documented exceptions;
- signed hashes, signature rotation/revocation, and offline verification;
- same-commit artifact matrix and retained build logs;
- malicious/missing manifest, downgrade, rollback, and interrupted-update tests.

No release may claim external audit coverage for code or artifacts outside the
immutable review scope.
