import { Coins } from "lucide-react";
import { formatAtomic, formatTimestamp } from "@shared/format";
import { TelemetryChart } from "../components/charts/TelemetryChart";
import { BarChart } from "../components/charts/BarChart";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { useApp } from "../model";

export function Rewards() {
  const { snapshot: n, wallet } = useApp();
  const mined = wallet
    ? n.recent_blocks.filter((block) => block.miner_address === wallet.address)
    : [];
  const sum = (field: "miner_reward_atomic" | "priority_fees_atomic" | "burned_fees_atomic") =>
    mined.reduce((total, block) => total + BigInt(block[field]), 0n).toString();
  const rewardSeries = [...mined].reverse().map((block) => ({
    value: Number(block.miner_reward_atomic) / 1e8,
    ts: block.timestamp * 1000
  }));
  const tipSeries = [...mined].reverse().map((block) => Number(block.priority_fees_atomic) / 1e8);

  return (
    <div className="page grid">
      <PageHero
        kicker="INDEXED MINING WINDOW"
        title="Rewards"
        titleAccent="& fees"
        description="Wallet-mined blocks in the current index window only. No lifetime totals, monthly forecasts or fiat conversion."
        side={
          <div className="rewards-hero-metrics">
            <span>
              <small>Protocol rewards</small>
              <strong>{formatAtomic(sum("miner_reward_atomic"))}</strong>
            </span>
            <span>
              <small>Priority tips</small>
              <strong>{formatAtomic(sum("priority_fees_atomic"))}</strong>
            </span>
            <span>
              <small>Blocks mined</small>
              <strong>{mined.length}</strong>
            </span>
          </div>
        }
      />

      <div className="grid cols-4 telemetry-strip">
        <StatCard
          label="Protocol rewards"
          value={formatAtomic(sum("miner_reward_atomic"))}
          detail={`${mined.length} wallet blocks`}
          tone="gold"
          icon={<Coins size={14} />}
        />
        <StatCard
          label="Priority fees"
          value={formatAtomic(sum("priority_fees_atomic"))}
          detail="Paid to this miner"
          tone="positive"
        />
        <StatCard
          label="Base fees burned"
          value={formatAtomic(sum("burned_fees_atomic"))}
          detail="Not miner income"
        />
        <StatCard
          label="Index window"
          value={`${n.recent_blocks.length} blocks`}
          detail="Not lifetime / monthly"
        />
      </div>

      <div className="grid cols-2">
        <Panel title="Rewards over blocks (ALVE)" detail="Wallet-mined only">
          {rewardSeries.length > 1 ? (
            <TelemetryChart values={rewardSeries} label="Reward" unit="ALVE" tone="gold" height={140} />
          ) : (
            <EmptyState>At least two wallet-mined indexed blocks are required for a trend.</EmptyState>
          )}
        </Panel>
        <Panel title="Priority tips (ALVE)" detail="Per mined block">
          {tipSeries.length > 1 ? (
            <BarChart values={tipSeries} label="Tips" unit="ALVE" tone="positive" height={140} />
          ) : (
            <EmptyState>No tip series yet for this wallet in the window.</EmptyState>
          )}
        </Panel>
      </div>

      <Panel title="Wallet-mined blocks" detail="Current index window">
        {mined.length ? (
          <table className="data-table">
            <thead>
              <tr>
                <th>Height</th>
                <th>Timestamp</th>
                <th>Reward</th>
                <th>Priority fee</th>
                <th>Burned</th>
              </tr>
            </thead>
            <tbody>
              {mined.map((block) => (
                <tr key={block.hash}>
                  <td className="positive mono">{block.height}</td>
                  <td>{formatTimestamp(block.timestamp)}</td>
                  <td className="gold mono">{formatAtomic(block.miner_reward_atomic)}</td>
                  <td className="positive mono">{formatAtomic(block.priority_fees_atomic)}</td>
                  <td className="mono">{formatAtomic(block.burned_fees_atomic)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <EmptyState>
            {wallet
              ? "No indexed block in this window was mined by the active wallet."
              : "Select a wallet to attribute mining rewards."}
          </EmptyState>
        )}
      </Panel>

      <Panel title="Analytics boundary" detail="Honest candidate telemetry">
        <EmptyState status="Not available">
          Fiat profitability, monthly income forecasts, referral rewards and electricity-cost
          estimates have no trusted data source and are intentionally omitted.
        </EmptyState>
      </Panel>
    </div>
  );
}
