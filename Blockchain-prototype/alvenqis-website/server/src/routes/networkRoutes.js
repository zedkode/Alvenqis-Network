import { Router } from 'express'
import { z } from 'zod'
import { getBlock, getNetworkStats, listBlocks } from '../controllers/networkController.js'
import { validate } from '../middleware/validate.js'

const router = Router()

const listSchema = z.object({
  params: z.object({}),
  query: z.object({
    limit: z.coerce.number().int().min(1).max(100).default(20),
    offset: z.coerce.number().int().min(0).default(0),
  }),
  body: z.unknown(),
})

const heightSchema = z.object({
  params: z.object({
    height: z.coerce.number().int().min(0),
  }),
  query: z.object({}),
  body: z.unknown(),
})

router.get('/api/network/blocks', validate(listSchema), listBlocks)
router.get('/api/network/blocks/:height', validate(heightSchema), getBlock)
router.get('/api/network/stats', getNetworkStats)

export default router
