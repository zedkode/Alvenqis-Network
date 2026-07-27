import type { CSSProperties } from "react";
import { useAnimatedNumber } from "../../hooks/useAnimatedNumber";

export function Donut({ value, total, label }: { value: number; total: number; label: string }) {
  const animated = useAnimatedNumber(value, 500) ?? 0;
  const degrees =
    total > 0 ? Math.max(0, Math.min(360, (animated / total) * 360)) : 0;
  return (
    <div
      className="gauge gauge-animated"
      style={{ "--progress": `${degrees}deg` } as CSSProperties}
    >
      <div className="gauge-value">
        <strong>{Math.round(animated)}</strong>
        <br />
        <small>{label}</small>
      </div>
    </div>
  );
}
