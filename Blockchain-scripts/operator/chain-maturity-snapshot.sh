#!/usr/bin/env bash
# Task 3 public/local chain maturity snapshot (read-only). No secrets.
set -Eeuo pipefail

rpc="${ALVENQIS_RPC_URL:-https://rpcnode.dohotstudio.com}"
rpc="${rpc%/}"
out_dir="${1:-}"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"

fail() { echo "FAIL: $*" >&2; exit 1; }
command -v curl >/dev/null || fail "curl required"
command -v python3 >/dev/null || fail "python3 required"

echo "Alvenqis Task 3 — chain maturity snapshot"
echo "RPC: $rpc"
echo "UTC: $stamp"

health="$(curl -fsS --connect-timeout 8 --max-time 20 "$rpc/health")" || fail "/health failed"
status="$(curl -fsS --connect-timeout 8 --max-time 20 "$rpc/status")" || fail "/status failed"
idx="$(curl -fsS --connect-timeout 8 --max-time 20 "$rpc/indexer/status" 2>/dev/null || true)"

python3 - "$health" "$status" "$idx" "$rpc" "$stamp" "$out_dir" <<'PY'
import json, sys, pathlib
health, status, idx_raw, rpc, stamp, out_dir = sys.argv[1:7]
h = json.loads(health)
s = json.loads(status)
idx = None
if idx_raw.strip():
    try:
        idx = json.loads(idx_raw)
    except Exception:
        idx = None
if h.get("ok") is not True:
    raise SystemExit("health.ok != true")
if s.get("initialized") is not True:
    raise SystemExit("status.initialized != true")
snap = {
    "utc": stamp,
    "rpc_url": rpc,
    "label": "Mainnet Candidate / Prototype",
    "g4_waiver": False,
    "health_mode": h.get("mode"),
    "network_id": s.get("network_id"),
    "height": s.get("height"),
    "tip_hash": s.get("tip_hash"),
    "block_count": s.get("block_count"),
    "cumulative_work": s.get("cumulative_work"),
    "index_in_sync": s.get("index_in_sync"),
    "index_lag_blocks": s.get("index_lag_blocks"),
    "index_height": s.get("index_height"),
    "indexer_in_sync": (idx or {}).get("in_sync") if idx else None,
    "pass": True,
}
print(f"height={snap['height']} tip={snap['tip_hash']}")
print(f"index_in_sync={snap['index_in_sync']} lag={snap['index_lag_blocks']}")
print(f"mode={snap['health_mode']}")
if out_dir:
    p = pathlib.Path(out_dir)
    p.mkdir(parents=True, exist_ok=True)
    path = p / f"chain-maturity-snapshot-{stamp}.json"
    path.write_text(json.dumps(snap, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")
print("PASS: snapshot captured (rehearsal only; not Mainnet Live).")
PY
