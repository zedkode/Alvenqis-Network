import {
  WIDGET_LAYOUT_VERSION,
  WIDGET_STORAGE_KEY,
  type PageWidgetLayout,
  type WidgetCatalogEntry,
  type WidgetRect
} from "./types";

type Store = Record<string, PageWidgetLayout>;

function readStore(): Store {
  try {
    const raw = localStorage.getItem(WIDGET_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Store;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function writeStore(store: Store) {
  try {
    localStorage.setItem(WIDGET_STORAGE_KEY, JSON.stringify(store));
  } catch {
    /* quota / private mode */
  }
}

export function loadPageLayout(pageId: string): PageWidgetLayout | null {
  const entry = readStore()[pageId];
  if (!entry || entry.version !== WIDGET_LAYOUT_VERSION) return null;
  if (!Array.isArray(entry.items)) return null;
  return entry;
}

export function savePageLayout(layout: PageWidgetLayout) {
  const store = readStore();
  store[layout.pageId] = layout;
  writeStore(store);
}

export function clearPageLayout(pageId: string) {
  const store = readStore();
  delete store[pageId];
  writeStore(store);
}

/** Build default layout: pack catalog left→right, top→bottom. */
export function defaultLayoutFromCatalog(
  pageId: string,
  catalog: WidgetCatalogEntry[],
  visibleIds?: string[]
): PageWidgetLayout {
  const allow = visibleIds ? new Set(visibleIds) : null;
  const items: WidgetRect[] = [];
  let x = 0;
  let y = 0;
  let rowH = 0;

  for (const entry of catalog) {
    const visible = allow ? allow.has(entry.id) : true;
    const w = Math.min(12, Math.max(1, entry.defaultW));
    const h = Math.max(1, entry.defaultH);
    if (x + w > 12) {
      x = 0;
      y += rowH;
      rowH = 0;
    }
    items.push({ id: entry.id, x, y, w, h, visible });
    x += w;
    rowH = Math.max(rowH, h);
  }

  return { pageId, version: WIDGET_LAYOUT_VERSION, items };
}

export function mergeLayoutWithCatalog(
  layout: PageWidgetLayout | null,
  pageId: string,
  catalog: WidgetCatalogEntry[],
  defaultVisible?: string[]
): PageWidgetLayout {
  const base = layout ?? defaultLayoutFromCatalog(pageId, catalog, defaultVisible);
  const byId = new Map(base.items.map((i) => [i.id, i]));
  const items: WidgetRect[] = catalog.map((entry) => {
    const prev = byId.get(entry.id);
    if (prev) {
      return {
        ...prev,
        w: Math.min(entry.maxW ?? 12, Math.max(entry.minW ?? 1, prev.w)),
        h: Math.min(entry.maxH ?? 24, Math.max(entry.minH ?? 1, prev.h))
      };
    }
    return {
      id: entry.id,
      x: 0,
      y: 0,
      w: entry.defaultW,
      h: entry.defaultH,
      visible: defaultVisible ? defaultVisible.includes(entry.id) : false
    };
  });
  return { pageId, version: WIDGET_LAYOUT_VERSION, items };
}
