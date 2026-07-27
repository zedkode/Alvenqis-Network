#!/usr/bin/env node
/**
 * Operator maturity health probe for Alvenqis Mainnet Candidate rehearsal hosts.
 *
 * Checks (read-only, public/gateway surface):
 *  - RPC /health + /status (incl. index lag fields when present)
 *  - Pool /api/v1/pool/status + optional /history
 *  - Indexer /indexer/status when available
 *  - Pool block maturity progress vs chain tip
 *
 * Usage:
 *   node scripts/operator/maturity-health.mjs
 *   ALVENQIS_RPC_URL=https://rpcnode.dohotstudio.com \
 *   ALVENQIS_POOL_URL=https://rpcnode.dohotstudio.com/pool \
 *     node scripts/operator/maturity-health.mjs
 *
 * Exit 0 = all critical checks green; 1 = one or more failures.
 */

const RPC = (process.env.ALVENQIS_RPC_URL || "https://rpcnode.dohotstudio.com").replace(/\/$/, "");
const POOL = (process.env.ALVENQIS_POOL_URL || `${RPC}/pool`).replace(/\/$/, "");
// Pool is optional for Task 3 chain maturity unless ALVENQIS_REQUIRE_POOL=1.
const REQUIRE_POOL =
  process.env.ALVENQIS_REQUIRE_POOL === "1" ||
  String(process.env.ALVENQIS_REQUIRE_POOL || "").toLowerCase() === "true";

const failures = [];
const warnings = [];

function ok(label, detail = "") {
  console.log(`  ✓ ${label}${detail ? ` — ${detail}` : ""}`);
}
function fail(label, detail = "") {
  console.log(`  ✗ ${label}${detail ? ` — ${detail}` : ""}`);
  failures.push(`${label}: ${detail}`);
}
function warn(label, detail = "") {
  console.log(`  ! ${label}${detail ? ` — ${detail}` : ""}`);
  warnings.push(`${label}: ${detail}`);
}

async function getJson(url) {
  const res = await fetch(url, { headers: { Accept: "application/json" } });
  const text = await res.text();
  let body = null;
  try {
    body = text ? JSON.parse(text) : null;
  } catch {
    body = { raw: text.slice(0, 200) };
  }
  return { status: res.status, body };
}

function maturity(height, tip, required, statusField) {
  const st = String(statusField || "").toLowerCase();
  if (st.includes("orphan")) return { label: "orphaned", remaining: 0, percent: 0 };
  if (st.includes("mature") && !st.includes("immature")) {
    return { label: "mature", remaining: 0, percent: 100 };
  }
  const req = Math.max(1, required || 12);
  if (tip == null) return { label: "unknown", remaining: req, percent: 0 };
  const matureAt = height + req;
  if (tip >= matureAt) return { label: "mature", remaining: 0, percent: 100 };
  const conf = tip < height ? 0 : Math.min(req, tip - height);
  return {
    label: `immature ${conf}/${req}`,
    remaining: Math.max(0, req - conf),
    percent: Math.round((conf / req) * 100),
    matureAt
  };
}

async function main() {
  console.log("Alvenqis maturity health");
  console.log(`RPC  ${RPC}`);
  console.log(`POOL ${POOL}`);
  console.log("");

  console.log("[RPC]");
  const health = await getJson(`${RPC}/health`);
  if (health.status === 200 && health.body?.ok) {
    ok("health", health.body.network_id || health.body.mode || "");
  } else {
    fail("health", `HTTP ${health.status}`);
  }

  const status = await getJson(`${RPC}/status`);
  if (status.status !== 200) {
    fail("status", `HTTP ${status.status}`);
  } else {
    const s = status.body || {};
    ok(
      "status",
      `height=${s.height} blocks=${s.block_count} label=${s.status_label || "?"}`
    );
    if (s.index_in_sync === true) {
      ok("index_in_sync", `lag=${s.index_lag_blocks ?? 0}`);
    } else if (s.index_in_sync === false) {
      warn("index lag", `lag_blocks=${s.index_lag_blocks} index_height=${s.index_height}`);
    } else {
      warn("index fields", "missing index_in_sync (older RPC binary?)");
    }
    if (!s.initialized) fail("chain initialized", "false");
  }

  console.log("");
  console.log("[Indexer]");
  const idx = await getJson(`${RPC}/indexer/status`);
  if (idx.status === 200) {
    const i = idx.body || {};
    if (i.in_sync) ok("indexer in_sync", `height=${i.indexed_height}`);
    else warn("indexer", `in_sync=${i.in_sync} lag=${i.lag_blocks} height=${i.indexed_height}`);
  } else {
    warn("indexer/status", `HTTP ${idx.status}`);
  }

  console.log("");
  console.log("[Pool]");
  const pool = await getJson(`${POOL}/api/v1/pool/status`);
  if (pool.status !== 200) {
    if (REQUIRE_POOL) {
      fail("pool status", `HTTP ${pool.status}`);
    } else {
      warn(
        "pool status",
        `HTTP ${pool.status} (optional; set ALVENQIS_REQUIRE_POOL=1 to fail)`
      );
    }
  } else {
    const p = pool.body || {};
    ok(
      "pool status",
      `${p.pool_name || "?"} workers=${p.connected_workers} blocks=${p.blocks_found} matured=${p.matured_blocks}`
    );
    ok("upstream", String(p.upstream_status || "?"));
    const tip = status.body?.height ?? null;
    const req = p.block_maturity_confirmations ?? 12;
    const blocks = p.recent_blocks || [];
    let immature = 0;
    let mature = 0;
    for (const b of blocks) {
      const m = maturity(b.height, tip, req, b.status);
      if (m.label.startsWith("immature") || m.label === "unknown") immature++;
      else if (m.label === "mature") mature++;
      console.log(
        `    block h=${b.height} ${m.label}` +
          (m.matureAt != null && m.remaining > 0 ? ` need_tip>=${m.matureAt}` : "")
      );
    }
    if (blocks.length) {
      ok("maturity sample", `immature=${immature} mature_or_ok=${mature}`);
    }
  }

  const hist = await getJson(`${POOL}/api/v1/pool/history`);
  if (hist.status === 200) {
    const h = hist.body || {};
    ok(
      "pool history",
      `blocks=${(h.blocks || []).length} shares=${(h.shares || []).length} accounts=${(h.accounts || []).length}`
    );
  } else if (REQUIRE_POOL) {
    warn("pool history", `HTTP ${hist.status} (deploy newer pool binary)`);
  } else {
    warn("pool history", `HTTP ${hist.status} (optional when pool disabled)`);
  }

  console.log("");
  if (failures.length) {
    console.log(`RESULT: FAIL (${failures.length} critical, ${warnings.length} warnings)`);
    process.exit(1);
  }
  console.log(`RESULT: PASS (${warnings.length} warnings)`);
  process.exit(0);
}

main().catch((err) => {
  console.error("fatal", err);
  process.exit(1);
});
