import { useEffect, type PropsWithChildren, type ReactNode } from "react";
import { X } from "lucide-react";

interface DetailDialogProps extends PropsWithChildren {
  title: string;
  subtitle?: ReactNode;
  onClose(): void;
  /** Optional footer actions (a Close button is always shown). */
  footer?: ReactNode;
  /** Wider panel for dense chain data (still clamped to the viewport). */
  wide?: boolean;
}

/**
 * Modal detail panel for chain data.
 * Never goes true-fullscreen — size is always clamped so the close control stays reachable.
 */
export function DetailDialog({
  title,
  subtitle,
  onClose,
  children,
  footer,
  wide = false
}: DetailDialogProps) {
  useEffect(() => {
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div
      className="detail-dialog-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section
        className={`detail-dialog ${wide ? "is-wide" : ""}`.trim()}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <header className="detail-dialog-header">
          <div className="detail-dialog-heading">
            <h2>{title}</h2>
            {subtitle ? <p>{subtitle}</p> : null}
          </div>
          <div className="detail-dialog-actions">
            <button type="button" title="Close (Esc)" aria-label="Close" onClick={onClose}>
              <X size={18} />
            </button>
          </div>
        </header>
        <div className="detail-dialog-body">{children}</div>
        <footer className="detail-dialog-footer">
          {footer}
          <button className="button" type="button" onClick={onClose}>
            Close
          </button>
        </footer>
      </section>
    </div>
  );
}
