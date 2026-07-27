/**
 * Predictive, user-facing error guidance for Control Center V2.
 * Maps common gateway / miner / wallet failures to actionable next steps.
 */

export type PredictedIssue = {
  code: string;
  title: string;
  severity: "info" | "warning" | "error";
  summary: string;
  actions: string[];
};

export function predictFromMessage(raw: string): PredictedIssue {
  const msg = raw.toLowerCase();

  if (msg.includes("mining template") || msg.includes("mining disabled") || msg.includes("/mining/template")) {
    return {
      code: "MINING_PATH_DISABLED",
      title: "Public mining path unavailable",
      severity: "warning",
      summary:
        "The public RPC profile keeps mining disabled. Solo work needs local RPC or VPS ENABLE_MINING_RPC / private mining-rpc.",
      actions: [
        "Use Miner mode Solo with a private mining RPC (loopback or enabled VPS mining profile).",
        "Or switch to Pool / Stratum when those endpoints are intentionally enabled.",
        "Run public smoke scripts: smoke-public-candidate / smoke-private-mining."
      ]
    };
  }

  if (msg.includes("stratum") && (msg.includes("connect") || msg.includes("resolve") || msg.includes("timeout"))) {
    return {
      code: "STRATUM_CONNECT",
      title: "Stratum endpoint unreachable",
      severity: "error",
      summary: "TCP/TLS handshake to the Stratum host failed.",
      actions: [
        "Verify host/port and TLS toggle match the pool operator configuration.",
        "Confirm firewall allows outbound TCP to the Stratum port.",
        "Retry the verified Stratum TLS endpoint, or switch to Solo RPC only when a synchronized local node is available."
      ]
    };
  }

  if (msg.includes("authorize") || msg.includes("unauthorized") || msg.includes("401") || msg.includes("403")) {
    return {
      code: "AUTH_REJECTED",
      title: "Authentication rejected",
      severity: "error",
      summary: "Wallet, pool worker, or admin token was not accepted.",
      actions: [
        "Confirm the active wallet is Mainnet Candidate (alve1…).",
        "For pool: check worker name and pool URL.",
        "For admin/Grafana: re-read /home/credentials.md on the VPS after rotation."
      ]
    };
  }

  if (msg.includes("cuda") || msg.includes("gpu") || msg.includes("no devices") || msg.includes("nvidia")) {
    return {
      code: "CUDA_DEVICE",
      title: "CUDA device problem",
      severity: "error",
      summary: "Product mining is NVIDIA CUDA-only; no CPU/OpenCL fallback exists.",
      actions: [
        "Install a supported NVIDIA driver and CUDA-capable GPU.",
        "Run Miner → devices (or console: devices) and select a CUDA id.",
        "Rebuild miner with CUDA feature if the sidecar is a stub."
      ]
    };
  }

  if (msg.includes("rate limit") || msg.includes("429") || msg.includes("throttl")) {
    return {
      code: "RATE_LIMIT",
      title: "Gateway rate limiting",
      severity: "warning",
      summary: "Refresh cadence is too aggressive for the remote RPC.",
      actions: [
        "Increase Settings → refresh interval (remote floor applies automatically).",
        "Prefer local loopback RPC for high-frequency development.",
        "Wait and retry; exponential backoff is applied while degraded."
      ]
    };
  }

  if (msg.includes("timeout") || msg.includes("timed out") || msg.includes("unreachable") || msg.includes("dns")) {
    return {
      code: "NETWORK_TIMEOUT",
      title: "Network timeout",
      severity: "warning",
      summary: "The desktop could not complete a request to the gateway or pool.",
      actions: [
        "Check internet connectivity and Cloudflare tunnel status on the VPS.",
        "Verify RPC URL under Settings → Network.",
        "Run Analytics/Overview after connectivity recovers."
      ]
    };
  }

  if (msg.includes("wallet") || msg.includes("mnemonic") || msg.includes("recovery")) {
    return {
      code: "WALLET",
      title: "Wallet / keystore issue",
      severity: "error",
      summary: "A wallet operation failed before signing or mining payout address selection.",
      actions: [
        "Create or import a wallet from the startup gate or Wallet page.",
        "Never paste recovery phrases into chat or logs.",
        "Confirm the address HRP is alve for Mainnet Candidate."
      ]
    };
  }

  return {
    code: "GENERIC",
    title: "Operation failed",
    severity: "error",
    summary: raw.slice(0, 240) || "Unknown error",
    actions: [
      "Open the relevant console (Miner / Activity) for the full log tail.",
      "Retry once; if it persists, export logs and check VPS health.",
      "See DESKTOP_V2_USER_GUIDE.md → Troubleshooting."
    ]
  };
}

export function formatPrediction(issue: PredictedIssue): string {
  const acts = issue.actions.map((a, i) => `${i + 1}. ${a}`).join(" ");
  return `${issue.title}: ${issue.summary} ${acts}`;
}
