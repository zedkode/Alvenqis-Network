/**
 * Pool block maturity helper — same rule as alvenqis-mining-pool:
 * mature when tipHeight >= blockHeight + requiredConfirmations.
 */
export interface MaturityProgress {
  status: "immature" | "mature" | "orphaned" | "unknown";
  confirmations: number;
  required: number;
  remaining: number;
  percent: number;
  /** Chain tip must reach this height (inclusive) for maturity. */
  matureAtTip: number;
  label: string;
}

export function poolBlockMaturity(
  blockHeight: number,
  tipHeight: number | null | undefined,
  requiredConfirmations = 12,
  poolStatusField?: string
): MaturityProgress {
  const required = Math.max(1, requiredConfirmations);
  const field = (poolStatusField ?? "").toLowerCase();
  const matureAtTip = blockHeight + required;

  if (field.includes("orphan")) {
    return {
      status: "orphaned",
      confirmations: 0,
      required,
      remaining: 0,
      percent: 0,
      matureAtTip,
      label: "orphaned"
    };
  }

  if (field === "mature" || field.includes("matured")) {
    return {
      status: "mature",
      confirmations: required,
      required,
      remaining: 0,
      percent: 100,
      matureAtTip,
      label: "mature"
    };
  }

  if (tipHeight == null) {
    return {
      status: "unknown",
      confirmations: 0,
      required,
      remaining: required,
      percent: 0,
      matureAtTip,
      label: "immature · tip unknown"
    };
  }

  if (tipHeight >= matureAtTip) {
    return {
      status: "mature",
      confirmations: required,
      required,
      remaining: 0,
      percent: 100,
      matureAtTip,
      label: "mature"
    };
  }

  const confirmations =
    tipHeight < blockHeight ? 0 : Math.min(required, Math.max(0, tipHeight - blockHeight));
  const remaining = Math.max(0, required - confirmations);
  const percent = Math.round((confirmations / required) * 100);

  return {
    status: "immature",
    confirmations,
    required,
    remaining,
    percent,
    matureAtTip,
    label: `immature · ${confirmations}/${required}`
  };
}
