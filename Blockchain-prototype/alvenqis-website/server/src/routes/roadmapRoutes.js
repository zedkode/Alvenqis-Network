import { Router } from 'express'
import { z } from 'zod'
import {
  createRoadmapItem,
  deleteRoadmapItem,
  listRoadmapItems,
  updateRoadmapItem,
} from '../controllers/roadmapController.js'
import { requireAuth, requireRole } from '../middleware/auth.js'
import { validate } from '../middleware/validate.js'

const router = Router()
const statuses = ['active', 'next', 'planned', 'research', 'future', 'completed']

const emptySchema = z.object({
  params: z.object({}),
  query: z.object({}),
  body: z.unknown(),
})

const roadmapBody = {
  phase: z.string().min(1).max(80),
  title: z.string().min(1).max(160),
  description: z.string().min(1).max(2000),
  status: z.enum(statuses).default('planned'),
  order: z.coerce.number().int().min(0).default(0),
  targetDate: z.string().datetime().nullable().optional(),
}

const createSchema = z.object({
  params: z.object({}),
  query: z.object({}),
  body: z.object(roadmapBody),
})

const updateSchema = z.object({
  params: z.object({ id: z.string().min(1) }),
  query: z.object({}),
  body: z.object({
    phase: roadmapBody.phase.optional(),
    title: roadmapBody.title.optional(),
    description: roadmapBody.description.optional(),
    status: z.enum(statuses).optional(),
    order: z.coerce.number().int().min(0).optional(),
    targetDate: roadmapBody.targetDate,
  }).refine((body) => Object.keys(body).length > 0, 'At least one field must be provided'),
})

const idSchema = z.object({
  params: z.object({ id: z.string().min(1) }),
  query: z.object({}),
  body: z.unknown(),
})

router.get('/api/roadmap', validate(emptySchema), listRoadmapItems)
router.get('/api/admin/roadmap', requireAuth, requireRole('content_editor'), validate(emptySchema), listRoadmapItems)
router.post('/api/admin/roadmap', requireAuth, requireRole('content_editor'), validate(createSchema), createRoadmapItem)
router.put('/api/admin/roadmap/:id', requireAuth, requireRole('content_editor'), validate(updateSchema), updateRoadmapItem)
router.delete('/api/admin/roadmap/:id', requireAuth, requireRole('content_editor'), validate(idSchema), deleteRoadmapItem)

export default router
