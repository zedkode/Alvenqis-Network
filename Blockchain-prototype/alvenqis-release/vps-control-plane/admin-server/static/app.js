/* Alvenqis Control Plane — operator UI (observed data only) */
const $ = (id) => document.getElementById(id);

const titles = {
  overview: ["Network operations", "PRIVATE OPERATOR SURFACE"],
  chainView: ["Chain & tip", "CANONICAL CHAIN"],
  servicesView: ["Service health", "HOST UNITS"],
  topology: ["Observed topology", "CONNECTION MAP"],
  nodes: ["VPS inventory", "AUTHENTICATED INVENTORY"],
  enroll: ["Add node", "SECURE ENROLLMENT"],
  invites: ["Invitations", "TOKEN LEDGER"],
  poolView: ["Mining pool", "REFERENCE POOL"],
  bootstrapView: ["Easy-install manifest", "SECRET-FREE GENERATOR"],
  auditView: ["Audit operations", "DURABLE CONTROL LOG"],
};

let lastCommand = "";
let lastInvite = null;
let overviewCache = null;
let summaryCache = null;

function toast(msg, ok = true) {
  const el = $("toast");
  el.textContent = msg;
  el.classList.toggle("ok", ok);
  el.classList.toggle("bad", !ok);
  el.classList.remove("hidden");
  clearTimeout(toast._t);
  toast._t = setTimeout(() => el.classList.add("hidden"), 3200);
}

function showError(msg) {
  $("error").textContent = msg;
  $("error").classList.remove("hidden");
}

function clearError() {
  $("error").classList.add("hidden");
}

function formatHashrate(value) {
  const units = ["H/s", "kH/s", "MH/s", "GH/s", "TH/s"];
  let index = 0;
  let result = Number(value || 0);
  while (result >= 1000 && index < units.length - 1) {
    result /= 1000;
    index += 1;
  }
  return `${result.toFixed(index ? 2 : 0)} ${units[index]}`;
}

function formatTime(unix) {
  if (!unix) return "—";
  return new Date(unix * 1000).toLocaleString();
}

function shortHash(h) {
  if (!h || typeof h !== "string") return "—";
  return h.length > 20 ? `${h.slice(0, 10)}…${h.slice(-8)}` : h;
}

function goView(view) {
  document.querySelectorAll("nav button, .view").forEach((item) => item.classList.remove("active"));
  const btn = document.querySelector(`nav button[data-view="${view}"]`);
  if (btn) btn.classList.add("active");
  const section = $(view);
  if (section) section.classList.add("active");
  const [title, kicker] = titles[view] || ["Network operations", "OPERATOR"];
  $("pageTitle").textContent = title;
  $("pageKicker").textContent = kicker;
  if (view === "invites") void loadInvitations();
  if (view === "auditView") void loadAudit();
  if (view === "enroll") setWizard(lastInvite ? 3 : 1);
}

document.querySelectorAll("nav button").forEach((button) => {
  button.addEventListener("click", () => goView(button.dataset.view));
});
document.querySelectorAll("[data-goto]").forEach((el) => {
  el.addEventListener("click", () => goView(el.dataset.goto));
});

function setWizard(step) {
  document.querySelectorAll(".wz").forEach((el) => {
    const n = Number(el.dataset.step);
    el.classList.toggle("active", n === step);
    el.classList.toggle("done", n < step);
  });
}

function serviceMatrix(services = {}, targetId = "services") {
  const host = $(targetId);
  if (!host) return;
  const entries = Object.entries(services);
  if (!entries.length) {
    host.innerHTML = `<div class="service"><span>no data</span><b>—</b></div>`;
    return;
  }
  host.innerHTML = entries
    .map(([name, state]) => {
      const ok = state === "active" || state === "not-applicable";
      return `<div class="service"><span>${name.replaceAll("_", " ")}</span><b class="${ok ? "active" : "down"}">${state}</b></div>`;
    })
    .join("");
}

