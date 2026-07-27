import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { formatAtomic } from "@shared/format";
import { useNotificationsOptional } from "../shared/notifications";

/**
 * Subscribes to real Tauri backend events and feeds the in-app notification store.
 * Never invents events — only reacts to payloads from Rust.
 */
export function useAlvenqisEvents(opts?: {
  minerWasRunning?: boolean;
  online?: boolean;
}) {
  const api = useNotificationsOptional();
  const minerWasRunning = useRef(opts?.minerWasRunning ?? false);
  const onlineWas = useRef(opts?.online ?? false);

  useEffect(() => {
    if (!api) return;
    let unsubs: Array<() => void> = [];

    void (async () => {
      unsubs.push(
        await listen<{ height: number; reward_atomic: string }>("alvenqis:block-mined", (event) => {
          const { height, reward_atomic } = event.payload;
          api.notify({
            kind: "mining",
            title: `Block #${height} mined`,
            body: `Reward ${formatAtomic(reward_atomic)} ALVE accepted on chain.`,
            severity: "both",
            source: "alvenqis:block-mined"
          });
        })
      );

      unsubs.push(
        await listen<{ title: string; body: string; kind?: string }>("alvenqis:notify", (event) => {
          const kind = (event.payload.kind as "info" | "success" | "warning" | "error" | "system") || "system";
          api.notify({
            kind,
            title: event.payload.title,
            body: event.payload.body,
            severity: kind === "error" || kind === "warning" ? "both" : "center",
            source: "alvenqis:notify"
          });
        })
      );

      // Auto-update notifications intentionally disabled.
    })();

    return () => {
      for (const u of unsubs) u();
    };
  }, [api]);

  const minerBaseline = useRef(false);
  const onlineBaseline = useRef(false);
  const sawOffline = useRef(false);

  // Real state transitions observed from snapshot props (not invented; skip first sample).
  useEffect(() => {
    if (!api || opts?.minerWasRunning === undefined) return;
    if (!minerBaseline.current) {
      minerBaseline.current = true;
      minerWasRunning.current = !!opts.minerWasRunning;
      return;
    }
    if (minerWasRunning.current && opts.minerWasRunning === false) {
      api.notify({
        kind: "warning",
        title: "Miner stopped",
        body: "Local miner is no longer running. Check Miner console if this was unexpected.",
        severity: "toast",
        source: "snapshot:miner"
      });
    }
    minerWasRunning.current = !!opts.minerWasRunning;
  }, [api, opts?.minerWasRunning]);

  useEffect(() => {
    if (!api || opts?.online === undefined) return;
    if (!onlineBaseline.current) {
      onlineBaseline.current = true;
      onlineWas.current = !!opts.online;
      return;
    }
    if (onlineWas.current && opts.online === false) {
      sawOffline.current = true;
      api.notify({
        kind: "warning",
        title: "Gateway offline",
        body: "RPC gateway is not reachable. Network views may be stale.",
        severity: "both",
        sticky: true,
        source: "snapshot:online"
      });
    } else if (!onlineWas.current && opts.online === true && sawOffline.current) {
      api.notify({
        kind: "success",
        title: "Gateway online",
        body: "RPC connectivity restored after offline period.",
        severity: "toast",
        source: "snapshot:online"
      });
    }
    onlineWas.current = !!opts.online;
  }, [api, opts?.online]);
}
