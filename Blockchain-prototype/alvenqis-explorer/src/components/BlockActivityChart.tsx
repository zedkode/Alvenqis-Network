import { IndexedBlock } from "../lib/api";
import { formatAtomic, formatCount } from "../lib/format";

interface BlockActivityChartProps {
  blocks: IndexedBlock[];
}

export function BlockActivityChart({ blocks }: BlockActivityChartProps) {
  const orderedBlocks = [...blocks].reverse();
  const maxTransactions = Math.max(1, ...orderedBlocks.map((block) => block.transaction_count));
  const totalTransactions = orderedBlocks.reduce((total, block) => total + block.transaction_count, 0);
  const totalFees = orderedBlocks.reduce((total, block) => total + block.fees_atomic, 0);
  const averageDifficulty = orderedBlocks.length
    ? orderedBlocks.reduce((total, block) => total + block.difficulty_leading_zero_bits, 0) / orderedBlocks.length
    : 0;

  return (
    <section className="panel activity-panel">
      <div className="panel-heading">
        <div>
          <div className="panel-kicker">Indexed block window</div>
          <h2>Block activity</h2>
        </div>
        <span className="window-label">Last {orderedBlocks.length} blocks</span>
      </div>

      {orderedBlocks.length ? (
        <>
          <div className="activity-chart" role="img" aria-label="Transactions per indexed block">
            <div className="chart-grid-lines" aria-hidden="true" />
            <div className="activity-bars">
              {orderedBlocks.map((block) => (
                <a
                  className="activity-column"
                  href={`/blocks/${block.height}`}
                  key={block.hash}
                  title={`Block ${block.height}: ${block.transaction_count} transactions`}
                >
                  <span
                    className="activity-bar"
                    style={{ height: `${Math.max(7, (block.transaction_count / maxTransactions) * 100)}%` }}
                  />
                  <span className="activity-height">{block.height}</span>
                </a>
              ))}
            </div>
          </div>
          <div className="chart-summary">
            <div><span>Transactions</span><strong>{formatCount(totalTransactions)}</strong></div>
            <div><span>Fees</span><strong>{formatAtomic(totalFees)}</strong></div>
            <div><span>Avg. difficulty</span><strong>{averageDifficulty.toFixed(2)} bits</strong></div>
          </div>
        </>
      ) : (
        <p className="muted">No indexed blocks are available for charting.</p>
      )}
    </section>
  );
}
