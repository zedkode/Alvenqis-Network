import type { ReactNode } from "react";
import { WidgetBoard } from "./WidgetBoard";
import { useWidgetLayout } from "./useWidgetLayout";
import { genericPageCatalog } from "./catalogs";

/**
 * Drop-in scaffold: wrap any page's major sections as resizable/draggable widgets.
 * Preserves children content; only changes chrome + layout engine.
 */
export function PageWidgetScaffold({
  pageId,
  panels
}: {
  pageId: string;
  panels: Array<{ id: string; label: string; w?: number; h?: number; node: ReactNode }>;
}) {
  const catalog = genericPageCatalog(
    panels.map((p) => ({ id: p.id, label: p.label, w: p.w, h: p.h }))
  );
  const defaults = panels.map((p) => p.id);
  const layoutApi = useWidgetLayout(pageId, catalog, defaults);
  const childrenMap: Record<string, ReactNode> = {};
  for (const p of panels) childrenMap[p.id] = p.node;

  return (
    <WidgetBoard
      pageId={pageId}
      editMode={layoutApi.editMode}
      items={layoutApi.visibleItems}
      catalogById={layoutApi.catalogById}
      hiddenCatalog={layoutApi.hiddenCatalog}
      getConstraints={layoutApi.getConstraints}
      onChangeItem={layoutApi.updateItem}
      onRemove={layoutApi.removeWidget}
      onAdd={layoutApi.addWidget}
      onReset={layoutApi.resetLayout}
      onToggleEdit={() => layoutApi.setEditMode((v) => !v)}
      childrenMap={childrenMap}
    />
  );
}
