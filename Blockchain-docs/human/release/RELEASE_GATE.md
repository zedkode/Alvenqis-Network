# Release Gate (G1)

Status: **Draft / Mainnet Candidate / Prototype**

> **This gate is not a public Mainnet launch approval.**  
> Passing it only means the repository is fit for **Mainnet Candidate rehearsal** (local build quality, hygiene, and docs presence).  
> See `docs/release/NETWORK_MATURITY.md` for the full maturity ladder (G0–G4).

---

## Purpose

The local release gate must pass before:

- tagging a **candidate** software release;
- building Control Center / VPS **rehearsal** artifacts;
- starting **controlled** multi-host candidate experiments.

It is a **software + hygiene** gate. It does **not** certify:

- a live public network;
- production operations readiness;
- external security audit completion;
- genesis independent of this repository.

---

## How to run

Windows:

```powershell
scripts/release/release-gate.ps1
```

Linux or macOS:

```bash
bash scripts/release/release-gate.sh
```

Help:

```powershell
scripts/release/release-gate.ps1 --help
```

---

## What the gate checks (G1)

- Secret scanner (`scripts/security/check-secrets.*`)
- Repository hygiene scanner
- Config safety scanner
- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --workspace --release`
- `npm install` and `npm run build` for `alvenqis-explorer` when present
- Required release and security documentation files
- Required mainnet-candidate config files

---

## What a green G1 means

| Allowed | Not allowed |
|---|---|
| “Release gate (G1) passed for Mainnet Candidate” | “Mainnet is live” |
| “Safe to build candidate installers / VPS control-plane bundle for rehearsal” | “Production-ready public network” |
| Continue G2 checklist / controlled VPS rehearsal | Skip external review or multi-host soak |

---

## Related gates

| Gate | Doc |
|---|---|
| Maturity source of truth | `docs/release/NETWORK_MATURITY.md` |
| G2 Mainnet Candidate checklist | `docs/release/MAINNET_CANDIDATE_CHECKLIST.md` |
| Security | `docs/security/SECURITY_GATE.md` |
| Genesis (candidate pin only) | `docs/release/GENESIS.md` |

---

## Failure policy

- Do not ship candidate artifacts if G1 fails.
- Do not weaken scanners to force a green run.
- Do not rewrite public copy to imply Mainnet because G1 passed.
