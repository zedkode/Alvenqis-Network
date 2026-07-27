import { ReactNode, useEffect } from "react";
import { Layout } from "./components/Layout";
import { RouteParamsProvider, useNavigate, usePathname } from "./lib/router";
import { AddressDetailsPage } from "./pages/AddressDetailsPage";
import { BlockDetailsPage } from "./pages/BlockDetailsPage";
import { DashboardPage } from "./pages/DashboardPage";
import { LatestBlocksPage } from "./pages/LatestBlocksPage";
import { NetworkStatusPage } from "./pages/NetworkStatusPage";
import { NotFoundPage } from "./pages/NotFoundPage";
import { TransactionDetailsPage } from "./pages/TransactionDetailsPage";
import { TransactionsPage } from "./pages/TransactionsPage";
import { AddressesPage } from "./pages/AddressesPage";
import { SupplyPage } from "./pages/SupplyPage";
import { MempoolPage } from "./pages/MempoolPage";
import { SearchPage } from "./pages/SearchPage";

function decodeRoutePart(value: string) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

export default function App() {
  const pathname = usePathname();
  const navigate = useNavigate();

  useEffect(() => {
    if (pathname === "/") {
      navigate("/dashboard", { replace: true });
    }
  }, [navigate, pathname]);

  let page: ReactNode = <NotFoundPage />;
  let params: Record<string, string | undefined> = {};

  if (pathname === "/" || pathname === "/dashboard") page = <DashboardPage />;
  else if (pathname === "/blocks") page = <LatestBlocksPage />;
  else if (pathname === "/transactions") page = <TransactionsPage />;
  else if (pathname === "/addresses") page = <AddressesPage />;
  else if (pathname === "/supply") page = <SupplyPage />;
  else if (pathname === "/mempool") page = <MempoolPage />;
  else if (pathname === "/network") page = <NetworkStatusPage />;
  else if (pathname.startsWith("/blocks/")) {
    params = { height: decodeRoutePart(pathname.slice("/blocks/".length)) };
    page = <BlockDetailsPage />;
  } else if (pathname.startsWith("/tx/")) {
    params = { hash: decodeRoutePart(pathname.slice("/tx/".length)) };
    page = <TransactionDetailsPage />;
  } else if (pathname.startsWith("/search/")) {
    params = { query: decodeRoutePart(pathname.slice("/search/".length)) };
    page = <SearchPage />;
  } else if (pathname.startsWith("/address/")) {
    params = { address: decodeRoutePart(pathname.slice("/address/".length)) };
    page = <AddressDetailsPage />;
  }

  return (
    <Layout>
      <RouteParamsProvider params={params}>{page}</RouteParamsProvider>
    </Layout>
  );
}
