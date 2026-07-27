import { networkProvider } from '../providers/network/index.js'

export async function listBlocks(req, res, next) {
  try {
    const { limit, offset } = req.validated.query
    const payload = await networkProvider.getBlocks({ limit, offset })
    return res.json(payload)
  } catch (error) {
    return next(error)
  }
}

export async function getBlock(req, res, next) {
  try {
    const payload = await networkProvider.getBlockByHeight(req.validated.params.height)

    if (!payload) {
      return res.status(404).json({ mode: 'mainnet_candidate', error: 'Block not found' })
    }

    return res.json(payload)
  } catch (error) {
    return next(error)
  }
}

export async function getNetworkStats(req, res, next) {
  try {
    const payload = await networkProvider.getStats()
    return res.json(payload)
  } catch (error) {
    return next(error)
  }
}
