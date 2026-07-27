import { useEffect, useRef, useState, type PropsWithChildren } from "react";
import {
  Blocks, Cpu, Database, Network, Radio, Send, ShieldCheck, WalletCards, Zap
} from "lucide-react";
import { formatAtomic, formatHashrate, shortHash } from "@shared/format";
import { AlvenqisLogo } from "../components/brand/AlvenqisLogo";
import { TelemetryChart } from "../components/charts/TelemetryChart";
import { BarChart } from "../components/charts/BarChart";
import { Gauge } from "../components/charts/Gauge";
import { Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { ChainOrb } from "../components/visual/ChainOrb";
import { appendSample, type SeriesPoint } from "../shared/chartPath";
import { useApp } from "../model";

function DashboardSection({ title, children }: PropsWithChildren<{ title: string }>) {
  return (
    <section className="dashboard-widget widget-full">
      <div className="widget-toolbar">
        <span>{title}</span>
      </div>
      {children}
    </section>
  );
}

export function Overview() {
  const { snapshot: n, wallet, setPage } = useApp();
  const [hashrateHistory, setHashrateHistory] = useState<SeriesPoint[]>([]);
  const [heightHistory, setHeightHistory] = useState<SeriesPoint[]>([]);
  const [mempoolHistory, setMempoolHistory] = useState<SeriesPoint[]>([]);
  const [heightPulse, setHeightPulse] = useState(false);
  const prevHeight = useRef<number | null>(null);

  useEffect(() => {
    const ts = Date.now();
    if (
      n.miner_hashrate_hs !== null
      && (n.stratum_connection.status === "online" || n.rpc_connection.status === "online")
    ) {
      setHashrateHistory((v) => appendSample(v, Math.round(n.miner_hashrate_hs!), 60, ts));
    }
    if (n.rpc_connection.status === "online" && n.height !== null) {
      setHeightHistory((v) => appendSample(v, n.height!, 60, ts));
      if (prevHeight.current !== null && n.height > prevHeight.current) {
        setHeightPulse(true);
        window.setTimeout(() => setHeightPulse(false), 650);
      }
      prevHeight.current = n.height;
    }
    if (n.rpc_connection.status === "online") {
      setMempoolHistory((v) => appendSample(v, n.mempool_count, 60, ts));
    }
  }, [
    n.height,
    n.mempool_count,
    n.miner_hashrate_hs,
    n.tip_hash,
    n.rpc_connection.status,
    n.stratum_connection.status
  ]);

  const txPerBlock = [...n.recent_blocks].reverse().map((b) => b.transaction_count);
  const rewardSeries = [...n.recent_blocks].reverse().map((b) => ({
    value: Number(b.miner_reward_atomic) / 1e8,
    ts: b.timestamp * 1000
  }));
  const serviceStates = [
    ["NODE", n.p2p_connection.status === "online", ShieldCheck],
    ["RPC", n.rpc_connection.status === "online", Radio],
    ["INDEXER", n.indexer_ready, Database],
    ["MINER", n.miner_running, Cpu]
  ] as const;

  return (
    <div className="page grid overview-page">
      <section className="command-hero">
        <div className="hero-brand-watermark" aria-hidden="true">
          <AlvenqisLogo size="xl" alt="" />
        </div>
        <div className="hero-copy">
          <span className="hero-kicker">
            <i /> LIVE · MAINNET CANDIDATE · V2
          </span>
          <h2>
            Alvenqis <b>network</b>
            <br />
            command center
          </h2>
          <p>
            Elevated V2 shell: deeper analytics, message inbox, denser telemetry and motion — still
            bound to real VPS gateway data. Keys stay on this device.
          </p>
          <div className="button-row hero-actions">
            <button className="button primary" type="button" onClick={() => setPage("mining")}>
              <Zap size={14} /> Miner control
            </button>
            <button className="button" type="button" onClick={() => setPage("analytics")}>
              <Cpu size={14} /> Analytics
            </button>
            <button className="button" type="button" onClick={() => setPage("wallet")}>
              <WalletCards size={14} /> Wallet
            </button>
            <button className="button" type="button" onClick={() => setPage("send")}>
              <Send size={14} /> Send ALVE
            </button>
            <button className="button" type="button" onClick={() => setPage("messages")}>
              <Network size={14} /> Messages
            </button>
            <button className="button" type="button" onClick={() => setPage("explorer")}>
              <Blocks size={14} /> Explorer
            </button>
          </div>
        </div>
        <div className={heightPulse ? "height-pulse" : undefined}>
          <ChainOrb height={n.height} tipHash={n.tip_hash} online={!!n.online} />
        </div>
        <div className="hero-telemetry">
          <span>
            <small>Network hashrate</small>
            <strong>{formatHashrate(n.observed_network_hashrate_hs)}</strong>
          </span>
          <span>
            <small>Mempool</small>
            <strong>{n.mempool_count} TX</strong>
          </span>
          <span>
            <small>Peers</small>
            <strong>{n.connected_peer_count}</strong>
          </span>
          <span>
            <small>Indexed tip</small>
            <strong>{n.indexed_height ?? "—"}</strong>
          </span>
        </div>
      </section>

      <div className="fixed-dashboard">
        <DashboardSection title="Portfolio & chain pulse">
          <div className="grid cols-4 telemetry-strip">
            <StatCard
              label="ALVE balance"
              value={n.balance_atomic !== null ? formatAtomic(n.balance_atomic) : "—"}
              detail="On-chain · no fiat"
              tone="gold"
              icon={<WalletCards size={14} />}
            />
            <StatCard
              label="Active wallet"
              value={wallet ? shortHash(wallet.address, 7) : "Not set"}
              detail={wallet?.display_name ?? "Create or import a wallet"}
              icon={<ShieldCheck size={14} />}
            />
            <StatCard
              label="Miner"
              value={n.miner_running ? "MINING" : "IDLE"}
              detail={`${n.miner_active_backend ?? n.miner_backend_mode ?? "gpu"} · ${n.miner_accepted_blocks ?? 0} blocks`}
              tone={n.miner_running ? "positive" : undefined}
              icon={<Cpu size={14} />}
            />
            <StatCard
              label="Emitted supply"
              value={n.emitted_supply_atomic ? formatAtomic(n.emitted_supply_atomic) : "—"}
              detail={`Cap ${n.max_supply_atomic ? formatAtomic(n.max_supply_atomic) : "—"}`}
              icon={<Database size={14} />}
            />
          </div>
        </DashboardSection>

        <DashboardSection title="Live telemetry">
          <div className="dashboard-analytics">
            <Panel title="Local hashrate" detail="Samples while open">
              {hashrateHistory.length > 1 ? (
                <TelemetryChart values={hashrateHistory} label="Hashrate" unit="H/s" height={150} />
              ) : (
                <div className="chart-wait">Start mining or wait for a second sample.</div>
              )}
            </Panel>
            <Panel title="Chain height" detail="Gateway tip">
              {heightHistory.length > 1 ? (
                <TelemetryChart values={heightHistory} label="Height" tone="positive" height={150} />
              ) : (
                <div className="chart-wait">Waiting for a second chain sample.</div>
              )}
            </Panel>
            <Panel title="Mempool pressure" detail="Pending TX">
              {mempoolHistory.length > 1 ? (
                <TelemetryChart values={mempoolHistory} label="Pending" tone="gold" height={150} />
              ) : (
                <div className="chart-wait">Waiting for a second mempool sample.</div>
              )}
            </Panel>
          </div>
        </DashboardSection>

        <DashboardSection title="Block economics">
          <div className="grid cols-2">
            <Panel title="Tx density (recent blocks)" detail="Count per block">
              {txPerBlock.length > 1 ? (
                <BarChart values={txPerBlock} label="Tx / block" height={130} />
              ) : (
                <div className="chart-wait">Need more indexed blocks.</div>
              )}
            </Panel>
            <Panel title="Block rewards (ALVE)" detail="Recent window">
              {rewardSeries.length > 1 ? (
                <TelemetryChart values={rewardSeries} label="Reward" unit="ALVE" tone="gold" height={130} />
              ) : (
                <div className="chart-wait">Need more reward samples.</div>
              )}
            </Panel>
          </div>
        </DashboardSection>

        <DashboardSection title="Chain activity">
          <div className="grid cols-3">
            <Panel title="Recent blocks" detail={`${n.block_count} canonical`}>
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Height</th>
                    <th>Hash</th>
                    <th>Tx</th>
                    <th>Reward</th>
                  </tr>
                </thead>
                <tbody>
                  {n.recent_blocks.slice(0, 7).map((block) => (
                    <tr key={block.hash}>
                      <td className="positive mono">{block.height}</td>
                      <td className="mono">{shortHash(block.hash, 5)}</td>
                      <td>{block.transaction_count}</td>
                      <td className="gold mono">{formatAtomic(block.miner_reward_atomic)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Panel>
            <Panel title="Latest transactions" detail={`${n.indexed_transactions} indexed`}>
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Txid</th>
                    <th>Amount</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  {n.recent_transactions.slice(0, 7).map((tx) => (
                    <tr key={tx.hash}>
                      <td className="mono">{shortHash(tx.hash, 5)}</td>
                      <td className="mono">{formatAtomic(tx.amount_atomic)}</td>
                      <td className="positive">{tx.lifecycle_status}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Panel>
            <Panel title="Service matrix" detail="Gateway + miner">
              <div className="service-matrix">
                {serviceStates.map(([label, active, Icon]) => (
                  <button
                    key={label}
                    type="button"
                    className={`service-node ${active ? "active" : ""}`}
                    onClick={() =>
                      setPage(label === "MINER" ? "mining" : label === "NODE" ? "node" : "overview")
                    }
                  >
                    <Icon size={18} />
                    <span>{label}</span>
                    <b>{active ? "ONLINE" : "OFFLINE"}</b>
                  </button>
                ))}
              </div>
              <div className="node-gauge-row">
                <Gauge
                  value={n.connected_peer_count}
                  max={Math.max(12, n.connected_peer_count)}
                  label="PEERS"
                />
                <div>
                  <p className="eyebrow">Validated peers</p>
                  <strong className="matrix-number">{n.validated_peer_count}</strong>
                  <span className="muted"> handshakes</span>
                  <p className="muted">Telemetry from VPS node and fleet controller.</p>
                </div>
              </div>
            </Panel>
          </div>
        </DashboardSection>

        <DashboardSection title="Network identity & shortcuts">
          <div className="grid cols-3">
            <Panel title="Identity" detail="Consensus-bound">
              <div className="identity-grid">
                <span>
                  <small>STATUS</small>
                  <b>{n.status_label}</b>
                </span>
                <span>
                  <small>TIP</small>
                  <b className="mono">{shortHash(n.tip_hash, 12)}</b>
                </span>
                <span>
                  <small>PEER ID</small>
                  <b className="mono">{shortHash(n.local_peer_id, 12)}</b>
                </span>
                <span>
                  <small>LISTEN</small>
                  <b>{n.p2p_listen_addresses.length}</b>
                </span>
              </div>
            </Panel>
            <Panel title="Pool snapshot" detail="Gateway pool">
              <Keyish label="Online" value={n.pool_online ? "Yes" : "No"} positive={n.pool_online} />
              <Keyish label="Workers" value={String(n.pool_workers)} />
              <Keyish label="Hashrate" value={formatHashrate(n.pool_hashrate_hs)} />
              <Keyish label="Blocks found" value={String(n.pool_blocks_found)} />
            </Panel>
            <Panel title="Quick actions" detail="Navigate">
              <div className="quick-command-grid">
                <button type="button" onClick={() => setPage("send")}>
                  <Send size={18} />
                  <span>Send</span>
                </button>
                <button type="button" onClick={() => setPage("wallet")}>
                  <WalletCards size={18} />
                  <span>Wallet</span>
                </button>
                <button type="button" onClick={() => setPage("mining")}>
                  <Cpu size={18} />
                  <span>Miner</span>
                </button>
                <button type="button" onClick={() => setPage("explorer")}>
                  <Blocks size={18} />
                  <span>Explorer</span>
                </button>
                <button type="button" onClick={() => setPage("node")}>
                  <Network size={18} />
                  <span>Network</span>
                </button>
                <button type="button" onClick={() => setPage("rewards")}>
                  <Database size={18} />
                  <span>Rewards</span>
                </button>
              </div>
            </Panel>
          </div>
        </DashboardSection>
      </div>
    </div>
  );
}

function Keyish({
  label,
  value,
  positive
}: {
  label: string;
  value: string;
  positive?: boolean;
}) {
  return (
    <div className="kv">
      <span>{label}</span>
      <span className={positive ? "positive mono" : "mono"}>{value}</span>
    </div>
  );
}
