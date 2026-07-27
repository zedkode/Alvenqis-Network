import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { fetchJson, IndexedBlock, IndexedTransaction } from "../lib/api";
import { formatAtomic, formatTimestamp } from "../lib/format";
import { PageHeader } from "../components/PageHeader";
import { ErrorPanel, LoadingPanel } from "../components/StatePanels";

export function BlockDetailsPage() {
  const { height } = useParams();
  const [block, setBlock] = useState<IndexedBlock | null>(null);
  const [transactions, setTransactions] = useState<IndexedTransaction[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    if (!height) {
      setError("Block height is required.");
      return;
    }

    (async () => {
      try {
        const loadedBlock = await fetchJson<IndexedBlock>(`/indexer/blocks/${height}`);
        const loadedTransactions = await Promise.all(
          loadedBlock.transaction_hashes.map((hash) =>
            fetchJson<IndexedTransaction>(`/indexer/tx/${hash}`)
          )
        );
        if (active) {
          setBlock(loadedBlock);
          setTransactions(loadedTransactions);
        }
      } catch (loadError) {
        if (active) {
          setError(loadError instanceof Error ? loadError.message : "Failed to load block");
        }
      }
    })();

    return () => {
      active = false;
    };
  }, [height]);

  return (
    <>
      <PageHeader
        title={`Block ${height ?? "Unavailable"}`}
        description="Indexed Mainnet Candidate block header data, fee accounting and transaction membership."
      />
      {error ? <ErrorPanel message={error} /> : null}
      {!block && !error ? <LoadingPanel message="Loading indexed block details..." /> : null}
      {block ? (
        <div className="grid two-col">
          <section className="panel">
            <h2>Block Header</h2>
            <div className="detail-list">
              <div className="detail-row">
                <div className="detail-label">Hash</div>
                <div className="hash-text">{block.hash}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Previous hash</div>
                <div className="hash-text">{block.previous_hash}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Merkle root</div>
                <div className="hash-text">{block.merkle_root}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Timestamp</div>
                <div>{formatTimestamp(block.timestamp)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Nonce</div>
                <div>{block.nonce}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Difficulty</div>
                <div>{block.difficulty_leading_zero_bits} leading zero bits</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Miner</div>
                <div>
                  <Link to={`/address/${block.miner_address}`}>{block.miner_address}</Link>
                </div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Coinbase payout</div>
                <div>{formatAtomic(block.coinbase_payout_atomic)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Miner reward</div>
                <div>{formatAtomic(block.miner_reward_atomic)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Collected fees</div>
                <div>{formatAtomic(block.fees_atomic)}</div>
              </div>
            </div>
          </section>

          <section className="panel">
            <h2>Transactions</h2>
            <div className="tx-list">
              {transactions.map((transaction) => (
                <div className="tx-pill" key={transaction.hash}>
                  <Link className="hash-text" to={`/tx/${transaction.hash}`}>
                    {transaction.hash}
                  </Link>
                  <span>{formatAtomic(transaction.amount_atomic)}</span>
                  <span>{transaction.authorization_state}</span>
                </div>
              ))}
            </div>
          </section>
        </div>
      ) : null}
    </>
  );
}
