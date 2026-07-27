import { describe, expect, it } from "vitest";
import { formatAtomic, formatCompactCount, formatHashrate } from "@shared/format";

describe("formatAtomic", () => {
  it("matches the Rust atomic formatting vectors", () => {
    expect(formatAtomic("100000001")).toBe("1.00000001");
    expect(formatAtomic("1")).toBe("0.00000001");
  });
});

describe("formatHashrate", () => {
  it("renders unavailable or invalid telemetry without inventing data", () => {
    expect(formatHashrate(undefined)).toBe("Not available");
    expect(formatHashrate(null)).toBe("Not available");
    expect(formatHashrate(Number.NaN)).toBe("Not available");
    expect(formatHashrate(-1)).toBe("Not available");
  });

  it("uses compact SI units", () => {
    expect(formatHashrate(0)).toBe("0 H/s");
    expect(formatHashrate(850)).toBe("850 H/s");
    expect(formatHashrate(12_500)).toBe("12.5K H/s");
    expect(formatHashrate(4_500_000)).toBe("4.5M H/s");
    expect(formatHashrate(1_200_000_000)).toBe("1.2G H/s");
    expect(formatHashrate(2_000_000_000_000)).toBe("2T H/s");
  });
});

describe("formatCompactCount", () => {
  it("formats hash totals like 1M / 1G", () => {
    expect(formatCompactCount(null)).toBe("—");
    expect(formatCompactCount(0)).toBe("0");
    expect(formatCompactCount(999)).toBe("999");
    expect(formatCompactCount(12_500)).toBe("12.5K");
    expect(formatCompactCount(4_500_000)).toBe("4.5M");
    expect(formatCompactCount("1200000000")).toBe("1.2G");
    expect(formatCompactCount(2_000_000_000_000)).toBe("2T");
  });
});
