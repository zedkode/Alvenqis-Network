import { KeyRound, ShieldCheck } from "lucide-react";
import { Dialog } from "./Dialog";

/**
 * Import confirmation — phrase is collected by the native keystore helper dialog,
 * never by this WebView (audit A-H08).
 */
export function RecoveryPhraseImportDialog({
  open,
  walletName,
  busy,
  onClose,
  onImport
}: {
  open: boolean;
  walletName: string;
  busy: boolean;
  onClose(): void;
  onImport(walletName: string): Promise<void>;
}) {
  const close = () => {
    if (busy) return;
    onClose();
  };

  const submit = async () => {
    if (!walletName.trim()) return;
    try {
      await onImport(walletName.trim());
      onClose();
    } catch {
      // Parent surfaces the error.
    }
  };

  return (
    <Dialog
      open={open}
      title="Import 24-word wallet"
      subtitle="Native keystore · OS credential vault"
      critical
      onClose={close}
      footer={
        <div className="button-row" style={{ justifyContent: "flex-end", width: "100%" }}>
          <button type="button" className="button" disabled={busy} onClick={close}>
            Cancel
          </button>
          <button
            type="button"
            className="button primary"
            disabled={busy || !walletName.trim()}
            onClick={() => void submit()}
          >
            {busy ? "Waiting for native dialog..." : "Open secure import"}
          </button>
        </div>
      }
    >
      <div className="secret-warning" style={{ marginBottom: 14 }}>
        <ShieldCheck size={18} />
        <span>
          Recovery words are entered only in the operating-system dialog owned by the Alvenqis
          keystore helper. They never enter React state or the WebView bridge.
        </span>
      </div>
      <p className="muted" style={{ margin: 0 }}>
        <KeyRound size={14} style={{ verticalAlign: "text-bottom", marginRight: 6 }} />
        Importing as <strong>{walletName.trim() || "Unnamed wallet"}</strong>
      </p>
    </Dialog>
  );
}