function render(data) {
  overviewCache = data;
  const topology = data.topology || {};
  const local = data.local || {};
  $("registered").textContent = topology.registered_node_count ?? 0;
  $("onlineHero").textContent = topology.online_node_count ?? 0;
  $("onlineCount").textContent = topology.online_node_count ?? 0;
  $("links").textContent = topology.direct_validated_connections ?? 0;
  $("miners").textContent = topology.observed_miner_count ?? 0;
  $("hashrate").textContent = formatHashrate(topology.observed_hashrate_hs);
  serviceMatrix(local.services || {});
  serviceMatrix(local.services || {}, "servicesFull");
  Object.entries(local.probes || {}).forEach(([name, probe]) => {
    $("servicesFull").insertAdjacentHTML(
      "beforeend",
      `<div class="service"><span>${escapeHtml(name.replaceAll("_", " "))} · ${probe.latency_ms ?? "—"} ms</span><b class="${probe.healthy ? "active" : "down"}">${probe.configured ? (probe.healthy ? "healthy" : "failed") : "not configured"}</b></div>`
    );
  });

  const status = local.status || {};
  const sync = local.sync || {};
  const indexer = local.indexer || {};
  const p2p = local.p2p || {};
  const mempool = local.mempool || {};

  $("chainBrief").innerHTML = `
    <div><strong>${status.height ?? "—"}</strong><span>CHAIN HEIGHT</span></div>
    <div><strong>${sync.sync_state ?? status.sync_status ?? "unknown"}</strong><span>SYNC</span></div>
    <div><strong>${indexer.indexed_height ?? "—"}</strong><span>INDEXED</span></div>
    <div><strong>${p2p.connected_peer_count ?? 0}</strong><span>P2P PEERS</span></div>`;

  $("chainDetail").innerHTML = [
    ["HEIGHT", status.height ?? "—"],
    ["BLOCK COUNT", status.block_count ?? "—"],
    ["TIP HASH", shortHash(status.tip_hash)],
    ["SYNC", sync.sync_state ?? status.sync_status ?? "unknown"],
    ["P2P PEERS", p2p.connected_peer_count ?? 0],
    ["VALIDATED", p2p.validated_peer_count ?? 0],
    ["BANNED PEERS", p2p.banned_peer_count ?? 0],
    ["INDEXED", indexer.indexed_height ?? "—"],
    ["MEMPOOL", mempool.count ?? mempool.transaction_count ?? "—"],
    ["SUPPLY (atomic)", status.emitted_supply_atomic ?? "—"],
    ["NETWORK", status.network_id ?? data.status_label ?? "—"],
    ["STATUS", status.status_label ?? data.status_label ?? "—"],
  ]
    .map(
      ([k, v]) =>
        `<article class="panel stat-card"><span>${k}</span><strong>${v}</strong></article>`
    )
    .join("");

  $("tipRaw").textContent = status.tip_hash || "—";

  $("fleetPulse").innerHTML = `
    <div class="service"><span>controller</span><b class="active">${local.node_name || "local"}</b></div>
    <div class="service"><span>host</span><b class="active">${local.advertise_host || "—"}</b></div>
    <div class="service"><span>mode</span><b class="active">${data.mode || "—"}</b></div>
    <div class="service"><span>topology mode</span><b>${topology.mode || "—"}</b></div>`;

  renderNodes(topology.nodes || []);
  renderTopology(topology.nodes || []);
}

