import {
  CheckCircle2,
  KeyRound,
  LoaderCircle,
  Plus,
  RadioTower,
  WalletCards
} from "lucide-react";
import { useEffect, useState } from "react";
import type { NetworkSnapshot, WalletMetadata } from "@shared/types";
import { AlvenqisLogo } from "../brand/AlvenqisLogo";
import { RecoveryPhraseImport } from "../wallet/RecoveryPhraseImport";
import { RecoveryPhraseReveal } from "../wallet/RecoveryPhraseReveal";
import { startupAccessMode } from "./startupPolicy";

type Stage = "boot" | "wallet" | "sync";

interface StartupGateProps {
  snapshot: NetworkSnapshot;
  wallets: WalletMetadata[];
  activeWallet: WalletMetadata | null;
  busy: boolean;
  error: string | null;
  onSelect(walletId: string): Promise<void>;
  onCreate(displayName: string): Promise<{ phrase: string; address: string; name: string } | null>;
  onImport(displayName: string, recoveryPhrase: string): Promise<void>;
  onStartServices(): Promise<void>;
  onAddSeed(seed: string): Promise<void>;
  onRefresh(): Promise<void>;
  onContinue(): void;
}

export function StartupGate(props: StartupGateProps) {
  const [stage, setStage] = useState<Stage>("boot");
  const [displayName, setDisplayName] = useState("Primary wallet");
  const [importOpen, setImportOpen] = useState(false);
  const [bootStep, setBootStep] = useState(0);
  const [bootDone, setBootDone] = useState(false);
  const [revealWords, setRevealWords] = useState<string[] | null>(null);
  const [revealMeta, setRevealMeta] = useState<{ name: string; address: string } | null>(null);

  // Boot sequence: logo + progressive checks, then wallet gate only.
  useEffect(() => {
    if (stage !== "boot") return;
    let cancelled = false;
    const steps = [
      400,
      900,
      1400,
      2000
    ];
    steps.forEach((ms, i) => {
      window.setTimeout(() => {
        if (!cancelled) setBootStep(i + 1);
      }, ms);
    });
    const finish = window.setTimeout(() => {
      if (cancelled) return;
      setBootDone(true);
      void props.onRefresh().finally(() => {
        if (!cancelled) setStage("wallet");
      });
    }, 2600);
    return () => {
      cancelled = true;
      window.clearTimeout(finish);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- boot once on mount
  }, []);

  useEffect(() => {
    if (stage === "boot") return;
    if (!props.activeWallet && stage === "sync") setStage("wallet");
  }, [props.activeWallet, stage]);

  // Auto-refresh while waiting for gateway readiness.
  useEffect(() => {
    if (stage !== "sync") return;
    const timer = window.setInterval(() => {
      void props.onRefresh();
    }, 4_000);
    return () => window.clearInterval(timer);
  }, [stage, props.onRefresh]);

  const syncKnown =
    props.snapshot.sync_target_height !== null && props.snapshot.sync_progress_percent !== null;
  const accessMode = startupAccessMode(props.snapshot);
  const canContinue = accessMode !== "blocked" && props.activeWallet !== null;
  const gatewayHeight = props.snapshot.height ?? 0;
  const remoteMode = /vps gateway/i.test(props.snapshot.detail) || props.snapshot.online;

  const bootMessages = [
    "Starting Alvenqis Control Center…",
    "Checking RPC gateway connection…",
    props.snapshot.online
      ? "RPC gateway is reachable."
      : "Waiting for RPC gateway…",
    props.snapshot.sync_status === "syncing"
      ? "Network reports chain synchronization in progress…"
      : props.snapshot.online
        ? "Chain tip available — ready for wallet."
        : "Could not verify chain yet — you can still choose a wallet."
  ];

  return (
    <div className="startup-fullscreen" role="dialog" aria-modal="true" aria-label="Alvenqis startup">
      {stage === "boot" ? (
        <section className="startup-boot">
          <div className="startup-boot-logo">
            <AlvenqisLogo size="xl" alt="Alvenqis Network" />
            <div className="startup-boot-ring" aria-hidden />
          </div>
          <h1>Alvenqis Network</h1>
          <p className="startup-boot-sub">Mainnet Candidate · Control Center</p>
          <div className="startup-boot-loader" aria-live="polite">
            <LoaderCircle className="spin" size={22} />
            <span>{bootMessages[Math.min(bootStep, bootMessages.length - 1)]}</span>
          </div>
          <ul className="startup-boot-checklist">
            <li className={bootStep >= 1 ? "done" : ""}>Application shell</li>
            <li className={bootStep >= 2 ? "done" : ""}>RPC connectivity</li>
            <li className={bootStep >= 3 ? "done" : ""}>
              {props.snapshot.sync_status === "syncing" ? "Chain sync check" : "Chain tip check"}
            </li>
          </ul>
          {!bootDone && props.snapshot.detail ? (
            <p className="startup-boot-detail muted">{props.snapshot.detail}</p>
          ) : null}
        </section>
      ) : (
        <section className="startup-gate startup-gate-solo">
          <div className="startup-rail">
            <div className="startup-emblem">
              <AlvenqisLogo size="md" alt="Alvenqis" />
            </div>
            <div className={stage === "wallet" ? "startup-step active" : "startup-step complete"}>
              <WalletCards size={18} />
              <span>
                <b>Wallet</b>
                <small>Select or create</small>
              </span>
            </div>
            <div className={stage === "sync" ? "startup-step active" : "startup-step"}>
              <RadioTower size={18} />
              <span>
                <b>Network</b>
                <small>RPC & sync</small>
              </span>
            </div>
            <p>
              Mainnet Candidate
              <br />
              <small>Prototype software. Not a public mainnet launch.</small>
            </p>
          </div>

          <div className="startup-content">
            {stage === "wallet" ? (
              <>
                <div className="startup-heading">
                  <KeyRound size={24} />
                  <div>
                    <span>Secure startup</span>
                    <h2>Choose your wallet</h2>
                    <p>
                      Create a new wallet or select an existing one. The full control panel opens
                      only after this step.
                    </p>
                  </div>
                </div>
                <div className="wallet-selector">
                  {props.wallets.map((wallet) => (
                    <button
                      key={wallet.wallet_id}
                      className={`wallet-option ${props.activeWallet?.wallet_id === wallet.wallet_id ? "selected" : ""}`}
                      disabled={props.busy}
                      onClick={() => void props.onSelect(wallet.wallet_id)}
                    >
                      <span className="wallet-option-icon">
                        <WalletCards size={20} />
                      </span>
                      <span>
                        <b>{wallet.display_name}</b>
                        <small>{shortAddress(wallet.address)}</small>
                      </span>
                      {props.activeWallet?.wallet_id === wallet.wallet_id && (
                        <CheckCircle2 className="positive" size={18} />
                      )}
                    </button>
                  ))}
                  {!props.wallets.length && (
                    <div className="wallet-empty">
                      <WalletCards size={30} />
                      <b>No wallet found</b>
                      <span>Create a new 24-word recovery wallet or import an existing one.</span>
                    </div>
                  )}
                </div>
                <label className="field startup-name">
                  <span>Wallet name</span>
                  <input
                    value={displayName}
                    maxLength={48}
                    onChange={(event) => setDisplayName(event.target.value)}
                  />
                </label>
                <div className="startup-actions">
                  <button
                    className="button"
                    disabled={props.busy || !displayName.trim()}
                    onClick={() => setImportOpen(true)}
                  >
                    <KeyRound size={16} />
                    Import 24 words
                  </button>
                  <button
                    className="button primary"
                    disabled={props.busy || !displayName.trim()}
                    onClick={() => {
                      void props.onCreate(displayName).then((result) => {
                        if (result?.phrase) {
                          setRevealWords(result.phrase.trim().split(/\s+/).filter(Boolean));
                          setRevealMeta({ name: result.name, address: result.address });
                        }
                      });
                    }}
                  >
                    <Plus size={16} />
                    Create new wallet
                  </button>
                  <button
                    className="button primary"
                    disabled={props.busy || !props.activeWallet}
                    onClick={() => setStage("sync")}
                  >
                    Continue
                  </button>
                </div>
              </>
            ) : (
              <>
                <div className="startup-heading">
                  <RadioTower size={24} />
                  <div>
                    <span>Network verification</span>
                    <h2>{syncTitle(props.snapshot.sync_status, accessMode)}</h2>
                    <p>{syncDescription(props.snapshot, accessMode)}</p>
                  </div>
                </div>
                <div className={`sync-visual sync-${props.snapshot.sync_status}`}>
                  <div className="sync-orbit">
                    <LoaderCircle size={38} />
                    <strong>
                      {syncKnown
                        ? `${props.snapshot.sync_progress_percent?.toFixed(2)}%`
                        : gatewayHeight > 0
                          ? "100%"
                          : "…"}
                    </strong>
                  </div>
                  <div className="sync-metrics">
                    <span>
                      <small>Gateway height</small>
                      <b>{gatewayHeight.toLocaleString()}</b>
                    </span>
                    <span>
                      <small>Network target</small>
                      <b>
                        {props.snapshot.sync_target_height?.toLocaleString() ??
                          (gatewayHeight > 0 ? gatewayHeight.toLocaleString() : "Discovering")}
                      </b>
                    </span>
                    <span>
                      <small>Remaining</small>
                      <b>
                        {props.snapshot.sync_remaining_blocks?.toLocaleString() ??
                          (gatewayHeight > 0 ? "0" : "Unknown")}
                      </b>
                    </span>
                    <span>
                      <small>RPC</small>
                      <b className={props.snapshot.online ? "positive" : "negative"}>
                        {props.snapshot.online ? "Online" : "Offline"}
                      </b>
                    </span>
                  </div>
                </div>
                <div className="sync-progress" aria-label="Blockchain synchronization progress">
                  <i
                    style={{
                      width: `${props.snapshot.sync_progress_percent ?? (gatewayHeight > 0 ? 100 : 12)}%`
                    }}
                  />
                </div>
                <p className="startup-status-line muted">{props.snapshot.detail}</p>
                <div className="startup-actions">
                  <button className="button" disabled={props.busy} onClick={() => setStage("wallet")}>
                    Change wallet
                  </button>
                  <button className="button" disabled={props.busy} onClick={() => void props.onRefresh()}>
                    Check RPC again
                  </button>
                  <button className="button primary" disabled={!canContinue} onClick={props.onContinue}>
                    {accessMode === "network-synced" || accessMode === "gateway-ready"
                      ? "Open control panel"
                      : accessMode === "local-isolated"
                        ? "Open isolated panel"
                        : "Waiting for RPC"}
                  </button>
                </div>
                {(accessMode === "gateway-ready" || accessMode === "network-synced") && (
                  <p className="startup-isolated">
                    Connected to the RPC gateway. Mining and transfers use the remote chain.
                  </p>
                )}
                {accessMode === "blocked" && (
                  <p className="startup-blocked">
                    {remoteMode
                      ? "Cannot reach the VPS RPC gateway yet. Check Settings → Network → RPC URL (default https://rpcnode.dohotstudio.com)."
                      : "RPC gateway is offline. Configure the endpoint and ensure the server is running."}
                  </p>
                )}
              </>
            )}
            {props.error && <div className="notice error startup-error">{props.error}</div>}
          </div>
          <RecoveryPhraseImport
            open={importOpen}
            walletName={displayName}
            busy={props.busy}
            onClose={() => setImportOpen(false)}
            onImport={props.onImport}
          />
          <RecoveryPhraseReveal
            open={Boolean(revealWords?.length)}
            words={revealWords ?? []}
            walletName={revealMeta?.name ?? displayName}
            address={revealMeta?.address ?? ""}
            onConfirmed={() => {
              setRevealWords(null);
              setRevealMeta(null);
              setStage("sync");
            }}
          />
        </section>
      )}
    </div>
  );
}

function shortAddress(address: string): string {
  return address.length > 22 ? `${address.slice(0, 12)}…${address.slice(-8)}` : address;
}

function syncTitle(
  status: NetworkSnapshot["sync_status"],
  mode: ReturnType<typeof startupAccessMode>
): string {
  if (mode === "network-synced" || status === "synced") return "Chain ready";
  if (status === "syncing" || mode === "gateway-ready") return "Gateway synchronized";
  if (status === "discovering") return "Contacting RPC gateway";
  return "RPC is offline";
}

function syncDescription(
  snapshot: NetworkSnapshot,
  mode: ReturnType<typeof startupAccessMode>
): string {
  if (mode === "network-synced" || snapshot.sync_status === "synced") {
    return "The gateway reports a live Mainnet Candidate tip. You can open the panel and mine against the remote chain.";
  }
  if (snapshot.sync_status === "syncing") {
    return `The node is still catching up (${snapshot.sync_remaining_blocks ?? 0} blocks remaining).`;
  }
  if (mode === "gateway-ready") {
    return "Gateway answered with chain data. Peer discovery may still be incomplete on the server.";
  }
  if (snapshot.sync_status === "discovering") {
    return "Waiting for the configured RPC gateway to answer /status with chain height.";
  }
  return "Configure the RPC URL and verify the server is reachable before opening the control panel.";
}
