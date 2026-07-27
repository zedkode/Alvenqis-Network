import type {
  AddressAccount,
  AddressBalance,
  ChainStatus,
  HealthResponse,
  NetworkLimits,
  PoolHistory,
  PoolStatus,
  SignedTransactionBody,
  SubmitTransactionResponse,
  AlvenqisClientOptions
} from "./types.js";
import { poolBlockMaturity } from "./maturity.js";

const DEFAULT_RPC = "https://rpcnode.dohotstudio.com";
const DEFAULT_POOL = "https://pool.dohotstudio.com";

/** Matches alvenqis-core FIRST_ACCOUNT_NONCE. */
export const ALVENQIS_FIRST_ACCOUNT_NONCE = 1;

function trimSlash(url: string): string {
  return url.trim().replace(/\/+$/, "");
}

export class AlvenqisError extends Error {
  readonly status?: number;
  readonly url?: string;

  constructor(message: string, opts?: { status?: number; url?: string; cause?: unknown }) {
    super(message, opts?.cause !== undefined ? { cause: opts.cause } : undefined);
    this.name = "AlvenqisError";
    this.status = opts?.status;
    this.url = opts?.url;
  }
}

/**
 * Alvenqis client for Mainnet Candidate gateways.
 * Read APIs + relay of pre-signed transactions. Does not hold keys or implement contracts.
 */
export class AlvenqisClient {
  readonly rpcUrl: string;
  readonly poolUrl: string;
  private readonly fetchImpl: typeof fetch;
  private readonly timeoutMs: number;

  constructor(options: AlvenqisClientOptions = {}) {
    this.rpcUrl = trimSlash(options.rpcUrl ?? DEFAULT_RPC);
    this.poolUrl = trimSlash(options.poolUrl ?? DEFAULT_POOL);
    this.fetchImpl = options.fetch ?? fetch.bind(globalThis);
    this.timeoutMs = options.timeoutMs ?? 12_000;
  }