function renderNodes(nodes) {
  $("nodeRows").innerHTML =
    nodes
      .map((node) => {
        const isLocal = node.node_id === "local-controller";
        return `<tr data-node-id="${escapeAttr(node.node_id)}">
      <td><b>${escapeHtml(node.node_name || "node")}</b><br><small>${escapeHtml(node.advertise_host || "")}</small>
        ${isLocal ? '<span class="badge local">CONTROLLER</span>' : ""}</td>
      <td class="${node.online ? "online-text" : "offline-text"}">${node.online ? "ONLINE" : "STALE"}</td>
      <td>${node.height ?? "—"}</td>
      <td>${node.connected_peers ?? 0}</td>
      <td>${node.mining_peers ?? 0}</td>
      <td>${formatHashrate(node.observed_hashrate_hs)}</td>
      <td>${node.health?.score ?? "—"} <small>${escapeHtml(node.health?.grade || "insufficient-data")}</small></td>
      <td>${formatTime(node.last_seen_unix_seconds)}</td>
      <td class="row-actions">
        <button type="button" class="btn tiny" data-action="detail" data-id="${escapeAttr(node.node_id)}">Detail</button>
        ${
          isLocal
            ? ""
            : `${node.banned
                ? `<button type="button" class="btn tiny" data-action="unban" data-id="${escapeAttr(node.node_id)}">Unban</button>`
                : `<button type="button" class="btn tiny danger" data-action="ban" data-id="${escapeAttr(node.node_id)}">Ban</button>`}
               <button type="button" class="btn tiny danger" data-action="remove" data-id="${escapeAttr(node.node_id)}">Remove</button>`
        }
      </td>
    </tr>`;
      })
      .join("") ||
    `<tr><td colspan="9" class="muted">No enrolled VPS yet. Use <b>Add node</b> to generate an invitation.</td></tr>`;

  $("nodeRows").querySelectorAll("button[data-action]").forEach((btn) => {
    btn.addEventListener("click", () => {
      if (btn.dataset.action === "detail") void openNodeDetail(btn.dataset.id);
      if (btn.dataset.action === "remove") void removeNode(btn.dataset.id);
      if (btn.dataset.action === "ban") void banNode(btn.dataset.id);
      if (btn.dataset.action === "unban") void unbanNode(btn.dataset.id);
    });
  });
}

function renderTopology(nodes) {
  $("topologyCanvas").innerHTML =
    nodes
      .map((node) => {
        const local = node.node_id === "local-controller";
        return `<div class="node ${node.online ? "online" : "offline"} ${local ? "controller" : ""}">
        <div class="node-top">
          <strong>${escapeHtml(node.node_name || "node")}</strong>
          <span class="dot ${node.online ? "on" : ""}"></span>
        </div>
        <span class="mono">${escapeHtml(node.p2p_multiaddr || node.advertise_host || "—")}</span>
        <span>${node.connected_peers ?? 0} connected · ${node.validating_peers ?? 0} validating</span>
        <span>${formatHashrate(node.observed_hashrate_hs)} · h=${node.height ?? "?"}</span>
        <button type="button" class="btn tiny" data-topo-id="${escapeAttr(node.node_id)}">Open</button>
      </div>`;
      })
      .join("") ||
    `<div class="node offline"><strong>No enrolled VPS</strong><span>Generate an invitation from Add node.</span></div>`;

  $("topologyCanvas").querySelectorAll("[data-topo-id]").forEach((btn) => {
    btn.addEventListener("click", () => {
      goView("nodes");
      void openNodeDetail(btn.dataset.topoId);
    });
  });
}

async function openNodeDetail(nodeId) {
  try {
    const res = await fetch(`./api/nodes/${encodeURIComponent(nodeId)}`, { cache: "no-store" });
    const payload = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(payload.error || `HTTP ${res.status}`);
    const { node, report, is_local_controller } = payload;
    $("nodeDetailPanel").classList.remove("hidden");
    $("nodeDetailTitle").textContent = `${node.node_name}${is_local_controller ? " (controller)" : ""}`;
    $("nodeDetailBody").innerHTML = `
      <div><span>Node ID</span><b class="mono">${escapeHtml(node.node_id)}</b></div>
      <div><span>Host</span><b>${escapeHtml(node.advertise_host)}</b></div>
      <div><span>Multiaddr</span><b class="mono">${escapeHtml(node.p2p_multiaddr || "—")}</b></div>
      <div><span>Peer ID</span><b class="mono">${escapeHtml(node.peer_id || "—")}</b></div>
      <div><span>Online</span><b class="${node.online ? "online-text" : "offline-text"}">${node.online ? "YES" : "NO"}</b></div>
      <div><span>Banned</span><b>${node.banned ? escapeHtml(node.ban_reason || "YES") : "NO"}</b></div>
      <div><span>Uptime / rating</span><b>${node.health?.uptime_percent ?? "—"}% / ${node.health?.score ?? "—"} (${escapeHtml(node.health?.grade || "insufficient-data")})</b></div>
      <div><span>Height</span><b>${node.height ?? "—"}</b></div>
      <div><span>Connected peers</span><b>${node.connected_peers ?? 0}</b></div>
      <div><span>Validating</span><b>${node.validating_peers ?? 0}</b></div>
      <div><span>Mining peers</span><b>${node.mining_peers ?? 0}</b></div>
      <div><span>Hashrate</span><b>${formatHashrate(node.observed_hashrate_hs)}</b></div>
      <div><span>Last seen</span><b>${formatTime(node.last_seen_unix_seconds)}</b></div>
      <div><span>Services</span><b>${Object.entries(node.services || {})
        .map(([k, v]) => `${k}:${v}`)
        .join(" · ")}</b></div>
      <div class="full"><span>Status JSON</span><pre class="code-block small">${escapeHtml(
        JSON.stringify(report.status || {}, null, 2)
      )}</pre></div>
      <div class="full"><span>P2P JSON</span><pre class="code-block small">${escapeHtml(
        JSON.stringify(report.p2p || {}, null, 2)
      )}</pre></div>`;
  } catch (e) {
    showError(e.message);
  }
}

