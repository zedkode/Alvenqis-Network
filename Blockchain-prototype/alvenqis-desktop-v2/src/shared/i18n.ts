import type { LanguageId, PageId } from "./pageMeta";

export type { PageId };

const titlesEn: Record<PageId, [string, string]> = {
  overview: ["Overview", "Command center · live tip · portfolio pulse"],
  analytics: ["Analytics", "Multi-series charts · rates · health"],
  wallet: ["Wallet", "Keys on device · balances from gateway"],
  send: ["Send & Receive", "Sign locally · broadcast to VPS"],
  mining: ["Miner", "GPU FiroPoW 0.9.4 · solo or pool"],
  pool: ["Pool", "Workers · hashrate · maturity · multi-pool"],
  explorer: ["Explorer", "Blocks, txs & addresses from indexer"],
  blocks: ["Blocks", "Canonical tip and recent heights"],
  transactions: ["Transactions", "Confirmed transfer lifecycle"],
  mempool: ["Mempool", "Pending queue on the gateway"],
  node: ["Network", "Peers, fleet & validator view"],
  activity: ["Activity", "Process logs and chain events"],
  messages: ["Messages", "Inbox · system · mining · security"],
  rewards: ["Rewards", "Mining rewards from indexed blocks"],
  assets: ["Assets", "Native ALVE ledger surface"],
  settings: ["Settings", "Appearance, RPC, miner, privacy, motion"]
};

export function pageTitle(page: string, _language: LanguageId = "en"): [string, string] {
  return titlesEn[page as PageId] ?? titlesEn.overview;
}

export const commandLabels = {
  palettePlaceholder: "Jump to page, chart, message, or setting…",
  noResults: "No matching commands",
  pages: "Pages",
  actions: "Actions",
  refresh: "Refresh telemetry",
  toggleTheme: "Toggle dark / light theme",
  openWallet: "Open wallet switcher",
  openSettings: "Open settings",
  startMining: "Go to miner",
  send: "Send ALVE",
  openMessages: "Open messages",
  openAnalytics: "Open analytics"
};
