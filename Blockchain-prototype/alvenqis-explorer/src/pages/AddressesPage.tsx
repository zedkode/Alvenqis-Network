import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Pagination } from "../components/Pagination";
import { PageHeader } from "../components/PageHeader";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/StatePanels";
import { AddressActivity, fetchJson, PaginatedResponse } from "../lib/api";
import { formatAtomic, shortHash } from "../lib/format";

const PAGE_SIZE = 20;

export function AddressesPage() {
  const [addresses, setAddresses] = useState<PaginatedResponse<AddressActivity> | null>(null);
  const [page, setPage] = useState(1);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setAddresses(null);
    setError(null);
    const offset = (page - 1) * PAGE_SIZE;
    fetchJson<PaginatedResponse<AddressActivity>>(`/indexer/addresses?offset=${offset}&limit=${PAGE_SIZE}`)
      .then((response) => active && setAddresses(response))
      .catch((loadError) => {
        if (active) setError(loadError instanceof Error ? loadError.message : "Failed to load addresses");
      });
    return () => { active = false; };
  }, [page]);

  const pageCount = Math.max(1, Math.ceil((addresses?.total ?? 0) / PAGE_SIZE));
  const visible = addresses?.items ?? [];

  return (
    <>
      <PageHeader title="Addresses" description="Indexed account balances and aggregate mined-chain activity. This is not a wallet or ownership directory." />
      {error ? <ErrorPanel message={error} /> : null}
      {!addresses && !error ? <LoadingPanel message="Loading indexed addresses..." /> : null}
      {addresses?.total === 0 ? <EmptyPanel message="No addresses are indexed." /> : null}
      {visible.length > 0 ? (
        <section className="panel">
          <div className="table-wrap">
            <table>
              <thead><tr><th>Address</th><th>Balance</th><th>Received</th><th>Sent incl. fees</th><th>Transactions</th><th>Mined blocks</th></tr></thead>
              <tbody>{visible.map((account) => (
                <tr key={account.address}>
                  <td><Link className="hash-text" to={`/address/${account.address}`}>{shortHash(account.address, 22)}</Link></td>
                  <td>{formatAtomic(account.balance_atomic)}</td>
                  <td>{formatAtomic(account.total_received_atomic)}</td>
                  <td>{formatAtomic(account.total_sent_atomic)}</td>
                  <td>{account.transaction_hashes.length.toLocaleString()}</td>
                  <td>{account.mined_block_heights.length.toLocaleString()}</td>
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
