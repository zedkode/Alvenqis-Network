import type { ReactNode } from "react";
import { LayoutGrid, Plus, RotateCcw } from "lucide-react";
import { WidgetShell } from "./WidgetShell";
import type { WidgetCatalogEntry, WidgetRect } from "./types";
import { WIDGET_COLS, WIDGET_GAP_PX, WIDGET_ROW_PX } from "./types";

export function WidgetBoard({
  pageId,
  editMode,
  items,
  catalogById,
  hiddenCatalog,
  getConstraints,
  onChangeItem,
  onRemove,
  onAdd,
  onReset,
  onToggleEdit,
  childrenMap
}: {
  pageId: string;
  editMode: boolean;
  items: WidgetRect[];
  catalogById: Map<string, WidgetCatalogEntry>;
  hiddenCatalog: WidgetCatalogEntry[];
  getConstraints(id: string): { minW: number; minH: number; maxW: number; maxH: number };
  onChangeItem(id: string, patch: Partial<WidgetRect>, opts?: { persist?: boolean }): void;
  onRemove(id: string): void;
  onAdd(id: string): void;
  onReset(): void;
  onToggleEdit(): void;
  childrenMap: Record<string, ReactNode>;
}) {
  const maxRow = items.reduce((m, i) => (i.visible ? Math.max(m, i.y + i.h) : m), 4);
  const boardMinHeight = maxRow * (WIDGET_ROW_PX + WIDGET_GAP_PX) + 24;

  return (
    <div className={`widget-board-wrap ${editMode ? "is-edit" : ""}`} data-page={pageId}>
      <div className="widget-board-toolbar glass-panel">
        <button
          type="button"
          className={`button glass-btn ${editMode ? "primary" : ""}`}
          onClick={onToggleEdit}
        >
          <LayoutGrid size={15} />
          {editMode ? "Done customizing" : "Customize layout"}
        </button>
        {editMode ? (
          <>
            <button type="button" className="button glass-btn" onClick={onReset}>
              <RotateCcw size={14} /> Reset layout
            </button>
            {hiddenCatalog.length ? (
              <div className="widget-add-menu">
                <span className="widget-add-label">
                  <Plus size={14} /> Add widget
                </span>
                <div className="widget-add-list">
                  {hiddenCatalog.map((c) => (
                    <button key={c.id} type="button" className="button glass-btn" onClick={() => onAdd(c.id)}>
                      {c.label}
                    </button>
                  ))}
                </div>
              </div>
            ) : (
              <span className="muted widget-toolbar-hint">All widgets are on the board</span>
            )}
            <span className="muted widget-toolbar-hint">
              Drag title bar · resize edges / corner · remove with ×
            </span>
          </>
        ) : null}
      </div>

      <div
        className="widget-board"
        style={{
          gridTemplateColumns: `repeat(${WIDGET_COLS}, minmax(0, 1fr))`,
          gridAutoRows: `${WIDGET_ROW_PX}px`,
          gap: WIDGET_GAP_PX,
          minHeight: boardMinHeight
        }}
      >
        {items
          .filter((i) => i.visible && childrenMap[i.id] != null)
          .map((item) => {
            const meta = catalogById.get(item.id);
            const c = getConstraints(item.id);
            return (
              <WidgetShell
                key={item.id}
                item={item}
                editMode={editMode}
                title={meta?.label ?? item.id}
                minW={c.minW}
                minH={c.minH}
                maxW={c.maxW}
                maxH={c.maxH}
                onChange={(patch, opts) => onChangeItem(item.id, patch, opts)}
                onRemove={() => onRemove(item.id)}
              >
                {childrenMap[item.id]}
              </WidgetShell>
            );
          })}
      </div>
    </div>
  );
}
