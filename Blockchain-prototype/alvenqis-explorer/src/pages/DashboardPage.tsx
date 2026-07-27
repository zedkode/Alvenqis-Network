import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  fetchJson,
  HealthResponse,
  IndexOverviewResponse,
  indexerLag,
  IndexerStatusResponse,
  IndexedBlock,
  IndexedTransaction,
  MempoolStatusResponse,
  P2pStatusResponse,
  StatusResponse,
} from "../lib/api";
import { formatAtomic, formatCount, formatHashrate, formatTimestamp, shortHash } from "../lib/format";
import { PageHeader } from "../components/PageHeader";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/StatePanels";
import { SummaryCard } from "../components/SummaryCard";
import { BlockActivityChart } from "../components/BlockActivityChart";

interface DashboardState {
  health: HealthResponse;
  chainStatus: StatusResponse;
  indexerStatus: IndexerStatusResponse;
  indexSummary: IndexOverviewResponse;
  latestBlocks: IndexedBlock[];
  latestTransactions: IndexedTransaction[];
  mempoolStatus: MempoolStatusResponse;
  p2pStatus: P2pStatusResponse;
}

export function DashboardPage() {
  const [state, setState] = useState<DashboardState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    (async () => {
      try {
        const [health, chainStatus, indexerStatus, indexSummary, mempoolStatus, p2pStatus] =
          await Promise.all([
            fetchJson<HealthResponse>("/health"),
            fetchJson<StatusResponse>("/status"),
            fetchJson<IndexerStatusResponse>("/indexer/status"),
            fetchJson<IndexOverviewResponse>("/indexer/overview?blocks=18&transactions=10"),
            fetchJson<MempoolStatusResponse>("/mempool/status").catch(() => ({
              status: "unavailable",
              pending_count: 0,
              anticipated_base_fee_atomic: 0,
              total_fees_atomic: 0,
              total_burned_fees_atomic: 0,
              total_priority_fees_atomic: 0,
              highest_priority_fee_atomic: 0,
              highest_max_fee_atomic: 0,
            })),
            fetchJson<P2pStatusResponse>("/p2p/status"),
          ]);
        if (active) {
          setState({
            health,
            chainStatus,
            indexerStatus,
            indexSummary,
            latestBlocks: indexSummary.recent_blocks,
            latestTransactions: indexSummary.recent_transactions,
            mempoolStatus,
            p2pStatus,
          });
        }
      } catch (loadError) {
        if (active) {
          setError(loadError instanceof Error ? loadError.message : "Unknown dashboard error");
        }
      }
    })();

    return () => {
      active = false;
    };
  }, []);

  return (
    <>
      <PageHeader
        title="Explorer Dashboard"
        description="Local prototype visibility for Alvenqis chain status, indexed state and recent block activity."
      />

      {error ? <ErrorPanel message={error} /> : null}
      {!state && !error ? <LoadingPanel message="Loading local RPC and indexer state..." /> : null}

      {state ? (
        <div className="grid" style={{ gap: 20 }}>
          {indexerLag(state.chainStatus.height, state.indexerStatus.indexed_height) !== 0 ? (
            <section className="panel warning-box">
              Indexed data is behind the chain by {formatCount(indexerLag(state.chainStatus.height, state.indexerStatus.indexed_height))} blocks.
              Mined lists and address aggregates may be stale until the local indexer refreshes.
            </section>
          ) : null}
          {state.mempoolStatus.status === "unavailable" ? (
            <section className="panel warning-box">
              Mempool data is unavailable. Mined chain and index data remain readable.
            </section>
          ) : null}
          <div className="grid cards-5">
            <SummaryCard
              label="Network"
              value={state.health.network_id}
              note={state.health.status_label}
            />
            <SummaryCard
              label="Chain Height"
              value={formatCount(state.chainStatus.height)}
              note={`Blocks: ${state.chainStatus.block_count.toLocaleString()}`}
            />
            <SummaryCard
              label="Indexed Height"
              value={formatCount(state.indexerStatus.indexed_height)}
              note={`Indexed txs: ${state.indexerStatus.transaction_count.toLocaleString()}`}
            />
            <SummaryCard
              label="Emitted Supply"
              value={formatAtomic(state.indexSummary.summary.supply.emitted_supply_atomic)}
              note={`Remaining: ${formatAtomic(state.indexSummary.summary.supply.remaining_supply_atomic)}`}
            />
            <SummaryCard
              label="Pending Txs"
              value={state.mempoolStatus.pending_count.toLocaleString()}
              note={`Pending fees: ${formatAtomic(state.mempoolStatus.total_fees_atomic)}`}
            />
            <SummaryCard
              label="Connected Peers"
              value={state.p2pStatus.connected_peer_count.toLocaleString()}
              note={`${state.p2pStatus.validated_peer_count} handshake validated`}
            />
          </div>

          <BlockActivityChart blocks={state.latestBlocks} />

          <div className="grid two-col">
            <section className="panel">
              <h2>Latest Indexed Blocks</h2>
              {state.latestBlocks.length === 0 ? (
                <EmptyPanel message="No indexed blocks are available yet. Run the local indexer first." />
              ) : (
                <div className="table-wrap">
                  <table>
                    <thead>
                      <tr>
                        <th>Height</th>
                        <th>Hash</th>
                        <th>Timestamp</th>
                        <th>Tx Count</th>
                        <th>Miner Reward</th>
                      </tr>
                    </thead>
                    <tbody>
                      {state.latestBlocks.map((block) => (
                        <tr key={block.hash}>
                          <td>
                            <Link to={`/blocks/${block.height}`}>{block.height}</Link>
                          </td>
                          <td className="hash-text">{shortHash(block.hash, 18)}</td>
                          <td>{formatTimestamp(block.timestamp)}</td>
                          <td>{block.transaction_count}</td>
                          <td>{formatAtomic(block.miner_reward_atomic)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </section>

            <section className="panel">
              <h2>Runtime Status</h2>
              <div className="detail-list">
                <div className="detail-row">
                  <div className="detail-label">RPC health</div>
                  <div>{state.health.ok ? "ok" : "unavailable"}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Network name</div>
                  <div>{state.chainStatus.network_name}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Network status</div>
                  <div>{state.chainStatus.status_label}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Chain initialized</div>
                  <div>{String(state.chainStatus.initialized)}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Indexer initialized</div>
                  <div>{String(state.indexerStatus.initialized)}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Indexer lag</div>
                  <div>{formatCount(indexerLag(state.chainStatus.height, state.indexerStatus.indexed_height))} blocks</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Tip hash</div>
                  <div className="hash-text">{state.chainStatus.tip_hash ?? "Unavailable"}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Indexed addresses</div>
                  <div>{state.indexSummary.summary.address_count.toLocaleString()}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Latest indexed block time</div>
                  <div>{formatTimestamp(state.indexSummary.summary.latest_block_timestamp)}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Pending mempool transactions</div>
                  <div>{state.mempoolStatus.pending_count.toLocaleString()}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Pending mempool fees</div>
                  <div>{formatAtomic(state.mempoolStatus.total_fees_atomic)}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">P2P chain sync</div>
                  <div>{state.p2pStatus.syncing ? "syncing" : "idle"}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Validating / mining peers</div>
                  <div>{state.p2pStatus.validating_peer_count} / {state.p2pStatus.mining_peer_count}</div>
                </div>
                <div className="detail-row">
                  <div className="detail-label">Observed network hashrate</div>
                  <div>{formatHashrate(state.p2pStatus.observed_network_hashrate_hs)}</div>
                </div>
              </div>
            </section>
          </div>

          <section className="panel">
            <h2>Latest Mined Transactions</h2>
            {state.latestTransactions.length === 0 ? (
              <EmptyPanel message="No mined transactions are indexed yet." />
            ) : (
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>Hash</th>
                      <th>Block</th>
                      <th>From</th>
                      <th>To</th>
                      <th>Amount</th>
                      <th>Fee</th>
                    </tr>
                  </thead>
                  <tbody>
                    {state.latestTransactions.map((transaction) => (
                      <tr key={transaction.hash}>
                        <td>
                          <Link className="hash-text" to={`/tx/${transaction.hash}`}>
                            {shortHash(transaction.hash, 20)}
                          </Link>
                        </td>
                        <td>
                          <Link to={`/blocks/${transaction.block_height}`}>
                            {transaction.block_height}
                          </Link>
                        </td>
                        <td className="hash-text">
                          {transaction.from ? shortHash(transaction.from, 18) : "Coinbase"}
                        </td>
                        <td className="hash-text">{shortHash(transaction.to, 18)}</td>
                        <td>{formatAtomic(transaction.amount_atomic)}</td>
                        <td>{formatAtomic(transaction.fee_atomic)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        </div>
      ) : null}
    </>
  );
}
