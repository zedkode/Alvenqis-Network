# Security Gate

Status: Draft / Mainnet Candidate / Prototype

This workspace uses a local security gate before any VPS deployment work.

Current gate scope:
- no secrets in repository files;
- no local wallet material in tracked source folders;
- no local runtime chain data in the repository tree;
- no unsafe candidate RPC bind to `0.0.0.0` without explicit opt-in;
- no release attempt before Rust validation, release build and explorer build pass.

Local security commands:

```powershell
scripts/security/check-secrets.ps1
scripts/security/check-repo-hygiene.ps1
scripts/security/check-config-safety.ps1
```

Rules:
- use `.env.example` only in the repository;
- keep real wallet files outside the repository;
- keep RPC bound to `127.0.0.1` by default;
- treat Mainnet Candidate as a local or operator-run prototype until the full release gate passes.
