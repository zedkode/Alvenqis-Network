import type { PropsWithChildren, ReactNode } from "react";

export function PageHero({
  kicker,
  title,
  titleAccent,
  description,
  actions,
  side,
  className = ""
}: PropsWithChildren<{
  kicker?: string;
  title: ReactNode;
  titleAccent?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  side?: ReactNode;
  className?: string;
}>) {
  return (
    <section className={`page-hero ${className}`.trim()}>
      <div className="page-hero-copy">
        {kicker ? (
          <span className="hero-kicker">
            <i /> {kicker}
          </span>
        ) : null}
        <h2>
          {title}
          {titleAccent ? (
            <>
              {" "}
              <b>{titleAccent}</b>
            </>
          ) : null}
        </h2>
        {description ? <p>{description}</p> : null}
        {actions ? <div className="button-row page-hero-actions">{actions}</div> : null}
      </div>
      {side ? <div className="page-hero-side">{side}</div> : null}
    </section>
  );
}

export function PageSection({
  title,
  detail,
  children,
  className = ""
}: PropsWithChildren<{ title: string; detail?: ReactNode; className?: string }>) {
  return (
    <section className={`page-section ${className}`.trim()}>
      <header className="page-section-head">
        <h3>{title}</h3>
        {detail ? <span>{detail}</span> : null}
      </header>
      <div className="page-section-body">{children}</div>
    </section>
  );
}

/** Compact metric strip for pages without a full hero. */
export function PageToolbar({
  title,
  subtitle,
  actions
}: {
  title: string;
  subtitle?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <div className="v2-page-toolbar">
      <div>
        <h2 className="v2-page-toolbar-title">{title}</h2>
        {subtitle ? <p className="v2-page-toolbar-sub muted">{subtitle}</p> : null}
      </div>
      {actions ? <div className="button-row">{actions}</div> : null}
    </div>
  );
}
