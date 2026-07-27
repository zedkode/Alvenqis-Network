export const ATOMIC_UNITS_PER_ALVE = 100_000_000n;

export function formatAtomic(value: string | bigint): string {
  const atomic = typeof value === "bigint" ? value : BigInt(value);
  const whole = atomic / ATOMIC_UNITS_PER_ALVE;
  const fraction = (atomic % ATOMIC_UNITS_PER_ALVE).toString().padStart(8, "0");
  return `${whole}.${fraction}`;
}

export function shortHash(value: string | null | undefined, edge = 8): string {
  if (!value) return "Not available";
  return value.length <= edge * 2 + 1 ? value : `${value.slice(0, edge)}…${value.slice(-edge)}`;
}

/**
 * Compact count (hashes, totals): 1.2K · 4.5M · 1.1G · 2T (no unit suffix).
 * Accepts number or decimal string from miner metrics.
 */
export function formatCompactCount(value: number | string | bigint | null | undefined): string {
  if (value == null) return "—";
  let n: number;
  if (typeof value === "bigint") {
    if (value < 0n) return "—";
    // Keep precision for large totals with integer division by SI steps.
    const steps: Array<[bigint, string]> = [
      [1_000_000_000_000_000n, "P"],
      [1_000_000_000_000n, "T"],
      [1_000_000_000n, "G"],
      [1_000_000n, "M"],
      [1_000n, "K"]
    ];
    for (const [step, suffix] of steps) {
      if (value >= step) {
        const whole = value / step;
        const frac = ((value % step) * 100n) / step;
        if (whole >= 100n || frac === 0n) return `${whole}${suffix}`;
        const fracStr = frac.toString().padStart(2, "0").replace(/0+$/, "");
        return fracStr ? `${whole}.${fracStr}${suffix}` : `${whole}${suffix}`;
      }
    }
    return value.toString();
  }
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (!trimmed) return "—";
    if (/^\d+$/.test(trimmed)) {
      try {
        return formatCompactCount(BigInt(trimmed));
      } catch {
        /* fall through */
      }
    }
    n = Number(trimmed);
  } else {
    n = value;
  }
  if (!Number.isFinite(n) || n < 0) return "—";
  if (n === 0) return "0";
  const tiers: Array<[number, string]> = [
    [1e15, "P"],
    [1e12, "T"],
    [1e9, "G"],
    [1e6, "M"],
    [1e3, "K"]
  ];
  for (const [threshold, suffix] of tiers) {
    if (n >= threshold) {
      const scaled = n / threshold;
      const digits = scaled >= 100 ? 0 : scaled >= 10 ? 1 : 2;
      return `${Number(scaled.toFixed(digits))}${suffix}`;
    }
  }
  return `${Math.round(n)}`;
}

/**
 * Compact live hashrate: 1.2K · 4.5M · 1.1G · 2T H/s (not long decimals).
 */
export function formatHashrate(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value) || value < 0) return "Not available";
  if (value === 0) return "0 H/s";
  return `${formatCompactCount(value)} H/s`;
}

export function confirmations(currentHeight: number | null, blockHeight: number): number {
  return currentHeight === null || blockHeight > currentHeight ? 0 : currentHeight - blockHeight + 1;
}

export function formatTimestamp(seconds: number): string {
  return new Date(seconds * 1000).toLocaleString();
}

export function formatBytes(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value) || value < 0) return "—";
  if (value < 1024) return `${Math.round(value)} B`;
  const units = ["KB", "MB", "GB", "TB"] as const;
  let n = value;
  let i = -1;
  do {
    n /= 1024;
    i += 1;
  } while (n >= 1024 && i < units.length - 1);
  return `${n.toFixed(n >= 10 ? 0 : 1)} ${units[i]}`;
}
