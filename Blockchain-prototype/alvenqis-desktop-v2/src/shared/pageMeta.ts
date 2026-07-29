export type PageId =
  | "overview"
  | "analytics"
  | "wallet"
  | "send"
  | "mining"
  | "pool"
  | "explorer"
  | "blocks"
  | "transactions"
  | "mempool"
  | "node"
  | "activity"
  | "messages"
  | "rewards"
  | "assets"
  | "settings";

export type LanguageId = "en";

export const PAGE_ORDER: PageId[] = [
  "overview",
  "analytics",
  "wallet",
  "send",
  "rewards",
  "assets",
  "mining",
  "pool",
  "explorer",
  "blocks",
  "transactions",
  "mempool",
  "node",
  "activity",
  "messages",
  "settings"
];

export const PAGE_LABELS: Record<PageId, string> = {
  overview: "Overview",
  analytics: "Analytics",
  wallet: "Wallet",
  send: "Send & Receive",
  rewards: "Rewards",
  assets: "Assets",
  mining: "Miner",
  pool: "Pool",
  explorer: "Explorer",
  blocks: "Blocks",
  transactions: "Transactions",
  mempool: "Mempool",
  node: "Network",
  activity: "Activity",
  messages: "Messages",
  settings: "Settings"
};
