import { Copy } from "lucide-react";
import { shortHash } from "@shared/format";
import { useNotificationsOptional } from "../../shared/notifications";

export function AddressChip({ value, full = false }: { value: string; full?: boolean }) {
  const notifications = useNotificationsOptional();
  const masked =
    typeof document !== "undefined" &&
    document.documentElement.dataset.maskAddresses === "true";
  const display = full && !masked ? value : shortHash(value, masked ? 4 : 10);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
      notifications?.notify({
        kind: "info",
        title: "Address copied",
        body: shortHash(value, 10),
        severity: "toast",
        ttlMs: 2500,
        source: "address-chip"
      });
    } catch {
      notifications?.notify({
        kind: "error",
        title: "Copy failed",
        body: "Clipboard is unavailable in this context.",
        sticky: true,
        source: "address-chip"
      });
    }
  };

  return (
    <button className="address-chip" type="button" title="Copy full address" onClick={() => void copy()}>
      {display} <Copy size={12} />
    </button>
  );
}
