import {
  Activity,
  BarChart3,
  Blocks,
  Box,
  Coins,
  Compass,
  Cpu,
  Gauge,
  Layers,
  ListTree,
  MessageSquare,
  Network,
  ScrollText,
  Send,
  Settings,
  WalletCards
} from "lucide-react";
import type { PageId } from "../../model";
import { AlvenqisLogo } from "../brand/AlvenqisLogo";

type NavItem = { id: PageId; label: string; icon: typeof Activity; badge?: number };

const groups: Array<{ label: string; items: NavItem[] }> = [
  {
    label: "Command",
    items: [
      { id: "overview", label: "Overview", icon: Gauge },
      { id: "analytics", label: "Analytics", icon: BarChart3 },
      { id: "messages", label: "Messages", icon: MessageSquare }
    ]
  },
  {
    label: "Portfolio",
    items: [
      { id: "wallet", label: "Wallet", icon: WalletCards },
      { id: "send", label: "Send & Receive", icon: Send },
      { id: "rewards", label: "Rewards", icon: Coins },
      { id: "assets", label: "Assets", icon: Box }
    ]
  },
  {
    label: "Network",
    items: [
      { id: "mining", label: "Miner", icon: Cpu },
      { id: "pool", label: "Pool", icon: Layers },
      { id: "explorer", label: "Explorer", icon: Compass },
      { id: "blocks", label: "Blocks", icon: Blocks },
      { id: "transactions", label: "Transactions", icon: Activity },
      { id: "mempool", label: "Mempool", icon: ListTree },
      { id: "node", label: "Network", icon: Network }
    ]
  },
  {
    label: "System",
    items: [
      { id: "activity", label: "Activity", icon: ScrollText },
      { id: "settings", label: "Settings", icon: Settings }
    ]
  }
];

export function Sidebar({
  page,
  setPage,
  height,
  online,
  unreadMessages = 0,
  peers = 0,
  mempool = 0
}: {
  page: PageId;
  setPage(page: PageId): void;
  height?: number | null;
  online?: boolean;
  unreadMessages?: number;
  peers?: number;
  mempool?: number;
}) {
  const live = Boolean(online);

  return (
    <aside className="sidebar" data-v2="sidebar">
      <div className="brand">
        <div className={`brand-mark ${live ? "is-online" : "is-offline"}`}>
          <AlvenqisLogo size="lg" alt="Alvenqis Network" />
        </div>
        <div className="brand-name">ALVENQIS</div>
        <div className="brand-subtitle">Control Center</div>
        <span className="brand-version">V2</span>
      </div>

      <nav className="nav" aria-label="Primary">
        {groups.map((group) => (
          <div key={group.label} className="nav-group">
            <div className="nav-group-label">{group.label}</div>
            {group.items.map(({ id, label, icon: Icon }) => {
              const badge = id === "messages" && unreadMessages > 0 ? unreadMessages : undefined;
              return (
                <button
                  key={id}
                  type="button"
                  className={`nav-button ${page === id ? "active" : ""}`}
                  onClick={() => setPage(id)}
                  aria-current={page === id ? "page" : undefined}
                >
                  <Icon size={16} strokeWidth={1.75} />
                  <span>{label}</span>
                  {badge != null ? <span className="nav-badge">{badge > 9 ? "9+" : badge}</span> : null}
                </button>
              );
            })}
          </div>
        ))}
      </nav>

      <div className="panel sidebar-status">
        <div className="eyebrow">Mainnet Candidate</div>
        <p className={live ? "positive" : "muted"}>
          {live ? "Gateway live" : "Gateway offline"}
        </p>
        <div className={`status-live ${live ? "" : "offline"}`}>
          <i />
          {height != null ? `Height ${height}` : "Waiting for tip"} · VPS RPC
        </div>
        <div className="status-metrics">
          <div>
            <small>Peers</small>
            <b>{peers}</b>
          </div>
          <div>
            <small>Mempool</small>
            <b>{mempool}</b>
          </div>
        </div>
      </div>
    </aside>
  );
}
