import { describe, expect, it } from "vitest";
import { appendSample, smoothLinePath, toSeries } from "./chartPath";

describe("chartPath", () => {
  it("converts bare numbers without inventing values", () => {
    const series = toSeries([10, 20, 30], 1_000_000);
    expect(series.map((s) => s.value)).toEqual([10, 20, 30]);
    expect(series).toHaveLength(3);
    expect(series[2]!.ts).toBe(1_000_000);
  });

  it("appends only real samples and trims", () => {
    let series = appendSample([], 1, 3, 1000);
    series = appendSample(series, 2, 3, 2000);
    series = appendSample(series, 3, 3, 3000);
    series = appendSample(series, 4, 3, 4000);
    expect(series.map((s) => s.value)).toEqual([2, 3, 4]);
  });

  it("builds a smooth path through real points", () => {
    const d = smoothLinePath([
      { x: 0, y: 10 },
      { x: 10, y: 20 },
      { x: 20, y: 5 }
    ]);
    expect(d.startsWith("M")).toBe(true);
    expect(d.includes("C")).toBe(true);
  });
});
