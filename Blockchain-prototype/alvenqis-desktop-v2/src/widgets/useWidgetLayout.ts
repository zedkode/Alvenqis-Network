import { useCallback, useMemo, useState } from "react";
import {
  clearPageLayout,
  defaultLayoutFromCatalog,
  loadPageLayout,
  mergeLayoutWithCatalog,
  savePageLayout
} from "./storage";
import type { WidgetCatalogEntry, WidgetRect } from "./types";

export function useWidgetLayout(
  pageId: string,
  catalog: WidgetCatalogEntry[],
  defaultVisible?: string[]
) {
  const [layout, setLayout] = useState(() =>
    mergeLayoutWithCatalog(loadPageLayout(pageId), pageId, catalog, defaultVisible)
  );
  const [editMode, setEditMode] = useState(false);

  const catalogById = useMemo(() => {
    const m = new Map<string, WidgetCatalogEntry>();
    for (const c of catalog) m.set(c.id, c);
    return m;
  }, [catalog]);

  const visibleItems = useMemo(
    () => layout.items.filter((i) => i.visible),
    [layout.items]
  );

  const hiddenCatalog = useMemo(
    () =>
      catalog.filter((c) => {
        const item = layout.items.find((i) => i.id === c.id);
        return !item || !item.visible;
      }),
    [catalog, layout.items]
  );

  const updateItem = useCallback(
    (id: string, patch: Partial<WidgetRect>, opts?: { persist?: boolean }) => {
      setLayout((prev) => {
        const next = {
          ...prev,
          items: prev.items.map((i) => (i.id === id ? { ...i, ...patch } : i))
        };
        if (opts?.persist !== false) savePageLayout(next);
        return next;
      });
    },
    []
  );

  const setItems = useCallback((items: WidgetRect[]) => {
    setLayout((prev) => {
      const next = { ...prev, items };
      savePageLayout(next);
      return next;
    });
  }, []);

  const removeWidget = useCallback(
    (id: string) => updateItem(id, { visible: false }),
    [updateItem]
  );

  const addWidget = useCallback(
    (id: string) => {
      const entry = catalogById.get(id);
      if (!entry) return;
      setLayout((prev) => {
        const existing = prev.items.find((i) => i.id === id);
        if (existing) {
          const next = {
            ...prev,
            items: prev.items.map((i) => (i.id === id ? { ...i, visible: true } : i))
          };
          savePageLayout(next);
          return next;
        }
        const maxY = prev.items.reduce(
          (m, i) => (i.visible ? Math.max(m, i.y + i.h) : m),
          0
        );
        const next = {
          ...prev,
          items: [
            ...prev.items,
            {
              id,
              x: 0,
              y: maxY,
              w: entry.defaultW,
              h: entry.defaultH,
              visible: true
            }
          ]
        };
        savePageLayout(next);
        return next;
      });
    },
    [catalogById]
  );

  const resetLayout = useCallback(() => {
    clearPageLayout(pageId);
    const next = defaultLayoutFromCatalog(pageId, catalog, defaultVisible);
    setLayout(next);
    savePageLayout(next);
  }, [pageId, catalog, defaultVisible]);

  const getConstraints = useCallback(
    (id: string) => {
      const c = catalogById.get(id);
      return {
        minW: c?.minW ?? 2,
        minH: c?.minH ?? 2,
        maxW: c?.maxW ?? 12,
        maxH: c?.maxH ?? 20
      };
    },
    [catalogById]
  );

  return {
    layout,
    visibleItems,
    hiddenCatalog,
    editMode,
    setEditMode,
    updateItem,
    setItems,
    removeWidget,
    addWidget,
    resetLayout,
    getConstraints,
    catalogById
  };
}
