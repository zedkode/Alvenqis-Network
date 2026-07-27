import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { fetchJson, RpcTransactionResponse } from "../lib/api";
import { formatAtomic } from "../lib/format";
import { PageHeader } from "../components/PageHeader";
import { ErrorPanel, LoadingPanel } from "../components/StatePanels";

export function TransactionDetailsPage() {
  const { hash } = useParams();
  const [transaction, setTransaction] = useState<RpcTransactionResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    if (!hash) {
      setError("Transaction hash is required.");
      return;
    }

    fetchJson<RpcTransactionResponse>(`/transactions/${hash}`)
      .then((loaded) => {
        if (active) {
          setTransaction(loaded);
        }
      })
      .catch((loadError) => {
        if (active) {
          setError(loadError instanceof Error ? loadError.message : "Failed to load transaction");
        }
      });

    return () => {
      active = false;
    };
  }, [hash]);

  return (
    <>
      <PageHeader
        title="Transaction Details"
        description="Read-only local transaction metadata, indexing linkage and draft signature state."
      />
      {error ? <ErrorPanel message={error} /> : null}
      {!transaction && !error ? <LoadingPanel message="Loading indexed transaction..." /> : null}
      {transaction ? (
        <section className="panel">
          <div className="detail-list">
            <div className="detail-row">
              <div className="detail-label">Transaction hash</div>
              <div className="hash-text">{transaction.hash}</div>
            </div>
            <div className="detail-row">
              <div className="detail-label">Block</div>
              <div>
                {transaction.block_height !== null ? (
                  <Link to={`/blocks/${transaction.block_height}`}>Block {transaction.block_height}</Link>
                ) : (
                  "Pending in local mempool"
                )}
              </div>
            </div>
            <div className="detail-row">
              <div className="detail-label">Block hash</div>
              <div className="hash-text">{transaction.block_hash ?? "Unavailable"}</div>
            </div>
            <div className="detail-row">
              <div className="detail-label">From</div>
              <div>
                {transaction.from ? (
                  <Link to={`/address/${transaction.from}`}>{transaction.from}</Link>
                ) : (
                  "Coinbase"
                )}
              </div>
            </div>
            <div className="detail-row">
              <div className="detail-label">To</div>
              <div>
                <Link to={`/address/${transaction.to}`}>{transaction.to}</Link>
              </div>
            </div>
            <div className="detail-row">
              <div className="detail-label">Amount</div>
              <div>{formatAtomic(transaction.amount_atomic)}</div>
            </div>
            <div className="detail-row">
              <div className="detail-label">Fee</div>
              <div>{formatAtomic(transaction.fee_atomic)}</div>
            </div>
            <div className="detail-row">
              <div className="detail-label">Lifecycle status</div>
              <div>{transaction.lifecycle_status}</div>
            </div>
            <div className="detail-row">
              <div className="detail-label">Authorization state</div>
              <div>{transaction.authorization_state}</div>
            </div>
            <div className="detail-row">
              <div className="detail-label">Sender public key</div>
              <div className="hash-text">{transaction.sender_public_key_hex ?? "Unavailable"}</div>
            </div>
            <div className="detail-row">
              <div className="detail-label">Signature</div>
              <div className="hash-text">{transaction.signature_hex ?? "Unavailable"}</div>
            </div>
            <div className="detail-row">
              <div className="detail-label">Memo hash</div>
              <div className="hash-text">{transaction.memo_hash ?? "Unavailable"}</div>
            </div>
          </div>
        </section>
      ) : null}
    </>
  );
}
