import { Copy } from "lucide-react";
import { useNotificationsOptional } from "../../shared/notifications";

/** Mono value with one-click copy — for hashes, peer ids, pool ids (public data only). */
export function CopyField({
  value,
  label,
  compact = false
}: {
  value: string;
  label?: string;
  compact?: boolean;
}) {
  const notifications = useNotificationsOptional();
  if (!value) {
    return <span className="muted">—</span>;
  }

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
      notifications?.notify({
        kind: "info",
        title: label ? `${label} copied` : "Copied",
        body: value.length > 48 ? `${value.slice(0, 24)}…${value.slice(-12)}` : value,
        severity: "toast",
        ttlMs: 2200,
        source: "copy-field"
      });
    } catch {
      notifications?.notify({
        kind: "error",
        title: "Copy failed",
        body: "Clipboard is unavailable.",
        sticky: true,
        source: "copy-field"
      });
    }
  };

  return (
    <button
      className={`copy-field ${compact ? "is-compact" : ""}`.trim()}
      type="button"
      title={label ? `Copy ${label}` : "Copy"}
      onClick={() => void copy()}
    >
      <code className="mono">{value}</code>
      <Copy size={12} />
    </button>
  );
}
