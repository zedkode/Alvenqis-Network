import { Router } from 'express'
import { z } from 'zod'
import {
  createFaqItem,
  deleteFaqItem,
  listFaqItems,
  updateFaqItem,
} from '../controllers/faqController.js'
import { requireAuth, requireRole } from '../middleware/auth.js'
import { validate } from '../middleware/validate.js'

const router = Router()
const lang = z.string().min(2).max(8)
const jsonValue = z.any().refine((value) => value !== undefined, 'contentJson is required')

const listSchema = z.object({
  params: z.object({}),
  query: z.object({ lang: lang.optional() }),
  body: z.unknown(),
})

const createSchema = z.object({
  params: z.object({}),
  query: z.object({}),
  body: z.object({
    question: z.string().min(1).max(300),
    contentJson: jsonValue,
    order: z.coerce.number().int().min(0).default(0),
    lang: lang.default('en'),
  }),
})

const updateSchema = z.object({
  params: z.object({ id: z.string().min(1) }),
  query: z.object({}),
  body: z.object({
    question: z.string().min(1).max(300).optional(),
    contentJson: jsonValue.optional(),
    order: z.coerce.number().int().min(0).optional(),
    lang: lang.optional(),
  }).refine((body) => Object.keys(body).length > 0, 'At least one field must be provided'),
})

const idSchema = z.object({
  params: z.object({ id: z.string().min(1) }),
  query: z.object({}),
  body: z.unknown(),
})

router.get('/api/faq', validate(listSchema), listFaqItems)
router.get('/api/admin/faq', requireAuth, requireRole('content_editor'), validate(listSchema), listFaqItems)
router.post('/api/admin/faq', requireAuth, requireRole('content_editor'), validate(createSchema), createFaqItem)
router.put('/api/admin/faq/:id', requireAuth, requireRole('content_editor'), validate(updateSchema), updateFaqItem)
router.delete('/api/admin/faq/:id', requireAuth, requireRole('content_editor'), validate(idSchema), deleteFaqItem)

export default router
