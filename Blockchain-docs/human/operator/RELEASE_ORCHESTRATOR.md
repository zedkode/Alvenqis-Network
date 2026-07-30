# Release orchestrator

`Blockchain-scripts\operator\release-orchestrator.cmd` is the single Windows
terminal entrypoint for local service control, desktop builds, candidate
releases, and authenticated VPS updates.

Run it from a plain `cmd.exe` or PowerShell terminal:

```bat
Blockchain-scripts\operator\release-orchestrator.cmd
```

The orchestrator first fetches `origin`, reports the current branch and
ahead/behind state, prints the last five `origin/main` commits and the porcelain
working-tree status, then performs read-only VPS health/stack queries. It asks
one round of questions: action, VPS environment when needed, what to do with a
dirty working tree, and one final confirmation. Enter at either destructive
confirmation is the safe answer.

After the final confirmation there are no more operator prompts. The first
failed command stops the pipeline and makes the process exit non-zero.

## Existing implementation it delegates to

The orchestrator coordinates existing entrypoints; it does not replace their
build, gate, or deployment logic:

| Requested operation | Existing implementation |
|---|---|
| Start/restart local services | `Blockchain-scripts/local/alvenqis-local.ps1` |
| Windows Control Center build | `Blockchain-scripts/release/build-windows-installer.ps1` |
| WSL2 Ubuntu preparation | `Blockchain-scripts/release/setup-wsl-linux-build-host.sh` and `wsl-linux-setup.sh` |
| Linux Control Center build | `Blockchain-scripts/release/build-linux-desktop.sh` |
| Software/security release gate | `Blockchain-scripts/release/release-gate.ps1` |
| Commit/push and Setup External bundle release | `Blockchain-scripts/github/sync-and-release-setup-external.ps1` |
| Candidate packages | existing candidate Windows, Linux, and VPS GitHub Actions workflows |
| Immutable Docker images | `.github/workflows/setup-external-images.yml` through `gh` |
| VPS stack update | authenticated `POST /api/deploy`, polled through `GET /api/job` |
| VPS versions/containers | authenticated `GET /api/stack` |

The legacy `Blockchain-prototype/alvenqis-release/vps/` tree is never used.
Only `alvenqis-release/alvenqis-setup-external/` is in the active path.

## Local VPS operator profile

VPS credentials and the complete, previously enrolled deploy payload remain
outside Git. By default the orchestrator reads:

```text
%LOCALAPPDATA%\Alvenqis\Operator\profiles\<environment>.json
```

The environment name must be one actually read from
`Blockchain-prototype/configs/*.toml`. A different existing profile directory
can be selected with `ALVENQIS_OPERATOR_PROFILE_DIR`; a single existing file can
be selected with `ALVENQIS_OPERATOR_PROFILE_FILE`. Neither path nor any secret
value is written to the run log.

Profile shape (placeholders only):

```json
{
  "environment": "name-from-configs",
  "adminServerUrl": "https://control.example.invalid",
  "setupToken": "stored-local-enrollment-token",
  "deployPayload": {
    "base_domain": "example.invalid",
    "node_name": "candidate-node",
    "admin_email": "operator@example.invalid"
  },
  "endpoints": {
    "rpcHealth": "https://rpc.example.invalid/health",
    "explorer": "https://explorer.example.invalid/",
    "website": "https://example.invalid/"
  }
}
```

`deployPayload` must be the complete payload retained from the enrolled
admin-server setup. The orchestrator refuses to invent it because `/api/deploy`
also writes stack configuration. At deploy time it changes only
`ALVENQIS_version` to the immutable tag whose image workflow passed. The setup
token is sent only in the `X-Alvenqis-Setup-Token` request header.

Protect the profile with the current Windows user's filesystem ACL and never
place it inside the repository.

## WSL requirement

Linux builds require an already installed WSL2 Ubuntu distribution. The
orchestrator never installs a distribution. If none exists, install it first,
for example:

```powershell
wsl --install -d Ubuntu-24.04
```

System packages are then prepared by the repository's existing setup scripts.
Use `-WslDistro Ubuntu-24.04` only when more than one Ubuntu distribution is
installed.

## Dry run and automation

To exercise the complete prompt-free plan without executing builds, tags,
pushes, workflows, or API deployment:

```powershell
.\Blockchain-scripts\operator\release-orchestrator.ps1 `
  -Action BuildWindows -DirtyPolicy Include -Yes -DryRun
```

`-DryRun` still performs the startup repository/status queries but skips every
execution command after confirmation. It never reports a live pipeline as
successful.

For a real non-interactive run, specify `-Action`, `-Environment` when the
action touches a VPS, `-DirtyPolicy Include` or `Abort`, and `-Yes`. Use this
only after reviewing the same final plan interactively.

## Run artifacts

Every invocation creates:

```text
D:\Blockchain-Core\Activity Startup\<yyyyMMdd-HHmmss>\
```

- `run.log`: timestamped and redacted merged command output;
- `summary.md`: commit, target results, HTTP status and response times;
- `error-state.md`: only on failure, with the failed step, exit code, last
  relevant lines, and one evidence-based first check;
- `vps-changes.md`: only when a VPS update ran, based on admin-server stack
  snapshots and its job report.

Release asset URLs and configured RPC, explorer, website and VPS endpoints are
requested over HTTP, retried once, and must return 2xx/3xx. Overall exit code is
zero only when every requested live step passed (or when an explicit dry run
completed without validation errors).
