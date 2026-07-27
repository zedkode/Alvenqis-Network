import { AlertTriangle, CheckCircle2, Coins, Info, X, XCircle } from "lucide-react";
import type { AppNotification } from "../../shared/notifications";
import { useNotifications } from "../../shared/notifications";

function iconFor(kind: AppNotification["kind"]) {
  switch (kind) {
    case "success":
      return <CheckCircle2 size={18} />;
    case "error":
      return <XCircle size={18} />;
    case "warning":
      return <AlertTriangle size={18} />;
    case "mining":
      return <Coins size={18} />;
    default:
      return <Info size={18} />;
  }
}

export function ToastStack() {
  const { toasts, dismissToast } = useNotifications();
  if (toasts.length === 0) return null;

  return (
    <div className="toast-stack" aria-live="polite" aria-relevant="additions">
      {toasts.map((toast) => (
        <article key={toast.id} className={`toast toast-${toast.kind}`} role="status">
          <span className="toast-icon">{iconFor(toast.kind)}</span>
          <div className="toast-copy">
            <strong>{toast.title}</strong>
            <p>{toast.body}</p>
          </div>
          <button
            type="button"
            className="icon-button toast-close"
            aria-label="Dismiss"
            onClick={() => dismissToast(toast.id)}
          >
            <X size={16} />
          </button>
        </article>
      ))}
    </div>
  );
}
