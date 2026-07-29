# VPS Rehearsal Ops (Task 1 — prototype hardening)

Status: **Mainnet Candidate / Prototype — not public Mainnet**

This is the operator runbook for **Priority 1 / Task 1**: make the controlled VPS
rehearsal stack **operable, backupable, and verifiable** before any further
`planned/` product work.

Package root on the monorepo:

```text
Blockchain-prototype/alvenqis-release/vps-control-plane/
```

Typical install path on the controlled host (adjust if different):

```text
/home/apps/alvenqis-network/Blockchain-prototype/alvenqis-release/vps-control-plane
# or legacy layout:
/home/apps/alvenqis-network/alvenqis-release/vps-control-plane
```

Project-operated rehearsal example: `https://rpcnode.dohotstudio.com`. It is
not an availability or trust dependency for an independent operator.

---

## 0. Honest boundaries

| Fact | Implication |
|---|---|
| Accepted public profile is **submit/read**, not public mining | Desktop solo mining must use a local loopback RPC |
| Label stays **Mainnet Candidate** | Not live mainnet |
| Upgrades are **manual only** | `tauri build` on a laptop does **not** update the VPS |
| Host has no CUDA miner in the standard stack | Mining is client-side (desktop) or uses an explicitly enabled pool role |
| Public-mining policy and current gateway smoke disagree | Do not treat the public mining surface as release-ready until the blocker in `../security/KNOWN_LIMITATIONS.md` is closed |

---

## 1. Baseline inventory (do this first)

On the VPS, as the operator user:

```bash
cd /path/to/vps-control-plane
test -f .env && echo "has .env"
test -f VERSION && cat VERSION
git rev-parse --short HEAD 2>/dev/null || echo "not a git checkout"
git status -sb 2>/dev/null || true
./scripts/compose.sh ps
```

Record in the session log (no secrets):

- hostname / public RPC host
- `VERSION` file
- git commit if available
- `ALVENQIS_OPERATOR_ROLE` and the rendered files from `compose/roles.json`
- public `/status` height + tip_hash

From any laptop with network (no SSH required):

```bash
# monorepo root
bash Blockchain-prototype/alvenqis-release/vps-control-plane/scripts/smoke-public-candidate.sh
# or PowerShell:
# powershell -File Blockchain-prototype/alvenqis-release/vps-control-plane/scripts/smoke-public-candidate.ps1
```

The health and status assertions remain useful. The mining assertion is not
closure evidence while the accepted HTTP 410 policy, gateway template, RPC
profile, and current smoke script disagree:

- `/health` → 200, mode contains `mining disabled` on public profile
- `/status` → initialized, tip matches pinned genesis while height is 0
  (`0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5`, `index_in_sync=true`)
- `/mining/template` → unresolved repository blocker; accepted policy requires HTTP 410

---

## 2. Backup (on the VPS)

```bash
cd /path/to/vps-control-plane
chmod +x scripts/*.sh docker/*.sh docker/backup-scheduler/*.sh 2>/dev/null || true
./scripts/backup-now.sh
ls -la state/backups/
```

Artifacts under `state/backups/<UTC-stamp>/`:

| File | Contents |
|---|---|
| `alvenqis-state.tar.gz` | data/control/pool/config generated + `.env` (no raw secrets tree) |
| `alvenqis-secrets.tar.gz.enc` | encrypted secrets (passphrase file) |
| `SHA256SUMS` | local integrity |

Also writes `state/metrics/alvenqis_backup.prom` for Prometheus/node-exporter.

**Never** commit backup archives or secrets to Git.

---

## 3. Restore drill (on a disposable host or same host with care)

Use the helper script (stops stack, restores archives, restarts):

```bash
RESTORE_CONFIRM=yes ./scripts/restore-from-backup.sh state/backups/<UTC-stamp>
./scripts/health-check-docker.sh
./scripts/smoke-public-candidate.sh   # if public DNS still points here
```

Success criteria:

1. All required compose services **running** / not unhealthy
2. In-container RPC `/health` OK
3. Public (or tunnel) `/status` returns same **tip_hash** and **height** as before backup
4. No `.env` or secrets left world-readable beyond package norms

If restore is only a dry-run on a **second** host: copy the backup directory offline (encrypted), restore there, compare tip — do not dual-write to the live chain.

---

## 4. Manual upgrade (code on VPS)

See also `MANUAL_UPGRADE.md`. Condensed:

```bash
cd /path/to/vps-control-plane
./scripts/backup-now.sh
git fetch --all --tags
# Explicitly check out the reviewed commit/tag (no auto latest)
git checkout <reviewed-ref>
./scripts/validate-stack.sh --require-docker
COMPOSE_PARALLEL_LIMIT=1 ./scripts/compose.sh up -d --build
./scripts/health-check-docker.sh
./scripts/verify-public-health.sh
# from laptop:
# bash .../scripts/smoke-public-candidate.sh
```

**Forbidden:** Watchtower, `docker compose pull` on mutable `latest`, `docker compose down -v` on a live chain without an explicit disaster procedure.

---

## 5. What “done” means for Task 1

| Checkpoint | Evidence |
|---|---|
| Inventory recorded | Session log: VERSION, commit, height, tip |
| Backup succeeded | Path under `state/backups/` + SHA256SUMS |
| Restore drill once | Health green + tip match after restore |
| Public smoke green | Health/status assertions plus the mining-policy blocker closed |
| Memory updated | `Blockchain-docs/internal/memory/*` |

Task 1 does **not** require mining height > 0 (that is Task 2 —
[PRIVATE_MINING_OPS.md](PRIVATE_MINING_OPS.md)).

Chain SQLite restore / reorg / multi-host maturity evidence is Task 3 —
[CHAIN_MATURITY_OPS.md](CHAIN_MATURITY_OPS.md).

---

## 6. Related paths

| Path | Role |
|---|---|
| `scripts/backup-now.sh` | Operator backup |
| `scripts/restore-from-backup.sh` | Operator restore drill |
| `scripts/health-check-docker.sh` | In-host compose health |
| `scripts/verify-public-health.sh` | DNS/tunnel URL wait |
| `scripts/smoke-public-candidate.sh` | Laptop/VPS public RPC assertions |
| `scripts/smoke-public-candidate.ps1` | Same for Windows operators |
| `MANUAL_UPGRADE.md` | Upgrade policy |
| `DOCKER_DEPLOYMENT.md` | Architecture / security edges |
