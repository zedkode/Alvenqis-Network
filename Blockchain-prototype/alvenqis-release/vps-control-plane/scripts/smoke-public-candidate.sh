#!/usr/bin/env bash
# Public Mainnet Candidate smoke (laptop or VPS). No SSH, no secrets.
# Exit 0 only when health, status and solo mining template pass.
set -Eeuo pipefail

base_url="${1:-${ALVENQIS_PUBLIC_RPC:-}}"
[[ -n "$base_url" ]] || {
  echo "Pass the public RPC URL or set ALVENQIS_PUBLIC_RPC." >&2
  exit 64
}
base_url="${base_url%/}"

# Pinned release genesis tip (height 0). Override only with explicit approval.
expected_genesis_tip="${ALVENQIS_EXPECTED_GENESIS_TIP:-0000c29213014578ac41a748c2be3489859f1e0b1f3555bd89b7e5301632a4c5}"
expected_network_id="${ALVENQIS_EXPECTED_NETWORK_ID:-alvenqis-mainnet-candidate}"
miner_address="${ALVENQIS_SMOKE_MINER_ADDRESS:-${POOL_ADDRESS:-}}"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

need_cmd curl
need_cmd python3

health_tmp="$(mktemp)"
status_tmp="$(mktemp)"
mining_tmp="$(mktemp)"
trap 'rm -f "$health_tmp" "$status_tmp" "$mining_tmp"' EXIT

echo "Public candidate smoke against: $base_url"
echo "UTC: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# --- /health ---
health_code="$(
  curl -sS -o "$health_tmp" -w '%{http_code}' \
    --connect-timeout 8 --max-time 20 \
    "${base_url}/health" || true
)"
[[ "$health_code" == "200" ]] || fail "/health HTTP $health_code (want 200); body=$(cat "$health_tmp")"

python3 - "$health_tmp" <<'PY' || fail "/health JSON assertion failed"
import json, sys
path = sys.argv[1]
data = json.loads(open(path, encoding="utf-8").read())
if data.get("ok") is not True:
    raise SystemExit(f"ok != true: {data!r}")
mode = str(data.get("mode") or "")
if "mining disabled" not in mode.lower():
    raise SystemExit(f"mode must mention mining disabled for public profile: {mode!r}")
nid = data.get("network_id")
print(f"health ok mode={mode!r} network_id={nid!r}")
PY

# --- /status ---
status_code="$(
  curl -sS -o "$status_tmp" -w '%{http_code}' \
    --connect-timeout 8 --max-time 20 \
    "${base_url}/status" || true
)"
[[ "$status_code" == "200" ]] || fail "/status HTTP $status_code (want 200); body=$(cat "$status_tmp")"

python3 - "$status_tmp" "$expected_network_id" "$expected_genesis_tip" <<'PY' || fail "/status JSON assertion failed"
import json, sys
path, expect_nid, expect_tip = sys.argv[1], sys.argv[2], sys.argv[3]
data = json.loads(open(path, encoding="utf-8").read())
if data.get("initialized") is not True:
    raise SystemExit(f"initialized != true: {data!r}")
nid = data.get("network_id")
if nid != expect_nid:
    raise SystemExit(f"network_id {nid!r} != {expect_nid!r}")
tip = data.get("tip_hash")
if not tip:
    raise SystemExit(f"missing tip_hash: {data!r}")
height = data.get("height")
# At height 0 the tip must match the pinned genesis; at higher heights only report.
if height == 0 and str(tip).lower() != expect_tip.lower():
    raise SystemExit(f"height 0 tip {tip!r} != pinned genesis {expect_tip!r}")
print(
    f"status ok initialized=true network_id={nid} height={height} tip_hash={tip} "
    f"index_in_sync={data.get('index_in_sync')} index_lag_blocks={data.get('index_lag_blocks')}"
)
PY

# --- /mining/template must return real Alvenqis work ---
[[ -n "$miner_address" ]] || fail "set ALVENQIS_SMOKE_MINER_ADDRESS to a valid Alvenqis address"
mining_code="$(
  curl -sS -o "$mining_tmp" -w '%{http_code}' \
    --connect-timeout 8 --max-time 20 \
    "${base_url}/mining/template?miner_address=${miner_address}" || true
)"
[[ "$mining_code" == 200 ]] || fail "/mining/template HTTP $mining_code; body=$(cat "$mining_tmp")"
python3 - "$mining_tmp" "$expected_network_id" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
if not data.get("template_id") or data.get("network_id") != sys.argv[2]:
    raise SystemExit("invalid Alvenqis mining template")
print(f"mining template ok id={data['template_id']} height={data.get('height')}")
PY

echo "PASS: public Mainnet Candidate smoke OK ($base_url)"
echo "NOTE: This does not prove VPS monorepo revision, backup, or restore drills."
