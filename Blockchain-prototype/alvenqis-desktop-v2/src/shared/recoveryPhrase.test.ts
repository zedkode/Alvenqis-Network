import { describe, expect, it } from "vitest";
import { normalizeRecoveryPhrase, recoveryWordCount, validateRecoveryPhraseInput } from "./recoveryPhrase";

describe("recovery phrase input", () => {
  it("normalizes whitespace without retaining formatting", () => {
    expect(normalizeRecoveryPhrase("  Alpha\n BETA   gamma ")).toBe("alpha beta gamma");
  });

  it("requires exactly twenty-four words", () => {
    const phrase = Array.from({ length: 24 }, (_, index) => `word${index}`).join(" ");
    expect(recoveryWordCount(phrase)).toBe(24);
    expect(validateRecoveryPhraseInput(phrase)).toBeNull();
    expect(validateRecoveryPhraseInput("too short")).toContain("Current count: 2");
  });
});
