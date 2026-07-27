import { useEffect, useState } from "react";
import { PageHeader } from "../components/PageHeader";
import { ErrorPanel, LoadingPanel } from "../components/StatePanels";
import { SummaryCard } from "../components/SummaryCard";
import { fetchJson, IndexOverviewResponse, NetworkResponse, StatusResponse } from "../lib/api";
import { formatAtomic, formatCount } from "../lib/format";

interface SupplyState { network: NetworkResponse; status: StatusResponse; index: IndexOverviewResponse; }

export function SupplyPage() {
  const [state, setState] = useState<SupplyState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    Promise.all([
      fetchJson<NetworkResponse>("/network"),
      fetchJson<StatusResponse>("/status"),
      fetchJson<IndexOverviewResponse>("/indexer/overview?blocks=1&transactions=1"),
    ]).then(([network, status, index]) => active && setState({ network, status, index }))
      .catch((loadError) => active && setError(loadError instanceof Error ? loadError.message : "Failed to load supply"));
    return () => { active = false; };
  }, []);

  const emitted = state?.index.summary.supply.emitted_supply_atomic ?? 0;
  const max = state?.index.summary.supply.max_supply_atomic ?? 0;
  const issuedPercent = max === 0 ? 0 : (emitted / max) * 100;

  return (
    <>
      <PageHeader title="Supply" description="Read-only candidate-chain issuance data. Emitted supply counts minted rewards, not transaction fees." />
      {error ? <ErrorPanel message={error} /> : null}
      {!state && !error ? <LoadingPanel message="Loading supply data..." /> : null}
      {state ? <div className="grid" style={{ gap: 20 }}>
        <div className="grid cards-4">
          <SummaryCard label="Emitted" value={formatAtomic(emitted)} note={`${issuedPercent.toFixed(6)}% of maximum`} />
          <SummaryCard label="Maximum" value={formatAtomic(max)} note="Protocol supply ceiling" />
          <SummaryCard label="Remaining" value={formatAtomic(state.index.summary.supply.remaining_supply_atomic)} note="Not yet emitted" />
          <SummaryCard label="Indexed Height" value={formatCount(state.index.summary.indexed_height)} note={`Chain height: ${formatCount(state.status.height)}`} />
        </div>
        <section className="panel">
          <h2>Issuance Parameters</h2>
          <div className="detail-list">
            <div className="detail-row"><div className="detail-label">Initial block reward</div><div>{formatAtomic(state.network.initial_block_reward_atomic)}</div></div>
            <div className="detail-row"><div className="detail-label">Halving interval</div><div>{state.network.halving_interval_blocks.toLocaleString()} blocks</div></div>
            <div className="detail-row"><div className="detail-label">Target block time</div><div>{state.network.block_time_seconds} seconds</div></div>
            <div className="detail-row"><div className="detail-label">Atomic units per ALVE</div><div>{state.network.atomic_units_per_alve.toLocaleString()}</div></div>
            <div className="detail-row"><div className="detail-label">Fee policy</div><div>{state.network.fee_policy}</div></div>
          </div>
        </section>
      </div> : null}
    </>
  );
}
