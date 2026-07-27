import { createContext, useContext } from "react";
import type { MinerStartOptions, NetworkSnapshot, OperatorCommand, WalletMetadata } from "@shared/types";
import type { PageId } from "./shared/pageMeta";

export type { PageId };

export interface AppModel {
  page: PageId;
  setPage(page: PageId): void;
  snapshot: NetworkSnapshot;
  wallet: WalletMetadata | null;
  wallets: WalletMetadata[];
  busy: boolean;
  notice: { error: boolean; text: string } | null;
  refresh(): Promise<void>;
  operator(command: OperatorCommand, minerOptions?: MinerStartOptions): Promise<string>;
  setNotice(notice: { error: boolean; text: string } | null): void;
  reloadWallet(): Promise<void>;
  selectWallet(walletId: string): Promise<void>;
}

export const AppContext = createContext<AppModel | null>(null);
export function useApp(): AppModel {
  const value = useContext(AppContext);
  if (!value) throw new Error("AppContext is unavailable");
  return value;
}
