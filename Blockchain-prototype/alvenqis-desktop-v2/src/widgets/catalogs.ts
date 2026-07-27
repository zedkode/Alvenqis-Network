import type { WidgetCatalogEntry } from "./types";

export const OVERVIEW_CATALOG: WidgetCatalogEntry[] = [
  { id: "chain-height", label: "Chain height", defaultW: 4, defaultH: 5, minW: 3, minH: 4, maxH: 8 },
  { id: "balance", label: "ALVE balance", defaultW: 2, defaultH: 3, minW: 2, minH: 2 },
  { id: "wallet", label: "Active wallet", defaultW: 2, defaultH: 3, minW: 2, minH: 2 },
  { id: "miner-status", label: "Miner status", defaultW: 2, defaultH: 3, minW: 2, minH: 2 },
  { id: "supply", label: "Emitted supply", defaultW: 2, defaultH: 3, minW: 2, minH: 2 },
  { id: "chart-hashrate", label: "Local hashrate", defaultW: 4, defaultH: 5, minW: 3, minH: 4 },
  { id: "chart-height", label: "Height chart", defaultW: 4, defaultH: 5, minW: 3, minH: 4 },
  { id: "chart-mempool", label: "Mempool pressure", defaultW: 4, defaultH: 5, minW: 3, minH: 4 },
  { id: "tx-density", label: "Tx density", defaultW: 6, defaultH: 5, minW: 4, minH: 4 },
  { id: "rewards-chart", label: "Block rewards", defaultW: 6, defaultH: 5, minW: 4, minH: 4 },
  { id: "recent-blocks", label: "Recent blocks", defaultW: 4, defaultH: 6, minW: 3, minH: 4, maxH: 12 },
  { id: "recent-txs", label: "Latest transactions", defaultW: 4, defaultH: 6, minW: 3, minH: 4, maxH: 12 },
  { id: "services", label: "Service matrix", defaultW: 4, defaultH: 6, minW: 3, minH: 4 },
  { id: "identity", label: "Network identity", defaultW: 4, defaultH: 5, minW: 3, minH: 3 },
  { id: "pool-snap", label: "Pool snapshot", defaultW: 4, defaultH: 5, minW: 3, minH: 3 },
  { id: "shortcuts", label: "Quick actions", defaultW: 4, defaultH: 5, minW: 3, minH: 3 }
];

export const OVERVIEW_DEFAULT_VISIBLE = OVERVIEW_CATALOG.map((c) => c.id);

export const MINING_CATALOG: WidgetCatalogEntry[] = [
  { id: "miner-kpi", label: "Mining KPIs", defaultW: 12, defaultH: 3, minW: 6, minH: 2, maxH: 5 },
  { id: "miner-scene", label: "Mining scene", defaultW: 5, defaultH: 8, minW: 4, minH: 6, maxH: 14 },
  { id: "miner-control", label: "Control deck", defaultW: 7, defaultH: 8, minW: 5, minH: 6, maxH: 16 },
  { id: "miner-presence", label: "Network presence", defaultW: 6, defaultH: 4, minW: 4, minH: 3 },
  { id: "miner-hash-chart", label: "Hashrate chart", defaultW: 6, defaultH: 4, minW: 4, minH: 3 },
  { id: "miner-console", label: "Miner console", defaultW: 12, defaultH: 7, minW: 6, minH: 4, maxH: 16 }
];

export const MINING_DEFAULT_VISIBLE = MINING_CATALOG.map((c) => c.id);

export const POOL_CATALOG: WidgetCatalogEntry[] = [
  { id: "pool-kpi", label: "Pool KPIs", defaultW: 12, defaultH: 3, minW: 6, minH: 2 },
  { id: "pool-selector", label: "Pool endpoints", defaultW: 4, defaultH: 6, minW: 3, minH: 4 },
  { id: "pool-main", label: "Pool data board", defaultW: 8, defaultH: 10, minW: 6, minH: 6, maxH: 18 },
  { id: "pool-maturity", label: "Maturity summary", defaultW: 12, defaultH: 3, minW: 4, minH: 2 }
];

export const POOL_DEFAULT_VISIBLE = POOL_CATALOG.map((c) => c.id);

/** Generic 2–4 panel pages share a simple catalog shape. */
export function genericPageCatalog(
  panels: Array<{ id: string; label: string; w?: number; h?: number }>
): WidgetCatalogEntry[] {
  return panels.map((p) => ({
    id: p.id,
    label: p.label,
    defaultW: p.w ?? 6,
    defaultH: p.h ?? 6,
    minW: 3,
    minH: 3,
    maxH: 16
  }));
}
