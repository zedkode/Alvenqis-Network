import {
  useCallback,
  useRef,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
  type ReactNode
} from "react";
import { GripVertical, Maximize2, X } from "lucide-react";
import { WIDGET_COLS, WIDGET_GAP_PX, WIDGET_ROW_PX, type WidgetRect } from "./types";

type DragMode = "move" | "resize-e" | "resize-s" | "resize-se";

export function WidgetShell({
  item,
  editMode,
  title,
  minW,
  minH,
  maxW,
  maxH,
  onChange,
  onRemove,
  children,
  className = ""
}: {
  item: WidgetRect;
  editMode: boolean;
  title?: string;
  minW: number;
  minH: number;
  maxW: number;
  maxH: number;
  onChange(next: Partial<WidgetRect>, opts?: { persist?: boolean }): void;
  onRemove(): void;
  children: ReactNode;
  className?: string;
}) {
  const shellRef = useRef<HTMLElement>(null);
  const dragRef = useRef<{
    mode: DragMode;
    startX: number;
    startY: number;
    orig: WidgetRect;
    colW: number;
  } | null>(null);

  const fontScale = Math.min(
    1.35,
    Math.max(0.78, Math.sqrt((item.w * item.h) / 12))
  );

  const style: CSSProperties = {
    gridColumn: `${item.x + 1} / span ${item.w}`,
    gridRow: `${item.y + 1} / span ${item.h}`,
    ["--widget-scale" as string]: String(fontScale)
  };

  const beginDrag = useCallback(
    (mode: DragMode, event: ReactPointerEvent) => {
      if (!editMode) return;
      event.preventDefault();
      event.stopPropagation();
      const board = shellRef.current?.parentElement;
      if (!board) return;
      const rect = board.getBoundingClientRect();
      const colW = (rect.width - WIDGET_GAP_PX * (WIDGET_COLS - 1)) / WIDGET_COLS;
      dragRef.current = {
        mode,
        startX: event.clientX,
        startY: event.clientY,
        orig: { ...item },
        colW
      };
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    },
    [editMode, item]
  );

  const onPointerMove = useCallback(
    (event: ReactPointerEvent) => {
      const drag = dragRef.current;
      if (!drag) return;
      const dx = event.clientX - drag.startX;
      const dy = event.clientY - drag.startY;
      const dCol = Math.round(dx / (drag.colW + WIDGET_GAP_PX));
      const dRow = Math.round(dy / (WIDGET_ROW_PX + WIDGET_GAP_PX));

      if (drag.mode === "move") {
        const nx = Math.max(0, Math.min(WIDGET_COLS - drag.orig.w, drag.orig.x + dCol));
        const ny = Math.max(0, drag.orig.y + dRow);
        if (nx !== item.x || ny !== item.y) onChange({ x: nx, y: ny }, { persist: false });
        return;
      }

      let w = drag.orig.w;
      let h = drag.orig.h;
      if (drag.mode === "resize-e" || drag.mode === "resize-se") {
        w = Math.max(minW, Math.min(maxW, drag.orig.w + dCol));
        w = Math.min(w, WIDGET_COLS - drag.orig.x);
      }
      if (drag.mode === "resize-s" || drag.mode === "resize-se") {
        h = Math.max(minH, Math.min(maxH, drag.orig.h + dRow));
      }
      if (w !== item.w || h !== item.h) onChange({ w, h }, { persist: false });
    },
    [item.h, item.w, item.x, item.y, maxH, maxW, minH, minW, onChange]
  );

  const onPointerUp = useCallback(
    (event: ReactPointerEvent) => {
      if (!dragRef.current) return;
      dragRef.current = null;
      // Final write to storage
      onChange({}, { persist: true });
      try {
        (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
      } catch {
        /* already released */
      }
    },
    [onChange]
  );

  return (
    <article
      ref={shellRef}
      className={`widget-shell glass-panel ${editMode ? "is-edit" : ""} ${className}`.trim()}
      style={style}
      data-widget-id={item.id}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
    >
      {editMode ? (
        <header className="widget-shell-chrome" onPointerDown={(e) => beginDrag("move", e)}>
          <span className="widget-drag-handle" title="Drag">
            <GripVertical size={14} />
          </span>
          <span className="widget-shell-title">{title ?? item.id}</span>
          <button
            type="button"
            className="widget-chrome-btn"
            title="Remove widget"
            aria-label="Remove widget"
            onPointerDown={(e) => e.stopPropagation()}
            onClick={onRemove}
          >
            <X size={14} />
          </button>
        </header>
      ) : null}

      <div className="widget-shell-body">{children}</div>

      {editMode ? (
        <>
          <i
            className="widget-resize widget-resize-e"
            title="Resize width"
            onPointerDown={(e) => beginDrag("resize-e", e)}
          />
          <i
            className="widget-resize widget-resize-s"
            title="Resize height"
            onPointerDown={(e) => beginDrag("resize-s", e)}
          />
          <i
            className="widget-resize widget-resize-se"
            title="Resize"
            onPointerDown={(e) => beginDrag("resize-se", e)}
          >
            <Maximize2 size={10} />
          </i>
        </>
      ) : null}
    </article>
  );
}
