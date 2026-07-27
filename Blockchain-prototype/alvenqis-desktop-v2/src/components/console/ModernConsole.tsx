import { useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from "react";
import {
  ArrowDownToLine,
  Copy,
  Filter,
  Play,
  Search,
  TerminalSquare,
  Trash2
} from "lucide-react";

export type ConsoleLineKind = "info" | "ok" | "warn" | "error" | "cmd" | "status" | "raw";

export interface ConsoleLine {
  id: string;
  kind: ConsoleLineKind;
  text: string;
  ts?: number;
}

function classifyLine(text: string): ConsoleLineKind {
  const t = text.toLowerCase();
  if (t.startsWith("$ ") || t.startsWith("> ")) return "cmd";
  if (t.includes("[status]")) return "status";
  if (t.includes("error") || t.includes("failed") || t.includes("panic") || t.includes("fatal")) {
    return "error";
  }
  if (t.includes("warn") || t.includes("stale") || t.includes("timeout")) return "warn";
  if (t.includes("accepted") || t.includes("started") || t.includes("ready") || t.includes("ok")) {
    return "ok";
  }
  return "info";
}

export function textToConsoleLines(text: string, baseId = "ln"): ConsoleLine[] {
  return text
    .split("\n")
    .filter((l) => l.trim().length > 0)
    .map((line, i) => ({
      id: `${baseId}-${i}-${line.length}`,
      kind: classifyLine(line),
      text: line,
      ts: Date.now()
    }));
}

export function ModernConsole({
  title = "Console",
  lines,
  rawText,
  onClear,
  onCopy,
  onExport,
  onRunCommand,
  commandHistory = [],
  presets = [],
  placeholder = "status | devices | config validate | benchmark --seconds 3",
  busy = false,
  autoScroll = true,
  onAutoScrollChange,
  footer
}: {
  title?: string;
  lines?: ConsoleLine[];
  /** Convenience: pass raw multiline text instead of pre-parsed lines. */
  rawText?: string;
  onClear?(): void;
  onCopy?(): void;
  onExport?(): void;
  onRunCommand?(line: string): void | Promise<void>;
  commandHistory?: string[];
  presets?: string[];
  placeholder?: string;
  busy?: boolean;
  autoScroll?: boolean;
  onAutoScrollChange?(next: boolean): void;
  footer?: ReactNode;
}) {
  const bodyRef = useRef<HTMLDivElement>(null);
  const [filter, setFilter] = useState<"all" | ConsoleLineKind>("all");
  const [query, setQuery] = useState("");
  const [cmd, setCmd] = useState("");
  const [histIdx, setHistIdx] = useState(-1);

  const resolved = useMemo(() => {
    if (lines && lines.length) return lines;
    if (rawText) return textToConsoleLines(rawText);
    return [];
  }, [lines, rawText]);

  const visible = useMemo(() => {
    return resolved.filter((l) => {
      if (filter !== "all" && l.kind !== filter) return false;
      if (query && !l.text.toLowerCase().includes(query.toLowerCase())) return false;
      return true;
    });
  }, [resolved, filter, query]);

  useEffect(() => {
    if (!autoScroll || !bodyRef.current) return;
    bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [visible, autoScroll]);

  const submit = async (event?: FormEvent) => {
    event?.preventDefault();
    const line = cmd.trim();
    if (!line || !onRunCommand || busy) return;
    setCmd("");
    setHistIdx(-1);
    await onRunCommand(line);
  };

  return (
    <section className="modern-console panel">
      <header className="modern-console-toolbar">
        <div className="modern-console-title">
          <TerminalSquare size={15} />
          <strong>{title}</strong>
          <span className="muted">{visible.length} lines</span>
        </div>
        <div className="modern-console-actions">
          <label className="console-search">
            <Search size={13} />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Filter text…"
              aria-label="Filter console"
            />
          </label>
          <label className="console-filter">
            <Filter size={13} />
            <select
              value={filter}
              onChange={(e) => setFilter(e.target.value as typeof filter)}
              aria-label="Severity filter"
            >
              <option value="all">All</option>
              <option value="cmd">Commands</option>
              <option value="status">Status</option>
              <option value="ok">OK</option>
              <option value="warn">Warn</option>
              <option value="error">Error</option>
              <option value="info">Info</option>
            </select>
          </label>
          <label className="console-autoscroll">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => onAutoScrollChange?.(e.target.checked)}
            />
            Auto-scroll
          </label>
          {onCopy ? (
            <button type="button" className="button" onClick={onCopy} title="Copy">
              <Copy size={14} />
            </button>
          ) : null}
          {onExport ? (
            <button type="button" className="button" onClick={onExport} title="Export">
              <ArrowDownToLine size={14} />
            </button>
          ) : null}
          {onClear ? (
            <button type="button" className="button" onClick={onClear} title="Clear">
              <Trash2 size={14} />
            </button>
          ) : null}
        </div>
      </header>

      <div className="modern-console-body" ref={bodyRef} role="log" aria-live="polite">
        {visible.length === 0 ? (
          <div className="modern-console-empty muted">Console idle — start miner or run a command.</div>
        ) : (
          visible.map((line) => (
            <div key={line.id} className={`console-line kind-${line.kind}`}>
              <span className="console-gutter">{line.kind}</span>
              <code>{line.text}</code>
            </div>
          ))
        )}
      </div>

      {onRunCommand ? (
        <form className="modern-console-input" onSubmit={(e) => void submit(e)}>
          <span className="prompt">$</span>
          <input
            value={cmd}
            disabled={busy}
            placeholder={placeholder}
            onChange={(e) => setCmd(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "ArrowUp" && commandHistory.length) {
                e.preventDefault();
                const next = Math.min(commandHistory.length - 1, histIdx + 1);
                setHistIdx(next);
                setCmd(commandHistory[commandHistory.length - 1 - next] ?? "");
              }
              if (e.key === "ArrowDown" && commandHistory.length) {
                e.preventDefault();
                const next = Math.max(-1, histIdx - 1);
                setHistIdx(next);
                setCmd(next < 0 ? "" : commandHistory[commandHistory.length - 1 - next] ?? "");
              }
            }}
            aria-label="Miner command"
            autoComplete="off"
            spellCheck={false}
          />
          <button className="button primary" type="submit" disabled={busy || !cmd.trim()}>
            <Play size={14} /> Run
          </button>
        </form>
      ) : null}

      {presets.length > 0 ? (
        <div className="modern-console-presets">
          {presets.map((p) => (
            <button
              key={p}
              type="button"
              className="chip"
              disabled={busy}
              onClick={() => void onRunCommand?.(p)}
            >
              {p}
            </button>
          ))}
        </div>
      ) : null}

      {footer ? <footer className="modern-console-footer">{footer}</footer> : null}
    </section>
  );
}
