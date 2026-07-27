import type { NetworkSnapshot } from "@shared/types";

export type StartupAccessMode = "network-synced" | "gateway-ready" | "local-isolated" | "blocked";

/**
 * Control Center is VPS/gateway-first.
 * - Remote gateway: ready when /status answers with a chain height.
 * - Local stack (optional/dev): node + rpc with a local-oriented detail string.
 */
export function startupAccessMode(snapshot: NetworkSnapshot): StartupAccessMode {
  const chainVisible =
    snapshot.online &&
    snapshot.rpc_running &&
    snapshot.height !== null &&
    snapshot.block_count > 0;

  if (!chainVisible) {
    return "blocked";
  }

  if (snapshot.sync_status === "synced") {
    return "network-synced";
  }

  // Local isolated: managed local node language, not VPS gateway copy.
  const remote = isRemoteGatewayDetail(snapshot.detail);
  if (
    snapshot.node_running &&
    !remote &&
    (snapshot.sync_status === "discovering" || /local/i.test(snapshot.detail))
  ) {
    return "local-isolated";
  }

  // Gateway answered with chain data (peers may still be discovering).
  return "gateway-ready";
}

export function isRemoteGatewayDetail(detail: string): boolean {
  return (
    /vps gateway/i.test(detail) ||
    /rpc verified at (?!127\.0\.0\.1|localhost)/i.test(detail) ||
    /rpcnode\.dohotstudio\.com/i.test(detail)
  );
}
