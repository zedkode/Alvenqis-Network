import { useMemo } from "react";
import { smoothLinePath, toSeries, type SeriesPoint } from "../../shared/chartPath";

export function Sparkline({
  values,
  live = true
}: {
  values: number[] | SeriesPoint[];
  live?: boolean;
}) {
  const series = useMemo(() => toSeries(values), [values]);
  if (series.length < 2) return null;

  const width = 100;
  const height = 52;
  const padY = 6;
  const nums = series.map((s) => s.value);
  const min = Math.min(...nums);
  const max = Math.max(...nums);
  const span = Math.max(1e-12, max - min);
  const points = series.map((s, index) => ({
    x: (index / (series.length - 1)) * width,
    y: padY + (1 - (s.value - min) / span) * (height - padY * 2)
  }));
  const d = smoothLinePath(points);
  const last = points[points.length - 1]!;

  return (
    <svg
      className={`sparkline sparkline-v2 ${live ? "is-live" : ""}`}
      viewBox={`0 0 ${width} ${height}`}
      preserveAspectRatio="none"
      aria-label="Real indexed data trend"
    >
      <path className="sparkline-path" d={d} fill="none" />
      <circle className="sparkline-tip" cx={last.x} cy={last.y} r="2.2" />
    </svg>
  );
}
