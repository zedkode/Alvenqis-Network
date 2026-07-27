import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ExternalLink,
  Layers,
  Plus,
  RefreshCw,
  Server,
  Star,
  Trash2,
  Users
} from "lucide-react";
import { formatAtomic, formatHashrate, formatTimestamp, shortHash } from "@shared/format";
import type {
  PoolBlockRow,
  PoolCatalog,
  PoolCatalogEntry,
  PoolPayoutRow,
  PoolShareRow,
  PoolSnapshot,
  PoolWorkerRow
} from "@shared/types";
import { AddressChip } from "../components/ui/AddressChip";
import { CopyField } from "../components/ui/CopyField";
import { DetailDialog } from "../components/ui/DetailDialog";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { KeyValue, Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { useAppSettings } from "../hooks/useAppSettings";
import { useApp } from "../model";

type TabId = "overview" | "workers" | "blocks" | "shares" | "payouts" | "accounts";

function asNum(v: unknown): number {
  if (typeof v === "number" && Number.isFinite(v)) return v;
  if (typeof v === "string" && v.trim()) {
    const n = Number(v);
    return Number.isFinite(n) ? n : 0;
  }
  return 0;
}

function asStr(v: unknown): string {
  return typeof v === "string" ? v : v == null ? "" : String(v);
}

function ageLabel(unix: number): string {
  if (!unix) return "—";
  const s = Math.max(0, Math.floor(Date.now() / 1000) - unix);
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86_400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86_400)}d ago`;
}

function statusTone(status: string): string {
  const s = status.toLowerCase();
  if (s.includes("mature") || s.includes("healthy") || s === "submitted") return "positive";
  if (s.includes("immature") || s.includes("prepared") || s.includes("degraded")) return "gold";
  if (s.includes("orphan") || s.includes("cancel") || s.includes("error")) return "danger";
  return "";
}

/** Pool maturity: tip must reach height + required confirmations. */
function maturityInfo(
  blockHeight: number,
  tipHeight: number | null | undefined,
  required: number,
  status: string
): {
  label: string;
  conf: number;
  required: number;
  remaining: number;
  percent: number;
  mature: boolean;
  orphaned: boolean;
} {
  const st = status.toLowerCase();
  const orphaned = st.includes("orphan");
  const forcedMature = st === "mature" || st.includes("matured");
  const req = Math.max(1, required || 12);
  if (orphaned) {
    return { label: "orphaned", conf: 0, required: req, remaining: 0, percent: 0, mature: false, orphaned: true };
  }
  if (forcedMature || (tipHeight != null && tipHeight >= blockHeight + req)) {
    return {
      label: "mature",
      conf: req,
      required: req,
      remaining: 0,
      percent: 100,
      mature: true,
      orphaned: false
    };
  }
  const conf =
    tipHeight == null || tipHeight < blockHeight
      ? 0
      : Math.min(req, Math.max(0, tipHeight - blockHeight));
  const remaining = Math.max(0, req - conf);
  const percent = Math.round((conf / req) * 100);
  return {
    label: `immature · ${conf}/${req}`,
    conf,
    required: req,
    remaining,
    percent,
    mature: false,
    orphaned: false
  };
}

function MaturityBar({
  blockHeight,
  tipHeight,
  required,
  status
}: {
  blockHeight: number;
  tipHeight: number | null | undefined;
  required: number;
  status: string;
}) {
  const m = maturityInfo(blockHeight, tipHeight, required, status);
  return (
    <div className={`maturity-meter ${m.orphaned ? "is-orphan" : m.mature ? "is-mature" : "is-immature"}`}>
      <div className="maturity-meter-top">
        <span className={statusTone(m.label)}>{m.label}</span>
        {!m.orphaned && !m.mature ? (
          <span className="muted mono">
            tip {tipHeight ?? "—"} · need ≥{blockHeight + m.required}
          </span>
        ) : null}
        {m.mature ? <span className="muted mono">payout-eligible when paid</span> : null}
        {m.orphaned ? <span className="muted">reorg / non-canonical</span> : null}
      </div>
      <div className="maturity-meter-track" aria-hidden="true">
        <i style={{ width: `${m.percent}%` }} />
      </div>
      {!m.orphaned && !m.mature ? (
        <small className="muted">
          {m.remaining} confirmation{m.remaining === 1 ? "" : "s"} left before mature
        </small>
      ) : null}
    </div>
  );
}

export function Pool() {
  const { wallet, setNotice, snapshot: network } = useApp();
  const { settings, update } = useAppSettings();
  const [catalog, setCatalog] = useState<PoolCatalog | null>(null);
  const [snapshot, setSnapshot] = useState<PoolSnapshot | null>(null);
  const [selectedUrl, setSelectedUrl] = useState(settings.default_pool_url);
  const [newPoolUrl, setNewPoolUrl] = useState("");
  const [tab, setTab] = useState<TabId>("overview");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [workerDetail, setWorkerDetail] = useState<PoolWorkerRow | null>(null);
  const [blockDetail, setBlockDetail] = useState<PoolBlockRow | null>(null);
  const [shareDetail, setShareDetail] = useState<PoolShareRow | null>(null);
  const [payoutDetail, setPayoutDetail] = useState<PoolPayoutRow | null>(null);

  const poolList = useMemo(() => {
    const fromSettings = settings.pool_urls?.length
      ? settings.pool_urls
      : settings.default_pool_url
        ? [settings.default_pool_url]
        : [];
    const fromCatalog = catalog?.pools.map((p) => p.pool_url) ?? [];
    const merged = [...fromSettings, ...fromCatalog, settings.default_pool_url].filter(Boolean);
    const seen = new Set<string>();
    return merged.filter((u) => {
      const key = u.trim().toLowerCase().replace(/\/$/, "");
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }, [settings.pool_urls, settings.default_pool_url, catalog]);

  const loadAll = useCallback(
    async (url?: string) => {
      const target = (url ?? selectedUrl ?? settings.default_pool_url).trim();
      if (!target) {
        setError("Configure at least one pool URL.");
        return;
      }
      setBusy(true);
      setError(null);
      try {
        const [cat, snap] = await Promise.all([
          window.alvenqis.pool.catalog(),
          window.alvenqis.pool.snapshot(target, wallet?.address ?? null)
        ]);
        setCatalog(cat);
        setSnapshot(snap);
        setSelectedUrl(snap.pool_url || target);
      } catch (err) {
        setSnapshot(null);
        setError(String(err).replace(/^Error:\s*/i, ""));
      } finally {
        setBusy(false);
      }
    },
    [selectedUrl, settings.default_pool_url, wallet?.address]
  );

  useEffect(() => {
    void loadAll(settings.default_pool_url);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- initial load + when default changes
  }, [settings.default_pool_url, wallet?.address]);

  useEffect(() => {
    const ms = Math.max(5_000, settings.refresh_interval_ms || 5_000);
    const timer = window.setInterval(() => {
      void loadAll(selectedUrl);
    }, ms);
    return () => window.clearInterval(timer);
  }, [loadAll, selectedUrl, settings.refresh_interval_ms]);

  const selectPool = async (url: string) => {
    setSelectedUrl(url);
    await loadAll(url);
  };

  const setAsDefault = async (url: string) => {
    const list = Array.from(
      new Set([url, ...(settings.pool_urls ?? []), settings.default_pool_url].filter(Boolean))
    );
    await update({ default_pool_url: url, pool_urls: list });
    setNotice({ error: false, text: `Default pool set to ${url}` });
    await selectPool(url);
  };

  const addPool = async () => {
    const raw = newPoolUrl.trim().replace(/\/$/, "");
    if (!raw) return;
    const next = Array.from(
      new Set([...(settings.pool_urls ?? []), settings.default_pool_url, raw].filter(Boolean))
    );
    await update({
      pool_urls: next,
      default_pool_url: settings.default_pool_url || raw
    });
    setNewPoolUrl("");
    setNotice({ error: false, text: `Pool added: ${raw}` });
    await selectPool(raw);
  };

  const removePool = async (url: string) => {
    const next = (settings.pool_urls ?? []).filter(
      (u) => u.trim().toLowerCase().replace(/\/$/, "") !== url.trim().toLowerCase().replace(/\/$/, "")
    );
    let defaultUrl = settings.default_pool_url;
    if (defaultUrl.trim().toLowerCase().replace(/\/$/, "") === url.trim().toLowerCase().replace(/\/$/, "")) {
      defaultUrl = next[0] ?? "";
    }
    await update({ pool_urls: next, default_pool_url: defaultUrl });
    setNotice({ error: false, text: `Removed pool ${url}` });
    if (defaultUrl) await selectPool(defaultUrl);
    else {
      setSnapshot(null);
      setSelectedUrl("");
    }
  };

  const status = snapshot?.status ?? {};
  const workers = (snapshot?.workers ?? []) as PoolWorkerRow[];
  const blocks = (snapshot?.blocks ?? []) as PoolBlockRow[];
  const shares = (snapshot?.shares ?? []) as PoolShareRow[];
  const payouts = (snapshot?.payouts ?? []) as PoolPayoutRow[];
  const accounts = snapshot?.accounts ?? [];
  const onlineWorkers = workers.filter((w) => w.online).length;
  const maturityRequired = asNum(status.block_maturity_confirmations) || 12;
  const chainTip = network.height;
  const maturitySummary = useMemo(() => {
    let immature = 0;
    let mature = 0;
    let orphaned = 0;
    let nearestRemaining = Number.POSITIVE_INFINITY;
    for (const b of blocks) {
      const m = maturityInfo(b.height, chainTip, maturityRequired, String(b.status));
      if (m.orphaned) orphaned += 1;
      else if (m.mature) mature += 1;
      else {
        immature += 1;
        nearestRemaining = Math.min(nearestRemaining, m.remaining);
      }
    }
    return {
      immature,
      mature,
      orphaned,
      nearestRemaining: Number.isFinite(nearestRemaining) ? nearestRemaining : null
    };
  }, [blocks, chainTip, maturityRequired]);
  const catalogEntry: PoolCatalogEntry | undefined = catalog?.pools.find(
    (p) =>
      p.pool_url.trim().toLowerCase().replace(/\/$/, "") ===
      selectedUrl.trim().toLowerCase().replace(/\/$/, "")
  );

  const tabs: Array<{ id: TabId; label: string; count?: number }> = [
    { id: "overview", label: "Overview" },
    { id: "workers", label: "Workers", count: workers.length },
    { id: "blocks", label: "Blocks", count: blocks.length },
    { id: "shares", label: "Shares", count: shares.length },
    { id: "payouts", label: "Payouts", count: payouts.length },
    { id: "accounts", label: "Accounts", count: accounts.length }
  ];

  return (
    <div className="page grid pool-page">
      <PageHero
        kicker="MINING POOL · PUBLIC NETWORK VIEW"
        title="Pool"
        titleAccent="network"
        description="Live multi-pool control surface: workers, block finds, share history, payouts and balances from public pool APIs. No admin tokens — read-only network data."
        actions={
          <button
            className="button primary"
            type="button"
            disabled={busy}
            onClick={() => void loadAll(selectedUrl)}
          >
            <RefreshCw size={15} /> {busy ? "Refreshing…" : "Refresh"}
          </button>
        }
        side={
          <>
            <div className="page-hero-metric">
              <small>Workers</small>
              <strong>
                {onlineWorkers}/{workers.length}
              </strong>
            </div>
            <div className="page-hero-metric">
              <small>Pool H/s</small>
              <strong style={{ fontSize: 14 }}>
                {formatHashrate(asNum(status.estimated_hashrate_hs ?? snapshot?.status?.estimated_hashrate_hs))}
              </strong>
            </div>
            <div className="page-hero-metric">
              <small>Blocks</small>
              <strong>{asNum(status.blocks_found)}</strong>
            </div>
          </>
        }
      />

      <Panel title="Pool selection" detail="Multi-pool · set default for miner">
        <div className="pool-select-grid">
          {poolList.map((url) => {
            const entry = catalog?.pools.find(
              (p) =>
                p.pool_url.trim().toLowerCase().replace(/\/$/, "") ===
                url.trim().toLowerCase().replace(/\/$/, "")
            );
            const active =
              selectedUrl.trim().toLowerCase().replace(/\/$/, "") ===
              url.trim().toLowerCase().replace(/\/$/, "");
            const isDefault =
              settings.default_pool_url.trim().toLowerCase().replace(/\/$/, "") ===
              url.trim().toLowerCase().replace(/\/$/, "");
            return (
              <article
                key={url}
                className={`pool-card ${active ? "active" : ""} ${entry?.online ? "online" : "offline"}`}
              >
                <header>
                  <Server size={16} />
                  <div>
                    <strong>{entry?.pool_name || "Pool"}</strong>
                    <small className="mono">{url}</small>
                  </div>
                  <span className={`hw-badge ${entry?.online ? "on" : ""}`}>
                    {entry?.online ? "online" : entry ? "offline" : "…"}
                  </span>
                </header>
                <div className="pool-card-stats">
                  <span>
                    Workers <b>{entry?.connected_workers ?? "—"}</b>
                  </span>
                  <span>
                    H/s <b>{entry?.estimated_hashrate_hs != null ? formatHashrate(entry.estimated_hashrate_hs) : "—"}</b>
                  </span>
                  <span>
                    Blocks <b>{entry?.blocks_found ?? "—"}</b>
                  </span>
                </div>
                <div className="button-row" style={{ marginTop: 10 }}>
                  <button
                    className="button primary"
                    type="button"
                    disabled={busy}
                    onClick={() => void selectPool(url)}
                  >
                    {active ? "Selected" : "Open"}
                  </button>
                  <button
                    className="button ghost"
                    type="button"
                    title="Use as miner default"
                    onClick={() => void setAsDefault(url)}
                  >
                    <Star size={14} /> {isDefault ? "Default" : "Set default"}
                  </button>
                  <button
                    className="button ghost"
                    type="button"
                    title="Remove from list"
                    onClick={() => void removePool(url)}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </article>
            );
          })}
        </div>

        <div className="explorer-search-row" style={{ marginTop: 14 }}>
          <input
            value={newPoolUrl}
            onChange={(e) => setNewPoolUrl(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void addPool()}
            placeholder="http://rpcnode.dohotstudio.com/pool or http://host:port"
            spellCheck={false}
          />
          <button className="button" type="button" onClick={() => void addPool()}>
            <Plus size={15} /> Add pool
          </button>
        </div>
        <p className="field-hint" style={{ marginTop: 8 }}>
          Default pool drives Miner → pool mode. History uses public{" "}
          <code className="mono">/api/v1/pool/status</code> +{" "}
          <code className="mono">/api/v1/pool/history</code> when available.
        </p>
      </Panel>

      {error ? <p className="hw-error">{error}</p> : null}

      {snapshot ? (
        <>
          <div className="grid cols-5">
            <StatCard
              label="Status"
              value={asStr(status.upstream_status) || (snapshot.online ? "online" : "offline")}
              detail={asStr(status.status_label) || asStr(status.pool_name)}
              tone={snapshot.online ? "positive" : undefined}
            />
            <StatCard
              label="Workers online"
              value={`${onlineWorkers}`}
              detail={`${workers.length} known`}
            />
            <StatCard
              label="Hashrate"
              value={formatHashrate(asNum(status.estimated_hashrate_hs))}
              detail="Pool aggregate"
              tone="gold"
            />
            <StatCard
              label="Accepted shares"
              value={asNum(status.accepted_shares)}
              detail={snapshot.history_available ? "counter + history" : "counter only"}
            />
            <StatCard
              label="Blocks / mature"
              value={`${asNum(status.blocks_found)} / ${asNum(status.matured_blocks)}`}
              detail={`fee ${asNum(status.pool_fee_basis_points)} bp · ${asStr(status.payout_scheme) || "—"}`}
            />
          </div>

          <Panel
            title="Block maturity"
            detail={`chain tip ${chainTip ?? "—"} · need ${maturityRequired} confirmations`}
          >
            <p className="muted" style={{ margin: "0 0 12px", lineHeight: 1.5, fontSize: 13 }}>
              New pool finds start as <strong className="gold">immature</strong>. They become{" "}
              <strong className="positive">mature</strong> only when the chain tip reaches{" "}
              <code className="mono">height + {maturityRequired}</code>. This protects payouts against
              short reorgs — it is expected, not a bug.
            </p>
            <div className="grid cols-4">
              <StatCard label="Immature" value={maturitySummary.immature} tone="gold" detail="waiting confs" />
              <StatCard label="Mature" value={maturitySummary.mature} tone="positive" detail="payout-ready path" />
              <StatCard label="Orphaned" value={maturitySummary.orphaned} detail="non-canonical" />
              <StatCard
                label="Next mature in"
                value={
                  maturitySummary.nearestRemaining != null
                    ? `${maturitySummary.nearestRemaining}`
                    : "—"
                }
                detail="confirmations (nearest block)"
              />
            </div>
            {blocks.length ? (
              <div className="maturity-list" style={{ marginTop: 14 }}>
                {blocks.slice(0, 8).map((b) => (
                  <div key={b.hash} className="maturity-list-row">
                    <button
                      type="button"
                      className="linkish mono"
                      onClick={() => setBlockDetail(b)}
                    >
                      #{b.height}
                    </button>
                    <MaturityBar
                      blockHeight={b.height}
                      tipHeight={chainTip}
                      required={maturityRequired}
                      status={String(b.status)}
                    />
                  </div>
                ))}
              </div>
            ) : (
              <EmptyState>No pool blocks yet — mature progress appears after the first find.</EmptyState>
            )}
          </Panel>

          <div className="pool-tabs" role="tablist">
            {tabs.map((t) => (
              <button
                key={t.id}
                type="button"
                role="tab"
                className={tab === t.id ? "active" : ""}
                aria-selected={tab === t.id}
                onClick={() => setTab(t.id)}
              >
                {t.label}
                {t.count != null ? <span className="mono">{t.count}</span> : null}
              </button>
            ))}
          </div>

          {tab === "overview" ? (
            <div className="grid cols-2">
              <Panel title="Pool identity" detail={snapshot.history_available ? "history API ok" : "status only"}>
                <div className="detail-grid detail-rich">
                  <KeyValue label="Name">{asStr(status.pool_name) || "—"}</KeyValue>
                  <KeyValue label="Network">{asStr(status.network_id) || "—"}</KeyValue>
                  <KeyValue label="Protocol">{asStr(status.protocol) || "—"}</KeyValue>
                  <KeyValue label="Mode">{asStr(status.mode) || "—"}</KeyValue>
                  <KeyValue label="Upstream">
                    <span className={statusTone(asStr(status.upstream_status))}>
                      {asStr(status.upstream_status) || "—"}
                    </span>
                  </KeyValue>
                  <KeyValue label="Vardiff">
                    {status.vardiff_enabled ? `on · target ${asNum(status.target_share_seconds)}s` : "off"}
                  </KeyValue>
                  <KeyValue label="Min payout" mono>
                    {formatAtomic(String(asNum(status.minimum_payout_atomic)))} ALVE
                  </KeyValue>
                  <KeyValue label="Maturity">{asNum(status.block_maturity_confirmations)} conf</KeyValue>
                  <div className="detail-span-full">
                    <KeyValue label="Pool coinbase address">
                      {asStr(status.pool_address) ? (
                        <AddressChip value={asStr(status.pool_address)} full />
                      ) : (
                        "—"
                      )}
                    </KeyValue>
                  </div>
                  <div className="detail-span-full">
                    <KeyValue label="Endpoint">
                      <CopyField value={snapshot.pool_url} label="pool url" />
                    </KeyValue>
                  </div>
                  {asStr(status.upstream_error) ? (
                    <div className="detail-span-full">
                      <KeyValue label="Upstream error">{asStr(status.upstream_error)}</KeyValue>
                    </div>
                  ) : null}
                  <KeyValue label="Rejected req">{asNum(status.rejected_requests)}</KeyValue>
                  <KeyValue label="Rate limited">{asNum(status.rate_limited_requests)}</KeyValue>
                  <KeyValue label="Active bans">{asNum(status.active_bans)}</KeyValue>
                  <KeyValue label="Fetched">{ageLabel(snapshot.fetched_at_unix_seconds)}</KeyValue>
                </div>
              </Panel>

              <Panel title="Your wallet on this pool" detail={wallet?.address ? "miner API" : "select wallet"}>
                {wallet?.address && snapshot.miner ? (
                  <div className="detail-grid detail-rich">
                    <div className="detail-span-full">
                      <KeyValue label="Address">
                        <AddressChip value={wallet.address} full />
                      </KeyValue>
                    </div>
                    <KeyValue label="Immature" mono>
                      {formatAtomic(
                        String(asNum((snapshot.miner as { balance?: { immature_atomic?: unknown } }).balance?.immature_atomic))
                      )}{" "}
                      ALVE
                    </KeyValue>
                    <KeyValue label="Mature" mono>
                      {formatAtomic(
                        String(asNum((snapshot.miner as { balance?: { mature_atomic?: unknown } }).balance?.mature_atomic))
                      )}{" "}
                      ALVE
                    </KeyValue>
                    <KeyValue label="Pending payout" mono>
                      {formatAtomic(
                        String(
                          asNum(
                            (snapshot.miner as { balance?: { pending_payout_atomic?: unknown } }).balance
                              ?.pending_payout_atomic
                          )
                        )
                      )}{" "}
                      ALVE
                    </KeyValue>
                    <KeyValue label="Paid" mono>
                      {formatAtomic(
                        String(asNum((snapshot.miner as { balance?: { paid_atomic?: unknown } }).balance?.paid_atomic))
                      )}{" "}
                      ALVE
                    </KeyValue>
                    <KeyValue label="Your workers">
                      {Array.isArray((snapshot.miner as { workers?: unknown[] }).workers)
                        ? (snapshot.miner as { workers: unknown[] }).workers.length
                        : 0}
                    </KeyValue>
                    <KeyValue label="Your payouts">
                      {Array.isArray((snapshot.miner as { payouts?: unknown[] }).payouts)
                        ? (snapshot.miner as { payouts: unknown[] }).payouts.length
                        : 0}
                    </KeyValue>
                  </div>
                ) : (
                  <EmptyState>
                    {wallet?.address
                      ? "No pool account yet for this wallet (mine with pool mode to appear)."
                      : "Select or create a wallet to load personal pool balances."}
                  </EmptyState>
                )}
                <div className="button-row" style={{ marginTop: 12 }}>
                  <button
                    className="button"
                    type="button"
                    onClick={() => window.open(snapshot.pool_url, "_blank", "noopener,noreferrer")}
                  >
                    <ExternalLink size={14} /> Open pool web UI
                  </button>
                </div>
              </Panel>

              <Panel title="Live workers (preview)" detail={`${onlineWorkers} online`}>
                {workers.length ? (
                  <table className="data-table interactive-table">
                    <thead>
                      <tr>
                        <th>Worker</th>
                        <th>Address</th>
                        <th>H/s</th>
                        <th>Shares</th>
                        <th></th>
                      </tr>
                    </thead>
                    <tbody>
                      {workers.slice(0, 8).map((w) => (
                        <tr key={`${w.miner_address}:${w.worker_name}`} onClick={() => setWorkerDetail(w)}>
                          <td className="mono">
                            {w.worker_name} {w.online ? "" : "· off"}
                          </td>
                          <td className="mono">{shortHash(w.miner_address, 5)}</td>
                          <td className="mono">{formatHashrate(w.estimated_hashrate_hs)}</td>
                          <td>{w.accepted_shares}</td>
                          <td className={w.online ? "positive" : "muted"}>{w.online ? "on" : "off"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                ) : (
                  <EmptyState>No workers reported.</EmptyState>
                )}
              </Panel>

              <Panel title="Recent pool blocks" detail={`${blocks.length} in history window`}>
                {blocks.length ? (
                  <table className="data-table interactive-table">
                    <thead>
                      <tr>
                        <th>Height</th>
                        <th>Status</th>
                        <th>Reward</th>
                        <th>Age</th>
                      </tr>
                    </thead>
                    <tbody>
                      {blocks.slice(0, 8).map((b) => (
                        <tr key={b.hash} onClick={() => setBlockDetail(b)}>
                          <td className="mono positive">{b.height}</td>
                          <td className={statusTone(String(b.status))}>{String(b.status)}</td>
                          <td className="mono gold">{formatAtomic(String(asNum(b.reward_atomic)))}</td>
                          <td>{ageLabel(b.found_at_unix_seconds)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                ) : (
                  <EmptyState>No pool blocks found yet.</EmptyState>
                )}
              </Panel>
            </div>
          ) : null}

          {tab === "workers" ? (
            <Panel title="All workers" detail={`${workers.length} · click for detail`}>
              {workers.length ? (
                <table className="data-table interactive-table">
                  <thead>
                    <tr>
                      <th>Worker</th>
                      <th>Miner address</th>
                      <th>Online</th>
                      <th>Hashrate</th>
                      <th>Shares</th>
                      <th>Blocks</th>
                      <th>Diff bits</th>
                      <th>Last share</th>
                    </tr>
                  </thead>
                  <tbody>
                    {workers.map((w) => (
                      <tr key={`${w.miner_address}:${w.worker_name}`} onClick={() => setWorkerDetail(w)}>
                        <td className="mono">{w.worker_name}</td>
                        <td className="mono">{shortHash(w.miner_address, 6)}</td>
                        <td className={w.online ? "positive" : "muted"}>{w.online ? "yes" : "no"}</td>
                        <td className="mono">{formatHashrate(w.estimated_hashrate_hs)}</td>
                        <td>{w.accepted_shares}</td>
                        <td>{w.blocks_found}</td>
                        <td>{w.assigned_difficulty_leading_zero_bits}</td>
                        <td>{ageLabel(w.last_share_unix_seconds)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <EmptyState>No workers on this pool.</EmptyState>
              )}
            </Panel>
          ) : null}

          {tab === "blocks" ? (
            <Panel title="Pool block history" detail="Immature · mature · orphaned">
              {blocks.length ? (
                <table className="data-table interactive-table">
                  <thead>
                    <tr>
                      <th>Height</th>
                      <th>Hash</th>
                      <th>Status</th>
                      <th>Reward</th>
                      <th>Distributable</th>
                      <th>Fee</th>
                      <th>Found</th>
                      <th>Miners</th>
                    </tr>
                  </thead>
                  <tbody>
                    {blocks.map((b) => {
                      const m = maturityInfo(
                        b.height,
                        chainTip,
                        maturityRequired,
                        String(b.status)
                      );
                      return (
                        <tr key={b.hash} onClick={() => setBlockDetail(b)}>
                          <td className="mono positive">{b.height}</td>
                          <td className="mono">{shortHash(b.hash, 8)}</td>
                          <td className={statusTone(m.label)}>
                            {m.label}
                            <div className="maturity-meter-track mini" aria-hidden="true">
                              <i style={{ width: `${m.percent}%` }} />
                            </div>
                          </td>
                          <td className="mono gold">
                            {formatAtomic(String(asNum(b.reward_atomic)))}
                          </td>
                          <td className="mono">
                            {formatAtomic(String(asNum(b.distributable_atomic)))}
                          </td>
                          <td className="mono">
                            {formatAtomic(String(asNum(b.pool_fee_atomic)))}
                          </td>
                          <td>{formatTimestamp(b.found_at_unix_seconds)}</td>
                          <td>{b.allocations ? Object.keys(b.allocations).length : 0}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              ) : (
                <EmptyState>No pool blocks in history.</EmptyState>
              )}
            </Panel>
          ) : null}

          {tab === "shares" ? (
            <Panel
              title="Accepted share history"
              detail={
                snapshot.history_available
                  ? `${shares.length} loaded (newest first)`
                  : "Upgrade pool for /api/v1/pool/history"
              }
            >
              {shares.length ? (
                <table className="data-table interactive-table">
                  <thead>
                    <tr>
                      <th>ID</th>
                      <th>Worker</th>
                      <th>Miner</th>
                      <th>Share bits</th>
                      <th>Net bits</th>
                      <th>Candidate</th>
                      <th>Hash</th>
                      <th>When</th>
                    </tr>
                  </thead>
                  <tbody>
                    {shares.map((s) => (
                      <tr key={s.share_id} onClick={() => setShareDetail(s)}>
                        <td className="mono">{s.share_id}</td>
                        <td className="mono">{s.worker_name}</td>
                        <td className="mono">{shortHash(s.miner_address, 5)}</td>
                        <td>{s.share_difficulty_leading_zero_bits}</td>
                        <td>{s.network_difficulty_leading_zero_bits}</td>
                        <td className={s.block_candidate ? "positive" : "muted"}>
                          {s.block_candidate ? "yes" : "no"}
                        </td>
                        <td className="mono">{shortHash(s.hash, 6)}</td>
                        <td>{ageLabel(s.accepted_at_unix_seconds)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <EmptyState>
                  {snapshot.history_available
                    ? "No shares stored yet."
                    : "This pool build does not expose share history yet. Deploy pool with /api/v1/pool/history."}
                </EmptyState>
              )}
            </Panel>
          ) : null}

          {tab === "payouts" ? (
            <Panel title="Payout batches" detail={`${payouts.length} records`}>
              {payouts.length ? (
                <table className="data-table interactive-table">
                  <thead>
                    <tr>
                      <th>ID</th>
                      <th>Status</th>
                      <th>Items</th>
                      <th>Tx hashes</th>
                      <th>Created</th>
                    </tr>
                  </thead>
                  <tbody>
                    {payouts.map((p) => (
                      <tr key={p.payout_id} onClick={() => setPayoutDetail(p)}>
                        <td className="mono">{shortHash(p.payout_id, 8)}</td>
                        <td className={statusTone(p.status)}>{p.status}</td>
                        <td>{p.items?.length ?? 0}</td>
                        <td>{p.transaction_hashes?.length ?? 0}</td>
                        <td>{formatTimestamp(p.created_at_unix_seconds)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <EmptyState>No payout batches yet.</EmptyState>
              )}
            </Panel>
          ) : null}

          {tab === "accounts" ? (
            <Panel title="Pool miner accounts" detail="Balances tracked by the pool">
              {accounts.length ? (
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Address</th>
                      <th>Immature</th>
                      <th>Mature</th>
                      <th>Pending</th>
                      <th>Paid</th>
                    </tr>
                  </thead>
                  <tbody>
                    {accounts.map((a) => (
                      <tr key={a.address}>
                        <td>
                          <AddressChip value={a.address} />
                        </td>
                        <td className="mono">{formatAtomic(String(asNum(a.immature_atomic)))}</td>
                        <td className="mono gold">{formatAtomic(String(asNum(a.mature_atomic)))}</td>
                        <td className="mono">{formatAtomic(String(asNum(a.pending_payout_atomic)))}</td>
                        <td className="mono">{formatAtomic(String(asNum(a.paid_atomic)))}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <EmptyState>
                  {snapshot.history_available
                    ? "No accounts yet."
                    : "Account ledger requires pool history API."}
                </EmptyState>
              )}
            </Panel>
          ) : null}
        </>
      ) : !busy ? (
        <EmptyState>Select or add a pool to load live network data.</EmptyState>
      ) : null}

      {workerDetail ? (
        <DetailDialog
          title={`Worker ${workerDetail.worker_name}`}
          subtitle={workerDetail.miner_address}
          onClose={() => setWorkerDetail(null)}
        >
          <div className="detail-grid detail-rich">
            <KeyValue label="Online">
              <span className={workerDetail.online ? "positive" : "muted"}>
                {workerDetail.online ? "Yes" : "No"}
              </span>
            </KeyValue>
            <KeyValue label="Hashrate" mono>
              {formatHashrate(workerDetail.estimated_hashrate_hs)}
            </KeyValue>
            <KeyValue label="Accepted shares">{workerDetail.accepted_shares}</KeyValue>
            <KeyValue label="Blocks found">{workerDetail.blocks_found}</KeyValue>
            <KeyValue label="Assigned difficulty">
              {workerDetail.assigned_difficulty_leading_zero_bits} bits
            </KeyValue>
            <KeyValue label="Last share">
              {formatTimestamp(workerDetail.last_share_unix_seconds)} (
              {ageLabel(workerDetail.last_share_unix_seconds)})
            </KeyValue>
            <div className="detail-span-full">
              <KeyValue label="Miner address">
                <AddressChip value={workerDetail.miner_address} full />
              </KeyValue>
            </div>
          </div>
        </DetailDialog>
      ) : null}

      {blockDetail ? (
        <DetailDialog
          title={`Pool block #${blockDetail.height}`}
          subtitle={blockDetail.hash}
          wide
          onClose={() => setBlockDetail(null)}
        >
          <div className="detail-grid detail-rich">
            <div className="detail-span-full">
              <KeyValue label="Maturity">
                <MaturityBar
                  blockHeight={blockDetail.height}
                  tipHeight={chainTip}
                  required={maturityRequired}
                  status={String(blockDetail.status)}
                />
              </KeyValue>
            </div>
            <KeyValue label="Pool status field">
              <span className={statusTone(String(blockDetail.status))}>
                {String(blockDetail.status)}
              </span>
            </KeyValue>
            <KeyValue label="Found">{formatTimestamp(blockDetail.found_at_unix_seconds)}</KeyValue>
            <div className="detail-span-full">
              <KeyValue label="Hash">
                <CopyField value={blockDetail.hash} label="block hash" />
              </KeyValue>
            </div>
            <KeyValue label="Reward" mono>
              {formatAtomic(String(asNum(blockDetail.reward_atomic)))} ALVE
            </KeyValue>
            <KeyValue label="Distributable" mono>
              {formatAtomic(String(asNum(blockDetail.distributable_atomic)))} ALVE
            </KeyValue>
            <KeyValue label="Pool fee" mono>
              {formatAtomic(String(asNum(blockDetail.pool_fee_atomic)))} ALVE
            </KeyValue>
            <div className="detail-span-full">
              <KeyValue label="Allocations (miner → atomic)">
                {blockDetail.allocations && Object.keys(blockDetail.allocations).length ? (
                  <div className="hash-list">
                    {Object.entries(blockDetail.allocations).map(([addr, amt]) => (
                      <div key={addr} className="hash-list-row">
                        <AddressChip value={addr} />
                        <span className="mono gold">{formatAtomic(String(asNum(amt)))} ALVE</span>
                      </div>
                    ))}
                  </div>
                ) : (
                  "—"
                )}
              </KeyValue>
            </div>
          </div>
        </DetailDialog>
      ) : null}

      {shareDetail ? (
        <DetailDialog
          title={`Share #${shareDetail.share_id}`}
          subtitle={shareDetail.hash}
          onClose={() => setShareDetail(null)}
        >
          <div className="detail-grid detail-rich">
            <KeyValue label="Worker">{shareDetail.worker_name}</KeyValue>
            <KeyValue label="Job" mono>
              {shareDetail.job_id}
            </KeyValue>
            <KeyValue label="Nonce" mono>
              {shareDetail.nonce}
            </KeyValue>
            <KeyValue label="Block candidate">
              {shareDetail.block_candidate ? "yes" : "no"}
            </KeyValue>
            <KeyValue label="Share difficulty">
              {shareDetail.share_difficulty_leading_zero_bits} bits
            </KeyValue>
            <KeyValue label="Network difficulty">
              {shareDetail.network_difficulty_leading_zero_bits} bits
            </KeyValue>
            <KeyValue label="Accepted">{formatTimestamp(shareDetail.accepted_at_unix_seconds)}</KeyValue>
            <div className="detail-span-full">
              <KeyValue label="Miner">
                <AddressChip value={shareDetail.miner_address} full />
              </KeyValue>
            </div>
            <div className="detail-span-full">
              <KeyValue label="Hash">
                <CopyField value={shareDetail.hash} label="share hash" />
              </KeyValue>
            </div>
          </div>
        </DetailDialog>
      ) : null}

      {payoutDetail ? (
        <DetailDialog
          title="Payout batch"
          subtitle={payoutDetail.payout_id}
          wide
          onClose={() => setPayoutDetail(null)}
        >
          <div className="detail-grid detail-rich">
            <KeyValue label="Status">
              <span className={statusTone(payoutDetail.status)}>{payoutDetail.status}</span>
            </KeyValue>
            <KeyValue label="Created">{formatTimestamp(payoutDetail.created_at_unix_seconds)}</KeyValue>
            <div className="detail-span-full">
              <KeyValue label="Payout ID">
                <CopyField value={payoutDetail.payout_id} label="payout id" />
              </KeyValue>
            </div>
            <div className="detail-span-full">
              <KeyValue label="Items">
                <div className="hash-list">
                  {(payoutDetail.items ?? []).map((item, i) => (
                    <div key={`${item.address}-${i}`} className="hash-list-row">
                      <AddressChip value={item.address} />
                      <span className="mono gold">
                        {formatAtomic(String(asNum(item.amount_atomic)))} ALVE
                      </span>
                    </div>
                  ))}
                </div>
              </KeyValue>
            </div>
            <div className="detail-span-full">
              <KeyValue label="Transaction hashes">
                {(payoutDetail.transaction_hashes ?? []).length ? (
                  <div className="hash-list">
                    {payoutDetail.transaction_hashes.map((h) => (
                      <CopyField key={h} value={h} label="tx hash" compact />
                    ))}
                  </div>
                ) : (
                  "None yet"
                )}
              </KeyValue>
            </div>
          </div>
        </DetailDialog>
      ) : null}

      {catalogEntry && !catalogEntry.online && catalogEntry.error ? (
        <p className="muted" style={{ fontSize: 12 }}>
          <Layers size={12} style={{ verticalAlign: "middle" }} /> Last catalog probe:{" "}
          {catalogEntry.error}
        </p>
      ) : null}

      <Panel title="Safety boundary" detail="Read-only">
        <p className="muted" style={{ margin: 0, lineHeight: 1.5, fontSize: 13 }}>
          <Users size={14} style={{ verticalAlign: "middle" }} /> This page only reads public pool
          endpoints (status, history, payouts, miner view). Admin payout prepare/confirm requires a
          separate operator token and is intentionally not exposed in the Control Center UI.
        </p>
      </Panel>
    </div>
  );
}
