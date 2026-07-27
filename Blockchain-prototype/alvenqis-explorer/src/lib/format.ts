export function formatAtomic(amountAtomic: number): string {
  const sign = amountAtomic < 0 ? "-" : "";
  const absolute = Math.abs(amountAtomic);
  const whole = Math.floor(absolute / 100_000_000);
  const fraction = String(absolute % 100_000_000).padStart(8, "0");
  return `${sign}${whole.toLocaleString()}.${fraction} ALVE`;
}

export function formatTimestamp(timestamp: number | null): string {
  if (timestamp === null) {
    return "Unavailable";
  }
  return new Date(timestamp * 1000).toLocaleString();
}

export function formatCount(value: number | null): string {
  if (value === null) {
    return "Unavailable";
  }
  return value.toLocaleString();
}

export function formatHashrate(hashrateHs: number): string {
  const units = ["H/s", "kH/s", "MH/s", "GH/s", "TH/s", "PH/s"];
  let value = Math.max(0, hashrateHs);
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  return `${value.toFixed(value >= 100 ? 0 : 2)} ${units[unit]}`;
}

export function shortHash(hash: string | null | undefined, max = 16): string {
  if (!hash) {
    return "Unavailable";
  }
  if (hash.length <= max) {
    return hash;
  }
  return `${hash.slice(0, Math.floor(max / 2))}...${hash.slice(-Math.floor(max / 2))}`;
}
