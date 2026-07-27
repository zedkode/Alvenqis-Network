import { useCallback, useEffect, useState } from 'react'

const RPC_BASE_URL = (import.meta.env.VITE_ALVENQIS_RPC_URL || 'https://rpcnode.dohotstudio.com').replace(/\/+$/, '')
const POOL_BASE_URL = (import.meta.env.VITE_ALVENQIS_POOL_URL || 'https://pool.dohotstudio.com').replace(/\/+$/, '')
const ATOMIC_UNITS = 100_000_000
const INITIAL_REWARD_ATOMIC = 1_902_587_519
const HALVING_INTERVAL = 1_576_800

const fallbackStats = {
  mode: 'mainnet_candidate',
  networkId: 'alvenqis-mainnet-candidate',
  networkName: 'Alvenqis Mainnet Candidate',
  statusLabel: 'Planned / Mainnet Candidate',
  height: -1,
  currentSupply: '0.00000000',
  maxSupply: '60000000.00000000',
  currentReward: '19.02587519',
  blockTimeSeconds: 60,
  halvingInterval: HALVING_INTERVAL,
  halvingCountdown: HALVING_INTERVAL,
  difficultyTarget: 'RPC offline',
  lastUpdatedAt: null,
}

function formatAtomic(value) {
  return (Number(value || 0) / ATOMIC_UNITS).toFixed(8)
}

function rewardAtHeight(height) {
  const halvings = Math.floor(Math.max(height, 0) / HALVING_INTERVAL)
  if (halvings >= 64) return '0.00000000'
  return formatAtomic(Math.floor(INITIAL_REWARD_ATOMIC / (2 ** halvings)))
}

async function fetchJson(path) {
  const response = await fetch(`${RPC_BASE_URL}${path}`)
  if (!response.ok) throw new Error(`Alvenqis RPC request failed with ${response.status}`)
  return response.json()
}

export function useNetworkStats({ pollMs = 15000 } = {}) {
  const [state, setState] = useState({ stats: fallbackStats, isLoading: true, error: null, source: 'fallback' })

  const load = useCallback(async () => {
    try {
      const [network, status, supply] = await Promise.all([
        fetchJson('/network'),
        fetchJson('/status'),
        fetchJson('/supply'),
      ])
      const height = status.height ?? -1
      const nextHeight = Math.max(height + 1, 0)
      setState({
        stats: {
          mode: 'mainnet_candidate',
          networkId: network.network_id,
          networkName: network.network_name,
          statusLabel: network.status_label,
          height,
          currentSupply: formatAtomic(supply.emitted_supply_atomic),
          maxSupply: formatAtomic(supply.max_supply_atomic),
          currentReward: rewardAtHeight(nextHeight),
          blockTimeSeconds: network.block_time_seconds,
          halvingInterval: HALVING_INTERVAL,
          halvingCountdown: HALVING_INTERVAL - (nextHeight % HALVING_INTERVAL),
          difficultyTarget: 'PoW',
          lastUpdatedAt: new Date().toISOString(),
        },
        isLoading: false,
        error: null,
        source: 'rpc',
      })
    } catch (error) {
      setState({ stats: fallbackStats, isLoading: false, error, source: 'fallback' })
    }
  }, [])

  useEffect(() => {
    load()
    const timer = setInterval(load, pollMs)
    return () => clearInterval(timer)
  }, [load, pollMs])

  return { ...state, refetch: load }
}

export function useNetworkBlocks({ limit = 8, offset = 0, pollMs = 15000 } = {}) {
  const [state, setState] = useState({ blocks: [], total: 0, mode: 'mainnet_candidate', isLoading: true, error: null, source: 'fallback' })

  const load = useCallback(async () => {
    try {
      const status = await fetchJson('/status')
      if (status.height === null || status.height === undefined) {
        setState({ blocks: [], total: 0, mode: 'mainnet_candidate', isLoading: false, error: null, source: 'rpc' })
        return
      }
      const start = Math.max(status.height - offset, 0)
      const heights = Array.from({ length: Math.min(limit, start + 1) }, (_, index) => start - index)
      const payloads = await Promise.all(heights.map((height) => fetchJson(`/blocks/${height}`)))
      const blocks = payloads.map((block) => ({
        id: `${block.network_id}-${block.height}`,
        height: block.height,
        hash: block.hash,
        prevHash: block.previous_hash,
        timestamp: block.timestamp,
        reward: formatAtomic(block.transactions?.[0]?.amount_atomic),
        txCount: block.transaction_count,
      }))
      setState({ blocks, total: status.block_count, mode: 'mainnet_candidate', isLoading: false, error: null, source: 'rpc' })
    } catch (error) {
      setState({ blocks: [], total: 0, mode: 'mainnet_candidate', isLoading: false, error, source: 'fallback' })
    }
  }, [limit, offset])

  useEffect(() => {
    load()
    const timer = setInterval(load, pollMs)
    return () => clearInterval(timer)
  }, [load, pollMs])

  return { ...state, refetch: load }
}

const fallbackPoolStats = {
  statusLabel: 'Pool API offline',
  connectedWorkers: 0,
  acceptedShares: 0,
  estimatedHashrateHs: 0,
  blocksFound: 0,
  poolFeeBasisPoints: 0,
}

export function usePoolStats({ pollMs = 15000 } = {}) {
  const [state, setState] = useState({ stats: fallbackPoolStats, isLoading: true, error: null, source: 'fallback' })

  const load = useCallback(async () => {
    try {
      const response = await fetch(`${POOL_BASE_URL}/api/v1/pool/status`)
      if (!response.ok) throw new Error(`Alvenqis pool status failed with ${response.status}`)
      const payload = await response.json()
      setState({
        stats: {
          statusLabel: payload.status_label || payload.upstream_status || 'online',
          connectedWorkers: payload.connected_workers ?? 0,
          acceptedShares: payload.accepted_shares ?? 0,
          estimatedHashrateHs: payload.estimated_hashrate_hs ?? 0,
          blocksFound: payload.blocks_found ?? 0,
          poolFeeBasisPoints: payload.pool_fee_basis_points ?? 0,
        },
        isLoading: false,
        error: null,
        source: 'pool',
      })
    } catch (error) {
      setState({ stats: fallbackPoolStats, isLoading: false, error, source: 'fallback' })
    }
  }, [])

  useEffect(() => {
    load()
    const timer = setInterval(load, pollMs)
    return () => clearInterval(timer)
  }, [load, pollMs])

  return { ...state, refetch: load }
}
