import { useEffect, useId, useMemo, useRef, useState, type MouseEvent } from "react";
import {
  areaPath,
  formatClock,
  prefersReducedMotion,
  smoothLinePath,
  toSeries,
  type Point2D,
  type SeriesPoint
} from "../../shared/chartPath";

interface TelemetryChartProps {
  values: number[] | SeriesPoint[];
  label: string;
  unit?: string;
  tone?: "primary" | "positive" | "gold";
  height?: number;
  showMean?: boolean;
}

function layout(
  series: SeriesPoint[],
  width: number,
  height: number,
  padX: number,
  padY: number
): { points: Point2D[]; min: number; max: number; mean: number } {
  const values = series.map((s) => s.value);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = Math.max(1e-12, max - min);
  const innerW = width - padX * 2;
  const innerH = height - padY * 2;
  const points = series.map((s, index) => ({
    x: padX + (index / Math.max(1, series.length - 1)) * innerW,
    y: padY + (1 - (s.value - min) / span) * innerH
  }));
  const mean = values.reduce((a, b) => a + b, 0) / values.length;
  return { points, min, max, mean };
}

function lerpPoints(a: Point2D[], b: Point2D[], t: number): Point2D[] {
  const n = Math.max(a.length, b.length);
  const out: Point2D[] = [];
  for (let i = 0; i < n; i++) {
    const pa = a[Math.min(i, a.length - 1)] ?? b[i];
    const pb = b[Math.min(i, b.length - 1)] ?? a[i];
    out.push({
      x: pa.x + (pb.x - pa.x) * t,
      y: pa.y + (pb.y - pa.y) * t
    });
  }
  return out;
}

