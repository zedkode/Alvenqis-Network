interface SummaryCardProps {
  label: string;
  value: string;
  note?: string;
}

export function SummaryCard({ label, value, note }: SummaryCardProps) {
  return (
    <section className="panel summary-card">
      <div className="summary-card-header">
        <div className="metric-label">{label}</div>
      </div>
      <div className="metric-value">{value}</div>
      {note ? <div className="summary-card-note">{note}</div> : null}
    </section>
  );
}
