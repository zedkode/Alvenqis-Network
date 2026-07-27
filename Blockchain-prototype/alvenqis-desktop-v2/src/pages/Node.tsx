import { useEffect, useRef, useState, type CSSProperties } from "react";
import { Activity, Download, Play, Radio, RefreshCw, ShieldCheck, Square } from "lucide-react";
import { LIVE_LOG_INTERVAL_MS } from "@shared/constants";
import { formatHashrate, shortHash } from "@shared/format";
import { Gauge } from "../components/charts/Gauge";
import { TelemetryChart } from "../components/charts/TelemetryChart";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { KeyValue, Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { useAppSettings } from "../hooks/useAppSettings";
import type { SeriesPoint } from "../shared/chartPath";
import { useApp } from "../model";
import { connectionAgeLabel, connectionLabel } from "../shared/connectivity";

function peerHealth(
  validated: boolean,
  error: string | null,
  bestHeight: number | null,
  localHeight: number | null
): "Healthy" | "Warning" | "Offline" {
  if (!validated || error) return "Offline";
  if (bestHeight !== null && localHeight !== null && localHeight - bestHeight > 2) return "Warning";
  return "Healthy";
}

export function Node() {
  const { snapshot: n, operator, setNotice, refresh } = useApp();
  const { settings } = useAppSettings();
  const [operatorMode, setOperatorMode] = useState(false);
  const [logService, setLogService] = useState("node");
  const [logs, setLogs] = useState("");
  const [operatorOutput, setOperatorOutput] = useState("");
  const [heightHistory, setHeightHistory] = useState<SeriesPoint[]>([]);
  const [peerHistory, setPeerHistory] = useState<SeriesPoint[]>([]);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [seedAddress, setSeedAddress] = useState("");
  const logsRef = useRef<HTMLPreElement>(null);

  const loadLogs = async () => setLogs(await window.alvenqis.logs.recent(logService, 220));

  useEffect(() => {
    void refresh();
  }, [refresh, settings.rpc_url]);

  useEffect(() => {
    void loadLogs();
    const logMs = Math.max(
      3_000,
      settings.live_log_interval_ms || LIVE_LOG_INTERVAL_MS
    );
    const timer = window.setInterval(() => void loadLogs(), logMs);
    return () => window.clearInterval(timer);
  }, [logService, settings.live_log_interval_ms]);

  useEffect(() => {
    const ts = Date.now();
    if (n.rpc_connection.status === "online" && n.height !== null) {
      setHeightHistory((v) => [...v, { value: n.height!, ts }].slice(-60));
    }
    if (n.p2p_connection.status === "online") {
      setPeerHistory((v) => [...v, { value: n.connected_peer_count, ts }].slice(-60));
    }
    setLastUpdated(new Date());
  }, [
    n.height,
    n.connected_peer_count,
    n.tip_hash,
    n.rpc_connection.status,
    n.p2p_connection.status
  ]);

  useEffect(() => {
    if (logsRef.current) logsRef.current.scrollTop = logsRef.current.scrollHeight;
  }, [logs]);

  const run = async (command: "start" | "stop" | "restart" | "validate" | "backup" | "status") => {
    try {
      const output = await operator(command);
      setOperatorOutput(output || `${command} completed without additional output.`);
      setNotice({ error: false, text: `Command "${command}" completed.` });
      await refresh();
    } catch (error) {
      setOperatorOutput(String(error));
      setNotice({ error: true, text: `Command "${command}" failed. See operator output.` });
    }
  };

  const addSeed = async () => {
    try {
      const message = await window.alvenqis.network.addSeed(seedAddress);
      setSeedAddress("");
      setNotice({ error: false, text: message });
    } catch (error) {
      setNotice({ error: true, text: String(error) });
    }
  };

  return (
    <div className="page grid node-page">
      <PageHero
        kicker="VPS GATEWAY · P2P VIEW"
        title="Network"
        titleAccent="ops"
        description="Observe peers, fleet, mining gossip and gateway health. Local start/stop of the full node stack is disabled in VPS mode."
        actions={
          <button className="button primary" type="button" onClick={() => void refresh()}>
            <Download size={14} /> Refresh telemetry
          </button>
        }
        side={
          <>
            <div className="page-hero-metric">
              <small>Peers</small>
              <strong>{n.p2p_connection.status === "online" ? n.connected_peer_count : "—"}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Validated</small>
              <strong>{n.p2p_connection.status === "online" ? n.validated_peer_count : "—"}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Observed HR</small>
              <strong>
                {n.p2p_connection.status === "online"
                  ? formatHashrate(n.observed_network_hashrate_hs)
                  : "—"}
              </strong>
            </div>
          </>
        }
      />

      <div className="live-ribbon">
        <span>
          <i /> AUTO REFRESH
        </span>
        <b>status 4s</b>
        <b>logs 2s</b>
        <b>last {lastUpdated?.toLocaleTimeString() ?? "—"}</b>
      </div>

      <div className="grid cols-5 telemetry-strip">
        <StatCard
          label="Gateway / validator"
          value={n.node_running ? "ACTIVE" : "INACTIVE"}
          detail="PoW rule authority on VPS"
          tone={n.node_running ? "positive" : undefined}
          icon={<ShieldCheck size={14} />}
        />
        <StatCard
          label="Connected peers"
          value={n.connected_peer_count}
          detail={`${n.validated_peer_count} handshake validated`}
        />
        <StatCard
          label="Observed miners"
          value={n.mining_peer_count}
          detail={formatHashrate(n.observed_network_hashrate_hs)}
        />
        <StatCard
          label="Validating peers"
          value={n.validating_peer_count}
          detail={n.p2p_syncing ? "Sync active" : "No active sync"}
        />
        <StatCard
          label="Temp bans"
          value={n.banned_peer_count ?? 0}
          detail={n.reputation_enabled === false ? "scoring off" : "peer reputation"}
          tone={(n.banned_peer_count ?? 0) > 0 ? "gold" : undefined}
        />
      </div>

      <div className="grid cols-2">
        {[["RPC", n.rpc_connection], ["P2P", n.p2p_connection]].map(([name, connection]) => (
          <Panel
            key={name as string}
            title={`${name} ${connectionLabel(connection as typeof n.rpc_connection)}`}
            detail={`circuit ${(connection as typeof n.rpc_connection).circuit}`}
          >
            <KeyValue label="Endpoint" mono>
              {(connection as typeof n.rpc_connection).endpoint}
            </KeyValue>
            <KeyValue label="Last success">
              {connectionAgeLabel(connection as typeof n.rpc_connection)}
            </KeyValue>
            <KeyValue label="Latency">
              {(connection as typeof n.rpc_connection).latency_ms !== null
                ? `${(connection as typeof n.rpc_connection).latency_ms} ms`
                : "—"}
            </KeyValue>
            {(connection as typeof n.rpc_connection).error ? (
              <p className="hw-error">{(connection as typeof n.rpc_connection).error}</p>
            ) : null}
          </Panel>
        ))}
      </div>

      <div className="node-command-deck">
        <Panel title="Command center" detail="VPS mode — no local stack" className="node-controls">
          <div className={`node-beacon ${n.rpc_connection.status === "online" ? "online" : ""}`}>
            <div className="beacon-rings">
              <ShieldCheck size={34} />
            </div>
            <strong>
              RPC {connectionLabel(n.rpc_connection)}
            </strong>
            <span>
              {n.detail
                || n.p2p_error
                || (n.sync_status === "syncing" || n.p2p_syncing
                  ? `Synchronizing chain${n.sync_remaining_blocks != null ? ` (${n.sync_remaining_blocks} blocks)` : ""}`
                  : n.sync_status === "synced"
                    ? `Synced to gateway tip · H ${n.height ?? 0}`
                    : "Canonical chain via remote RPC")}
            </span>
          </div>
          <p className="muted" style={{ marginBottom: 12 }}>
            This app connects to the VPS RPC gateway. Mine from Miner against remote templates. Use
            the VPS admin panel for server services.
          </p>
          <label className="operator-toggle">
            <input
              type="checkbox"
              checked={operatorMode}
              onChange={(e) => setOperatorMode(e.target.checked)}
            />{" "}
            Show legacy local-stack controls (disabled in VPS mode)
          </label>
          <div className="button-row">
            <button className="button primary" type="button" disabled title="Local stack disabled">
              <Play size={14} /> Start local
            </button>
            <button className="button" type="button" disabled title="Local stack disabled">
              <RefreshCw size={14} /> Restart
            </button>
            <button className="button danger" type="button" disabled title="Local stack disabled">
              <Square size={14} /> Stop
            </button>
            <button
              className="button"
              type="button"
              disabled={!operatorMode}
              onClick={() => void run("status")}
            >
              Gateway status
            </button>
            <button className="button" type="button" onClick={() => void refresh()}>
              <Download size={14} /> Refresh
            </button>
          </div>
        </Panel>

        <Panel title="Network pulse" detail="P2P view" className="network-pulse-panel">
          <div className={`network-pulse ${n.p2p_connection.status === "online" ? "online" : ""}`}>
            <div className="pulse-center">
              <Radio size={25} />
              <strong>
                {n.p2p_connection.status === "online" ? n.connected_peer_count : "—"}
              </strong>
              <small>CONNECTED</small>
            </div>
            {Array.from({ length: Math.min(8, n.connected_peer_count) }, (_, index) => (
              <i key={index} style={{ "--peer-index": index } as CSSProperties} />
            ))}
          </div>
          <div className="network-counters">
            <span>
              <b>{n.p2p_connection.status === "online" ? n.validated_peer_count : "—"}</b> validated
            </span>
            <span>
              <b>{n.p2p_connection.status === "online" ? n.mining_peer_count : "—"}</b> mining
            </span>
            <span>
              <b>
                {n.p2p_connection.status === "online"
                  ? formatHashrate(n.observed_network_hashrate_hs)
                  : "—"}
              </b>{" "}
              observed
            </span>
          </div>
        </Panel>

        <Panel title="Sync & identity" detail="Noise + Yamux">
          <Gauge
            value={n.connected_peer_count}
            max={Math.max(12, n.connected_peer_count)}
            label="PEERS"
          />
          <KeyValue label="Peer ID" mono>
            {n.local_peer_id ?? "—"}
          </KeyValue>
          <KeyValue label="Height">{n.height ?? "—"}</KeyValue>
          <KeyValue label="Indexed">{n.indexed_height ?? "—"}</KeyValue>
          <KeyValue label="Listen endpoints">{n.p2p_listen_addresses.length}</KeyValue>
          <KeyValue label="Seeds">{n.configured_seed_count}</KeyValue>
        </Panel>
      </div>

      <div className="grid cols-2">
        <Panel title="Height timeline" detail="While page open">
          {heightHistory.length > 1 ? (
            <TelemetryChart values={heightHistory} label="Height" tone="positive" height={130} />
          ) : (
            <EmptyState>Waiting for a second status sample.</EmptyState>
          )}
        </Panel>
        <Panel title="Peer timeline" detail="Connected count">
          {peerHistory.length > 1 ? (
            <TelemetryChart values={peerHistory} label="Peers" tone="gold" height={130} />
          ) : (
            <EmptyState>Waiting for a second peer sample.</EmptyState>
          )}
        </Panel>
      </div>

      <Panel title="Peer cards" detail="No synthetic geography">
        {n.peers.length ? (
          <div className="node-topology">
            {n.peers.map((peer) => {
              const health = peerHealth(
                peer.handshake_validated,
                peer.last_error,
                peer.best_height,
                n.height
              );
              const score = peer.reputation_score ?? 50;
              const banned = Boolean(peer.banned);
              return (
                <article key={peer.peer_id} className="node-peer-card">
                  <header>
                    <b className="mono">{shortHash(peer.peer_id, 8)}</b>
                    <span
                      className={`health ${
                        banned
                          ? "negative"
                          : health === "Healthy"
                            ? "positive"
                            : health === "Offline"
                              ? "negative"
                              : "gold"
                      }`}
                    >
                      {banned ? "Banned" : health}
                    </span>
                  </header>
                  <KeyValue label="Addr" mono>
                    {peer.address ?? "—"}
                  </KeyValue>
                  <KeyValue label="Height">{peer.best_height ?? "—"}</KeyValue>
                  <KeyValue label="Score" mono>
                    {score}
                  </KeyValue>
                  <KeyValue label="Mining">{peer.mining ? formatHashrate(peer.hashrate_hs) : "No"}</KeyValue>
                </article>
              );
            })}
          </div>
        ) : (
          <EmptyState>
            No remote peers connected. One node alone is not a decentralized network.
          </EmptyState>
        )}
      </Panel>

      <Panel title="Peer table" detail="Full list">
        {n.peers.length ? (
          <table className="data-table">
            <thead>
              <tr>
                <th>Peer</th>
                <th>Address</th>
                <th>Height</th>
                <th>Score</th>
                <th>Validating</th>
                <th>Mining</th>
                <th>Hashrate</th>
                <th>Health</th>
              </tr>
            </thead>
            <tbody>
              {n.peers.map((peer) => {
                const health = peerHealth(
                  peer.handshake_validated,
                  peer.last_error,
                  peer.best_height,
                  n.height
                );
                const banned = Boolean(peer.banned);
                return (
                  <tr key={peer.peer_id}>
                    <td className="mono">{shortHash(peer.peer_id, 8)}</td>
                    <td className="mono">{peer.address ?? "—"}</td>
                    <td>{peer.best_height ?? "—"}</td>
                    <td className="mono">{peer.reputation_score ?? 50}</td>
                    <td>{peer.validating ? "Yes" : "No"}</td>
                    <td>{peer.mining ? "Yes" : "No"}</td>
                    <td>{peer.mining ? formatHashrate(peer.hashrate_hs) : "—"}</td>
                    <td
                      className={
                        banned
                          ? "negative"
                          : health === "Healthy"
                            ? "positive"
                            : health === "Offline"
                              ? "negative"
                              : "gold"
                      }
                    >
                      {banned ? "Banned" : health}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        ) : (
          <EmptyState>No peer rows.</EmptyState>
        )}
      </Panel>

      <Panel title="Full-node mining presence" detail="P2P gossip · expires ~30s">
        {n.miners.length ? (
          <table className="data-table">
            <thead>
              <tr>
                <th>Miner peer</th>
                <th>Scope</th>
                <th>Hashrate</th>
                <th>Template H</th>
                <th>Last signal</th>
              </tr>
            </thead>
            <tbody>
              {n.miners.map((miner) => (
                <tr key={miner.peer_id}>
                  <td className="mono">{shortHash(miner.peer_id, 8)}</td>
                  <td>{miner.local ? "Local" : "P2P"}</td>
                  <td>{formatHashrate(miner.hashrate_hs)}</td>
                  <td>{miner.template_height}</td>
                  <td>
                    {Math.max(0, Math.floor(Date.now() / 1000) - miner.updated_at_unix_seconds)}s ago
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <EmptyState>
            No full-node miner signal. HTTP pool miners appear on the Pool workers page, not as
            P2P peers.
          </EmptyState>
        )}
      </Panel>

      <Panel title="VPS fleet" detail="Controller reports">
        {n.fleet_nodes?.length ? (
          <table className="data-table">
            <thead>
              <tr>
                <th>VPS</th>
                <th>Host</th>
                <th>State</th>
                <th>Height</th>
                <th>Peers</th>
                <th>Miners</th>
                <th>Hashrate</th>
              </tr>
            </thead>
            <tbody>
              {n.fleet_nodes.map((node) => (
                <tr key={node.node_id}>
                  <td>{node.node_name}</td>
                  <td className="mono">{node.advertise_host}</td>
                  <td className={node.online ? "positive" : "negative"}>
                    {node.online ? "Online" : "Stale"}
                  </td>
                  <td>{node.height ?? "—"}</td>
                  <td>{node.connected_peers}</td>
                  <td>{node.mining_peers}</td>
                  <td>{formatHashrate(node.observed_hashrate_hs)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <EmptyState>No fleet endpoint on this RPC. Direct P2P above remains authoritative.</EmptyState>
        )}
      </Panel>

      <Panel title="Listen addresses" detail="libp2p endpoints">
        <div className="endpoint-grid">
          {n.p2p_listen_addresses.length ? (
            n.p2p_listen_addresses.map((address) => (
              <div className="endpoint-chip mono" key={address}>
                <Radio size={13} />
                {address}
              </div>
            ))
          ) : (
            <EmptyState>No P2P listen address reported.</EmptyState>
          )}
        </div>
      </Panel>

      <Panel title="P2P readiness" detail="TCP + Noise + Yamux">
        <div className="p2p-readiness">
          <span className={n.p2p_listen_addresses.length ? "ready" : "blocked"}>
            <b>1</b>
            <small>LISTEN</small>
            <strong>{n.p2p_listen_addresses.length ? "ADVERTISED" : "MISSING"}</strong>
          </span>
          <i />
          <span className={n.configured_seed_count ? "ready" : "warning"}>
            <b>2</b>
            <small>SEEDS</small>
            <strong>
              {n.configured_seed_count ? `${n.configured_seed_count} SEEDS` : "NO SEEDS"}
            </strong>
          </span>
          <i />
          <span className={n.connected_peer_count ? "ready" : "warning"}>
            <b>3</b>
            <small>HANDSHAKE</small>
            <strong>
              {n.connected_peer_count ? `${n.connected_peer_count} CONNECTED` : "WAITING"}
            </strong>
          </span>
        </div>
        <p className="muted p2p-guidance">
          A second device needs the same network ID/genesis, reachable TCP, and this multiaddress as
          seed.
        </p>
        <div className="seed-control">
          <input
            value={seedAddress}
            onChange={(e) => setSeedAddress(e.target.value)}
            placeholder="Remote seed, e.g. 192.168.1.20:20787"
          />
          <button
            className="button"
            type="button"
            disabled={!seedAddress.trim()}
            onClick={() => void addSeed()}
          >
            Add seed
          </button>
          <span>Restart node after saving. Remote port must be reachable.</span>
        </div>
      </Panel>

      {operatorOutput ? (
        <Panel title="Operator output" detail="Latest command">
          <pre className="log-view">{operatorOutput}</pre>
        </Panel>
      ) : null}

      <Panel
        title="Live event console"
        detail={
          <span className="console-live">
            <i /> STREAMING
          </span>
        }
        className="console-panel"
      >
        <div className="console-toolbar">
          <select value={logService} onChange={(e) => setLogService(e.target.value)}>
            <option value="node">Node</option>
            <option value="rpc">RPC</option>
            <option value="miner">Miner</option>
            <option value="explorer">Explorer</option>
          </select>
          <span>
            <Activity size={14} /> auto-scroll
          </span>
          <button className="button" type="button" onClick={() => void loadLogs()}>
            Refresh
          </button>
          <button className="button" type="button" onClick={() => window.alvenqis.logs.export(logService)}>
            Export
          </button>
        </div>
        <pre className="miner-console" ref={logsRef}>
          {logs || "No service log lines available."}
        </pre>
      </Panel>
    </div>
  );
}