$("closeNodeDetail").addEventListener("click", () => {
  $("nodeDetailPanel").classList.add("hidden");
});

async function removeNode(nodeId) {
  if (!confirm(`Remove node ${nodeId} from inventory? This does not wipe the remote host.`)) return;
  try {
    const res = await controlFetch(`./api/nodes/${encodeURIComponent(nodeId)}`, { method: "DELETE" });
    if (!res.ok) {
      const p = await res.json().catch(() => ({}));
      throw new Error(p.error || `HTTP ${res.status}`);
    }
    toast("Node removed from inventory");
    await refresh();
  } catch (e) {
    showError(e.message);
  }
}

function idempotencyKey() {
  return globalThis.crypto?.randomUUID?.() || `admin-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function controlFetch(url, options = {}) {
  return fetch(url, {
    ...options,
    headers: { ...(options.headers || {}), "Idempotency-Key": idempotencyKey() },
  });
}

async function banNode(nodeId) {
  const reason = prompt("Ban reason (stored in audit log):");
  if (!reason) return;
  await nodeControl(nodeId, "ban", { reason });
}

async function unbanNode(nodeId) {
  if (!confirm(`Unban node ${nodeId}?`)) return;
  await nodeControl(nodeId, "unban");
}

async function nodeControl(nodeId, action, payload) {
  try {
    const res = await controlFetch(`./api/nodes/${encodeURIComponent(nodeId)}/${action}`, {
      method: "POST",
      headers: payload ? { "content-type": "application/json" } : {},
      body: payload ? JSON.stringify(payload) : undefined,
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
    toast(`Node ${action} applied`);
    await refresh();
  } catch (error) {
    showError(error.message);
  }
}

async function fetchPool() {
  try {
    const response = await fetch("/pool/api/v1/pool/status", { cache: "no-store" });
    if (!response.ok) return null;
    return await response.json();
  } catch {
    return null;
  }
}

function renderPool(pool) {
  const host = $("poolPanel");
  if (!host) return;
  if (!pool) {
    host.innerHTML = `<div class="service"><span>pool</span><b class="down">unreachable</b></div>`;
    $("poolMetrics").innerHTML = "";
    $("poolDetail").innerHTML = `<div class="service"><span>status</span><b class="down">unreachable</b></div>`;
    return;
  }
  host.innerHTML = `
    <div class="service"><span>upstream</span><b class="${pool.upstream_status === "healthy" ? "active" : "down"}">${pool.upstream_status || "unknown"}</b></div>
    <div class="service"><span>workers</span><b class="active">${pool.connected_workers ?? 0}</b></div>
    <div class="service"><span>hashrate</span><b class="active">${formatHashrate(pool.estimated_hashrate_hs)}</b></div>
    <div class="service"><span>blocks found</span><b class="active">${pool.blocks_found ?? 0}</b></div>
    <div class="service"><span>shares</span><b class="active">${pool.accepted_shares ?? 0}</b></div>
    <div class="service"><span>name</span><b class="active">${escapeHtml(pool.pool_name || "—")}</b></div>`;

  $("poolMetrics").innerHTML = [
    ["Workers", pool.connected_workers ?? 0],
    ["Hashrate", formatHashrate(pool.estimated_hashrate_hs)],
    ["Blocks", pool.blocks_found ?? 0],
    ["Matured", pool.matured_blocks ?? 0],
    ["Shares", pool.accepted_shares ?? 0],
    ["Bans", pool.active_bans ?? 0],
  ]
    .map(
      ([k, v]) =>
        `<article><span>${k}</span><strong>${v}</strong><small>pool</small></article>`
    )
    .join("");

  $("poolDetail").innerHTML = `
    <div class="service"><span>pool address</span><b class="mono">${escapeHtml(pool.pool_address || "—")}</b></div>
    <div class="service"><span>scheme</span><b>${escapeHtml(pool.payout_scheme || "—")}</b></div>
    <div class="service"><span>fee bps</span><b>${pool.pool_fee_basis_points ?? "—"}</b></div>
    <div class="service"><span>maturity</span><b>${pool.block_maturity_confirmations ?? "—"} conf</b></div>
    <div class="service"><span>vardiff</span><b>${pool.vardiff_enabled ? "on" : "off"}</b></div>
    <div class="service"><span>target share</span><b>${pool.target_share_seconds ?? "—"}s</b></div>`;
}

async function loadInvitations() {
  try {
    const res = await fetch("./api/invitations", { cache: "no-store" });
    const list = await res.json();
    if (!res.ok) throw new Error(list.error || `HTTP ${res.status}`);
    $("pendingInvites").textContent = list.filter((i) => i.status === "pending").length;
    $("inviteRows").innerHTML =
      list
        .map((inv) => {
          const st = inv.status;
          return `<tr>
          <td class="mono">${escapeHtml(inv.invitation_id.slice(0, 12))}…</td>
          <td>${escapeHtml(inv.node_name)}</td>
          <td>${escapeHtml(inv.advertise_host)}</td>
          <td><span class="badge ${st}">${st.toUpperCase()}</span></td>
          <td>${formatTime(inv.expires_at_unix_seconds)}</td>
          <td>${
            st === "pending"
              ? `<button type="button" class="btn tiny danger" data-revoke="${escapeAttr(inv.invitation_id)}">Revoke</button>`
              : "—"
          }</td>
        </tr>`;
        })
        .join("") || `<tr><td colspan="6" class="muted">No invitations yet.</td></tr>`;

    $("inviteRows").querySelectorAll("[data-revoke]").forEach((btn) => {
      btn.addEventListener("click", () => void revokeInvite(btn.dataset.revoke));
    });
  } catch (e) {
    showError(e.message);
  }
}

async function revokeInvite(id) {
  if (!confirm("Revoke this invitation? Existing install scripts with this token will fail.")) return;
  const res = await controlFetch(`./api/invitations/${encodeURIComponent(id)}`, { method: "DELETE" });
  if (!res.ok) {
    const p = await res.json().catch(() => ({}));
    showError(p.error || `HTTP ${res.status}`);
    return;
  }
  toast("Invitation revoked");
  await loadInvitations();
  await refreshSummary();
}

async function refreshSummary() {
  try {
    const res = await fetch("./api/fleet/summary", { cache: "no-store" });
    if (!res.ok) return;
    summaryCache = await res.json();
    $("pendingInvites").textContent = summaryCache.pending_invitations ?? 0;
    if (summaryCache.advertise_host) {
      $("truthMeta").textContent = `${summaryCache.controller || "controller"} · ${summaryCache.advertise_host}`;
      $("publicRpcLink").href = `https://${summaryCache.advertise_host}`;
    }
    if (!$("adminDomain").value && summaryCache.advertise_host) {
      $("adminDomain").placeholder = summaryCache.advertise_host;
    }
  } catch {
    /* ignore */
  }
}