export function TelemetryChart({
  values,
  label,
  unit = "",
  tone = "primary",
  height = 150,
  showMean = true
}: TelemetryChartProps) {
  const gradientId = useId().replace(/:/g, "");
  const meanId = useId().replace(/:/g, "");
  const series = useMemo(() => toSeries(values), [values]);
  const width = 640;
  const padX = 14;
  const padY = 18;

  const targetLayout = useMemo(
    () => (series.length >= 2 ? layout(series, width, height, padX, padY) : null),
    [series, height]
  );

  const [drawPoints, setDrawPoints] = useState<Point2D[] | null>(null);
  const [hoverIndex, setHoverIndex] = useState<number | null>(null);
  const prevPointsRef = useRef<Point2D[] | null>(null);
  const rafRef = useRef(0);

  useEffect(() => {
    if (!targetLayout) {
      setDrawPoints(null);
      prevPointsRef.current = null;
      return;
    }

    const next = targetLayout.points;
    if (prefersReducedMotion() || !prevPointsRef.current || prevPointsRef.current.length === 0) {
      setDrawPoints(next);
      prevPointsRef.current = next;
      return;
    }

    const from = prevPointsRef.current;
    const start = performance.now();
    const duration = 500;
    cancelAnimationFrame(rafRef.current);

    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / duration);
      const eased = 1 - Math.pow(1 - t, 3);
      setDrawPoints(lerpPoints(from, next, eased));
      if (t < 1) {
        rafRef.current = requestAnimationFrame(tick);
      } else {
        setDrawPoints(next);
        prevPointsRef.current = next;
      }
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [targetLayout]);

  if (!targetLayout || series.length < 2 || !drawPoints) return null;

  const { min, max, mean } = targetLayout;
  const baselineY = height - padY;
  const line = smoothLinePath(drawPoints);
  const area = areaPath(line, drawPoints, baselineY);
  const last = drawPoints[drawPoints.length - 1];
  const meanY =
    padY +
    (1 - (mean - min) / Math.max(1e-12, max - min)) * (height - padY * 2);

  // Time axis labels from real sample timestamps
  const tickIndexes = [
    0,
    Math.floor((series.length - 1) / 2),
    series.length - 1
  ].filter((v, i, arr) => arr.indexOf(v) === i);

  const onMove = (event: MouseEvent<SVGSVGElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();
    const x = ((event.clientX - rect.left) / rect.width) * width;
    let best = 0;
    let bestDist = Infinity;
    drawPoints.forEach((p, i) => {
      const d = Math.abs(p.x - x);
      if (d < bestDist) {
        bestDist = d;
        best = i;
      }
    });
    setHoverIndex(best);
  };

  const hover = hoverIndex !== null ? series[hoverIndex] : null;
  const hoverPt = hoverIndex !== null ? drawPoints[hoverIndex] : null;

  return (
    <div className={`telemetry-chart chart-v2 chart-${tone}`}>
      <div className="chart-readout">
        <span>{label}</span>
        <strong>
          {series[series.length - 1]!.value.toLocaleString()}
          {unit ? ` ${unit}` : ""}
        </strong>
      </div>
      <div className="chart-svg-wrap">
        <svg
          viewBox={`0 0 ${width} ${height}`}
          preserveAspectRatio="none"
          role="img"
          aria-label={`${label} from real local telemetry`}
          onMouseMove={onMove}
          onMouseLeave={() => setHoverIndex(null)}
        >
          <defs>
            <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="currentColor" stopOpacity="0.42" />
              <stop offset="55%" stopColor="currentColor" stopOpacity="0.12" />
              <stop offset="100%" stopColor="currentColor" stopOpacity="0" />
            </linearGradient>
            <pattern id={meanId} width="6" height="1" patternUnits="userSpaceOnUse">
              <line x1="0" y1="0" x2="3" y2="0" stroke="currentColor" strokeWidth="1" opacity="0.35" />
            </pattern>
          </defs>
          <g className="chart-grid">
            {[0.25, 0.5, 0.75].map((t) => (
              <line
                key={t}
                x1={padX}
                y1={padY + (height - padY * 2) * t}
                x2={width - padX}
                y2={padY + (height - padY * 2) * t}
              />
            ))}
          </g>
          {showMean ? (
            <line
              className="chart-mean"
              x1={padX}
              y1={meanY}
              x2={width - padX}
              y2={meanY}
              stroke="currentColor"
              strokeDasharray="4 5"
              strokeWidth="1"
              opacity="0.35"
            />
          ) : null}
          <path className="chart-area" d={area} fill={`url(#${gradientId})`} />
          <path className="chart-line" d={line} />
          <circle className="chart-point" cx={last.x} cy={last.y} r="4.5" />
          <circle className="chart-point-ring" cx={last.x} cy={last.y} r="9" />
          {hoverPt && hover ? (
            <>
              <line
                className="chart-crosshair"
                x1={hoverPt.x}
                y1={padY}
                x2={hoverPt.x}
                y2={baselineY}
              />
              <circle className="chart-hover-dot" cx={hoverPt.x} cy={hoverPt.y} r="5" />
            </>
          ) : null}
        </svg>
        {hover && hoverPt ? (
          <div
            className="chart-tooltip"
            style={{
              left: `${(hoverPt.x / width) * 100}%`,
              top: `${(hoverPt.y / height) * 100}%`
            }}
          >
            <strong>
              {hover.value.toLocaleString()}
              {unit ? ` ${unit}` : ""}
            </strong>
            <span>{formatClock(hover.ts)}</span>
          </div>
        ) : null}
      </div>
      <div className="chart-axis-time">
        {tickIndexes.map((i) => (
          <span key={i}>{formatClock(series[i]!.ts)}</span>
        ))}
      </div>
      <div className="chart-range">
        <span>
          min {min.toLocaleString()}
          {unit ? ` ${unit}` : ""}
        </span>
        <span>
          {series.length} real samples · mean{" "}
          {mean.toLocaleString(undefined, { maximumFractionDigits: 1 })}
        </span>
        <span>
          max {max.toLocaleString()}
          {unit ? ` ${unit}` : ""}
        </span>
      </div>
    </div>
  );
}
