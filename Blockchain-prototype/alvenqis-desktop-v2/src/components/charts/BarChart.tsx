import { useMemo } from "react";

interface BarChartProps {
  values: number[];
  labels?: string[];
  label?: string;
  unit?: string;
  tone?: "primary" | "positive" | "gold";
  height?: number;
}

export function BarChart({
  values,
  labels,
  label = "Series",
  unit = "",
  tone = "primary",
  height = 140
}: BarChartProps) {
  const chart = useMemo(() => {
    if (values.length === 0) return null;
    const width = 600;
    const pad = 14;
    const max = Math.max(...values, 1);
    const gap = 4;
    const barW = Math.max(3, (width - pad * 2 - gap * (values.length - 1)) / values.length);
    const bars = values.map((value, index) => {
      const h = Math.max(2, (value / max) * (height - pad * 2));
      const x = pad + index * (barW + gap);
      const y = height - pad - h;
      return { x, y, h, value, label: labels?.[index] ?? String(index + 1) };
    });
    return { width, max, bars, last: values.at(-1)! };
  }, [values, labels, height]);

  if (!chart) return null;

  return (
    <div className={`telemetry-chart chart-${tone} chart-bars`}>
      <div className="chart-readout">
        <span>{label}</span>
        <strong>
          {chart.last.toLocaleString()}
          {unit ? ` ${unit}` : ""}
        </strong>
      </div>
      <svg viewBox={`0 0 ${chart.width} ${height}`} preserveAspectRatio="none" role="img" aria-label={label}>
        {chart.bars.map((bar, i) => (
          <g key={i}>
            <rect
              className="chart-bar"
              x={bar.x}
              y={bar.y}
              width={Math.max(2, (chart.width - 28) / values.length - 4)}
              height={bar.h}
              rx="2"
            >
              <title>
                {bar.label}: {bar.value.toLocaleString()}
                {unit ? ` ${unit}` : ""}
              </title>
            </rect>
          </g>
        ))}
      </svg>
      <div className="chart-range">
        <span>{values.length} samples</span>
        <span>
          peak {chart.max.toLocaleString()}
          {unit ? ` ${unit}` : ""}
        </span>
      </div>
    </div>
  );
}
