import type { ReactNode } from "react";

export type EmptyStateStatus =
  | "Planned"
  | "Research"
  | "Not available"
  | "Implemented"
  | "Experimental";

export function EmptyState({
  status = "Not available",
  children
}: {
  status?: EmptyStateStatus;
  children: ReactNode;
}) {
  return (
    <div className="empty-state">
      <strong>{status}</strong>
      <div className="empty-state-body">{children}</div>
    </div>
  );
}
