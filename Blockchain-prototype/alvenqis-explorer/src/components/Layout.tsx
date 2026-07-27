import { useEffect, useState } from "react";
import { Link, NavLink } from "react-router-dom";
import { fetchJson, NetworkResponse, RPC_BASE_URL } from "../lib/api";
import { StatusBadge } from "./StatusBadge";
import { ExplorerSearch } from "./ExplorerSearch";

interface LayoutProps {
  children: React.ReactNode;
}

export function Layout({ children }: LayoutProps) {
  const [network, setNetwork] = useState<NetworkResponse | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const websiteUrl = (import.meta.env.VITE_ALVENQIS_WEBSITE_URL ?? "https://dohotstudio.com").replace(/\/+$/, "");
  const navItems = [
    ["OV", "Dashboard", "/dashboard"],
    ["BL", "Blocks", "/blocks"],
    ["TX", "Transactions", "/transactions"],
    ["AD", "Addresses", "/addresses"],
    ["SP", "Supply", "/supply"],
    ["MP", "Mempool", "/mempool"],
    ["NW", "Network", "/network"],
  ];

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
      <aside className={`sidebar ${menuOpen ? "open" : ""}`}>
        <div className="sidebar-frame">
          <Link className="brand-mark" to="/dashboard" onClick={() => setMenuOpen(false)}>
            <img className="brand-logo" src="/alvenqis-logo.png" alt="Alvenqis" />
            <div>
              <div className="brand-title">Alvenqis Explorer</div>
              <div className="brand-subtitle">
                {network
                  ? `${network.network_name} · ${network.status_label}`
                  : "Read-only chain observability"}
              </div>
            </div>
          </Link>

          <div className="sidebar-section-label">Navigation</div>
          <nav className="nav-list">
            {navItems.map(([code, label, path]) => (
              <NavLink className="nav-link" to={path} key={path} onClick={() => setMenuOpen(false)}>
                <span className="nav-code">{code}</span>
                <span>{label}</span>
              </NavLink>
            ))}
          </nav>

          <div className="sidebar-note">
            <div className="sidebar-note-title">
              <span className={`live-dot ${network ? "online" : ""}`} />
              RPC connection
            </div>
            <div className="badge-grid">
              <StatusBadge label="Read Only" />
              <StatusBadge label={network?.status_label ?? "Unavailable"} tone={network ? undefined : "warn"} />
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
              Reads the configured Alvenqis RPC and indexer. It never requests keys,
              connects wallets or submits transactions.
            </p>
            <a className="sidebar-public-link" href={websiteUrl}>
              Public website <span aria-hidden="true">↗</span>
            </a>
          </div>
        </div>
      </aside>

      <main className="main-shell">
        <header className="mobile-topbar">
          <Link className="mobile-brand" to="/dashboard">
            <img src="/alvenqis-logo.png" alt="" />
            <span>Alvenqis Explorer</span>
          </Link>
          <button type="button" aria-label="Toggle navigation" onClick={() => setMenuOpen((open) => !open)}>
            {menuOpen ? "Close" : "Menu"}
          </button>
        </header>
        <div className="content-frame">
          <ExplorerSearch />
          {children}
          <footer className="explorer-footer">
            <span>Alvenqis Explorer · Read-only Mainnet Candidate data</span>
            <span className="hash-text">{RPC_BASE_URL}</span>
          </footer>
        </div>
      </main>
      {menuOpen ? <button className="sidebar-backdrop" type="button" aria-label="Close navigation" onClick={() => setMenuOpen(false)} /> : null}
    </div>
  );
}
