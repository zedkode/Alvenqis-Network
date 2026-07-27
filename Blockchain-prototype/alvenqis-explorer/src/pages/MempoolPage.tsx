import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { PageHeader } from "../components/PageHeader";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/StatePanels";
import { SummaryCard } from "../components/SummaryCard";
import { fetchJson, MempoolResponse } from "../lib/api";
import { formatAtomic, shortHash } from "../lib/format";

export function MempoolPage() {
  const [mempool, setMempool] = useState<MempoolResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    fetchJson<MempoolResponse>("/mempool")
      .then((response) => active && setMempool(response))
      .catch((loadError) => active && setError(loadError instanceof Error ? loadError.message : "Failed to load mempool"));
    return () => { active = false; };
  }, []);

  return (
    <>
      <PageHeader title="Mempool" description="Pending transactions currently accepted by the local candidate node. Entries are not confirmations." />
      {error ? <ErrorPanel message={error} /> : null}
      {!mempool && !error ? <LoadingPanel message="Loading pending transactions..." /> : null}
      {mempool ? <div className="grid" style={{ gap: 20 }}>
        <div className="grid cards-4">
          <SummaryCard label="Pending" value={mempool.pending_count.toLocaleString()} note={mempool.status} />
          <SummaryCard label="Total Fees" value={formatAtomic(mempool.total_fees_atomic)} note="Effective if mined next" />
          <SummaryCard label="Burned Fees" value={formatAtomic(mempool.total_burned_fees_atomic)} note="Anticipated base fee portion" />
          <SummaryCard label="Priority Fees" value={formatAtomic(mempool.total_priority_fees_atomic)} note={`Base fee: ${formatAtomic(mempool.anticipated_base_fee_atomic)}`} />
        </div>
        {mempool.transactions.length === 0 ? <EmptyPanel message="The local mempool is empty." /> : (
          <section className="panel">
            <div className="table-wrap"><table>
              <thead><tr><th>Hash</th><th>From</th><th>To</th><th>Amount</th><th>Max fee</th><th>Priority fee</th><th>Nonce</th></tr></thead>
              <tbody>{mempool.transactions.map((tx) => <tr key={tx.hash}>
                <td><Link className="hash-text" to={`/tx/${tx.hash}`}>{shortHash(tx.hash, 20)}</Link></td>
                <td className="hash-text">{tx.from ? shortHash(tx.from, 16) : "Unavailable"}</td>
                <td><Link className="hash-text" to={`/address/${tx.to}`}>{shortHash(tx.to, 16)}</Link></td>
                <td>{formatAtomic(tx.amount_atomic)}</td>
                <td>{formatAtomic(tx.max_fee_atomic)}</td>
                <td>{formatAtomic(tx.priority_fee_atomic)}</td>
                <td>{tx.nonce.toLocaleString()}</td>
              </tr>)}</tbody>
            </table></div>
          </section>
        )}
      </div> : null}
    </>
  );
}
