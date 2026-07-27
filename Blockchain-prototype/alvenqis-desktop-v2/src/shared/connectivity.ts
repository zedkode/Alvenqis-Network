import type { ConnectionAvailability } from "@shared/types";

export function connectionPollDelay(
  baseMs: number,
  minimumMs: number,
  failStreak: number,
  connection?: ConnectionAvailability | null,
  nowMs = Date.now()
): number {
  const base = Math.max(minimumMs, baseMs);
  if (
    connection?.circuit === "open" &&
    connection.next_retry_at_unix_seconds !== null
  ) {
    const retryIn = connection.next_retry_at_unix_seconds * 1000 - nowMs;
    return Math.max(1_000, Math.min(60_000, retryIn));
  }
  if (connection?.status === "online" && failStreak === 0) return base;
  const exponent = Math.min(4, Math.max(1, failStreak));
  return Math.min(60_000, base * 2 ** exponent);
}

export function connectionLabel(connection: ConnectionAvailability): string {
  if (connection.status === "idle") return "IDLE";
  if (connection.circuit === "open") return "OFFLINE · CIRCUIT OPEN";
  if (connection.circuit === "half_open") return "RECOVERING";
  return connection.status.toUpperCase();
}

export function connectionAgeLabel(
  connection: ConnectionAvailability,
  nowUnixSeconds = Math.floor(Date.now() / 1000)
): string {
  if (connection.last_success_at_unix_seconds === null) return "never";
  const age = Math.max(0, nowUnixSeconds - connection.last_success_at_unix_seconds);
  return age === 0 ? "now" : `${age}s ago`;
}
