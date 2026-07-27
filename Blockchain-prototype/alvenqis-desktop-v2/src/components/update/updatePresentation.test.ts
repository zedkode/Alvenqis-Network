import { describe, expect, it } from "vitest";
import { formatUpdateBytes, isBlockingUpdate, shouldShowUpdateNotice, updateNoticeKey } from "./updatePresentation";

describe("update presentation", () => {
  it("blocks the panel only during transfer and installation phases", () => {
    expect(isBlockingUpdate("available")).toBe(false);
    expect(isBlockingUpdate("downloading")).toBe(true);
    expect(isBlockingUpdate("downloaded")).toBe(true);
    expect(isBlockingUpdate("installing")).toBe(true);
  });

  it("formats transfer statistics without invalid values", () => {
    expect(formatUpdateBytes(0)).toBe("0 B");
    expect(formatUpdateBytes(Number.NaN)).toBe("0 B");
    expect(formatUpdateBytes(1_048_576)).toBe("1.0 MB");
  });

  it("creates a stable key for dismissible notices", () => {
    const state = {
      phase: "error" as const, current_version: "0.3.4", available_version: null,
      release_name: null, release_date: null, message: "Feed unavailable", manual: false, progress: null
    };
    expect(updateNoticeKey(state)).toBe("error:0.3.4:Feed unavailable");
    expect(shouldShowUpdateNotice(state, null)).toBe(true);
    expect(shouldShowUpdateNotice(state, updateNoticeKey(state))).toBe(false);
  });
});
