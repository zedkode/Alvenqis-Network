import { FleetTopologyResponse, P2pStatusResponse } from "../lib/api";
import { formatHashrate } from "../lib/format";

interface NetworkTopologyProps {
  fleet: FleetTopologyResponse | null;
  p2p: P2pStatusResponse;
}

export function NetworkTopology({ fleet, p2p }: NetworkTopologyProps) {
  const nodes = fleet?.nodes ?? [];

  return (
    <section className="panel topology-panel">
      <div className="panel-heading">
        <div>
          <div className="panel-kicker">Observed topology</div>
          <h2>Peer and fleet connections</h2>
        </div>
        <span className={`connection-state ${p2p.validated_peer_count > 0 ? "online" : ""}`}>
          {p2p.validated_peer_count} validated peers
        </span>
      </div>

      <div className="topology-grid">
        <div className="topology-core">
          <img src="/alvenqis-logo.png" alt="" />
          <strong>Configured RPC view</strong>
          <span>{p2p.local_peer_id || "P2P unavailable"}</span>
        </div>
        <div className="topology-links" aria-hidden="true" />
        <div className="topology-nodes">
          {nodes.length ? nodes.slice(0, 8).map((node) => (
            <article className={`topology-node ${node.online ? "online" : ""}`} key={node.node_id}>
              <div><span className="live-dot" /> {node.node_name}</div>
              <strong>{node.height === null ? "No height" : `#${node.height}`}</strong>
              <span>{node.connected_peers} peers · {formatHashrate(node.observed_hashrate_hs)}</span>
            </article>
          )) : (
            <div className="topology-empty">
              Fleet endpoint is not exposed. Local P2P reports {p2p.connected_peer_count} connected and {p2p.validated_peer_count} validated peers.
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
