interface StatePanelProps {
  message: string;
}

export function LoadingPanel({ message }: StatePanelProps) {
  return <section className="panel empty-box">{message}</section>;
}

export function ErrorPanel({ message }: StatePanelProps) {
  return (
    <section className="panel error-box">
      <div className="state-panel-title">Load Error</div>
      <div>{message}</div>
    </section>
  );
}

export function EmptyPanel({ message }: StatePanelProps) {
  return <section className="panel empty-box">{message}</section>;
}
