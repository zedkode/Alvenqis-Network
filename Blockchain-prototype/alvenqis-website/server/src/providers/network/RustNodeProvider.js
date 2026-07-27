import { env } from '../../config/env.js'
import { INetworkProvider } from './INetworkProvider.js'

const MODE = 'mainnet_candidate'
const ATOMIC_UNITS = 100_000_000
const INITIAL_REWARD_ATOMIC = 1_902_587_519
const HALVING_INTERVAL = 1_576_800

function formatAtomic(value) {
  return (Number(value || 0) / ATOMIC_UNITS).toFixed(8)
}

function rewardForHeight(height) {
  const halvings = Math.floor(Math.max(height, 0) / HALVING_INTERVAL)
  if (halvings >= 64) return '0.00000000'
  return formatAtomic(Math.floor(INITIAL_REWARD_ATOMIC / (2 ** halvings)))
}

function blockPayload(block) {
  return {
    id: `${block.network_id}-${block.height}`,
    height: block.height,
    hash: block.hash,
    prevHash: block.previous_hash,
    timestamp: new Date(block.timestamp * 1000).toISOString(),
    reward: formatAtomic(block.transactions?.[0]?.amount_atomic),
    minerPlaceholder: block.transactions?.[0]?.to || null,
    txCount: block.transaction_count,
  }
}

export class RustNodeProvider extends INetworkProvider {
  constructor(baseUrl = env.ALVENQIS_RPC_URL) {
    super()
    this.baseUrl = baseUrl.replace(/\/+$/, '')
  }

  async request(path) {
    const response = await fetch(`${this.baseUrl}${path}`)
    if (!response.ok) {
      const error = new Error(`Alvenqis RPC request failed with ${response.status}`)
      error.status = response.status
      throw error
    }
    return response.json()
  }

  async getBlocks({ limit = 20, offset = 0 } = {}) {
    const safeLimit = Math.min(Math.max(Number(limit) || 20, 1), 100)
    const safeOffset = Math.max(Number(offset) || 0, 0)
    const status = await this.request('/status')
    if (status.height === null || status.height === undefined) {
      return { mode: MODE, limit: safeLimit, offset: safeOffset, total: 0, items: [] }
    }

    const start = Math.max(status.height - safeOffset, 0)
    const heights = Array.from({ length: Math.min(safeLimit, start + 1) }, (_, index) => start - index)
    const blocks = await Promise.all(heights.map((height) => this.request(`/blocks/${height}`)))
    return {
      mode: MODE,
      limit: safeLimit,
      offset: safeOffset,
      total: status.block_count,
      items: blocks.map(blockPayload),
    }
  }

  async getBlockByHeight(height) {
    try {
      return { mode: MODE, item: blockPayload(await this.request(`/blocks/${Number(height)}`)) }
    } catch (error) {
      if (error.status === 404) return null
      throw error
    }
  }

  async getStats() {
    const [network, status, supply] = await Promise.all([
      this.request('/network'),
      this.request('/status'),
      this.request('/supply'),
    ])
    const nextHeight = Math.max((status.height ?? -1) + 1, 0)
    return {
      mode: MODE,
      networkId: network.network_id,
      networkName: network.network_name,
      statusLabel: network.status_label,
      height: status.height ?? -1,
      currentSupply: formatAtomic(supply.emitted_supply_atomic),
      maxSupply: formatAtomic(supply.max_supply_atomic),
      currentReward: rewardForHeight(nextHeight),
      blockTimeSeconds: network.block_time_seconds,
      halvingInterval: HALVING_INTERVAL,
      halvingCountdown: HALVING_INTERVAL - (nextHeight % HALVING_INTERVAL),
      difficultyTarget: 'PoW',
      lastUpdatedAt: new Date().toISOString(),
    }
  }
}
