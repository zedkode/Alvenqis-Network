import { useMemo, useState } from "react";
import {
  AlertTriangle,
  Bell,
  CheckCheck,
  Inbox,
  MessageSquare,
  Shield,
  Trash2,
  Zap
} from "lucide-react";
import { useNotifications } from "../shared/notifications";
import { EmptyState } from "../components/ui/EmptyState";
import { PageToolbar } from "../components/ui/PageHero";
import { Panel } from "../components/ui/Panel";

type FilterId = "all" | "unread" | "mining" | "system" | "error" | "security";

const filters: Array<{ id: FilterId; label: string; icon: typeof Inbox }> = [
  { id: "all", label: "All messages", icon: Inbox },
  { id: "unread", label: "Unread", icon: Bell },
  { id: "mining", label: "Mining", icon: Zap },
  { id: "system", label: "System", icon: MessageSquare },
  { id: "error", label: "Errors", icon: AlertTriangle },
  { id: "security", label: "Security", icon: Shield }
];

export function Messages() {
  const { items, markRead, markAllRead, clearCenter, notify } = useNotifications();
  const [filter, setFilter] = useState<FilterId>("all");

  const filtered = useMemo(() => {
    return items.filter((item) => {
      if (filter === "all") return true;
      if (filter === "unread") return !item.read;
      if (filter === "mining") return item.kind === "mining";
      if (filter === "system") return item.kind === "system" || item.kind === "info";
      if (filter === "error") return item.kind === "error" || item.kind === "warning";
      if (filter === "security") {
        return (
          item.kind === "system" ||
          item.title.toLowerCase().includes("wallet") ||
          item.title.toLowerCase().includes("security") ||
          item.title.toLowerCase().includes("seed")
        );
      }
      return true;
    });
  }, [filter, items]);

  const unread = items.filter((i) => !i.read).length;

  return (
    <div className="page">
      <PageToolbar
        title="Messages"
        subtitle="Unified inbox for system, mining, and security notices. High-severity events still surface as toasts."
        actions={
          <>
            <button className="button ghost" type="button" onClick={() => markAllRead()} disabled={!unread}>
              <CheckCheck size={14} /> Mark all read
            </button>
            <button
              className="button ghost"
              type="button"
              onClick={() => clearCenter()}
              disabled={items.length === 0}
            >
              <Trash2 size={14} /> Clear
            </button>
            <button
              className="button primary"
              type="button"
              onClick={() =>
                notify({
                  kind: "system",
                  title: "System check",
                  body: "Message center is healthy. Telemetry and alerts will land here.",
                  severity: "center"
                })
              }
            >
              <Bell size={14} /> Test notice
            </button>
          </>
        }
      />
      <div className="messages-page">
      <aside className="messages-rail">
        <Panel title="Filters" detail={`${unread} unread`}>
          <div>
          {filters.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              className={filter === id ? "active" : ""}
              onClick={() => setFilter(id)}
            >
              <Icon size={14} strokeWidth={1.75} style={{ marginRight: 8, verticalAlign: "middle" }} />
              {label}
              {id === "unread" && unread > 0 ? ` (${unread})` : ""}
            </button>
          ))}
          </div>
        </Panel>
      </aside>

      <section className="messages-list">
        <div className="v2-section-title">
          <h3>
            {filter === "all" ? "All messages" : filters.find((f) => f.id === filter)?.label}
          </h3>
          <span className="muted">{filtered.length} items</span>
        </div>

        {filtered.length === 0 ? (
          <EmptyState status="Not available">
            No messages in this view. Mining events, wallet security notices, and gateway alerts
            appear here as the stack runs.
          </EmptyState>
        ) : (
          filtered.map((item) => (
            <article
              key={item.id}
              className={`message-card ${item.read ? "" : "unread"}`}
              onClick={() => markRead(item.id)}
            >
              <header>
                <div>
                  <div className="kind">
                    {item.kind}
                    {item.source ? ` · ${item.source}` : ""}
                  </div>
                  <strong>{item.title}</strong>
                </div>
                <time className="muted" dateTime={new Date(item.ts).toISOString()}>
                  {new Date(item.ts).toLocaleString()}
                </time>
              </header>
              <p style={{ margin: 0 }}>{item.body}</p>
            </article>
          ))
        )}
      </section>
      </div>
    </div>
  );
}
