import { useMemo, useState } from "react";
import { formatAtomic, shortHash } from "@shared/format";
import type { DesktopBlock, DesktopTransaction } from "@shared/types";
import {
  BlockDetailBody,
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

function elapsed(seconds: number): string {
  const value = Math.max(0, Math.floor(Date.now() / 1000) - seconds);
  if (value < 60) return `${value}s ago`;
  if (value < 3600) return `${Math.floor(value / 60)}m ago`;
  if (value < 86_400) return `${Math.floor(value / 3600)}h ago`;
  return `${Math.floor(value / 86_400)}d ago`;
}

export function Blocks() {
  const { snapshot: n } = useApp();
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const [txDetail, setTxDetail] = useState<DesktopTransaction | null>(null);
  const selected = n.recent_blocks.find((b) => b.hash === selectedHash) ?? null;
  const previous = selected
    ? (n.recent_blocks.find((b) => b.height + 1 === selected.height) ?? null)
    : null;
  const interval =
    selected && previous && selected.timestamp >= previous.timestamp
      ? `${selected.timestamp - previous.timestamp}s`
      : "n/a in window";

  const txSeries = useMemo(
    () => [...n.recent_blocks].reverse().map((b) => b.transaction_count),
    [n.recent_blocks]
  );
  const feeSeries = useMemo(
    () => [...n.recent_blocks].reverse().map((b) => Number(b.fees_atomic) / 1e8),
    [n.recent_blocks]
  );

  const openTxHash = async (hash: string) => {
    const local = n.recent_transactions.find((t) => t.hash === hash);
    if (local) {
      setTxDetail(local);
      return;
    }
    try {
      const result = await window.alvenqis.explorer.lookup(hash);
      if (result.kind === "transaction" && result.data) {
        setTxDetail(
          normalizeTx(((result.raw ?? result.data) as unknown as DesktopTransaction))
        );
      }
    } catch {
      /* keep block dialog open */
    }
  };

  return (
    <div className="page grid">
      <PageHero
        kicker="CANONICAL CHAIN"
        title="Blocks"
        titleAccent="& tip"
        description="Recent canonical heights from the indexer. Click a block for full header, miner payout, fee split and transaction hashes."
        side={
          <>
            <div className="page-hero-metric">
              <small>Tip height</small>
              <strong>{n.height ?? "—"}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Indexed blocks</small>
              <strong>{n.indexed_blocks}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Window</small>
              <strong>{n.recent_blocks.length}</strong>
            </div>
          </>
        }
      />

      <div className="grid cols-4 telemetry-strip">
        <StatCard
          label="Tip"
          value={n.height ?? "—"}
          detail={shortHash(n.tip_hash, 6)}
          tone="positive"
        />
        <StatCard label="Indexed" value={n.indexed_blocks} detail="Canonical count" />
        <StatCard
          label="Latest reward"
          value={
            n.latest_block_reward_atomic ? formatAtomic(n.latest_block_reward_atomic) : "—"
          }
          detail="ALVE"
          tone="gold"
        />
        <StatCard
          label="Latest fees"
          value={n.latest_block_fees_atomic ? formatAtomic(n.latest_block_fees_atomic) : "—"}
          detail={`${n.latest_block_transactions} tx in tip block`}
        />
      </div>

      <div className="grid cols-2">
        <Panel title="Tx per block" detail="Indexed window">
          {txSeries.length > 1 ? (
            <BarChart values={txSeries} label="Transactions" height={120} />
          ) : (
            <EmptyState>Need more blocks for a chart.</EmptyState>
          )}
        </Panel>
        <Panel title="Fees per block (ALVE)" detail="Indexed window">
          {feeSeries.length > 1 ? (
            <BarChart values={feeSeries} label="Fees" unit="ALVE" tone="gold" height={120} />
          ) : (
            <EmptyState>Need more blocks for a chart.</EmptyState>
          )}
        </Panel>
      </div>

      <div className="grid cols-2">
        <Panel title="Block timeline" detail="Click to inspect">
          {n.recent_blocks.length ? (
            <div className="chain-timeline">
              {n.recent_blocks.map((block) => (
                <div
                  key={block.hash}
                  className={`chain-timeline-item ${selectedHash === block.hash ? "selected" : ""}`}
                  onClick={() => setSelectedHash(block.hash)}
                >
                  <div className="chain-timeline-rail">
                    <i className="chain-timeline-dot" />
                  </div>
                  <div className="chain-timeline-body">
                    <header>
                      <b>#{block.height}</b>
                      <time>{elapsed(block.timestamp)}</time>
                    </header>
                    <div className="chain-timeline-meta">
                      <span>{shortHash(block.hash, 8)}</span>
                      <span>{block.transaction_count} tx</span>
                      <span className="gold">{formatAtomic(block.miner_reward_atomic)} ALVE</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <EmptyState>No indexed blocks. Indexer must catch up.</EmptyState>
          )}
        </Panel>

        <Panel title="Table view" detail={`${n.indexed_blocks} indexed`}>
          {n.recent_blocks.length ? (
            <table className="data-table interactive-table">
              <thead>
                <tr>
                  <th>Height</th>
                  <th>Hash</th>
                  <th>Age</th>
                  <th>Tx</th>
                  <th>Diff</th>
                  <th>Reward</th>
                  <th>Fees</th>
                </tr>
              </thead>
              <tbody>
                {n.recent_blocks.map((block: DesktopBlock) => (
                  <tr key={block.hash} onClick={() => setSelectedHash(block.hash)}>
                    <td className="positive mono">{block.height}</td>
                    <td className="mono">{shortHash(block.hash, 7)}</td>
                    <td>{elapsed(block.timestamp)}</td>
                    <td>{block.transaction_count}</td>
                    <td>{block.difficulty_leading_zero_bits}b</td>
                    <td className="mono gold">{formatAtomic(block.miner_reward_atomic)}</td>
                    <td className="mono">{formatAtomic(block.fees_atomic)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <EmptyState>No indexed blocks.</EmptyState>
          )}
        </Panel>
      </div>

      {selected ? (
        <DetailDialog
          title={`Block ${selected.height}`}
          subtitle={selected.hash}
          wide
          onClose={() => setSelectedHash(null)}
        >
          <BlockDetailBody
            block={selected}
            tipHeight={n.height}
            intervalLabel={interval}
            onOpenTx={(hash) => void openTxHash(hash)}
          />
        </DetailDialog>
      ) : null}

      {txDetail ? (
        <DetailDialog
          title="Transaction"
          subtitle={txDetail.hash}
          wide
          onClose={() => setTxDetail(null)}
        >
          <TransactionDetailBody tx={txDetail} tipHeight={n.height} />
        </DetailDialog>
      ) : null}
    </div>
  );
}
