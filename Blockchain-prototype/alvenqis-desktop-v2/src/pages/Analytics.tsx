import { useEffect, useMemo, useState } from "react";
import { Activity, Gauge as GaugeIcon, Network, Zap } from "lucide-react";
import { formatHashrate } from "@shared/format";
import { TelemetryChart } from "../components/charts/TelemetryChart";
import { BarChart } from "../components/charts/BarChart";
import { Gauge } from "../components/charts/Gauge";
import { Donut } from "../components/charts/Donut";
import { Sparkline } from "../components/charts/Sparkline";
import { StatCard } from "../components/ui/StatCard";
import { appendSample, type SeriesPoint } from "../shared/chartPath";
import { PageToolbar } from "../components/ui/PageHero";
import { useApp } from "../model";

export function Analytics() {
  const { snapshot: n } = useApp();
  const [hashrateHistory, setHashrateHistory] = useState<SeriesPoint[]>([]);
  const [heightHistory, setHeightHistory] = useState<SeriesPoint[]>([]);
  const [mempoolHistory, setMempoolHistory] = useState<SeriesPoint[]>([]);
  const [peerHistory, setPeerHistory] = useState<SeriesPoint[]>([]);
  const [indexLagHistory, setIndexLagHistory] = useState<SeriesPoint[]>([]);

  useEffect(() => {
    const ts = Date.now();
    if (
      n.miner_hashrate_hs !== null
      && (n.stratum_connection.status === "online" || n.rpc_connection.status === "online")
    ) {
      setHashrateHistory((v) => appendSample(v, Math.round(n.miner_hashrate_hs!), 90, ts));
    }
    if (n.rpc_connection.status === "online" && n.height !== null) {
      setHeightHistory((v) => appendSample(v, n.height!, 90, ts));
    }
    if (n.rpc_connection.status === "online") {
      setMempoolHistory((v) => appendSample(v, n.mempool_count, 90, ts));
    }
    if (n.p2p_connection.status === "online") {
      setPeerHistory((v) => appendSample(v, n.connected_peer_count, 90, ts));
    }
    const lag =
      n.height != null && n.indexed_height != null
        ? Math.max(0, n.height - n.indexed_height)
        : 0;
    if (n.rpc_connection.status === "online") {
      setIndexLagHistory((v) => appendSample(v, lag, 90, ts));
    }
  }, [
    n.height,
    n.indexed_height,
    n.mempool_count,
    n.miner_hashrate_hs,
    n.connected_peer_count,
    n.tip_hash,
    n.rpc_connection.status,
    n.p2p_connection.status,
    n.stratum_connection.status
  ]);

  const txPerBlock = useMemo(
    () => [...n.recent_blocks].reverse().map((b) => b.transaction_count),
    [n.recent_blocks]
  );

  const rewardSeries = useMemo(
    () =>
      [...n.recent_blocks].reverse().map((b) => ({
        value: Number(b.miner_reward_atomic) / 1e8,
        ts: b.timestamp * 1000
      })),
    [n.recent_blocks]
  );

  const serviceOnline = [n.node_running, n.rpc_running, n.indexer_ready, n.miner_running].filter(
    Boolean
  ).length;
  const peerRatio =
    n.connected_peer_count > 0
      ? Math.min(100, Math.round((n.validated_peer_count / Math.max(1, n.connected_peer_count)) * 100))
      : 0;
  const indexHealth =
    n.height != null && n.indexed_height != null
      ? Math.max(0, 100 - Math.min(100, (n.height - n.indexed_height) * 20))
      : n.indexer_ready
        ? 80
        : 20;

  return (
    <div className="page">
      <PageToolbar
        title="Analytics"
        subtitle="Multi-series live telemetry from the gateway — height, hashrate, mempool, peers and rewards. Mainnet Candidate only."
      />

      <div className="v2-kpi-grid" style={{ marginBottom: 16 }}>
        <StatCard
          label="Height"
          value={n.height ?? "—"}
          detail={n.tip_hash ? n.tip_hash.slice(0, 16) + "…" : "tip unknown"}
          icon={<Activity size={16} />}
        />
        <StatCard
          label="Hashrate"
          value={n.miner_hashrate_hs != null ? formatHashrate(n.miner_hashrate_hs) : "—"}
          detail={n.miner_running ? "miner running" : "miner idle"}
          icon={<Zap size={16} />}
        />
        <StatCard
          label="Peers"
          value={n.connected_peer_count}
          detail={`${n.validated_peer_count} validated · ${n.banned_peer_count} banned`}
          icon={<Network size={16} />}
        />
        <StatCard
          label="Services"
          value={`${serviceOnline}/4`}
          detail="node · rpc · indexer · miner"
          icon={<GaugeIcon size={16} />}
        />
      </div>

      <div className="analytics-grid">
        <div className="analytics-card span-8">
          <h4>Tip height trajectory</h4>
          <TelemetryChart values={heightHistory} height={180} label="height" />
        </div>
        <div className="analytics-card span-4">
          <h4>Index health</h4>
          <Gauge value={indexHealth} max={100} label="sync score" />
          <p className="muted" style={{ marginTop: 8, fontSize: 12 }}>
            Indexed {n.indexed_height ?? "—"} / tip {n.height ?? "—"} · lag sparkline below
          </p>
          <Sparkline values={indexLagHistory.map((p) => p.value)} />
        </div>

        <div className="analytics-card span-6">
          <h4>Miner hashrate</h4>
          <TelemetryChart values={hashrateHistory} height={160} label="H/s" />
        </div>
        <div className="analytics-card span-6">
          <h4>Mempool depth</h4>
          <TelemetryChart values={mempoolHistory} height={160} label="pending" />
        </div>

        <div className="analytics-card span-4">
          <h4>Peer validation ratio</h4>
          <Gauge value={peerRatio} max={100} label="% validated" />
          <Sparkline values={peerHistory.map((p) => p.value)} />
        </div>
        <div className="analytics-card span-4">
          <h4>Tx per recent block</h4>
          <BarChart values={txPerBlock.length ? txPerBlock : [0]} height={140} />
        </div>
        <div className="analytics-card span-4">
          <h4>Services online</h4>
          <Donut value={serviceOnline} total={4} label="online / 4" />
        </div>

        <div className="analytics-card span-12">
          <h4>Block rewards (ALVE) — recent window</h4>
          <TelemetryChart values={rewardSeries} height={170} label="ALVE" />
        </div>
      </div>
    </div>
  );
}
