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

export type LanguageId = "en" | "ro";

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

export const PAGE_LABELS_RO: Record<PageId, string> = {
  overview: "Prezentare",
  analytics: "Analiză",
  wallet: "Portofel",
  send: "Trimite & Primește",
  rewards: "Recompense",
  assets: "Active",
  mining: "Miner",
  pool: "Pool",
  explorer: "Explorer",
  blocks: "Blocuri",
  transactions: "Tranzacții",
  mempool: "Mempool",
  node: "Rețea",
  activity: "Activitate",
  messages: "Mesaje",
  settings: "Setări"
};
