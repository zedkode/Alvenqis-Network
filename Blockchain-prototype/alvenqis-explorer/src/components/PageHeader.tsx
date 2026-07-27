import { StatusBadge } from "./StatusBadge";

interface PageHeaderProps {
  title: string;
  description: string;
}

export function PageHeader({ title, description }: PageHeaderProps) {
  return (
    <header className="page-header">
      <div className="page-header-copy">
        <div className="page-kicker">Alvenqis Network Explorer</div>
        <h1 className="page-title">{title}</h1>
        <p className="page-description">{description}</p>
      </div>
      <aside className="page-header-panel">
        <div className="page-header-panel-title">Current Mode</div>
        <div className="badge-grid compact">
          <StatusBadge label="Mainnet Candidate" tone="warn" />
          <StatusBadge label="Read Only" />
          <StatusBadge label="Live RPC Data" />
        </div>
      </aside>
    </header>
  );
}
