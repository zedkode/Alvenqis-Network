import { useEffect, useState } from "react";
import { Clock3, Flame, Layers3, ShieldCheck } from "lucide-react";
import { formatAtomic, shortHash } from "@shared/format";
import { DetailDialog } from "../components/ui/DetailDialog";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { KeyValue, Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { useApp } from "../model";

export function Mempool() {
  const { snapshot: n } = useApp();
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const selected = n.mempool_transactions.find((t) => t.hash === selectedHash) ?? null;

  useEffect(() => {
    if (selectedHash && !n.mempool_transactions.some((t) => t.hash === selectedHash)) {
      setSelectedHash(null);
    }
  }, [n.mempool_transactions, selectedHash]);

  return (
    <div className="page grid mempool-page">
      <PageHero
        kicker="GATEWAY PENDING QUEUE"
        title="Mempool"
        titleAccent="pipeline"
        description="Validated transfers waiting for inclusion. Fees shown are projections against current admission rules — not market prices."
        side={
          <>
            <div className="page-hero-metric">
              <small>Pending</small>
              <strong>{n.mempool_count}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Base fee</small>
              <strong>{formatAtomic(n.mempool_anticipated_base_fee_atomic)}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Tips total</small>
              <strong>{formatAtomic(n.mempool_total_priority_fees_atomic)}</strong>
            </div>
          </>
        }
      />

      <div className="grid cols-4 telemetry-strip">
        <StatCard
          label="Pending"
          value={n.mempool_count}
          detail="Validated queue"
          tone={n.mempool_count ? "gold" : "positive"}
          icon={<Clock3 size={14} />}
        />
        <StatCard
          label="Base fee"
          value={formatAtomic(n.mempool_anticipated_base_fee_atomic)}
          detail="ALVE / gas unit"
        />
        <StatCard
          label="Fee value"
          value={formatAtomic(n.mempool_total_fees_atomic)}
          detail="Across entries"
        />
        <StatCard
          label="Priority tips"
          value={formatAtomic(n.mempool_total_priority_fees_atomic)}
          detail="Miner tips"
          tone="positive"
          icon={<Flame size={14} />}
        />
      </div>

      <div className="mempool-flow">
        <div className={`flow-stage ${n.mempool_count ? "active" : ""}`}>
          <ShieldCheck size={20} />
          <b>VALIDATED</b>
          <span>Signature, address, nonce, balance</span>
        </div>
        <i />
        <div className={`flow-stage ${n.mempool_count ? "active" : ""}`}>
          <Clock3 size={20} />
          <b>PENDING</b>
          <span>
            {n.mempool_count} tx waiting
          </span>
        </div>
        <i />
        <div className="flow-stage">
          <Layers3 size={20} />
          <b>NEXT BLOCK</b>
          <span>Selected by active miner</span>
        </div>
      </div>

      <Panel title="Pending queue" detail="GET /mempool · click for detail">
        {n.mempool_transactions.length ? (
          <table className="data-table interactive-table">
            <thead>
              <tr>
                <th>Tx hash</th>
                <th>From</th>
                <th>Amount</th>
                <th>Fee</th>
                <th>State</th>
              </tr>
            </thead>
            <tbody>
              {n.mempool_transactions.map((tx) => (
                <tr
                  key={tx.hash}
                  className={selected?.hash === tx.hash ? "selected" : ""}
                  onClick={() => setSelectedHash(tx.hash)}
                >
                  <td className="mono">{shortHash(tx.hash, 7)}</td>
                  <td className="mono">{tx.from ? shortHash(tx.from, 5) : "Coinbase"}</td>
                  <td className="mono">{formatAtomic(tx.amount_atomic)}</td>
                  <td className="mono gold">{formatAtomic(tx.effective_fee_atomic)}</td>
                  <td className="positive">{tx.lifecycle_status || "Pending"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <EmptyState>
            Mempool is healthy and empty. Signed transfers appear after RPC admission.
          </EmptyState>
        )}
      </Panel>

      <Panel title="Fee disposition" detail="Current projection">
        <div className="fee-disposition">
          <span>
            <Flame size={18} />
            <small>Projected burn</small>
            <b>{formatAtomic(n.mempool_total_burned_fees_atomic)} ALVE</b>
          </span>
          <span>
            <Layers3 size={18} />
            <small>Miner tips</small>
            <b>{formatAtomic(n.mempool_total_priority_fees_atomic)} ALVE</b>
          </span>
          <span>
            <ShieldCheck size={18} />
            <small>Admission</small>
            <b>Gateway consensus state</b>
          </span>
        </div>
      </Panel>

      {selected ? (
        <DetailDialog title="Pending transaction" subtitle={selected.hash} onClose={() => setSelectedHash(null)}>
          <div className="detail-grid">
            <KeyValue label="Lifecycle">
              <span className="gold">{selected.lifecycle_status || "Pending"}</span>
            </KeyValue>
            <KeyValue label="Admission">Validated against current state</KeyValue>
            <div className="detail-span-full">
              <KeyValue label="Hash" mono>
                {selected.hash}
              </KeyValue>
            </div>
            <div className="detail-span-full">
              <KeyValue label="From" mono>
                {selected.from ?? "Coinbase"}
              </KeyValue>
            </div>
            <div className="detail-span-full">
              <KeyValue label="To" mono>
                {selected.to}
              </KeyValue>
            </div>
            <KeyValue label="Nonce">{selected.nonce}</KeyValue>
            <KeyValue label="Authorization">{selected.authorization_state}</KeyValue>
            <KeyValue label="Amount" mono>
              {formatAtomic(selected.amount_atomic)} ALVE
            </KeyValue>
            <KeyValue label="Fee" mono>
              {formatAtomic(selected.effective_fee_atomic)} ALVE
            </KeyValue>
            <KeyValue label="Burn projection" mono>
              {formatAtomic(selected.burned_fee_atomic)} ALVE
            </KeyValue>
            <KeyValue label="Tip projection" mono>
              {formatAtomic(selected.effective_priority_fee_atomic)} ALVE
            </KeyValue>
          </div>
        </DetailDialog>
      ) : null}
    </div>
  );
}
