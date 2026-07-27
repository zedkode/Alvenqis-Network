# Blockchain-scripts

Operator, CI, local, release, and security scripts for Alvenqis Network.

Repo root is two levels up from any `Blockchain-scripts/<area>/` script.

| Area | Purpose |
|------|---------|
| `lib/` | Shared path helpers (`repo-paths.ps1` / `.sh`) |
| `local/` | Start/stop local node+RPC+explorer smoke stack |
| `dev/` | Devnet init / mine / reset / status |
| `browser/` | Native messaging host register / health |
| `release/` | Installers, release gate G1, version bump |
| `security/` | Secrets, hygiene, config safety, workflow pinning |
| `git/` | Forbidden-file checks, push helpers |
| `github/` | VPS sync/release helpers |
| `operator/` | Maturity health, VPS env prep |
| `pipeline/` | Optional multi-agent GitHub queue helpers |
| `docs/` | Doc audit utilities |

## Examples

```powershell
# From repo root
.\Blockchain-scripts\local\start-all.ps1
.\Blockchain-scripts\release\release-gate.ps1
.\Blockchain-scripts\browser\register-native-host.ps1 -ExtensionId <id> -Build
```

```bash
bash Blockchain-scripts/release/release-gate.sh
bash Blockchain-scripts/security/check-secrets.sh
```

Implemented code lives in `../Blockchain-prototype/`.
Docs live in `../Blockchain-docs/`.
