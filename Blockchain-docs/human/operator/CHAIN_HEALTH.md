# Chain health monitoring (Mainnet Candidate)

Status: Prototype / Mainnet Candidate — **not** Mainnet Live.

## Badge

After the workflow is on the default branch:

[![candidate-chain-health](https://github.com/zedkode/Alvenqis-Network/actions/workflows/candidate-chain-health.yml/badge.svg)](https://github.com/zedkode/Alvenqis-Network/actions/workflows/candidate-chain-health.yml)

## What is checked

`alvenqis-browser-host --check-health`:

1. RPC reachable (`GET /status`)
2. Tip available (`GET /chain/tip`)
3. Optional indexer enforcement:
   - `--require-indexer-sync` — fail if indexer missing / not initialized / out of policy
   - `--max-indexer-lag <n>` — allow lag up to `n` blocks (height delta or `lag_blocks`)

If neither indexer flag is set, indexer issues are **warnings only**.

### Exit codes

| Code | Meaning |
|---:|---|
| 0 | healthy |
| 1 | transport / client error |
| 2 | chain not ready |
| 3 | indexer lag / policy fail (when enforced) |

## Local / operator

```powershell
cargo run -q -p alvenqis-browser-host -- --check-health --json
cargo run -q -p alvenqis-browser-host -- --check-health --require-indexer-sync --json
cargo run -q -p alvenqis-browser-host -- --check-health --max-indexer-lag 2 --json

.\Blockchain-scripts\browser\probe-chain.ps1 -Strict
.\Blockchain-scripts\browser\probe-chain.ps1 -MaxIndexerLag 2
.\Blockchain-scripts\browser\probe-chain.ps1 -Watch -IntervalSec 30 -Strict -WebhookUrl $env:ALVENQIS_HEALTH_WEBHOOK_URL
```

```bash
./Blockchain-scripts/browser/check-health.sh --strict --json
./Blockchain-scripts/browser/check-health.sh --max-indexer-lag 2 --json
./Blockchain-scripts/browser/probe-chain.sh --strict --watch --interval 30
```

Webhook payload (JSON POST): `text`, `code`, `health` (body of health JSON when available).

Env alias: `ALVENQIS_HEALTH_WEBHOOK_URL`.

## GitHub Actions

Workflow: `.github/workflows/candidate-chain-health.yml`

- schedule: every 30 minutes
- manual: `workflow_dispatch` (RPC URL, strict, max lag)
- builds `alvenqis-browser-host` and runs `--check-health --json` (+ indexer policy)
- uploads `health.json` artifact
- optional failure webhook via repo secret **`ALVENQIS_HEALTH_WEBHOOK_URL`**

## Windows Task Scheduler (local cron)

```powershell
# Every 30 minutes, strict indexer, optional webhook from env
.\Blockchain-scripts\browser\register-health-task.ps1 -Strict

# Custom interval + lag tolerance
.\Blockchain-scripts\browser\register-health-task.ps1 -IntervalMinutes 15 -MaxIndexerLag 2

# With webhook
.\Blockchain-scripts\browser\register-health-task.ps1 -Strict -WebhookUrl $env:ALVENQIS_HEALTH_WEBHOOK_URL

# Remove
.\Blockchain-scripts\browser\register-health-task.ps1 -Unregister
```

Logs: `%LOCALAPPDATA%\Alvenqis\health\probe-*.log`

Manual run once:

```powershell
Start-ScheduledTask -TaskName 'AlvenqisCandidateChainHealth'
```

## Notes

- Public default RPC is the Mainnet Candidate gateway from `alvenqis-sdk-rust`.
- Failures are operational signals, not protocol consensus proofs.
- Do not market a green check as "mainnet live".
- For SQLite restore / reorg / multi-host maturity **evidence** (Task 3), see
  [CHAIN_MATURITY_OPS.md](CHAIN_MATURITY_OPS.md).
