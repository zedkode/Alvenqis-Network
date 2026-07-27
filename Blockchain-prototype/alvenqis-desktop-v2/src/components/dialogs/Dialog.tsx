import { useEffect, useId, useRef, type PropsWithChildren, type ReactNode } from "react";
import { X } from "lucide-react";

export interface DialogProps extends PropsWithChildren {
  open: boolean;
  title: string;
  subtitle?: ReactNode;
  /** Critical dialogs cannot be dismissed via Esc or backdrop click. */
  critical?: boolean;
  onClose(): void;
  footer?: ReactNode;
  wide?: boolean;
  className?: string;
}

export function Dialog({
  open,
  title,
  subtitle,
  critical = false,
  onClose,
  footer,
  wide = false,
  className = "",
  children
}: DialogProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;
    previousFocus.current = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    const focusable = panel?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
    );
    focusable?.[0]?.focus();

    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !critical) {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "Tab" || !panel || !focusable?.length) return;
      const list = Array.from(focusable);
      const first = list[0]!;
      const last = list[list.length - 1]!;
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      previousFocus.current?.focus?.();
    };
  }, [open, critical, onClose]);

  if (!open) return null;

  return (
    <div
      className="v-dialog-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !critical) onClose();
      }}
    >
      <section
        ref={panelRef}
        className={`v-dialog ${wide ? "is-wide" : ""} ${critical ? "is-critical" : ""} ${className}`.trim()}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <header className="v-dialog-header">
          <div>
            <h2 id={titleId}>{title}</h2>
            {subtitle ? <p className="v-dialog-subtitle">{subtitle}</p> : null}
          </div>
          {!critical ? (
            <button type="button" className="icon-button" aria-label="Close dialog" onClick={onClose}>
              <X size={18} />
            </button>
          ) : null}
        </header>
        <div className="v-dialog-body">{children}</div>
        {footer ? <footer className="v-dialog-footer">{footer}</footer> : null}
      </section>
    </div>
  );
}
