import { Navigate, Route, Routes } from "react-router-dom";
import { Layout } from "./components/Layout";
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

export default function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/" element={<Navigate to="/dashboard" replace />} />
        <Route path="/dashboard" element={<DashboardPage />} />
        <Route path="/blocks" element={<LatestBlocksPage />} />
        <Route path="/blocks/:height" element={<BlockDetailsPage />} />
        <Route path="/tx/:hash" element={<TransactionDetailsPage />} />
        <Route path="/transactions" element={<TransactionsPage />} />
        <Route path="/addresses" element={<AddressesPage />} />
        <Route path="/supply" element={<SupplyPage />} />
        <Route path="/mempool" element={<MempoolPage />} />
        <Route path="/search/:query" element={<SearchPage />} />
        <Route path="/address/:address" element={<AddressDetailsPage />} />
        <Route path="/network" element={<NetworkStatusPage />} />
        <Route path="*" element={<NotFoundPage />} />
      </Routes>
    </Layout>
  );
}
