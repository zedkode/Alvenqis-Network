import type { ReactNode } from "react";
import { useAnimatedNumber } from "../../hooks/useAnimatedNumber";

function isPlainNumber(value: ReactNode): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

export function StatCard({
  label,
  value,
  detail,
  tone,
  icon,
  animate = true
}: {
  label: string;
  value: ReactNode;
  detail?: ReactNode;
  tone?: "gold" | "positive" | "negative";
  icon?: ReactNode;
  animate?: boolean;
}) {
  const numeric = isPlainNumber(value) ? value : null;
  const animated = useAnimatedNumber(animate ? numeric : null);
  const shown =
    numeric !== null && animated !== null
      ? Number.isInteger(numeric)
        ? Math.round(animated)
        : animated
      : value;

  return (
    <article className={`stat-card motion-card ${tone ? `tone-${tone}` : ""}`.trim()}>
      <div className="stat-card-top">
        <div className="eyebrow">{label}</div>
        {icon ? <span className="stat-card-icon">{icon}</span> : null}
      </div>
      <div className={`stat-value ${tone ?? ""}`}>{shown}</div>
      {detail ? <div className="stat-detail">{detail}</div> : null}
    </article>
  );
}
