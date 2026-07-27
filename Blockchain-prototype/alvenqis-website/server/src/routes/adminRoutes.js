import { Router } from 'express'
import { z } from 'zod'
import {
  createUser,
  getAdminDashboard,
  getNetworkParams,
  listAuditLogs,
  listUsers,
  updateNetworkParam,
  updateUser,
} from '../controllers/adminController.js'
import { requireAuth, requireRole } from '../middleware/auth.js'
import { validate } from '../middleware/validate.js'

const router = Router()
const roles = ['superadmin', 'content_editor', 'network_operator']

const emptySchema = z.object({
  params: z.object({}),
  query: z.object({}),
  body: z.unknown(),
})

const userCreateSchema = z.object({
  params: z.object({}),
  query: z.object({}),
  body: z.object({
    email: z.string().email(),
    password: z.string().min(8),
    role: z.enum(roles),
    isActive: z.boolean().default(true),
  }),
})

const userUpdateSchema = z.object({
  params: z.object({ id: z.string().min(1) }),
  query: z.object({}),
  body: z.object({
    email: z.string().email().optional(),
    password: z.string().min(8).optional(),
    role: z.enum(roles).optional(),
    isActive: z.boolean().optional(),
  }).refine((body) => Object.keys(body).length > 0, 'At least one field must be provided'),
})

const auditSchema = z.object({
  params: z.object({}),
  query: z.object({
    userId: z.string().min(1).optional(),
    action: z.string().min(1).optional(),
    from: z.string().datetime().optional(),
    to: z.string().datetime().optional(),
    limit: z.coerce.number().int().min(1).max(100).default(30),
    offset: z.coerce.number().int().min(0).default(0),
  }),
  body: z.unknown(),
})

const networkParamUpdateSchema = z.object({
  params: z.object({ key: z.string().min(1).max(80).regex(/^[a-z0-9_:-]+$/) }),
  query: z.object({}),
  body: z.object({
    value: z.union([z.string(), z.number(), z.boolean(), z.record(z.any()), z.array(z.any())]),
    confirmCritical: z.boolean().optional(),
  }),
})

router.use('/api/admin', requireAuth)

router.get('/api/admin/dashboard', validate(emptySchema), getAdminDashboard)

router.get('/api/admin/users', requireRole('superadmin'), validate(emptySchema), listUsers)
router.post('/api/admin/users', requireRole('superadmin'), validate(userCreateSchema), createUser)
router.put('/api/admin/users/:id', requireRole('superadmin'), validate(userUpdateSchema), updateUser)

router.get('/api/admin/audit-log', requireRole('superadmin'), validate(auditSchema), listAuditLogs)

router.get('/api/admin/network-params', requireRole('network_operator'), validate(emptySchema), getNetworkParams)
router.put('/api/admin/network-params/:key', requireRole('network_operator'), validate(networkParamUpdateSchema), updateNetworkParam)

export default router
