import type { ReactNode } from "react";

/**
 * Modern page header — replaces dense hero blocks.
 * Live metrics sit in a glass strip; title is structured eyebrow + name.
 */
export function PageChrome({
  eyebrow,
  title,
  subtitle,
  metrics,
  actions,
  className = ""
}: {
  eyebrow?: string;
  title: ReactNode;
  subtitle?: ReactNode;
  metrics?: Array<{ label: string; value: ReactNode; tone?: "positive" | "gold" | "danger" }>;
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <header className={`page-chrome glass-panel ${className}`.trim()}>
      <div className="page-chrome-main">
        {eyebrow ? <p className="page-chrome-eyebrow">{eyebrow}</p> : null}
        <h1 className="page-chrome-title">{title}</h1>
        {subtitle ? <p className="page-chrome-sub">{subtitle}</p> : null}
      </div>
      {metrics && metrics.length ? (
        <div className="page-chrome-metrics">
          {metrics.map((m) => (
            <div key={m.label} className={`page-chrome-metric ${m.tone ?? ""}`.trim()}>
              <span>{m.label}</span>
              <strong>{m.value}</strong>
            </div>
          ))}
        </div>
      ) : null}
      {actions ? <div className="page-chrome-actions button-row">{actions}</div> : null}
    </header>
  );
}