  private async requestJson<T>(
    base: string,
    path: string,
    init?: { method?: string; body?: unknown }
  ): Promise<T> {
    const url = `${trimSlash(base)}${path.startsWith("/") ? path : `/${path}`}`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);
    try {
      const headers: Record<string, string> = { Accept: "application/json" };
      const method = init?.method ?? "GET";
      let body: string | undefined;
      if (init?.body !== undefined) {
        headers["Content-Type"] = "application/json";
        body = JSON.stringify(init.body);
      }
      const response = await this.fetchImpl(url, {
        method,
        headers,
        body,
        signal: controller.signal
      });
      if (!response.ok) {
        let detail = "";
        try {
          detail = (await response.text()).slice(0, 400);
        } catch {
          /* ignore */
        }
        throw new AlvenqisError(
          `HTTP ${response.status} for ${url}${detail ? `: ${detail}` : ""}`,
          {
            status: response.status,
            url
          }
        );
      }
      return (await response.json()) as T;
    } catch (error) {
      if (error instanceof AlvenqisError) throw error;
      throw new AlvenqisError(`Request failed for ${url}: ${String(error)}`, {
        url,
        cause: error
      });
    } finally {
      clearTimeout(timer);
    }
  }

  private getJson<T>(base: string, path: string): Promise<T> {
    return this.requestJson<T>(base, path);
  }

  // —— RPC / gateway ——

  health(): Promise<HealthResponse> {
    return this.getJson(this.rpcUrl, "/health");
  }

  status(): Promise<ChainStatus> {
    return this.getJson(this.rpcUrl, "/status");
  }

  chainTip(): Promise<unknown> {
    return this.getJson(this.rpcUrl, "/chain/tip");
  }

  blockByHeight(height: number): Promise<unknown> {
    if (!Number.isInteger(height) || height < 0) {
      return Promise.reject(new AlvenqisError("height must be a non-negative integer"));
    }
    return this.getJson(this.rpcUrl, `/blocks/${height}`);
  }

  transaction(hash: string): Promise<unknown> {
    const h = hash.trim().toLowerCase().replace(/^0x/, "");
    if (!/^[a-f0-9]{64}$/.test(h)) {
      return Promise.reject(new AlvenqisError("transaction hash must be 64 hex chars"));
    }
    return this.getJson(this.rpcUrl, `/transactions/${h}`);
  }

  addressBalance(address: string): Promise<AddressBalance> {
    const a = address.trim();
    if (!a.startsWith("alve1")) {
      return Promise.reject(new AlvenqisError("address must be a alve1… Mainnet Candidate address"));
    }
    return this.getJson(this.rpcUrl, `/addresses/${encodeURIComponent(a)}/balance`);
  }

  addressAccount(address: string): Promise<AddressAccount> {
    const a = address.trim();
    if (!a.startsWith("alve1")) {
      return Promise.reject(new AlvenqisError("address must be a alve1… Mainnet Candidate address"));
    }
    return this.getJson(this.rpcUrl, `/addresses/${encodeURIComponent(a)}/account`);
  }

  /**
   * Ledger-backed next sequential spend nonce for `address`.
   * Prefer this over inventing nonces client-side.
   */
  async nextNonce(address: string): Promise<number> {
    const account = await this.addressAccount(address);
    const n = Number(account.next_nonce);
    if (!Number.isFinite(n) || n < ALVENQIS_FIRST_ACCOUNT_NONCE) {
      return ALVENQIS_FIRST_ACCOUNT_NONCE;
    }
    return Math.trunc(n);
  }

  /** Protocol body/timing limits from `GET /network`. */
  network(): Promise<NetworkLimits> {
    return this.getJson(this.rpcUrl, "/network");
  }

  /**
   * Relay a pre-signed transaction to `POST /transactions`.
   * Does not sign; body must already include signature + sender_public_key.
   */
  submitTransaction(tx: SignedTransactionBody): Promise<SubmitTransactionResponse> {
    if (!tx || typeof tx !== "object") {
      return Promise.reject(new AlvenqisError("transaction body is required"));
    }
    if (!tx.from || !tx.to || tx.nonce === undefined) {
      return Promise.reject(new AlvenqisError("transaction requires from, to, and nonce"));
    }
    if (!tx.signature && !tx.sender_public_key) {
      return Promise.reject(
        new AlvenqisError("unsigned transactions cannot be submitted (missing signature fields)")
      );
    }
    return this.requestJson(this.rpcUrl, "/transactions", {
      method: "POST",
      body: tx
    });
  }

  indexerSummary(): Promise<unknown> {
    return this.getJson(this.rpcUrl, "/indexer/summary");
  }

  p2pStatus(): Promise<unknown> {
    return this.getJson(this.rpcUrl, "/p2p/status");
  }

  // —— Pool (public read APIs only) ——

  poolStatus(): Promise<PoolStatus> {
    return this.getJson(this.poolUrl, "/api/v1/pool/status");
  }

  poolHistory(): Promise<PoolHistory> {
    return this.getJson(this.poolUrl, "/api/v1/pool/history");
  }

  poolMiner(address: string): Promise<unknown> {
    const a = address.trim();
    if (!a.startsWith("alve1")) {
      return Promise.reject(new AlvenqisError("miner address must be alve1…"));
    }
    return this.getJson(this.poolUrl, `/api/v1/miners/${encodeURIComponent(a)}`);
  }

  poolPayouts(): Promise<unknown> {
    return this.getJson(this.poolUrl, "/api/v1/payouts");
  }

  /**
   * Convenience: pool status + chain tip → maturity progress for each recent/history block.
   */
  async poolBlocksWithMaturity(): Promise<
    Array<{
      height: number;
      hash: string;
      status: string;
      reward_atomic?: string | number;
      maturity: ReturnType<typeof poolBlockMaturity>;
    }>
  > {
    const [chain, pool] = await Promise.all([this.status(), this.poolStatus()]);
    const tip = chain.height ?? null;
    const required = pool.block_maturity_confirmations ?? 12;
    let blocks = pool.recent_blocks ?? [];
    try {
      const history = await this.poolHistory();
      if (history.blocks?.length) blocks = history.blocks;
    } catch {
      /* history optional on older pool builds */
    }
    return blocks.map((b) => ({
      height: b.height,
      hash: b.hash,
      status: String(b.status ?? "unknown"),
      reward_atomic: b.reward_atomic,
      maturity: poolBlockMaturity(b.height, tip, required, String(b.status ?? ""))
    }));
  }
}

export function createAlvenqisClient(options?: AlvenqisClientOptions): AlvenqisClient {
  return new AlvenqisClient(options);
}