function renderInviteResult(payload) {
  lastInvite = payload;
  lastCommand = payload.install_command || "";
  $("installCommand").textContent = lastCommand;
  $("copyCommand").disabled = !lastCommand;
  $("inviteMeta").innerHTML = `
    <span class="badge pending">PENDING</span>
    <b>${escapeHtml(payload.node_name)}</b> → <span class="mono">${escapeHtml(payload.advertise_host)}</span>
    · seed <span class="mono">${escapeHtml(payload.seed_multiaddr || "")}</span>`;
  $("inviteExpiry").textContent = `Expires ${formatTime(payload.expires_at_unix_seconds)} · TTL ${payload.ttl_seconds || "?"}s · single-use`;
  $("enrollSteps").innerHTML = (payload.steps || [])
    .map(
      (step, i) => `<li class="${i === 2 ? "focus" : ""}">
      <strong>${escapeHtml(step.title)}</strong>
      <p>${escapeHtml(step.detail)}</p>
      ${step.code ? `<pre class="code-block small">${escapeHtml(step.code)}</pre>` : ""}
    </li>`
    )
    .join("");
  setWizard(3);
}

async function refresh() {
  $("refreshState").innerHTML = "<i></i> REFRESHING";
  try {
    const [response, pool] = await Promise.all([
      fetch("./api/overview", { cache: "no-store" }),
      fetchPool(),
    ]);
    if (!response.ok) {
      const payload = await response.json().catch(() => ({}));
      throw new Error(payload.error || `HTTP ${response.status}`);
    }
    render(await response.json());
    renderPool(pool);
    await refreshSummary();
    clearError();
    $("refreshState").innerHTML = "<i></i> LIVE";
  } catch (error) {
    showError(error.message);
    $("refreshState").innerHTML = "<i></i> DEGRADED";
  }
}

