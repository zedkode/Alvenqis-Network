import {
  AlertTriangle,
  Bell,
  CheckCircle2,
  Coins,
  Info,
  Network,
  Trash2,
  X,
  XCircle
} from "lucide-react";
import type { AppNotification } from "../../shared/notifications";
import { useNotifications } from "../../shared/notifications";

function iconFor(kind: AppNotification["kind"]) {
  switch (kind) {
    case "success":
      return <CheckCircle2 size={16} />;
    case "error":
      return <XCircle size={16} />;
    case "warning":
      return <AlertTriangle size={16} />;
    case "mining":
      return <Coins size={16} />;
    case "system":
      return <Network size={16} />;
    default:
      return <Info size={16} />;
  }
}

function dayLabel(ts: number): string {
  const d = new Date(ts);
  const today = new Date();
  if (d.toDateString() === today.toDateString()) return "Today";
  const y = new Date(today);
  y.setDate(today.getDate() - 1);
  if (d.toDateString() === y.toDateString()) return "Yesterday";
  return d.toLocaleDateString();
}

function groupByDay(items: AppNotification[]): Array<[string, AppNotification[]]> {
  const map = new Map<string, AppNotification[]>();
  for (const item of items) {
    const key = dayLabel(item.ts);
    const list = map.get(key) ?? [];
    list.push(item);
    map.set(key, list);
  }
  return Array.from(map.entries());
}

export function NotificationBell() {
  const { unreadCount, centerOpen, setCenterOpen } = useNotifications();
  return (
    <button
      type="button"
      className={`button theme-toggle notification-bell ${centerOpen ? "is-open" : ""}`}
      title="Notifications"
      aria-label="Open notification center"
      onClick={() => setCenterOpen(!centerOpen)}
    >
      <Bell size={15} />
      {unreadCount > 0 ? <i className="notification-badge">{unreadCount > 9 ? "9+" : unreadCount}</i> : null}
    </button>
  );
}

export function NotificationCenterPanel() {
  const {
    items,
    centerOpen,
    setCenterOpen,
    markRead,
    markAllRead,
    clearCenter
  } = useNotifications();

  if (!centerOpen) return null;
  const groups = groupByDay(items);

  return (
    <>
      <div className="notification-scrim" onClick={() => setCenterOpen(false)} aria-hidden="true" />
      <aside className="notification-center" role="dialog" aria-label="Notification center">
        <header className="notification-center-head">
          <div>
            <strong>Notifications</strong>
            <span className="muted">{items.length} this session</span>
          </div>
          <div className="button-row">
            <button type="button" className="button" onClick={markAllRead} disabled={!items.some((i) => !i.read)}>
              Mark read
            </button>
            <button type="button" className="button" onClick={clearCenter} disabled={items.length === 0}>
              <Trash2 size={14} />
            </button>
            <button
              type="button"
              className="icon-button"
              aria-label="Close"
              onClick={() => setCenterOpen(false)}
            >
              <X size={18} />
            </button>
          </div>
        </header>
        <div className="notification-center-body">
          {groups.length === 0 ? (
            <p className="muted notification-empty">No notifications yet this session.</p>
          ) : (
            groups.map(([day, list]) => (
              <section key={day} className="notification-day">
                <h4>{day}</h4>
                {list.map((item) => (
                  <article
                    key={item.id}
                    className={`notification-item kind-${item.kind} ${item.read ? "is-read" : ""}`}
                    onClick={() => markRead(item.id)}
                  >
                    <span className="notification-item-icon">{iconFor(item.kind)}</span>
                    <div>
                      <b>{item.title}</b>
                      <p>{item.body}</p>
                      <time>
                        {new Date(item.ts).toLocaleTimeString([], {
                          hour: "2-digit",
                          minute: "2-digit"
                        })}
                      </time>
                    </div>
                  </article>
                ))}
              </section>
            ))
          )}
        </div>
      </aside>
    </>
  );
}
