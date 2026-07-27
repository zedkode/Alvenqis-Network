import { useState } from "react";
import { Command, Moon, RefreshCw, Sun, WalletCards } from "lucide-react";
import type { NetworkSnapshot, WalletMetadata } from "@shared/types";
import { formatAtomic, shortHash } from "@shared/format";
import { WalletSwitcherDialog } from "../dialogs/WalletSwitcherDialog";
import {
  NotificationBell,
  NotificationCenterPanel
} from "../notifications/NotificationCenter";
import { pageTitle } from "../../shared/i18n";
import { useTheme } from "../../shared/theme";
import { useAppSettings } from "../../hooks/useAppSettings";
import { useApp } from "../../model";
import { connectionAgeLabel, connectionLabel } from "../../shared/connectivity";

export function TopBar({
  page,
  snapshot,
  wallet,
  busy,
  refresh,
  refreshMs = 5000,
  onOpenPalette
}: {
  page: string;
  snapshot: NetworkSnapshot;
  wallet: WalletMetadata | null;
  busy: boolean;
  refresh(): void;
  refreshMs?: number;
  onOpenPalette?(): void;
}) {
  const { settings, update } = useAppSettings();
  const { theme, toggleTheme } = useTheme();
  const { wallets, selectWallet, setPage } = useApp();
  const [switcherOpen, setSwitcherOpen] = useState(false);
  const [title, subtitle] = pageTitle(page, settings.language);
  const rpc = snapshot.rpc_connection;
  const online = rpc.status === "online";
  const runtimeLabel = online
    ? `Gateway · H ${snapshot.height ?? "—"}`
    : "Gateway offline";
  const cadence = Math.max(1, Math.round(refreshMs / 1000));
  const hideBalances = settings.hide_balances;

  const onToggleTheme = () => {
    toggleTheme();
    const next = theme === "dark" ? "light" : "dark";
    void update({ theme: next });
  };

  return (
    <header className="topbar">
      <div className="topbar-title-block">
        <div className="eyebrow">{subtitle}</div>
        <h1>{title}</h1>
      </div>
      <div className="topbar-status" role="toolbar" aria-label="Network status">
        {snapshot.balance_atomic !== null ? (
          <div className="topbar-balance" title="Active wallet balance">
            <small>Balance</small>
            <strong className={hideBalances ? "is-private" : undefined}>
              {hideBalances ? "••••••" : `${formatAtomic(snapshot.balance_atomic)} ALVE`}
            </strong>
          </div>
        ) : null}
        <div className="network-switch" aria-label="Network">
          <span className="active">MAINNET</span>
          <span className="net-tag">CANDIDATE</span>
        </div>
        <div
          className={`status-pill ${online ? "online" : ""}`}
          title={[
            rpc.endpoint,
            connectionLabel(rpc),
            `circuit ${rpc.circuit}`,
            `last success ${connectionAgeLabel(rpc)}`,
            rpc.error
          ].filter(Boolean).join(" · ")}
        >
          <i className="status-dot" />
          {runtimeLabel}
        </div>
        <span className="live-cadence">
          <i /> {online ? `LIVE ${cadence}s` : "AUTO RETRY"}
        </span>
        {wallet ? (
          <button
            type="button"
            className="address-chip"
            title="Switch wallet"
            onClick={() => setSwitcherOpen(true)}
          >
            <WalletCards size={12} /> {shortHash(wallet.address, settings.mask_addresses ? 4 : 6)}
          </button>
        ) : null}
        {onOpenPalette ? (
          <button
            type="button"
            className="button theme-toggle"
            title="Command palette (Ctrl+K)"
            aria-label="Open command palette"
            onClick={onOpenPalette}
          >
            <Command size={15} strokeWidth={1.75} />
          </button>
        ) : null}
        <NotificationBell />
        <button
          type="button"
          className="button ghost theme-toggle"
          onClick={onToggleTheme}
          title={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
          aria-label={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
        >
          {theme === "dark" ? <Sun size={15} strokeWidth={1.75} /> : <Moon size={15} strokeWidth={1.75} />}
        </button>
        <button
          className={`button ghost refresh-button ${busy ? "is-refreshing" : ""}`}
          onClick={refresh}
          disabled={busy}
          title="Refresh gateway telemetry"
          type="button"
        >
          <RefreshCw size={15} strokeWidth={1.75} />
        </button>
      </div>
      <NotificationCenterPanel />
      <WalletSwitcherDialog
        open={switcherOpen}
        wallets={wallets}
        activeId={wallet?.wallet_id}
        busy={busy}
        onSelect={async (id) => {
          await selectWallet(id);
          setSwitcherOpen(false);
        }}
        onClose={() => setSwitcherOpen(false)}
        onManage={() => setPage("wallet")}
      />
    </header>
  );
}
