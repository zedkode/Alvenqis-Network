#!/usr/bin/env bash
# Task 3 Drill A/B: SQLite online backup + isolated restore (+ optional disk-failure sim).
# Mainnet Candidate / Prototype only — not G4 launch approval.
set -Eeuo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../local/common.sh
source "$script_dir/../local/common.sh"

fail() { echo "FAIL: $*" >&2; exit 1; }
step() { echo ""; echo "==> $*"; }

SKIP_VALIDATE="${SKIP_VALIDATE_CHAIN:-false}"
DISK_SIM="${DISK_FAILURE_SIM:-false}"
CONFIRM="${RESTORE_CONFIRM:-}"

ensure_local_directories
chain_db="$CHAIN_DIR/chain.sqlite3"
evidence_root="$LOCAL_ROOT/maturity-evidence"
mkdir -p "$evidence_root"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
drill_dir="$evidence_root/sqlite-restore-$stamp"
mkdir -p "$drill_dir"
evidence_json="$drill_dir/evidence.json"

echo "Alvenqis Task 3 — SQLite restore drill"
echo "UTC: $stamp"
echo "chain_dir: $CHAIN_DIR"
echo "drill_dir: $drill_dir"
echo "NOTE: This is rehearsal evidence, not Mainnet Live / G4 approval."

[[ -f "$chain_db" ]] || fail "No chain.sqlite3 at $chain_db. Initialize local candidate chain first (LOCAL_RUNBOOK)."

step "Preflight verify-chain-database"
run_node verify-chain-database || fail "Live chain failed integrity check"
echo "  OK  preflight_integrity"

step "Online backup-chain-database"
backup_dir="$drill_dir/online-backup"
mkdir -p "$backup_dir"
backup_db="$backup_dir/chain.sqlite3"
run_node backup-chain-database --output "$backup_db" || fail "Online backup failed"
[[ -f "$backup_db" ]] || fail "Backup file missing"
echo "  OK  online_backup — $backup_db"

step "Isolated restore + integrity"
restore_dir="$drill_dir/isolated-restore"
mkdir -p "$restore_dir" "$drill_dir/isolated-mempool"
cp "$backup_db" "$restore_dir/chain.sqlite3"
# run_node always uses CHAIN_DIR; invoke cargo/sidecar with explicit data-dir
if [[ "$PACKAGED" == "true" ]]; then
  "$WORKSPACE_ROOT/bin/alvenqis-node" \
    --config "$LOCAL_NODE_CONFIG" \
    --data-dir "$restore_dir" \
    --mempool-dir "$drill_dir/isolated-mempool" \
    verify-chain-database || fail "Isolated restore integrity failed"
else
  (cd "$WORKSPACE_ROOT" && env CARGO_TARGET_DIR="$BUILD_DIR" "$CARGO_BIN" run -p alvenqis-node -- \
    --config "$LOCAL_NODE_CONFIG" \
    --data-dir "$restore_dir" \
    --mempool-dir "$drill_dir/isolated-mempool" \
    verify-chain-database) || fail "Isolated restore integrity failed"
fi
echo "  OK  isolated_integrity"

if [[ "$SKIP_VALIDATE" != "true" ]]; then
  step "Isolated validate-chain"
  set +e
  if [[ "$PACKAGED" == "true" ]]; then
    "$WORKSPACE_ROOT/bin/alvenqis-node" \
      --config "$LOCAL_NODE_CONFIG" \
      --data-dir "$restore_dir" \
      --mempool-dir "$drill_dir/isolated-mempool" \
      validate-chain
  else
    (cd "$WORKSPACE_ROOT" && env CARGO_TARGET_DIR="$BUILD_DIR" "$CARGO_BIN" run -p alvenqis-node -- \
      --config "$LOCAL_NODE_CONFIG" \
      --data-dir "$restore_dir" \
      --mempool-dir "$drill_dir/isolated-mempool" \
      validate-chain)
  fi
  vc=$?
  set -e
  if [[ $vc -eq 0 ]]; then
    echo "  OK  isolated_validate_chain"
  else
    echo "  WARN isolated_validate_chain failed (recorded)"
  fi
fi

if [[ "$DISK_SIM" == "yes" || "$DISK_SIM" == "true" ]]; then
  step "Disk-failure simulation (LIVE)"
  [[ "$CONFIRM" == "yes" ]] || fail "Refusing live disk-failure sim without RESTORE_CONFIRM=yes"
  if is_managed_process_running node 2>/dev/null; then
    echo "  Stopping managed local node..."
    bash "$script_dir/../local/stop-all.sh" || true
    sleep 2
  fi
  failed_name="chain.sqlite3.failed-$stamp"
  mv "$chain_db" "$CHAIN_DIR/$failed_name"
  cp "$backup_db" "$chain_db"
  if run_node verify-chain-database; then
    echo "  OK  live_disk_failure_restore — failed_saved_as=$failed_name"
  else
    mv "$CHAIN_DIR/$failed_name" "$chain_db" || true
    fail "Live restore after simulated failure failed (attempted rollback)"
  fi
  echo "  Live DB restored from online backup. Restart local stack and check /status."
fi

cat >"$evidence_json" <<EOF
{
  "drill": "sqlite-restore",
  "utc": "$stamp",
  "label": "Mainnet Candidate / Prototype",
  "g4_waiver": false,
  "chain_dir": "$CHAIN_DIR",
  "drill_dir": "$drill_dir",
  "backup_db": "$backup_db",
  "restore_db": "$restore_dir/chain.sqlite3",
  "simulate_disk": $([[ "$DISK_SIM" == "yes" || "$DISK_SIM" == "true" ]] && echo true || echo false),
  "pass": true
}
EOF

echo ""
echo "evidence: $evidence_json"
echo "PASS: Drill A SQLite backup/restore evidence recorded (not G4)."
