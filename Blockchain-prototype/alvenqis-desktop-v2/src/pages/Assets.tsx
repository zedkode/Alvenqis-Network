import { useState } from "react";
import { Box, Coins, Database, FileCheck, Fingerprint, KeyRound, Shapes } from "lucide-react";
import { formatAtomic, shortHash } from "@shared/format";
import { DetailDialog } from "../components/ui/DetailDialog";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { KeyValue, Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { useApp } from "../model";

const planned = [
  [
    "ALVE asset records",
    Shapes,
    "Planned",
    "Additional native fungible records require a reviewed consensus model."
  ],
  [
    "NFT records",
    Box,
    "Research",
    "Unique asset standards and off-chain metadata are not implemented."
  ],
  [
    "Identity / Passport",
    Fingerprint,
    "Research",
    "Selective identity proofs remain research; no profile is created here."
  ],
  [
    "Software licenses",
    KeyRound,
    "Planned",
    "License proofs currently have no executable ledger model."
  ],
  [
    "Files & proofs",
    FileCheck,
    "Research",
    "Files stay off-chain. Future protocol may commit hashes only."
  ]
] as const;

function supplyPercent(emitted: string | null, maximum: string | null): number | null {
  if (!emitted || !maximum) return null;
  const max = BigInt(maximum);
  if (max === 0n) return null;
  return Number((BigInt(emitted) * 10_000n) / max) / 100;
}

export function Assets() {
  const { snapshot: n, wallet, setPage } = useApp();
  const [showAlve, setShowAlve] = useState(false);
  const issuedPercent = supplyPercent(n.emitted_supply_atomic, n.max_supply_atomic);

  return (
    <div className="page grid assets-page">
      <PageHero
        kicker="NATIVE LEDGER ASSET"
        title="ALVE"
        titleAccent="asset control"
        description="Real native-currency state from the selected RPC and local wallet. No synthetic tokens, prices or ownership records."
        actions={
          <>
            <button className="button primary" type="button" onClick={() => setPage("send")}>
              Send ALVE
            </button>
            <button className="button" type="button" onClick={() => setShowAlve(true)}>
              Inspect ALVE
            </button>
          </>
        }
        side={
          <button
            className="native-asset-emblem"
            type="button"
            onClick={() => setShowAlve(true)}
            aria-label="Inspect ALVE"
            style={{ position: "relative", margin: "0 auto" }}
          >
            <Coins size={34} />
            <strong>ALVE</strong>
            <small>8 DECIMALS</small>
          </button>
        }
      />

      <div className="grid cols-4 telemetry-strip">
        <StatCard label="Native asset" value="ALVE" detail="Protocol currency · 8 decimals" tone="gold" icon={<Coins size={14} />} />
        <StatCard
          label="Emitted"
          value={n.emitted_supply_atomic ? formatAtomic(n.emitted_supply_atomic) : "—"}
          detail={issuedPercent === null ? "RPC unavailable" : `${issuedPercent.toFixed(4)}% of max`}
        />
        <StatCard
          label="Maximum"
          value={n.max_supply_atomic ? formatAtomic(n.max_supply_atomic) : "—"}
          detail="Consensus ceiling"
        />
        <StatCard
          label="Wallet holding"
          value={n.balance_atomic !== null ? formatAtomic(n.balance_atomic) : "—"}
          detail={wallet ? shortHash(wallet.address, 7) : "No wallet"}
          tone="gold"
        />
      </div>

      <div className="grid cols-2">
        <Panel title="Native ALVE record" detail="Implemented ledger primitive">
          <button className="asset-record" type="button" onClick={() => setShowAlve(true)}>
            <span>
              <Coins size={28} />
            </span>
            <div>
              <strong>Alvenqis native currency</strong>
              <small>Settlement, fees and PoW rewards</small>
            </div>
            <b>INSPECT</b>
          </button>
          <div className="asset-supply-track" aria-label="ALVE emitted supply">
            <i style={{ width: `${Math.min(100, issuedPercent ?? 0)}%` }} />
          </div>
          <div className="asset-action-row">
            <button className="button primary" type="button" onClick={() => setPage("send")}>
              Send ALVE
            </button>
            <button className="button" type="button" onClick={() => setPage("wallet")}>
              Wallet
            </button>
            <button className="button" type="button" onClick={() => setPage("explorer")}>
              Ledger
            </button>
          </div>
        </Panel>
        <Panel title="Protocol boundary" detail="Honest status">
          <KeyValue label="Implemented">Native ALVE balances, transfers, fees, mining rewards</KeyValue>
          <KeyValue label="Indexed">Native TX and address activity</KeyValue>
          <KeyValue label="Not implemented">Custom tokens, NFTs, Passport, licenses, marketplace</KeyValue>
          <KeyValue label="Storage rule">Large payloads remain off-chain</KeyValue>
        </Panel>
      </div>

      <Panel title="Future asset modules" detail="No live claims">
        <div className="grid cols-3">
          {planned.map(([title, Icon, status, detail]) => (
            <article className="asset-roadmap-card" key={title}>
              <Icon size={20} />
              <div>
                <strong>{title}</strong>
                <small>{status}</small>
                <p>{detail}</p>
              </div>
            </article>
          ))}
        </div>
      </Panel>

      {showAlve ? (
        <DetailDialog title="ALVE native asset" subtitle="Consensus currency" onClose={() => setShowAlve(false)}>
          <div className="detail-grid">
            <KeyValue label="Project">Alvenqis Network</KeyValue>
            <KeyValue label="Ticker">ALVE</KeyValue>
            <KeyValue label="Decimals">8</KeyValue>
            <KeyValue label="Atomic / ALVE" mono>
              100,000,000
            </KeyValue>
            <KeyValue label="Status">{n.status_label}</KeyValue>
            <KeyValue label="Height">{n.height ?? "—"}</KeyValue>
            <KeyValue label="Emitted" mono>
              {n.emitted_supply_atomic ? `${formatAtomic(n.emitted_supply_atomic)} ALVE` : "—"}
            </KeyValue>
            <KeyValue label="Maximum" mono>
              {n.max_supply_atomic ? `${formatAtomic(n.max_supply_atomic)} ALVE` : "—"}
            </KeyValue>
            <KeyValue label="Emission ratio">
              {issuedPercent === null ? "—" : `${issuedPercent.toFixed(6)}%`}
            </KeyValue>
            <KeyValue label="Addresses">{n.indexed_addresses}</KeyValue>
            <div className="detail-span-full">
              <KeyValue label="Wallet">{wallet ? wallet.address : "None"}</KeyValue>
            </div>
            <KeyValue label="Holding" mono>
              {n.balance_atomic !== null ? `${formatAtomic(n.balance_atomic)} ALVE` : "—"}
            </KeyValue>
            <KeyValue label="Source">
              <span className={n.online ? "positive" : "negative"}>
                <Database size={13} /> {n.online ? "RPC / indexer" : "RPC offline"}
              </span>
            </KeyValue>
            <div className="detail-span-full">
              <EmptyState status="Not available">
                Custom asset issuance is not enabled by this interface or consensus.
              </EmptyState>
            </div>
          </div>
        </DetailDialog>
      ) : null}
    </div>
  );
}
