import { useEffect, useState } from "react";
import { Link, useParams } from "../lib/router";
import { PageHeader } from "../components/PageHeader";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/StatePanels";
import { AddressActivity, fetchJson, IndexedBlock, RpcTransactionResponse } from "../lib/api";

type SearchResult =
  | { kind: "block"; height: number; hash: string }
  | { kind: "transaction"; hash: string; lifecycle: string }
  | { kind: "address"; address: string };

export function SearchPage() {
  const { query = "" } = useParams();
  const [result, setResult] = useState<SearchResult | null>(null);
  const [missing, setMissing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setResult(null);
    setMissing(false);
    setError(null);
    const value = query.trim();

    (async () => {
      if (/^\d+$/.test(value)) {
        try {
          const block = await fetchJson<IndexedBlock>(`/indexer/blocks/${value}`);
          return { kind: "block", height: block.height, hash: block.hash } as SearchResult;
        } catch {
          return null;
        }
      }
      if (value.startsWith("alve1")) {
        try {
          const activity = await fetchJson<AddressActivity>(`/indexer/address/${encodeURIComponent(value)}`);
          return { kind: "address", address: activity.address } as SearchResult;
        } catch {
          return null;
        }
      }
      try {
        const block = await fetchJson<IndexedBlock>(`/indexer/blocks/hash/${value}`);
        return { kind: "block", height: block.height, hash: block.hash } as SearchResult;
      } catch {
        // A 64-character query may be a transaction hash instead.
      }
      try {
        const tx = await fetchJson<RpcTransactionResponse>(`/transactions/${value}`);
        return { kind: "transaction", hash: tx.hash, lifecycle: tx.lifecycle_status } as SearchResult;
      } catch {
        return null;
      }
    })().then((match) => {
      if (!active) return;
      if (match) setResult(match);
      else setMissing(true);
    }).catch((loadError) => {
      if (active) setError(loadError instanceof Error ? loadError.message : "Search failed");
    });

    return () => { active = false; };
  }, [query]);

  return (
    <>
      <PageHeader title="Search" description="Exact lookup across indexed block heights, block hashes, transaction hashes, and addresses." />
      {error ? <ErrorPanel message={error} /> : null}
      {!result && !missing && !error ? <LoadingPanel message={`Searching for ${query}...`} /> : null}
      {missing ? <EmptyPanel message="No exact candidate-chain or mempool match was found." /> : null}
      {result ? <section className="panel search-result">
        <div className="page-kicker">{result.kind}</div>
        {result.kind === "block" ? <Link to={`/blocks/${result.height}`}>Open block {result.height} ({result.hash})</Link> : null}
        {result.kind === "transaction" ? <Link to={`/tx/${result.hash}`}>Open {result.lifecycle} transaction {result.hash}</Link> : null}
        {result.kind === "address" ? <Link to={`/address/${result.address}`}>Open address {result.address}</Link> : null}
      </section> : null}
    </>
  );
}
