import { useCallback, useState } from "react";
import {
  Blocks,
  ExternalLink,
  ListTree,
  Network,
  Search,
  Users,
  WalletCards
} from "lucide-react";
import { formatAtomic, shortHash } from "@shared/format";
import type {
  DesktopBlock,
  DesktopTransaction,
  ExplorerLookupResult
} from "@shared/types";
import {
  AddressDetailBody,
  BlockDetailBody,
  normalizeBlock,
  normalizeTx,
  PeerDetailBody,
  PoolWorkerDetailBody,
  TransactionDetailBody
} from "../components/explorer/ChainDetailPanels";
import { DetailDialog } from "../components/ui/DetailDialog";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { KeyValue, Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { useApp } from "../model";

function explorerPath(query: string): string {
  const value = query.trim();
  if (/^\d+$/.test(value)) return `blocks/${value}`;
  if (/^alve1/i.test(value)) return `address/${encodeURIComponent(value)}`;
  if (/^(0x)?[a-f0-9]{64}$/i.test(value)) return `search?q=${encodeURIComponent(value)}`;
  return value ? `search?q=${encodeURIComponent(value)}` : "";
}

export function Explorer() {
  const { snapshot: n, setPage } = useApp();
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lookup, setLookup] = useState<ExplorerLookupResult | null>(null);
  const [dialogBlock, setDialogBlock] = useState<DesktopBlock | null>(null);
  const [dialogTx, setDialogTx] = useState<DesktopTransaction | null>(null);

  const openExternal = async (path = explorerPath(query)) => {
    setError(null);
    try {
      await window.alvenqis.explorer.open(path);
    } catch (err) {
      setError(`Cannot open the configured public Explorer: ${String(err).replace(/^Error:\s*/i, "")}`);
    }
  };

  const runLookup = useCallback(
    async (raw?: string) => {
      const q = (raw ?? query).trim();
      if (!q) {
        setError(
          "Enter a height, 64-char hash, alve1 address, peer id, or pool worker name."
        );
        return;
      }
      setBusy(true);
      setError(null);
      setDialogBlock(null);
      setDialogTx(null);
      try {
        const result = (await window.alvenqis.explorer.lookup(q)) as ExplorerLookupResult;
        setLookup(result);
        if (result.kind === "block" && result.data) {
          setDialogBlock(normalizeBlock(result.data as unknown as DesktopBlock));
        } else if (result.kind === "transaction" && result.data) {
          // Prefer raw indexer payload when present (more fields).
          const source = (result.raw ?? result.data) as unknown as DesktopTransaction;
          setDialogTx(normalizeTx(source));
        }
        if (result.kind === "not_found") {
          setError(result.message || "Nothing matched this query on the gateway.");
        }
      } catch (err) {
        setLookup(null);
        setError(String(err).replace(/^Error:\s*/i, ""));
      } finally {
        setBusy(false);
      }
    },
    [query]
  );

  const inlineLookup =
    lookup &&
    lookup.data &&
    (lookup.kind === "address" ||
      lookup.kind === "peer" ||
      lookup.kind === "pool_worker" ||
      lookup.kind === "pool")
      ? lookup
      : null;

  return (
    <div className="page grid explorer-page">
      <PageHero
        kicker="INDEXED CHAIN · PUBLIC GATEWAY"
        title="Chain"
        titleAccent="explorer"
        description="Look up heights, transaction hashes, alve1 addresses, pool payout/workers and peer IDs against the configured RPC — public data only, no secrets."
        actions={
          <button className="button" type="button" onClick={() => void openExternal()}>
            <ExternalLink size={15} /> Public browser
          </button>
        }
        side={
          <>
            <div className="page-hero-metric">
              <small>Height</small>
              <strong>{n.height ?? "—"}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Indexed TX</small>
              <strong>{n.indexed_transactions}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Peers</small>
              <strong>{n.connected_peer_count}</strong>
            </div>
          </>
        }
      />

      <Panel
        title="In-app lookup"
        detail="Height · tx/block hash · alve1 · peer id · pool worker"
      >
        <div className="explorer-search-row">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void runLookup()}
            placeholder="e.g. 42 · tx hash · alve1… · 12D3KooW… · worker-name"
            spellCheck={false}
            autoComplete="off"
          />
          <button
            className="button primary"
            type="button"
            disabled={busy}
            onClick={() => void runLookup()}
          >
            <Search size={16} /> {busy ? "Searching…" : "Lookup"}
          </button>
          <button className="button ghost" type="button" onClick={() => void openExternal()}>
            <ExternalLink size={15} /> Browser
          </button>
        </div>
        <p className="field-hint" style={{ marginTop: 10 }}>
          Safe sources: indexer, chain RPC, pool status, P2P presence. Never enters wallet secrets or
          keystore material.
        </p>
        {error ? (
          <p className="hw-error" style={{ marginTop: 8 }}>
            {error}
          </p>
        ) : null}
      </Panel>

      {inlineLookup ? (
        <Panel
          title={`Result · ${inlineLookup.kind.replace("_", " ")}`}
          detail={
            <span className="mono" style={{ fontSize: 11 }}>
              sources: {(inlineLookup.sources ?? []).join(", ") || "—"}
            </span>
          }
        >
          {inlineLookup.notes?.length ? (
            <ul className="muted" style={{ margin: "0 0 12px", paddingLeft: 18, fontSize: 12 }}>
              {inlineLookup.notes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          ) : null}
          {inlineLookup.kind === "address" && inlineLookup.data ? (
            <AddressDetailBody
              data={inlineLookup.data}
              onOpenTx={(hash) => void runLookup(hash)}
              onOpenBlock={(h) => void runLookup(String(h))}
            />
          ) : null}
          {inlineLookup.kind === "peer" && inlineLookup.data ? (
            <PeerDetailBody data={inlineLookup.data} />
          ) : null}
          {inlineLookup.kind === "pool_worker" && inlineLookup.data ? (
            <PoolWorkerDetailBody data={inlineLookup.data} />
          ) : null}
          {inlineLookup.kind === "pool" && inlineLookup.data ? (
            <div className="detail-grid detail-rich">
              <KeyValue label="Pool">{String(inlineLookup.data.pool_name ?? "—")}</KeyValue>
              <KeyValue label="Workers">
                {String(inlineLookup.data.connected_workers ?? "—")}
              </KeyValue>
              <KeyValue label="Hashrate" mono>
                {String(inlineLookup.data.estimated_hashrate_hs ?? "—")} H/s
              </KeyValue>
              <KeyValue label="Blocks found">
                {String(inlineLookup.data.blocks_found ?? "—")}
              </KeyValue>
              <KeyValue label="Upstream">
                {String(inlineLookup.data.upstream_status ?? "—")}
              </KeyValue>
              <KeyValue label="Scheme">{String(inlineLookup.data.payout_scheme ?? "—")}</KeyValue>
              <div className="detail-span-full">
                <KeyValue label="Pool address">
                  <span className="mono">{String(inlineLookup.data.pool_address ?? "—")}</span>
                </KeyValue>
              </div>
            </div>
          ) : null}
        </Panel>
      ) : null}

      <div className="explorer-rail">
        <button type="button" onClick={() => setPage("blocks")}>
          <Blocks size={17} />
          <span>Blocks page</span>
        </button>
        <button type="button" onClick={() => setPage("transactions")}>
          <ListTree size={17} />
          <span>Transactions</span>
        </button>
        <button type="button" onClick={() => setPage("mempool")}>
          <ListTree size={17} />
          <span>Mempool</span>
        </button>
        <button type="button" onClick={() => setPage("node")}>
          <Users size={17} />
          <span>Peers / node</span>
        </button>
        <button type="button" onClick={() => void openExternal("supply")}>
          <WalletCards size={17} />
          <span>Supply</span>
        </button>
        <button type="button" onClick={() => void openExternal("network")}>
          <Network size={17} />
          <span>Network</span>
        </button>
      </div>

      <div className="grid cols-5">
        <StatCard label="Height" value={n.height ?? "—"} detail={shortHash(n.tip_hash, 5)} />
        <StatCard
          label="Mempool"
          value={n.mempool_count}
          detail="Pending"
          tone={n.mempool_count ? "gold" : undefined}
        />
        <StatCard
          label="Emitted"
          value={n.emitted_supply_atomic ? formatAtomic(n.emitted_supply_atomic) : "—"}
          detail="ALVE"
          tone="gold"
        />
        <StatCard
          label="Indexed TX"
          value={n.indexed_transactions}
          detail={`${n.indexed_addresses} addresses`}
        />
        <StatCard
          label="Pool workers"
          value={n.pool_workers ?? 0}
          detail={n.pool_online ? n.pool_name || "online" : "offline"}
        />
      </div>

      <div className="grid cols-2 explorer-tables">
        <Panel title="Latest blocks" detail="Click for detail">
          {n.recent_blocks.length ? (
            <table className="data-table interactive-table">
              <thead>
                <tr>
                  <th>Height</th>
                  <th>Hash</th>
                  <th>Tx</th>
                  <th>Reward</th>
                  <th>Fees</th>
                </tr>
              </thead>
              <tbody>
                {n.recent_blocks.map((block) => (
                  <tr
                    key={block.hash}
                    className={dialogBlock?.hash === block.hash ? "selected" : ""}
                    onClick={() => {
                      setLookup(null);
                      setDialogTx(null);
                      setDialogBlock(block);
                    }}
                    onDoubleClick={() => void openExternal(`blocks/${block.height}`)}
                  >
                    <td className="positive mono">{block.height}</td>
                    <td className="mono">{shortHash(block.hash, 6)}</td>
                    <td>{block.transaction_count}</td>
                    <td className="mono">{formatAtomic(block.miner_reward_atomic)}</td>
                    <td className="mono gold">{formatAtomic(block.fees_atomic)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <EmptyState>No indexed blocks available.</EmptyState>
          )}
        </Panel>
        <Panel title="Latest transactions" detail="Click for detail">
          {n.recent_transactions.length ? (
            <table className="data-table interactive-table">
              <thead>
                <tr>
                  <th>Hash</th>
                  <th>Block</th>
                  <th>From</th>
                  <th>Amount</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                {n.recent_transactions.map((tx) => (
                  <tr
                    key={tx.hash}
                    className={dialogTx?.hash === tx.hash ? "selected" : ""}
                    onClick={() => {
                      setLookup(null);
                      setDialogBlock(null);
                      setDialogTx(tx);
                    }}
                    onDoubleClick={() => void openExternal(`tx/${tx.hash}`)}
                  >
                    <td className="mono">{shortHash(tx.hash, 6)}</td>
                    <td>{tx.block_height}</td>
                    <td className="mono">{tx.from ? shortHash(tx.from, 4) : "Coinbase"}</td>
                    <td className="mono">{formatAtomic(tx.amount_atomic)}</td>
                    <td className="positive">{tx.lifecycle_status}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <EmptyState>No indexed transactions available.</EmptyState>
          )}
        </Panel>
      </div>

      {n.peers?.length ? (
        <Panel title="Connected peers (snapshot)" detail="Click peer id to look up">
          <table className="data-table interactive-table">
            <thead>
              <tr>
                <th>Peer ID</th>
                <th>Height</th>
                <th>Flags</th>
                <th>H/s</th>
              </tr>
            </thead>
            <tbody>
              {n.peers.slice(0, 12).map((peer) => (
                <tr
                  key={peer.peer_id}
                  onClick={() => {
                    setQuery(peer.peer_id);
                    void runLookup(peer.peer_id);
                  }}
                >
                  <td className="mono">{shortHash(peer.peer_id, 10)}</td>
                  <td>{peer.best_height ?? "—"}</td>
                  <td>
                    {[
                      peer.handshake_validated ? "validated" : null,
                      peer.validating ? "validator" : null,
                      peer.mining ? "mining" : null
                    ]
                      .filter(Boolean)
                      .join(" · ") || "—"}
                  </td>
                  <td className="mono">{peer.hashrate_hs || 0}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {n.local_peer_id ? (
            <p className="field-hint" style={{ marginTop: 10 }}>
              Local peer id:{" "}
              <button
                type="button"
                className="linkish mono"
                onClick={() => {
                  setQuery(n.local_peer_id || "");
                  void runLookup(n.local_peer_id || "");
                }}
              >
                {n.local_peer_id}
              </button>
            </p>
          ) : null}
        </Panel>
      ) : null}

      {dialogBlock ? (
        <DetailDialog
          title={`Block ${dialogBlock.height}`}
          subtitle={dialogBlock.hash}
          wide
          onClose={() => setDialogBlock(null)}
        >
          <BlockDetailBody
            block={dialogBlock}
            tipHeight={n.height}
            onOpenTx={(hash) => {
              setDialogBlock(null);
              void runLookup(hash);
            }}
            onOpenExternal={() => void openExternal(`blocks/${dialogBlock.height}`)}
          />
        </DetailDialog>
      ) : null}

      {dialogTx ? (
        <DetailDialog
          title="Transaction"
          subtitle={dialogTx.hash}
          wide
          onClose={() => setDialogTx(null)}
        >
          <TransactionDetailBody
            tx={dialogTx}
            tipHeight={n.height}
            onOpenBlock={(h) => {
              setDialogTx(null);
              void runLookup(String(h));
            }}
            onOpenExternal={() => void openExternal(`tx/${dialogTx.hash}`)}
          />
        </DetailDialog>
      ) : null}
    </div>
  );
}
