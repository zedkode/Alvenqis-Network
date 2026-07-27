import { Check, WalletCards } from "lucide-react";
import type { WalletMetadata } from "@shared/types";
import { shortHash } from "@shared/format";
import { Dialog } from "./Dialog";

export function WalletSwitcherDialog({
  open,
  wallets,
  activeId,
  busy,
  onSelect,
  onClose,
  onManage
}: {
  open: boolean;
  wallets: WalletMetadata[];
  activeId: string | null | undefined;
  busy?: boolean;
  onSelect(walletId: string): void | Promise<void>;
  onClose(): void;
  onManage(): void;
}) {
  return (
    <Dialog
      open={open}
      title="Switch wallet"
      subtitle={`${wallets.length} profile${wallets.length === 1 ? "" : "s"} on this device`}
      onClose={onClose}
      footer={
        <div className="button-row" style={{ justifyContent: "space-between", width: "100%" }}>
          <button
            type="button"
            className="button"
            onClick={() => {
              onClose();
              onManage();
            }}
          >
            Open wallet page
          </button>
          <button type="button" className="button" onClick={onClose}>
            Close
          </button>
        </div>
      }
    >
      {wallets.length === 0 ? (
        <p className="muted">No local wallets yet. Create or import one from the Wallet page.</p>
      ) : (
        <div className="grid" style={{ gap: 8 }}>
          {wallets.map((item) => {
            const active = item.wallet_id === activeId;
            return (
              <button
                key={item.wallet_id}
                type="button"
                className={`wallet-list-item ${active ? "active" : ""}`}
                disabled={busy}
                onClick={() => void onSelect(item.wallet_id)}
              >
                <span className="wl-icon">
                  <WalletCards size={18} />
                </span>
                <span className="wl-meta">
                  <b>{item.display_name}</b>
                  <small>{shortHash(item.address, 10)}</small>
                </span>
                {active ? (
                  <span className="badge gold">
                    <Check size={12} /> ACTIVE
                  </span>
                ) : null}
              </button>
            );
          })}
        </div>
      )}
    </Dialog>
  );
}
