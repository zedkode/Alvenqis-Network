import { useEffect, useMemo, useState } from "react";
import {
  Compass, Cpu, Gauge, Network, RefreshCw, Send, Settings, Sun, WalletCards, Coins, Box,
  Blocks, Activity, ListTree, ScrollText, Layers, BarChart3, MessageSquare
} from "lucide-react";
import type { LanguageId } from "@shared/types";
import { commandLabels } from "../shared/i18n";
import { PAGE_LABELS, PAGE_LABELS_RO, PAGE_ORDER, type PageId } from "../shared/pageMeta";
import { Dialog } from "./dialogs/Dialog";

const pageIcons: Record<PageId, typeof Gauge> = {
  overview: Gauge,
  analytics: BarChart3,
  wallet: WalletCards,
  send: Send,
  rewards: Coins,
  assets: Box,
  mining: Cpu,
  pool: Layers,
  explorer: Compass,
  blocks: Blocks,
  transactions: Activity,
  mempool: ListTree,
  node: Network,
  activity: ScrollText,
  messages: MessageSquare,
  settings: Settings
};

type CommandItem = {
  id: string;
  group: "pages" | "actions";
  label: string;
  hint?: string;
  icon: typeof Gauge;
  run(): void;
};

export function CommandPalette({
  open,
  language = "en",
  onClose,
  onNavigate,
  onRefresh,
  onToggleTheme,
  onOpenWalletSwitcher
}: {
  open: boolean;
  language?: LanguageId;
  onClose(): void;
  onNavigate(page: PageId): void;
  onRefresh(): void;
  onToggleTheme(): void;
  onOpenWalletSwitcher(): void;
}) {
  const t = commandLabels[language === "ro" ? "ro" : "en"];
  const labels = language === "ro" ? PAGE_LABELS_RO : PAGE_LABELS;
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActive(0);
    }
  }, [open]);

  const commands = useMemo<CommandItem[]>(() => {
    const pages: CommandItem[] = PAGE_ORDER.map((id) => ({
      id: `page:${id}`,
      group: "pages",
      label: labels[id],
      hint: id,
      icon: pageIcons[id],
      run: () => {
        onNavigate(id);
        onClose();
      }
    }));
    const actions: CommandItem[] = [
      {
        id: "action:refresh",
        group: "actions",
        label: t.refresh,
        icon: RefreshCw,
        run: () => {
          onRefresh();
          onClose();
        }
      },
      {
        id: "action:theme",
        group: "actions",
        label: t.toggleTheme,
        icon: Sun,
        run: () => {
          onToggleTheme();
          onClose();
        }
      },
      {
        id: "action:wallet",
        group: "actions",
        label: t.openWallet,
        icon: WalletCards,
        run: () => {
          onOpenWalletSwitcher();
          onClose();
        }
      },
      {
        id: "action:mining",
        group: "actions",
        label: t.startMining,
        icon: Cpu,
        run: () => {
          onNavigate("mining");
          onClose();
        }
      },
      {
        id: "action:send",
        group: "actions",
        label: t.send,
        icon: Send,
        run: () => {
          onNavigate("send");
          onClose();
        }
      }
    ];
    return [...pages, ...actions];
  }, [labels, t, onNavigate, onClose, onRefresh, onToggleTheme, onOpenWalletSwitcher]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands;
    return commands.filter(
      (c) => c.label.toLowerCase().includes(q) || c.id.toLowerCase().includes(q) || (c.hint ?? "").includes(q)
    );
  }, [commands, query]);

  useEffect(() => {
    setActive(0);
  }, [query]);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setActive((i) => Math.min(filtered.length - 1, i + 1));
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        setActive((i) => Math.max(0, i - 1));
      } else if (event.key === "Enter") {
        event.preventDefault();
        filtered[active]?.run();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, filtered, active]);

  return (
    <Dialog
      open={open}
      title="Command palette"
      subtitle="Ctrl+K · navigate without leaving the keyboard"
      onClose={onClose}
      wide
      className="command-palette-dialog"
    >
      <input
        className="command-palette-input"
        autoFocus
        value={query}
        placeholder={t.palettePlaceholder}
        onChange={(e) => setQuery(e.target.value)}
      />
      <div className="command-palette-list" role="listbox">
        {filtered.length === 0 ? (
          <p className="muted command-palette-empty">{t.noResults}</p>
        ) : (
          filtered.map((item, index) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                type="button"
                role="option"
                aria-selected={index === active}
                className={`command-palette-item ${index === active ? "is-active" : ""}`}
                onMouseEnter={() => setActive(index)}
                onClick={() => item.run()}
              >
                <Icon size={16} />
                <span>
                  <b>{item.label}</b>
                  <small>{item.group === "pages" ? t.pages : t.actions}</small>
                </span>
              </button>
            );
          })
        )}
      </div>
    </Dialog>
  );
}
