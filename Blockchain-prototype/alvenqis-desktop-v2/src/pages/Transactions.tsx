import { useMemo, useState } from "react";
import { formatAtomic, shortHash } from "@shared/format";
import type { DesktopBlock, DesktopTransaction } from "@shared/types";
import {
  BlockDetailBody,
  normalizeBlock,
  normalizeTx,
  TransactionDetailBody
} from "../components/explorer/ChainDetailPanels";
import { DetailDialog } from "../components/ui/DetailDialog";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { BarChart } from "../components/charts/BarChart";
import { useApp } from "../model";

export function Transactions() {
  const { snapshot: n } = useApp();
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const [enriched, setEnriched] = useState<DesktopTransaction | null>(null);
  const [blockDetail, setBlockDetail] = useState<DesktopBlock | null>(null);
  const selected = n.recent_transactions.find((t) => t.hash === selectedHash) ?? null;
  const dialogTx = enriched ?? selected;

  const amountSeries = useMemo(
    () =>
      [...n.recent_transactions]
        .reverse()
        .map((t) => Number(t.amount_atomic) / 1e8)
        .slice(-30),
    [n.recent_transactions]
  );
  const feeSeries = useMemo(
    () =>
      [...n.recent_transactions]
        .reverse()
        .map((t) => Number(t.effective_fee_atomic) / 1e8)
        .slice(-30),
    [n.recent_transactions]
  );

  const openBlock = async (height: number) => {
    const local = n.recent_blocks.find((b) => b.height === height);
    if (local) {
      setBlockDetail(local);
      return;
    }
    try {
      const result = await window.alvenqis.explorer.lookup(String(height));
      if (result.kind === "block" && result.data) {
        setBlockDetail(normalizeBlock(result.data as unknown as DesktopBlock));
      }
    } catch {
      /* ignore */
    }
  };

  const enrichAndOpen = async (tx: DesktopTransaction) => {
    setSelectedHash(tx.hash);
    setEnriched(null);
    try {
      const result = await window.alvenqis.explorer.lookup(tx.hash);
      if (result.kind === "transaction" && (result.raw || result.data)) {
        const merged = normalizeTx({
          ...tx,
          ...((result.raw as object) ?? {}),
          ...((result.data as object) ?? {})
        } as DesktopTransaction);
        setEnriched(merged);
        return;
      }
    } catch {
      /* use list row */
    }
  };

  return (
    <div className="page grid">
      <PageHero
        kicker="CONFIRMED TRANSFERS"
        title="Transactions"
        titleAccent="ledger"
        description="Indexed confirmed transfers and coinbase issuance. Click a row for lifecycle, fee split, parties and public authorization material."
        side={
          <>
            <div className="page-hero-metric">
              <small>Indexed total</small>
              <strong>{n.indexed_transactions}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Window</small>
              <strong>{n.recent_transactions.length}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Mempool</small>
              <strong>{n.mempool_count}</strong>
            </div>
          </>
        }
      />

      <div className="grid cols-4 telemetry-strip">
        <StatCard label="Indexed TX" value={n.indexed_transactions} detail="All-time index" />
        <StatCard
          label="Window"
          value={n.recent_transactions.length}
          detail="Recent list"
          tone="positive"
        />
        <StatCard
          label="Pending"
          value={n.mempool_count}
          detail="Not yet mined"
          tone={n.mempool_count ? "gold" : undefined}
        />
        <StatCard label="Addresses" value={n.indexed_addresses} detail="Known set" />
      </div>

      <div className="grid cols-2">
        <Panel title="Transfer amounts (ALVE)" detail="Recent window">
          {amountSeries.length > 1 ? (
            <BarChart values={amountSeries} label="Amount" unit="ALVE" height={120} />
          ) : (
            <EmptyState>Need more transactions for a chart.</EmptyState>
          )}
        </Panel>
        <Panel title="Effective fees (ALVE)" detail="Recent window">
          {feeSeries.length > 1 ? (
            <BarChart values={feeSeries} label="Fee" unit="ALVE" tone="gold" height={120} />
          ) : (
            <EmptyState>Need more transactions for a chart.</EmptyState>
          )}
        </Panel>
      </div>

      <div className="grid cols-2">
        <Panel title="Activity timeline" detail="Click for detail">
          {n.recent_transactions.length ? (
            <div className="chain-timeline">
              {n.recent_transactions.map((tx) => (
                <div
                  key={tx.hash}
                  className={`chain-timeline-item ${selectedHash === tx.hash ? "selected" : ""}`}
                  onClick={() => void enrichAndOpen(tx)}
                >
                  <div className="chain-timeline-rail">
                    <i className="chain-timeline-dot" />
                  </div>
                  <div className="chain-timeline-body">
                    <header>
                      <b>{shortHash(tx.hash, 8)}</b>
                      <time>block {tx.block_height}</time>
                    </header>
                    <div className="chain-timeline-meta">
                      <span>{tx.from ? shortHash(tx.from, 5) : "coinbase"}</span>
                      <span>→ {shortHash(tx.to, 5)}</span>
                      <span className="gold">{formatAtomic(tx.amount_atomic)} ALVE</span>
                      <span className="positive">{tx.lifecycle_status}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <EmptyState>No indexed transactions.</EmptyState>
          )}
        </Panel>

        <Panel title="Table" detail={`${n.indexed_transactions} total`}>
          {n.recent_transactions.length ? (
            <table className="data-table interactive-table">
              <thead>
                <tr>
                  <th>Hash</th>
                  <th>Block</th>
                  <th>From</th>
                  <th>To</th>
                  <th>Amount</th>
                  <th>Fee</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                {n.recent_transactions.map((tx) => (
                  <tr key={tx.hash} onClick={() => void enrichAndOpen(tx)}>
                    <td className="mono">{shortHash(tx.hash, 7)}</td>
                    <td>{tx.block_height}</td>
                    <td className="mono">{tx.from ? shortHash(tx.from, 5) : "Coinbase"}</td>
                    <td className="mono">{shortHash(tx.to, 5)}</td>
                    <td className="mono">{formatAtomic(tx.amount_atomic)}</td>
                    <td className="mono gold">{formatAtomic(tx.effective_fee_atomic)}</td>
                    <td className="positive">{tx.lifecycle_status}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <EmptyState>No indexed transactions.</EmptyState>
          )}
        </Panel>
      </div>

      {dialogTx ? (
        <DetailDialog
          title="Transaction"
          subtitle={dialogTx.hash}
          wide
          onClose={() => {
            setSelectedHash(null);
            setEnriched(null);
          }}
        >
          <TransactionDetailBody
            tx={dialogTx}
            tipHeight={n.height}
            onOpenBlock={(h) => void openBlock(h)}
          />
        </DetailDialog>
      ) : null}

      {blockDetail ? (
        <DetailDialog
          title={`Block ${blockDetail.height}`}
          subtitle={blockDetail.hash}
          wide
          onClose={() => setBlockDetail(null)}
        >
          <BlockDetailBody block={blockDetail} tipHeight={n.height} />
        </DetailDialog>
      ) : null}
    </div>
  );
}
