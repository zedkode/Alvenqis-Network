import type { CSSProperties } from "react";
import { useAnimatedNumber } from "../../hooks/useAnimatedNumber";

export function Gauge({
  value,
  max,
  label,
  target
}: {
  value: number;
  max: number;
  label: string;
  /** Optional secondary ring threshold (real metric only). */
  target?: number;
}) {
  const safeMax = Math.max(1, max);
  const animated = useAnimatedNumber(value, 500) ?? 0;
  const safe = Math.max(0, Math.min(animated, safeMax));
  const degrees = (safe / safeMax) * 360;
  const targetDeg =
    target !== undefined && Number.isFinite(target)
      ? Math.max(0, Math.min(360, (target / safeMax) * 360))
      : null;

  return (
    <div
      className="gauge gauge-animated"
      style={
        {
          "--progress": `${degrees}deg`,
          ...(targetDeg !== null ? { "--target": `${targetDeg}deg` } : {})
        } as CSSProperties
      }
    >
      {targetDeg !== null ? <div className="gauge-target-ring" aria-hidden="true" /> : null}
      <div className="gauge-value">
        <strong>{Math.round(safe)}</strong>
        <br />
        <small>{label}</small>
      </div>
    </div>
  );
}
