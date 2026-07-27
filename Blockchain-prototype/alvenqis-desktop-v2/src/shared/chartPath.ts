/** Real telemetry sample — value always from IPC; ts is wall-clock when sample was observed. */
export interface SeriesPoint {
  value: number;
  ts: number;
}

export function toSeries(values: number[] | SeriesPoint[], now = Date.now()): SeriesPoint[] {
  if (values.length === 0) return [];
  if (typeof values[0] === "number") {
    const nums = values as number[];
    // Even spacing ending at now when only bare numbers are available (legacy call sites).
    const step = 5_000;
    return nums.map((value, index) => ({
      value,
      ts: now - (nums.length - 1 - index) * step
    }));
  }
  return values as SeriesPoint[];
}

export function appendSample(
  series: SeriesPoint[],
  value: number,
  max = 60,
  ts = Math.floor(Date.now() / 1000) * 1000
): SeriesPoint[] {
  return [...series, { value, ts }].slice(-max);
}

export interface Point2D {
  x: number;
  y: number;
}

/** Catmull-Rom → cubic Bézier path (smooth curve through real sample points). */
export function smoothLinePath(points: Point2D[]): string {
  if (points.length === 0) return "";
  if (points.length === 1) return `M${points[0].x},${points[0].y}`;
  if (points.length === 2) {
    return `M${points[0].x},${points[0].y} L${points[1].x},${points[1].y}`;
  }

  let d = `M${points[0].x.toFixed(2)},${points[0].y.toFixed(2)}`;
  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[Math.max(0, i - 1)];
    const p1 = points[i];
    const p2 = points[i + 1];
    const p3 = points[Math.min(points.length - 1, i + 2)];
    const cp1x = p1.x + (p2.x - p0.x) / 6;
    const cp1y = p1.y + (p2.y - p0.y) / 6;
    const cp2x = p2.x - (p3.x - p1.x) / 6;
    const cp2y = p2.y - (p3.y - p1.y) / 6;
    d += ` C${cp1x.toFixed(2)},${cp1y.toFixed(2)} ${cp2x.toFixed(2)},${cp2y.toFixed(2)} ${p2.x.toFixed(2)},${p2.y.toFixed(2)}`;
  }
  return d;
}

export function areaPath(line: string, points: Point2D[], baselineY: number): string {
  if (points.length === 0 || !line) return "";
  const first = points[0];
  const last = points[points.length - 1];
  return `${line} L${last.x.toFixed(2)},${baselineY.toFixed(2)} L${first.x.toFixed(2)},${baselineY.toFixed(2)} Z`;
}

export function formatClock(ts: number): string {
  const d = new Date(ts);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

export function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
}
