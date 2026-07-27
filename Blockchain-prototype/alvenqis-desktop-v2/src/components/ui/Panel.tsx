import type { PropsWithChildren, ReactNode } from "react";

export function Panel({
  title,
  detail,
  children,
  className = ""
}: PropsWithChildren<{ title: string; detail?: ReactNode; className?: string }>) {
  return (
    <section className={`panel ${className}`.trim()}>
      <header className="section-header">
        <h2>{title}</h2>
        {detail ? <span>{detail}</span> : null}
      </header>
      {children}
    </section>
  );
}

export function KeyValue({
  label,
  children,
  mono = false
}: PropsWithChildren<{ label: string; mono?: boolean }>) {
  return (
    <div className="kv">
      <span>{label}</span>
      <span className={mono ? "mono" : ""}>{children}</span>
    </div>
  );
}
