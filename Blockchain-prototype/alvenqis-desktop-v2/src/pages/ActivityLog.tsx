import { useEffect, useMemo, useRef, useState } from "react";
import { Activity, Blocks, Cpu, Database, Network, Radio } from "lucide-react";
import { LIVE_LOG_INTERVAL_MS } from "@shared/constants";
import { formatAtomic, formatTimestamp, shortHash } from "@shared/format";
import { DetailDialog } from "../components/ui/DetailDialog";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { KeyValue, Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { useAppSettings } from "../hooks/useAppSettings";
import { useApp } from "../model";

const services = ["node", "rpc", "miner", "explorer"] as const;
type Service = (typeof services)[number];

export function ActivityLog() {
  const { snapshot: n } = useApp();
  const { settings } = useAppSettings();
  const [service, setService] = useState<Service | "canonical" | "all">("canonical");
  const [logs, setLogs] = useState<Record<Service, string>>({
    node: "",
    rpc: "",
    miner: "",
    explorer: ""
  });
  const [selectedEventKey, setSelectedEventKey] = useState<string | null>(null);
  const consoleRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    const load = async () => {
      const entries = await Promise.all(
        services.map(async (name) => [name, await window.alvenqis.logs.recent(name, 260)] as const)
      );
      setLogs(Object.fromEntries(entries) as Record<Service, string>);
    };
    void load();
    const interval = Math.max(1000, settings.live_log_interval_ms || LIVE_LOG_INTERVAL_MS);
    const timer = window.setInterval(() => void load(), interval);
    return () => window.clearInterval(timer);
  }, [settings.live_log_interval_ms]);

  const canonicalEvents = useMemo(() => {
    const blockEvents = n.recent_blocks.map((block) => ({
      key: `block-${block.hash}`,
      type: "BLOCK",
      tone: "positive",
      time: block.timestamp,
      title: `Block ${block.height} accepted`,
      detail: `${shortHash(block.hash, 8)} | ${block.transaction_count} tx | ${formatAtomic(block.miner_reward_atomic)} ALVE`
    }));
    const transactionEvents = n.recent_transactions
      .filter((tx) => tx.from !== null)
      .map((tx) => ({
        key: `tx-${tx.hash}`,
        type: "TX",
        tone: "",
        time: n.recent_blocks.find((b) => b.height === tx.block_height)?.timestamp ?? 0,
        title: `TX confirmed in block ${tx.block_height}`,
        detail: `${shortHash(tx.hash, 8)} | ${formatAtomic(tx.amount_atomic)} ALVE | ${tx.lifecycle_status}`
      }));
    const pendingEvents = n.mempool_transactions.map((tx) => ({
      key: `pending-${tx.hash}`,
      type: "MEMPOOL",
      tone: "gold",
      time: 0,
      title: "Transaction pending",
      detail: `${shortHash(tx.hash, 8)} | ${formatAtomic(tx.amount_atomic)} ALVE | ${tx.authorization_state}`
    }));
    return [...pendingEvents, ...blockEvents, ...transactionEvents]
      .sort((a, b) => b.time - a.time)
      .slice(0, 120);
  }, [n.mempool_transactions, n.recent_blocks, n.recent_transactions]);

  const selectedLogs =
    service === "all"
      ? services
          .map((name) => `===== ${name.toUpperCase()} =====\n${logs[name] || "No log entries."}`)
          .join("\n\n")
      : service === "canonical"
        ? ""
        : logs[service];
  const selectedEvent = canonicalEvents.find((e) => e.key === selectedEventKey) ?? null;

  useEffect(() => {
    if (consoleRef.current) consoleRef.current.scrollTop = consoleRef.current.scrollHeight;
  }, [selectedLogs]);

  return (
    <div className="page grid activity-page">
      <PageHero
        kicker="UNIFIED OBSERVABILITY"
        title="Activity"
        titleAccent="stream"
        description="Canonical events, mempool state and local service logs in one place. Remote peers do not expose private process logs."
        side={
          <>
            <div className="page-hero-metric">
              <small>Blocks in window</small>
              <strong>{n.recent_blocks.length}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Indexed TX</small>
              <strong>{n.indexed_transactions}</strong>
            </div>
            <div className="page-hero-metric">
              <small>P2P sessions</small>
              <strong>{n.connected_peer_count}</strong>
            </div>
          </>
        }
      />

      <div className="activity-scope">
        <Activity size={18} />
        <div>
          <b>Local network observability</b>
          <span>
            Chain events + mempool + P2P telemetry + process logs. Auto-refresh from settings.
          </span>
        </div>
      </div>

      <div className="grid cols-4 telemetry-strip">
        <StatCard
          label="Blocks observed"
          value={n.recent_blocks.length}
          detail={`${n.block_count} total`}
          icon={<Blocks size={14} />}
        />
        <StatCard
          label="Indexed TX"
          value={n.indexed_transactions}
          detail={`${n.recent_transactions.length} in window`}
        />
        <StatCard
          label="Mempool"
          value={n.mempool_count}
          detail="Validated queue"
          tone={n.mempool_count ? "gold" : "positive"}
        />
        <StatCard
          label="P2P sessions"
          value={n.connected_peer_count}
          detail={`${n.validated_peer_count} validated`}
          icon={<Network size={14} />}
        />
      </div>

      <div className="activity-tabs">
        <button
          type="button"
          className={service === "canonical" ? "active" : ""}
          onClick={() => setService("canonical")}
        >
          <Blocks size={14} /> Canonical
        </button>
        <button
          type="button"
          className={service === "node" ? "active" : ""}
          onClick={() => setService("node")}
        >
          <Network size={14} /> Node
        </button>
        <button
          type="button"
          className={service === "rpc" ? "active" : ""}
          onClick={() => setService("rpc")}
        >
          <Radio size={14} /> RPC
        </button>
        <button
          type="button"
          className={service === "miner" ? "active" : ""}
          onClick={() => setService("miner")}
        >
          <Cpu size={14} /> Miner
        </button>
        <button
          type="button"
          className={service === "explorer" ? "active" : ""}
          onClick={() => setService("explorer")}
        >
          <Database size={14} /> Explorer
        </button>
        <button
          type="button"
          className={service === "all" ? "active" : ""}
          onClick={() => setService("all")}
        >
          <Activity size={14} /> All logs
        </button>
      </div>

      {service === "canonical" ? (
        <Panel title="Canonical event stream" detail="Click for evidence">
          {canonicalEvents.length ? (
            <div className="event-stream">
              {canonicalEvents.map((event) => (
                <article key={event.key} onClick={() => setSelectedEventKey(event.key)}>
                  <span className={`event-type ${event.tone}`}>{event.type}</span>
                  <div>
                    <b>{event.title}</b>
                    <p className="mono">{event.detail}</p>
                  </div>
                  <time>{event.time ? formatTimestamp(event.time) : "Pending now"}</time>
                </article>
              ))}
            </div>
          ) : (
            <EmptyState>No canonical or pending activity yet.</EmptyState>
          )}
        </Panel>
      ) : (
        <Panel
          title={`${service === "all" ? "Combined" : service.toUpperCase()} process log`}
          detail="Auto-updated"
        >
          <pre className="network-log-console" ref={consoleRef}>
            {selectedLogs || "No local log lines available."}
          </pre>
        </Panel>
      )}

      {selectedEvent ? (
        <DetailDialog
          title={`${selectedEvent.type} event`}
          subtitle={selectedEvent.key}
          onClose={() => setSelectedEventKey(null)}
        >
          <div className="detail-grid">
            <KeyValue label="Event">{selectedEvent.title}</KeyValue>
            <KeyValue label="Observed">
              {selectedEvent.time ? formatTimestamp(selectedEvent.time) : "Current mempool"}
            </KeyValue>
            <div className="detail-span-full">
              <KeyValue label="Evidence" mono>
                {selectedEvent.detail}
              </KeyValue>
            </div>
            <KeyValue label="Source">
              {selectedEvent.type === "MEMPOOL" ? "RPC mempool" : "Chain index"}
            </KeyValue>
            <KeyValue label="Scope">Local observation</KeyValue>
            <div className="detail-span-full">
              <KeyValue label="Audit boundary">
                Reconstructed from current state. Durable append-only journal not implemented yet.
              </KeyValue>
            </div>
          </div>
        </DetailDialog>
      ) : null}
    </div>
  );
}
