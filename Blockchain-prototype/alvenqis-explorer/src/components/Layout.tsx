import { useEffect, useState } from "react";
import { NavLink } from "react-router-dom";
import { fetchJson, NetworkResponse } from "../lib/api";
import { StatusBadge } from "./StatusBadge";
import { ExplorerSearch } from "./ExplorerSearch";

interface LayoutProps {
  children: React.ReactNode;
}

export function Layout({ children }: LayoutProps) {
  const [network, setNetwork] = useState<NetworkResponse | null>(null);

  useEffect(() => {
    let active = true;
    fetchJson<NetworkResponse>("/network")
      .then((response) => {
        if (active) {
          setNetwork(response);
        }
      })
      .catch(() => {
        if (active) {
          setNetwork(null);
        }
      });

    return () => {
      active = false;
    };
  }, []);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="sidebar-frame">
          <div className="brand-mark">
            <div className="brand-badge" />
            <div>
              <div className="brand-title">Alvenqis Explorer</div>
              <div className="brand-subtitle">
                {network
                  ? `${network.network_name} read-only local UI`
                  : "Local network read-only UI"}
              </div>
            </div>
          </div>

          <div className="sidebar-section-label">Navigation</div>
          <nav className="nav-list">
            <NavLink className="nav-link" to="/dashboard">
              Dashboard
            </NavLink>
            <NavLink className="nav-link" to="/blocks">
              Blocks
            </NavLink>
            <NavLink className="nav-link" to="/transactions">
              Transactions
            </NavLink>
            <NavLink className="nav-link" to="/addresses">
              Addresses
            </NavLink>
            <NavLink className="nav-link" to="/supply">
              Supply
            </NavLink>
            <NavLink className="nav-link" to="/mempool">
              Mempool
            </NavLink>
            <NavLink className="nav-link" to="/network">
              Network Status
            </NavLink>
          </nav>

          <div className="sidebar-note">
            <div className="sidebar-note-title">Environment</div>
            <div className="badge-grid">
              <StatusBadge label="Draft" tone="warn" />
              <StatusBadge label="Read Only" />
              <StatusBadge label="Prototype" />
              <StatusBadge label="Not Live Mainnet" tone="warn" />
            </div>
            <div className="sidebar-meta">
              <div className="sidebar-meta-row">
                <span className="sidebar-meta-label">Network</span>
                <span>{network?.network_name ?? "Pending"}</span>
              </div>
              <div className="sidebar-meta-row">
                <span className="sidebar-meta-label">Identifier</span>
                <span>{network?.network_id ?? "Unavailable"}</span>
              </div>
              <div className="sidebar-meta-row">
                <span className="sidebar-meta-label">Prefix</span>
                <span>{network?.address_prefix ?? "Pending"}</span>
              </div>
            </div>
            <p>
              This explorer reads local RPC and indexer data only. It does not send
              transactions, connect wallets or expose public infrastructure.
            </p>
          </div>
        </div>
      </aside>

      <main className="main-shell">
        <div className="content-frame">
          <ExplorerSearch />
          {children}
        </div>
      </main>
    </div>
  );
}
