export function normalizeRecoveryPhrase(value: string): string {
  return value.trim().split(/\s+/u).join(" ").toLowerCase();
}

export function recoveryWordCount(value: string): number {
  const normalized = normalizeRecoveryPhrase(value);
  return normalized ? normalized.split(" ").length : 0;
}

export function validateRecoveryPhraseInput(value: string): string | null {
  const count = recoveryWordCount(value);
  if (count !== 24) return `Enter exactly 24 recovery words. Current count: ${count}.`;
  return null;
}
