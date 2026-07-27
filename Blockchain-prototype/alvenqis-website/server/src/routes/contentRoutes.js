import { Router } from 'express'
import { z } from 'zod'
import {
  createContentBlock,
  deleteContentBlock,
  getPageContent,
  listContentBlocks,
  updateContentBlock,
} from '../controllers/contentController.js'
import { requireAuth, requireRole } from '../middleware/auth.js'
import { validate } from '../middleware/validate.js'

const router = Router()

const slug = z.string().min(1).max(80).regex(/^[a-z0-9-]+$/)
const lang = z.string().min(2).max(8).default('en')
const jsonValue = z.any().refine((value) => value !== undefined, 'contentJson is required')

const pageContentSchema = z.object({
  params: z.object({ page_slug: slug }),
  query: z.object({ lang }),
  body: z.unknown(),
})

const adminListSchema = z.object({
  params: z.object({}),
  query: z.object({
    pageSlug: slug.optional(),
    lang: z.string().min(2).max(8).optional(),
  }),
  body: z.unknown(),
})

const createSchema = z.object({
  params: z.object({}),
  query: z.object({}),
  body: z.object({
    pageSlug: slug,
    sectionKey: z.string().min(1).max(120).regex(/^[a-zA-Z0-9_.:-]+$/),
    contentJson: jsonValue,
    lang: z.string().min(2).max(8).default('en'),
  }),
})

const updateSchema = z.object({
  params: z.object({ id: z.string().min(1) }),
  query: z.object({}),
  body: z.object({
    pageSlug: slug.optional(),
    sectionKey: z.string().min(1).max(120).regex(/^[a-zA-Z0-9_.:-]+$/).optional(),
    contentJson: jsonValue.optional(),
    lang: z.string().min(2).max(8).optional(),
  }).refine((body) => Object.keys(body).length > 0, 'At least one field must be provided'),
})

const deleteSchema = z.object({
  params: z.object({ id: z.string().min(1) }),
  query: z.object({}),
  body: z.unknown(),
})

router.get('/api/content/:page_slug', validate(pageContentSchema), getPageContent)

router.get(
  '/api/admin/content',
  requireAuth,
  requireRole('content_editor'),
  validate(adminListSchema),
  listContentBlocks,
)

router.post(
  '/api/admin/content',
  requireAuth,
  requireRole('content_editor'),
  validate(createSchema),
  createContentBlock,
)

router.put(
  '/api/admin/content/:id',
  requireAuth,
  requireRole('content_editor'),
  validate(updateSchema),
  updateContentBlock,
)

router.delete(
  '/api/admin/content/:id',
  requireAuth,
  requireRole('content_editor'),
  validate(deleteSchema),
  deleteContentBlock,
)

export default router