$("refresh").addEventListener("click", () => void refresh());

$("inviteForm").addEventListener("submit", async (event) => {
  event.preventDefault();
  $("genInviteBtn").disabled = true;
  setWizard(2);
  try {
    const response = await controlFetch("./api/invitations", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        role: $("inviteRole").value,
        node_name: $("nodeName").value.trim(),
        advertise_host: $("advertiseHost").value.trim(),
        admin_domain: $("adminDomain").value.trim() || null,
        acme_email: $("acmeEmail").value.trim(),
      }),
    });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || "Invitation failed");
    renderInviteResult(payload);
    toast("Invitation created — copy the install script");
    await loadInvitations();
    await refreshSummary();
  } catch (e) {
    showError(e.message);
    setWizard(1);
  } finally {
    $("genInviteBtn").disabled = false;
  }
});

$("manifestForm").addEventListener("submit", async (event) => {
  event.preventDefault();
  try {
    const response = await fetch("./api/bootstrap/manifests", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        role: $("manifestRole").value,
        node_name: $("manifestNodeName").value.trim(),
        advertise_host: $("manifestHost").value.trim(),
        acme_email: $("manifestEmail").value.trim(),
        admin_domain: $("manifestAdminDomain").value.trim() || null,
      }),
    });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || `HTTP ${response.status}`);
    $("manifestOutput").textContent = JSON.stringify(payload, null, 2);
    toast("Secret-free manifest generated");
  } catch (error) {
    showError(error.message);
  }
});

async function loadAudit() {
  try {
    const response = await fetch("./api/audit", { cache: "no-store" });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || `HTTP ${response.status}`);
    $("auditRows").innerHTML = payload.map((entry) => `<tr>
      <td>${formatTime(entry.occurred_at_unix_seconds)}</td>
      <td>${escapeHtml(entry.actor)}</td><td>${escapeHtml(entry.action)}</td>
      <td class="mono">${escapeHtml(entry.target)}</td><td>${escapeHtml(entry.outcome)}</td>
      <td class="mono">${escapeHtml(entry.operation_id)}</td></tr>`).join("") ||
      '<tr><td colspan="6" class="muted">No control mutations recorded.</td></tr>';
  } catch (error) {
    showError(error.message);
  }
}

$("refreshAudit").addEventListener("click", () => void loadAudit());

$("copyCommand").addEventListener("click", async () => {
  if (!lastCommand) return;
  try {
    await navigator.clipboard.writeText(lastCommand);
    toast("Install script copied");
    setWizard(4);
  } catch {
    toast("Copy failed — select the script manually", false);
  }
});

function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
function escapeAttr(s) {
  return escapeHtml(s).replace(/'/g, "&#39;");
}

void refresh();
setInterval(() => void refresh(), 12000);
