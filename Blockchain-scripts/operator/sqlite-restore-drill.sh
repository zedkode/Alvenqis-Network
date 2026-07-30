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
transcript_log="$drill_dir/drill.log"
exec > >(tee -a "$transcript_log") 2>&1

validation_field() {
  local output="$1"
  local field="$2"
  local value
  value="$(printf '%s\n' "$output" | sed -n "s/.*${field}=\\([^[:space:]]*\\).*/\\1/p" | tail -n 1)"
  [[ -n "$value" ]] || fail "validate-chain output did not contain $field"
  printf '%s\n' "$value"
}

sha256_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  else
    fail "Neither sha256sum nor shasum is available"
  fi
}

echo "Alvenqis Task 3 — SQLite restore drill"
echo "UTC: $stamp"
echo "chain_dir: $CHAIN_DIR"
echo "drill_dir: $drill_dir"
echo "transcript: $transcript_log"
echo "NOTE: This is rehearsal evidence, not Mainnet Live / G4 approval."

[[ -f "$chain_db" ]] || fail "No chain.sqlite3 at $chain_db. Initialize local candidate chain first (LOCAL_RUNBOOK)."

step "Preflight verify-chain-database"
run_node verify-chain-database || fail "Live chain failed integrity check"
echo "  OK  preflight_integrity"

step "Capture source chain identity"
source_validation="$(run_node validate-chain)" || fail "Source chain validation failed"
echo "$source_validation"
source_network_id="$(validation_field "$source_validation" network_id)"
source_height="$(validation_field "$source_validation" height)"
source_blocks="$(validation_field "$source_validation" blocks)"
source_tip_hash="$(validation_field "$source_validation" tip_hash)"
echo "  OK  source_identity — network_id=$source_network_id height=$source_height blocks=$source_blocks tip_hash=$source_tip_hash"

step "Online backup-chain-database"
backup_dir="$drill_dir/online-backup"
mkdir -p "$backup_dir"
backup_db="$backup_dir/chain.sqlite3"
run_node backup-chain-database --output "$backup_db" || fail "Online backup failed"
[[ -f "$backup_db" ]] || fail "Backup file missing"
backup_sha256="$(sha256_file "$backup_db")"
echo "  OK  online_backup — $backup_db sha256=$backup_sha256"

step "Isolated restore + integrity"
restore_dir="$drill_dir/isolated-restore"
mkdir -p "$restore_dir" "$drill_dir/isolated-mempool"
cp "$backup_db" "$restore_dir/chain.sqlite3"
# run_node always uses CHAIN_DIR; invoke cargo/sidecar with explicit data-dir
if [[ "$PACKAGED" == "true" ]]; then
  "$SIDECAR_DIR/alvenqis-node" \
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

restore_sha256="$(sha256_file "$restore_dir/chain.sqlite3")"
[[ "$restore_sha256" == "$backup_sha256" ]] || fail "Restored SQLite copy hash does not match online backup"
echo "  OK  restored_file_hash — sha256=$restore_sha256"

identity_verified=false
restore_network_id=""
restore_height=""
restore_blocks=""
restore_tip_hash=""
if [[ "$SKIP_VALIDATE" != "true" ]]; then
  step "Isolated validate-chain + identity comparison"
  if [[ "$PACKAGED" == "true" ]]; then
    restore_validation="$("$SIDECAR_DIR/alvenqis-node" \
      --config "$LOCAL_NODE_CONFIG" \
      --data-dir "$restore_dir" \
      --mempool-dir "$drill_dir/isolated-mempool" \
      validate-chain)" || fail "Isolated validate-chain failed"
  else
    restore_validation="$(cd "$WORKSPACE_ROOT" && env CARGO_TARGET_DIR="$BUILD_DIR" "$CARGO_BIN" run -p alvenqis-node -- \
      --config "$LOCAL_NODE_CONFIG" \
      --data-dir "$restore_dir" \
      --mempool-dir "$drill_dir/isolated-mempool" \
      validate-chain)" || fail "Isolated validate-chain failed"
  fi
  echo "$restore_validation"
  restore_network_id="$(validation_field "$restore_validation" network_id)"
  restore_height="$(validation_field "$restore_validation" height)"
  restore_blocks="$(validation_field "$restore_validation" blocks)"
  restore_tip_hash="$(validation_field "$restore_validation" tip_hash)"
  if [[ "$restore_network_id" != "$source_network_id" ||
        "$restore_height" != "$source_height" ||
        "$restore_blocks" != "$source_blocks" ||
        "$restore_tip_hash" != "$source_tip_hash" ]]; then
    fail "Restored chain identity does not match source"
  fi
  identity_verified=true
  echo "  OK  restored_identity_match — network_id=$restore_network_id height=$restore_height blocks=$restore_blocks tip_hash=$restore_tip_hash"
else
  echo "  SKIP restored_identity_match — SKIP_VALIDATE_CHAIN=true"
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
  "transcript_log": "$transcript_log",
  "backup_db": "$backup_db",
  "restore_db": "$restore_dir/chain.sqlite3",
  "backup_sha256": "$backup_sha256",
  "restore_sha256": "$restore_sha256",
  "identity_verified": $identity_verified,
  "source_identity": {
    "network_id": "$source_network_id",
    "height": $source_height,
    "blocks": $source_blocks,
    "tip_hash": "$source_tip_hash"
  },
  "restore_identity": {
    "network_id": "$restore_network_id",
    "height": ${restore_height:-null},
    "blocks": ${restore_blocks:-null},
    "tip_hash": "$restore_tip_hash"
  },
  "simulate_disk": $([[ "$DISK_SIM" == "yes" || "$DISK_SIM" == "true" ]] && echo true || echo false),
  "steps": [
    {"name": "preflight_integrity", "ok": true},
    {"name": "source_identity", "ok": true},
    {"name": "online_backup", "ok": true},
    {"name": "isolated_integrity", "ok": true},
    {"name": "restored_file_hash", "ok": true},
    {"name": "restored_identity_match", "ok": $identity_verified}
  ],
  "pass": true
}
EOF

echo ""
echo "evidence: $evidence_json"
echo "PASS: Drill A SQLite backup/restore evidence recorded (not G4)."
