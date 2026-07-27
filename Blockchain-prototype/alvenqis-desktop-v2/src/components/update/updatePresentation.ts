import type { UpdatePhase, UpdateState } from "@shared/types";

export function isBlockingUpdate(phase: UpdatePhase): boolean {
  return phase === "downloading" || phase === "downloaded" || phase === "installing";
}

export function updateNoticeKey(state: UpdateState): string {
  return [state.phase, state.available_version ?? state.current_version, state.message].join(":");
}

export function shouldShowUpdateNotice(state: UpdateState, dismissedNotice: string | null): boolean {
  const dismissible =
    state.phase === "available"
    || state.phase === "error"
    || state.phase === "checking"
    || (state.manual && state.phase === "unavailable")
    || (state.manual && state.phase === "idle" && Boolean(state.available_version));
  return dismissible && updateNoticeKey(state) !== dismissedNotice;
}

export function updateHeading(state: UpdateState): string {
  if (state.phase === "downloaded") return "Verified update ready";
  if (state.phase === "installing") return "Applying approved update";
  if (state.phase === "available") return "Verified update available";
  return "Downloading approved update from GitHub";
}

export function formatUpdateBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const exponent = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${(value / 1024 ** exponent).toFixed(exponent === 0 ? 0 : 1)} ${units[exponent]}`;
}
