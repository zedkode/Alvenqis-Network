import type { ReactNode } from "react";
import { AlertTriangle } from "lucide-react";
import { Dialog } from "./Dialog";

export function ConfirmDialog({
  open,
  title,
  description,
  consequences = [],
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  danger = false,
  busy = false,
  onConfirm,
  onClose
}: {
  open: boolean;
  title: string;
  description: ReactNode;
  consequences?: string[];
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  busy?: boolean;
  onConfirm(): void | Promise<void>;
  onClose(): void;
}) {
  return (
    <Dialog
      open={open}
      title={title}
      subtitle="Please confirm this action"
      critical
      onClose={onClose}
      footer={
        <div className="button-row" style={{ justifyContent: "flex-end", width: "100%" }}>
          <button type="button" className="button" disabled={busy} onClick={onClose}>
            {cancelLabel}
          </button>
          <button
            type="button"
            className={`button ${danger ? "danger" : "primary"}`}
            disabled={busy}
            onClick={() => void onConfirm()}
          >
            {confirmLabel}
          </button>
        </div>
      }
    >
      <div className="confirm-dialog-body">
        <div className="confirm-dialog-icon" data-danger={danger ? "true" : "false"}>
          <AlertTriangle size={22} />
        </div>
        <div className="confirm-dialog-copy">
          <p>{description}</p>
          {consequences.length > 0 ? (
            <ul className="confirm-consequences">
              {consequences.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          ) : null}
        </div>
      </div>
    </Dialog>
  );
}
