import { describe, expect, it } from "vitest";
import type { ConnectionAvailability } from "@shared/types";
import {
  connectionAgeLabel,
  connectionLabel,
  connectionPollDelay
} from "./connectivity";

function connection(
  overrides: Partial<ConnectionAvailability> = {}
): ConnectionAvailability {
  return {
    status: "online",
    circuit: "closed",
    endpoint: "https://rpc.example/status",
    checked_at_unix_seconds: 100,
    last_success_at_unix_seconds: 100,
    next_retry_at_unix_seconds: null,
    consecutive_failures: 0,
    latency_ms: 12,
    error: null,
    ...overrides
  };
}

describe("connectionPollDelay", () => {
  it("uses the configured healthy cadence", () => {
    expect(connectionPollDelay(5_000, 3_000, 0, connection(), 100_000)).toBe(5_000);
  });

  it("backs off failed connections without exceeding the cap", () => {
    expect(
      connectionPollDelay(
        5_000,
        3_000,
        4,
        connection({ status: "offline" }),
        100_000
      )
    ).toBe(60_000);
  });

  it("schedules the circuit recovery probe at retry time", () => {
    expect(
      connectionPollDelay(
        15_000,
        12_000,
        3,
        connection({
          status: "offline",
          circuit: "open",
          next_retry_at_unix_seconds: 106
        }),
        100_000
      )
    ).toBe(6_000);
  });
});

describe("connection presentation", () => {
  it("never labels an open circuit online", () => {
    expect(connectionLabel(connection({ status: "offline", circuit: "open" }))).toBe(
      "OFFLINE · CIRCUIT OPEN"
    );
  });

  it("reports the real last-success age", () => {
    expect(connectionAgeLabel(connection(), 105)).toBe("5s ago");
    expect(
      connectionAgeLabel(connection({ last_success_at_unix_seconds: null }), 105)
    ).toBe("never");
  });
});
