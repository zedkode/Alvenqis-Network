import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Pagination } from "../components/Pagination";
import { PageHeader } from "../components/PageHeader";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/StatePanels";
import { fetchJson, IndexedBlock, PaginatedResponse } from "../lib/api";
import { formatAtomic, formatTimestamp, shortHash } from "../lib/format";

const PAGE_SIZE = 20;

export function LatestBlocksPage() {
  const [blocks, setBlocks] = useState<PaginatedResponse<IndexedBlock> | null>(null);
  const [page, setPage] = useState(1);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setBlocks(null);
    setError(null);
    const offset = (page - 1) * PAGE_SIZE;
    fetchJson<PaginatedResponse<IndexedBlock>>(`/indexer/blocks?offset=${offset}&limit=${PAGE_SIZE}`)
      .then((response) => active && setBlocks(response))
      .catch((loadError) => active && setError(loadError instanceof Error ? loadError.message : "Failed to load blocks"));
    return () => { active = false; };
  }, [page]);

  const pageCount = Math.max(1, Math.ceil((blocks?.total ?? 0) / PAGE_SIZE));
  const visible = blocks?.items ?? [];

  return <>
    <PageHeader title="Blocks" description="Paginated mined blocks from the local Mainnet Candidate index snapshot." />
    {error ? <ErrorPanel message={error} /> : null}
    {!blocks && !error ? <LoadingPanel message="Loading indexed blocks..." /> : null}
    {blocks?.total === 0 ? <EmptyPanel message="No indexed blocks are available." /> : null}
    {visible.length > 0 ? <section className="panel">
      <div className="table-wrap"><table>
        <thead><tr><th>Height</th><th>Hash</th><th>Time</th><th>Transactions</th><th>Base fee</th><th>Reward</th><th>Fees</th></tr></thead>
        <tbody>{visible.map((block) => <tr key={block.hash}>
          <td><Link to={`/blocks/${block.height}`}>{block.height}</Link></td>
          <td className="hash-text">{shortHash(block.hash, 20)}</td>
          <td>{formatTimestamp(block.timestamp)}</td>
          <td>{block.transaction_count}</td>
          <td>{formatAtomic(block.base_fee_atomic)}</td>
          <td>{formatAtomic(block.miner_reward_atomic)}</td>
          <td>{formatAtomic(block.fees_atomic)}</td>
        </tr>)}</tbody>
      </table></div>
      <Pagination page={page} pageCount={pageCount} onPageChange={setPage} />
    </section> : null}
  </>;
}
