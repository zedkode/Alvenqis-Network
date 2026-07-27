import {
  createContext,
  createElement,
  useCallback,
  useContext,
  useMemo,
  useReducer,
  type ReactNode
} from "react";

export type NoticeKind = "info" | "success" | "warning" | "error" | "mining" | "system";
export type NoticeSeverity = "toast" | "native" | "both" | "center";

export interface AppNotification {
  id: string;
  kind: NoticeKind;
  title: string;
  body: string;
  ts: number;
  /** When true, toast stays until dismissed. */
  sticky?: boolean;
  /** Toast auto-dismiss ms; ignored if sticky. */
  ttlMs?: number;
  read?: boolean;
  source?: string;
}

type State = {
  items: AppNotification[];
  toasts: AppNotification[];
  centerOpen: boolean;
};

type Action =
  | { type: "push"; item: AppNotification; asToast: boolean }
  | { type: "dismiss-toast"; id: string }
  | { type: "mark-read"; id: string }
  | { type: "mark-all-read" }
  | { type: "clear-center" }
  | { type: "set-center"; open: boolean };

function uid(): string {
  return `n_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "push": {
      const items = [action.item, ...state.items].slice(0, 200);
      const toasts = action.asToast
        ? [action.item, ...state.toasts].slice(0, 5)
        : state.toasts;
      return { ...state, items, toasts };
    }
    case "dismiss-toast":
      return { ...state, toasts: state.toasts.filter((t) => t.id !== action.id) };
    case "mark-read":
      return {
        ...state,
        items: state.items.map((i) => (i.id === action.id ? { ...i, read: true } : i))
      };
    case "mark-all-read":
      return { ...state, items: state.items.map((i) => ({ ...i, read: true })) };
    case "clear-center":
      return { ...state, items: [] };
    case "set-center":
      return { ...state, centerOpen: action.open };
    default:
      return state;
  }
}

export type NotifyInput = {
  kind?: NoticeKind;
  title: string;
  body: string;
  /** Default: toast for info/success, both for error/warning/mining. */
  severity?: NoticeSeverity;
  sticky?: boolean;
  ttlMs?: number;
  source?: string;
};

export interface NotificationsApi {
  items: AppNotification[];
  toasts: AppNotification[];
  centerOpen: boolean;
  unreadCount: number;
  notify(input: NotifyInput): string;
  dismissToast(id: string): void;
  markRead(id: string): void;
  markAllRead(): void;
  clearCenter(): void;
  setCenterOpen(open: boolean): void;
}

const NotificationsContext = createContext<NotificationsApi | null>(null);

function defaultSeverity(kind: NoticeKind): NoticeSeverity {
  if (kind === "error" || kind === "mining" || kind === "warning") return "both";
  if (kind === "system") return "center";
  return "toast";
}

export function NotificationsProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, {
    items: [],
    toasts: [],
    centerOpen: false
  });

  const dismissToast = useCallback((id: string) => {
    dispatch({ type: "dismiss-toast", id });
  }, []);

  const notify = useCallback(
    (input: NotifyInput): string => {
      const kind = input.kind ?? "info";
      const severity = input.severity ?? defaultSeverity(kind);
      const sticky = input.sticky ?? kind === "error";
      const ttlMs =
        input.ttlMs ?? (kind === "error" ? undefined : kind === "success" ? 5000 : 5000);
      const id = uid();
      const item: AppNotification = {
        id,
        kind,
        title: input.title,
        body: input.body,
        ts: Date.now(),
        sticky,
        ttlMs,
        read: false,
        source: input.source
      };
      const asToast = severity === "toast" || severity === "both";
      dispatch({ type: "push", item, asToast });

      if (!sticky && ttlMs && asToast) {
        window.setTimeout(() => dispatch({ type: "dismiss-toast", id }), ttlMs);
      }
      return id;
    },
    []
  );

  const api = useMemo<NotificationsApi>(
    () => ({
      items: state.items,
      toasts: state.toasts,
      centerOpen: state.centerOpen,
      unreadCount: state.items.filter((i) => !i.read).length,
      notify,
      dismissToast,
      markRead: (id) => dispatch({ type: "mark-read", id }),
      markAllRead: () => dispatch({ type: "mark-all-read" }),
      clearCenter: () => dispatch({ type: "clear-center" }),
      setCenterOpen: (open) => dispatch({ type: "set-center", open })
    }),
    [state, notify, dismissToast]
  );

  return createElement(NotificationsContext.Provider, { value: api }, children);
}

export function useNotifications(): NotificationsApi {
  const ctx = useContext(NotificationsContext);
  if (!ctx) throw new Error("useNotifications requires NotificationsProvider");
  return ctx;
}

/** Safe variant for components that may render outside provider during tests. */
export function useNotificationsOptional(): NotificationsApi | null {
  return useContext(NotificationsContext);
}
