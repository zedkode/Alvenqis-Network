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

const titlesRo: Record<PageId, [string, string]> = {
  overview: ["Prezentare", "Centru de comandă · tip live · portofoliu"],
  analytics: ["Analiză", "Grafice multi-serie · rate · sănătate"],
  wallet: ["Portofel", "Chei pe dispozitiv · solduri din gateway"],
  send: ["Trimite & Primește", "Semnezi local · trimiți la VPS"],
  mining: ["Miner", "GPU FiroPoW 0.9.4 · solo sau pool"],
  pool: ["Pool", "Workers · hashrate · maturitate · multi-pool"],
  explorer: ["Explorer", "Blocuri, tx și adrese din indexer"],
  blocks: ["Blocuri", "Vârful canonic și înălțimi recente"],
  transactions: ["Tranzacții", "Ciclu de viață transferuri"],
  mempool: ["Mempool", "Coadă pending pe gateway"],
  node: ["Rețea", "Peers, fleet și validatori"],
  activity: ["Activitate", "Loguri și evenimente de lanț"],
  messages: ["Mesaje", "Inbox · sistem · mining · securitate"],
  rewards: ["Recompense", "Recompense mining din blocuri indexate"],
  assets: ["Active", "Suprafață ledger ALVE nativ"],
  settings: ["Setări", "Aspect, RPC, miner, confidențialitate, mișcare"]
};

export function pageTitle(page: string, language: LanguageId = "en"): [string, string] {
  const map = language === "ro" ? titlesRo : titlesEn;
  return map[page as PageId] ?? titlesEn.overview;
}

export const commandLabels = {
  en: {
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
  },
  ro: {
    palettePlaceholder: "Navighează la pagină, grafic, mesaj sau setare…",
    noResults: "Nicio comandă potrivită",
    pages: "Pagini",
    actions: "Acțiuni",
    refresh: "Reîmprospătează telemetria",
    toggleTheme: "Comută temă dark / light",
    openWallet: "Deschide selector portofel",
    openSettings: "Deschide setările",
    startMining: "Mergi la miner",
    send: "Trimite ALVE",
    openMessages: "Deschide mesaje",
    openAnalytics: "Deschide analiză"
  }
};
