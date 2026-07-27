import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Pagination } from "../components/Pagination";
import { PageHeader } from "../components/PageHeader";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/StatePanels";
import { fetchJson, IndexedTransaction, PaginatedResponse } from "../lib/api";
import { formatAtomic, shortHash } from "../lib/format";

const PAGE_SIZE = 20;

export function TransactionsPage() {
  const [transactions, setTransactions] = useState<PaginatedResponse<IndexedTransaction> | null>(null);
  const [page, setPage] = useState(1);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setTransactions(null);
    setError(null);
    const offset = (page - 1) * PAGE_SIZE;
    fetchJson<PaginatedResponse<IndexedTransaction>>(`/indexer/transactions?offset=${offset}&limit=${PAGE_SIZE}`)
      .then((response) => active && setTransactions(response))
      .catch((loadError) => {
        if (active) setError(loadError instanceof Error ? loadError.message : "Failed to load transactions");
      });
    return () => { active = false; };
  }, [page]);

  const pageCount = Math.max(1, Math.ceil((transactions?.total ?? 0) / PAGE_SIZE));
  const visible = transactions?.items ?? [];

  return (
    <>
      <PageHeader title="Transactions" description="Paginated mined transaction history from the local Mainnet Candidate index snapshot." />
      {error ? <ErrorPanel message={error} /> : null}
      {!transactions && !error ? <LoadingPanel message="Loading indexed transactions..." /> : null}
      {transactions?.total === 0 ? <EmptyPanel message="No mined transactions are indexed." /> : null}
      {visible.length > 0 ? (
        <section className="panel">
          <div className="table-wrap">
            <table>
              <thead><tr><th>Hash</th><th>Block</th><th>From</th><th>To</th><th>Amount</th><th>Fee</th><th>Type</th></tr></thead>
              <tbody>{visible.map((tx) => (
                <tr key={tx.hash}>
                  <td><Link className="hash-text" to={`/tx/${tx.hash}`}>{shortHash(tx.hash, 20)}</Link></td>
                  <td><Link to={`/blocks/${tx.block_height}`}>{tx.block_height}</Link></td>
                  <td className="hash-text">{tx.from ? shortHash(tx.from, 16) : "Coinbase"}</td>
                  <td><Link className="hash-text" to={`/address/${tx.to}`}>{shortHash(tx.to, 16)}</Link></td>
                  <td>{formatAtomic(tx.amount_atomic)}</td>
                  <td>{formatAtomic(tx.effective_fee_atomic)}</td>
                  <td>{tx.authorization_state}</td>
                </tr>
              ))}</tbody>
            </table>
          </div>
          <Pagination page={page} pageCount={pageCount} onPageChange={setPage} />
        </section>
      ) : null}
    </>
  );
}
