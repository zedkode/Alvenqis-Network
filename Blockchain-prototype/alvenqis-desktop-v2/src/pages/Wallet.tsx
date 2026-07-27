import { useState } from "react";
import { QRCodeSVG } from "qrcode.react";
import { Copy, KeyRound, Send, Shield, WalletCards } from "lucide-react";
import { formatAtomic, shortHash } from "@shared/format";
import { AddressChip } from "../components/ui/AddressChip";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { KeyValue, Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { ConfirmDialog } from "../components/dialogs/ConfirmDialog";
import { RecoveryPhraseImport } from "../components/wallet/RecoveryPhraseImport";
import { useAppSettings } from "../hooks/useAppSettings";
import { useNotificationsOptional } from "../shared/notifications";
import { useApp } from "../model";

export function Wallet() {
  const { wallet, wallets, snapshot, reloadWallet, selectWallet, setNotice, setPage } = useApp();
  const { settings } = useAppSettings();
  const notifications = useNotificationsOptional();
  const [displayName, setDisplayName] = useState("Additional wallet");
  const [importOpen, setImportOpen] = useState(false);
  const [importing, setImporting] = useState(false);
  const [disconnectOpen, setDisconnectOpen] = useState(false);
  const [disconnectBusy, setDisconnectBusy] = useState(false);

  const createWallet = async () => {
    try {
      await window.alvenqis.wallet.create(displayName);
      await reloadWallet();
      setNotice({
        error: false,
        text: "Wallet created. Recovery confirmation completed in the secure native dialog."
      });
      notifications?.notify({
        kind: "success",
        title: "Wallet created",
        body: "Recovery phrase was confirmed in the secure native dialog.",
        source: "wallet:create"
      });
    } catch (error) {
      setNotice({ error: true, text: String(error) });
    }
  };

  const importWallet = async (walletName: string, recoveryPhrase: string) => {
    setImporting(true);
    try {
      await window.alvenqis.wallet.import(walletName, recoveryPhrase);
      await reloadWallet();
      setNotice({
        error: false,
        text: "Wallet imported and stored in the OS credential vault."
      });
      notifications?.notify({
        kind: "success",
        title: "Wallet imported",
        body: "Keys stored in the OS credential vault.",
        source: "wallet:import"
      });
    } catch (error) {
      setNotice({ error: true, text: String(error) });
      throw error;
    } finally {
      setImporting(false);
    }
  };

  const disconnectWallet = async () => {
    setDisconnectBusy(true);
    try {
      await window.alvenqis.wallet.remove();
      await reloadWallet();
      setDisconnectOpen(false);
      setNotice({ error: false, text: "Active wallet disconnected from this profile." });
      notifications?.notify({
        kind: "warning",
        title: "Wallet disconnected",
        body: "Active profile removed from this device session.",
        source: "wallet:remove"
      });
    } catch (error) {
      setNotice({ error: true, text: String(error) });
    } finally {
      setDisconnectBusy(false);
    }
  };

  const copyAddress = async () => {
    if (!wallet) return;
    try {
      await navigator.clipboard.writeText(wallet.address);
      setNotice({ error: false, text: "Address copied to clipboard." });
      notifications?.notify({
        kind: "info",
        title: "Address copied",
        body: shortHash(wallet.address, 10),
        severity: "toast",
        ttlMs: 3000,
        source: "wallet:copy"
      });
    } catch {
      setNotice({ error: true, text: "Could not copy address." });
    }
  };

  return (
    <div className="page grid">
      <PageHero
        kicker="DEVICE KEYS · GATEWAY BALANCES"
        title="Wallet"
        titleAccent="center"
        description="Create, import and switch local wallets. Private keys stay in the OS credential vault. Balances come from the VPS RPC."
        actions={
          <>
            <button
              className="button primary"
              type="button"
              disabled={!wallet}
              onClick={() => setPage("send")}
            >
              <Send size={15} /> Send ALVE
            </button>
            <button className="button" type="button" disabled={!wallet} onClick={() => void copyAddress()}>
              <Copy size={15} /> Copy address
            </button>
          </>
        }
        side={
          <>
            <div className="page-hero-metric">
              <small>Balance</small>
              <strong>
                {snapshot.balance_atomic !== null ? formatAtomic(snapshot.balance_atomic) : "—"} ALVE
              </strong>
            </div>
            <div className="page-hero-metric">
              <small>Profiles</small>
              <strong>{wallets.length}</strong>
            </div>
            <div className="page-hero-metric">
              <small>Active</small>
              <strong>{wallet ? shortHash(wallet.address, 6) : "None"}</strong>
            </div>
          </>
        }
      />

      <div className="grid cols-3 telemetry-strip">
        <StatCard
          label="Portfolio"
          value={snapshot.balance_atomic !== null ? formatAtomic(snapshot.balance_atomic) : "—"}
          detail="Atomic ledger · no fiat"
          tone="gold"
          icon={<WalletCards size={14} />}
        />
        <StatCard
          label="Network"
          value="Mainnet Candidate"
          detail={snapshot.online ? "Gateway online" : "Gateway offline"}
          tone={snapshot.online ? "positive" : undefined}
          icon={<Shield size={14} />}
        />
        <StatCard
          label="Key protection"
          value="OS vault"
          detail={wallet?.key_origin ?? "No wallet"}
          icon={<KeyRound size={14} />}
        />
      </div>

      <div className="wallet-hero">
        <div className="wallet-hero-card">
          <div className="balance-label">Total portfolio</div>
          <div className={`balance-value ${settings.hide_balances ? "is-private" : ""}`}>
            {settings.hide_balances
              ? "••••••••"
              : snapshot.balance_atomic !== null
                ? formatAtomic(snapshot.balance_atomic)
                : "—"}
          </div>
          <div className="balance-unit">ALVE · on-chain · no fiat conversion</div>
          <div className="button-row">
            <button
              className="button primary"
              type="button"
              disabled={!wallet}
              onClick={() => setPage("send")}
            >
              <Send size={15} /> Send
            </button>
            <button className="button" type="button" disabled={!wallet} onClick={() => void copyAddress()}>
              <Copy size={15} /> Copy
            </button>
          </div>
        </div>

        <div className="wallet-side-card">
          <div className="section-header">
            <h2>Identity</h2>
            <span>{wallet ? "Active" : "None"}</span>
          </div>
          {wallet ? (
            <>
              <KeyValue label="Name">{wallet.display_name}</KeyValue>
              <KeyValue label="Address">
                <AddressChip value={wallet.address} />
              </KeyValue>
              <KeyValue label="Network">Mainnet Candidate</KeyValue>
              <KeyValue label="Path" mono>
                {wallet.derivation_path}
              </KeyValue>
              <KeyValue label="Vault">
                <span className="positive">OS credential vault</span>
              </KeyValue>
            </>
          ) : (
            <EmptyState>No wallet configured for this OS user.</EmptyState>
          )}
        </div>

        <div className="wallet-side-card">
          <div className="section-header">
            <h2>Receive</h2>
            <span>QR</span>
          </div>
          {wallet ? (
            <>
              <div className="qr-wrap">
                <QRCodeSVG value={wallet.address} size={140} level="M" />
              </div>
              <div style={{ marginTop: 12 }}>
                <AddressChip value={wallet.address} full />
              </div>
            </>
          ) : (
            <EmptyState>Create or import a wallet to receive ALVE.</EmptyState>
          )}
        </div>
      </div>

      <Panel title="Wallets on this device" detail={`${wallets.length} profile${wallets.length === 1 ? "" : "s"}`}>
        {wallets.length === 0 ? (
          <EmptyState>No local wallets yet.</EmptyState>
        ) : (
          <div className="grid" style={{ gap: 8 }}>
            {wallets.map((item) => (
              <button
                key={item.wallet_id}
                type="button"
                className={`wallet-list-item ${item.wallet_id === wallet?.wallet_id ? "active" : ""}`}
                onClick={() => void selectWallet(item.wallet_id)}
              >
                <span className="wl-icon">
                  <WalletCards size={18} />
                </span>
                <span className="wl-meta">
                  <b>{item.display_name}</b>
                  <small>{shortHash(item.address, 10)}</small>
                </span>
                {item.wallet_id === wallet?.wallet_id ? (
                  <span className="badge gold">ACTIVE</span>
                ) : null}
              </button>
            ))}
          </div>
        )}
      </Panel>

      <Panel title="Secure wallet ops" detail="Local keystore · never leaves the device">
        <p className="muted" style={{ marginTop: 0, lineHeight: 1.55 }}>
          <Shield size={14} style={{ verticalAlign: -2, marginRight: 6 }} />
          Creation and import use Rust-controlled native dialogs only — recovery words never enter the WebView. Import
          stays in-app and sends the phrase only to the local keystore helper.
        </p>
        <label className="field">
          <span>Wallet name</span>
          <input
            value={displayName}
            maxLength={48}
            onChange={(e) => setDisplayName(e.target.value)}
          />
        </label>
        <div className="button-row">
          <button
            className="button primary"
            type="button"
            disabled={!displayName.trim()}
            onClick={() => void createWallet()}
          >
            Create wallet
          </button>
          <button
            className="button"
            type="button"
            disabled={!displayName.trim()}
            onClick={() => setImportOpen(true)}
          >
            Import 24-word phrase
          </button>
          <button
            className="button danger"
            type="button"
            disabled={!wallet}
            onClick={() => setDisconnectOpen(true)}
          >
            Disconnect active wallet
          </button>
        </div>
      </Panel>

      <RecoveryPhraseImport
        open={importOpen}
        walletName={displayName}
        busy={importing}
        onClose={() => setImportOpen(false)}
        onImport={importWallet}
      />
      <ConfirmDialog
        open={disconnectOpen}
        title="Disconnect active wallet?"
        description="This removes the active wallet profile from this OS user. Recovery still requires the 24-word phrase if you need the same keys later."
        consequences={[
          "Signing and mining rewards attribution stop for this profile",
          "Keys stay in the OS vault until you clear them separately",
          "This cannot be undone without re-import"
        ]}
        danger
        busy={disconnectBusy}
        confirmLabel="Disconnect wallet"
        onConfirm={disconnectWallet}
        onClose={() => {
          if (!disconnectBusy) setDisconnectOpen(false);
        }}
      />
    </div>
  );
}
