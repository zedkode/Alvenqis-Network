import { useEffect, useState } from "react";
import {
  fetchJson,
  FleetTopologyResponse,
  HealthResponse,
  IndexOverviewResponse,
  indexerLag,
  IndexerStatusResponse,
  MempoolStatusResponse,
  NetworkResponse,
  P2pStatusResponse,
  PoolStatusResponse,
  StatusResponse,
} from "../lib/api";
import { formatAtomic, formatCount, formatHashrate } from "../lib/format";
import { PageHeader } from "../components/PageHeader";
import { ErrorPanel, LoadingPanel } from "../components/StatePanels";

interface NetworkState {
  health: HealthResponse;
  network: NetworkResponse;
  status: StatusResponse;
  indexerStatus: IndexerStatusResponse;
  summary: IndexOverviewResponse;
  mempool: MempoolStatusResponse;
  p2p: P2pStatusResponse;
  fleet: FleetTopologyResponse | null;
  pool: PoolStatusResponse | null;
}

export function NetworkStatusPage() {
  const [state, setState] = useState<NetworkState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    Promise.all([
      fetchJson<HealthResponse>("/health"),
      fetchJson<NetworkResponse>("/network"),
      fetchJson<StatusResponse>("/status"),
      fetchJson<IndexerStatusResponse>("/indexer/status"),
      fetchJson<IndexOverviewResponse>("/indexer/overview?blocks=1&transactions=1"),
      fetchJson<MempoolStatusResponse>("/mempool/status"),
      fetchJson<P2pStatusResponse>("/p2p/status"),
      fetchJson<FleetTopologyResponse>("/fleet/status").catch(() => null),
      fetchJson<PoolStatusResponse>("/pool/api/v1/pool/status").catch(() => null),
    ])
      .then(([health, network, status, indexerStatus, summary, mempool, p2p, fleet, pool]) => {
        if (active) {
          setState({ health, network, status, indexerStatus, summary, mempool, p2p, fleet, pool });
        }
      })
      .catch((loadError) => {
        if (active) {
          setError(loadError instanceof Error ? loadError.message : "Failed to load status");
        }
      });

    return () => {
      active = false;
    };
  }, []);

  return (
    <>
      <PageHeader
        title="Network Status"
        description="Static and dynamic local network status pulled from the local RPC gateway and stored index."
      />
      {error ? <ErrorPanel message={error} /> : null}
      {!state && !error ? <LoadingPanel message="Loading network metadata..." /> : null}
      {state ? (
        <div className="grid two-col">
          <section className="panel">
            <h2>RPC Status</h2>
            <div className="detail-list">
              <div className="detail-row">
                <div className="detail-label">Service</div>
                <div>{state.health.service}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Mode</div>
                <div>{state.health.mode}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Network</div>
                <div>{state.network.network_id}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Human name</div>
                <div>{state.network.network_name}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Status label</div>
                <div>{state.network.status_label}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Ticker</div>
                <div>{state.network.ticker}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Address prefix</div>
                <div>{state.network.address_prefix}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Block time</div>
                <div>{state.network.block_time_seconds} seconds</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Decimals</div>
                <div>{state.network.decimals}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Default RPC port</div>
                <div>{state.network.default_rpc_port}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Default P2P port</div>
                <div>{state.network.default_p2p_port}</div>
              </div>
            </div>
          </section>

          <section className="panel">
            <h2>Indexed Chain Summary</h2>
            <div className="detail-list">
              <div className="detail-row">
                <div className="detail-label">Chain initialized</div>
                <div>{String(state.status.initialized)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Chain height</div>
                <div>{formatCount(state.status.height)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Indexed height</div>
                <div>{formatCount(state.indexerStatus.indexed_height)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Indexed blocks</div>
                <div>{state.indexerStatus.indexed_block_count.toLocaleString()}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Indexer lag</div>
                <div>{formatCount(indexerLag(state.status.height, state.indexerStatus.indexed_height))} blocks</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Indexed transactions</div>
                <div>{state.indexerStatus.transaction_count.toLocaleString()}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Indexed addresses</div>
                <div>{state.indexerStatus.address_count.toLocaleString()}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Emitted supply</div>
                <div>{formatAtomic(state.summary.summary.supply.emitted_supply_atomic)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Remaining supply</div>
                <div>{formatAtomic(state.summary.summary.supply.remaining_supply_atomic)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Pending mempool transactions</div>
                <div>{state.mempool.pending_count.toLocaleString()}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Pending mempool fees</div>
                <div>{formatAtomic(state.mempool.total_fees_atomic)}</div>
              </div>
            </div>
          </section>

          <section className="panel">
            <h2>Local P2P View</h2>
            <div className="detail-list">
              <div className="detail-row"><div className="detail-label">Local peer ID</div><div className="hash-text">{state.p2p.local_peer_id || "P2P offline"}</div></div>
              <div className="detail-row"><div className="detail-label">Protocol version</div><div>{state.p2p.protocol_version}</div></div>
              <div className="detail-row"><div className="detail-label">Connected peers</div><div>{state.p2p.connected_peer_count}</div></div>
              <div className="detail-row"><div className="detail-label">Handshake validated</div><div>{state.p2p.validated_peer_count}</div></div>
              <div className="detail-row"><div className="detail-label">Validating peers</div><div>{state.p2p.validating_peer_count}</div></div>
              <div className="detail-row"><div className="detail-label">Mining peers</div><div>{state.p2p.mining_peer_count}</div></div>
              <div className="detail-row"><div className="detail-label">Observed network hashrate</div><div>{formatHashrate(state.p2p.observed_network_hashrate_hs)}</div></div>
              <div className="detail-row"><div className="detail-label">Chain sync</div><div>{state.p2p.syncing ? "syncing" : "idle"}</div></div>
              <div className="detail-row"><div className="detail-label">Listen address</div><div className="hash-text">{state.p2p.listen_addresses[0] ?? "Unavailable"}</div></div>
              <div className="detail-row"><div className="detail-label">Configured seeds</div><div>{state.p2p.configured_seed_count}</div></div>
              <div className="detail-row"><div className="detail-label">Last P2P error</div><div>{state.p2p.last_error ?? "None"}</div></div>
            </div>
          </section>

          <section className="panel">
            <h2>Observed VPS Fleet</h2>
            {state.fleet ? <div className="detail-list">
              <div className="detail-row"><div className="detail-label">Scope</div><div>{state.fleet.mode}</div></div>
              <div className="detail-row"><div className="detail-label">Registered VPS nodes</div><div>{state.fleet.registered_node_count}</div></div>
              <div className="detail-row"><div className="detail-label">Fresh reports</div><div>{state.fleet.online_node_count}</div></div>
              <div className="detail-row"><div className="detail-label">Direct validated links</div><div>{state.fleet.direct_validated_connections}</div></div>
              <div className="detail-row"><div className="detail-label">Observed miners</div><div>{state.fleet.observed_miner_count}</div></div>
              <div className="detail-row"><div className="detail-label">Observed hashrate</div><div>{formatHashrate(state.fleet.observed_hashrate_hs)}</div></div>
            </div> : <p className="muted">The selected RPC does not expose the optional sanitized fleet endpoint.</p>}
          </section>

          <section className="panel">
            <h2>Mining Pool</h2>
            {state.pool ? <div className="detail-list">
              <div className="detail-row"><div className="detail-label">Pool</div><div>{state.pool.pool_name}</div></div>
              <div className="detail-row"><div className="detail-label">Status</div><div>{state.pool.status_label}</div></div>
              <div className="detail-row"><div className="detail-label">Upstream node</div><div>{state.pool.upstream_status}</div></div>
              <div className="detail-row"><div className="detail-label">Payout scheme</div><div>{state.pool.payout_scheme}</div></div>
              <div className="detail-row"><div className="detail-label">Online workers</div><div>{state.pool.connected_workers}</div></div>
              <div className="detail-row"><div className="detail-label">Estimated hashrate</div><div>{formatHashrate(state.pool.estimated_hashrate_hs)}</div></div>
              <div className="detail-row"><div className="detail-label">Blocks found / mature</div><div>{state.pool.blocks_found} / {state.pool.matured_blocks}</div></div>
              <div className="detail-row"><div className="detail-label">Pool fee</div><div>{(state.pool.pool_fee_basis_points / 100).toFixed(2)}%</div></div>
              <div className="detail-row"><div className="detail-label">Variable difficulty</div><div>{state.pool.vardiff_enabled ? `${state.pool.target_share_seconds}s target` : "Fixed"}</div></div>
              <div className="detail-row"><div className="detail-label">Rejected / limited</div><div>{state.pool.rejected_requests} / {state.pool.rate_limited_requests}</div></div>
              <div className="detail-row"><div className="detail-label">Active bans</div><div>{state.pool.active_bans}</div></div>
            </div> : <p className="muted">No mining-pool service is exposed by the selected endpoint.</p>}
          </section>
        </div>
      ) : null}
    </>
  );
}
