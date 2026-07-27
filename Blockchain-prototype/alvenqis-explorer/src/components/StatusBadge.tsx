type BadgeTone = "default" | "warn" | "ok";

interface StatusBadgeProps {
  label: string;
  tone?: BadgeTone;
}

export function StatusBadge({ label, tone = "default" }: StatusBadgeProps) {
  const className =
    tone === "warn"
      ? "status-badge warn"
      : tone === "ok"
        ? "status-badge ok"
        : "status-badge";
  return <span className={className}>{label}</span>;
}
