import bcrypt from 'bcryptjs'
import { prisma } from '../prisma/client.js'
import { networkProvider } from '../providers/network/index.js'
import { writeAuditLog } from '../services/auditService.js'

function userPayload(user) {
  return {
    id: user.id,
    email: user.email,
    role: user.role,
    isActive: user.isActive,
    createdAt: user.createdAt,
    lastLogin: user.lastLogin,
  }
}

export async function listUsers(req, res, next) {
  try {
    const users = await prisma.user.findMany({
      orderBy: { createdAt: 'desc' },
    })

    return res.json({ items: users.map(userPayload) })
  } catch (error) {
    return next(error)
  }
}

export async function createUser(req, res, next) {
  try {
    const { email, password, role, isActive = true } = req.validated.body
    const passwordHash = await bcrypt.hash(password, 12)
    const user = await prisma.user.create({
      data: { email, passwordHash, role, isActive },
    })

    await writeAuditLog({
      userId: req.user.id,
      action: 'user.created',
      entity: 'users',
      entityId: user.id,
      diffJson: { after: userPayload(user) },
    })

    return res.status(201).json({ item: userPayload(user) })
  } catch (error) {
    if (error.code === 'P2002') {
      return res.status(409).json({ error: 'User email already exists' })
    }

    return next(error)
  }
}

export async function updateUser(req, res, next) {
  try {
    const { id } = req.validated.params
    const before = await prisma.user.findUnique({ where: { id } })

    if (!before) {
      return res.status(404).json({ error: 'User not found' })
    }

    const data = { ...req.validated.body }
    if (data.password) {
      data.passwordHash = await bcrypt.hash(data.password, 12)
      delete data.password
    }

    const user = await prisma.user.update({ where: { id }, data })

    await writeAuditLog({
      userId: req.user.id,
      action: 'user.updated',
      entity: 'users',
      entityId: user.id,
      diffJson: { before: userPayload(before), after: userPayload(user) },
    })

    return res.json({ item: userPayload(user) })
  } catch (error) {
    if (error.code === 'P2002') {
      return res.status(409).json({ error: 'User email already exists' })
    }

    return next(error)
  }
}

export async function listAuditLogs(req, res, next) {
  try {
    const { userId, action, from, to, limit, offset } = req.validated.query
    const where = {
      ...(userId ? { userId } : {}),
      ...(action ? { action: { contains: action, mode: 'insensitive' } } : {}),
      ...((from || to) ? {
        createdAt: {
          ...(from ? { gte: new Date(from) } : {}),
          ...(to ? { lte: new Date(to) } : {}),
        },
      } : {}),
    }

    const [items, total] = await prisma.$transaction([
      prisma.auditLog.findMany({
        where,
        orderBy: { createdAt: 'desc' },
        take: limit,
        skip: offset,
        include: { user: { select: { email: true } } },
      }),
      prisma.auditLog.count({ where }),
    ])

    return res.json({
      total,
      limit,
      offset,
      items: items.map((item) => ({
        id: item.id,
        userId: item.userId,
        userEmail: item.user?.email || null,
        action: item.action,
        entity: item.entity,
        entityId: item.entityId,
        diffJson: item.diffJson,
        createdAt: item.createdAt,
      })),
    })
  } catch (error) {
    return next(error)
  }
}

export async function getNetworkParams(req, res, next) {
  try {
    const items = await prisma.networkParam.findMany({
      orderBy: { key: 'asc' },
      include: { updater: { select: { email: true } } },
    })

    return res.json({
      items: items.map((item) => ({
        key: item.key,
        value: item.value,
        updatedBy: item.updatedBy,
        updatedByEmail: item.updater?.email || null,
        updatedAt: item.updatedAt,
      })),
    })
  } catch (error) {
    return next(error)
  }
}

export async function updateNetworkParam(req, res, next) {
  try {
    const { key } = req.validated.params
    const { value, confirmCritical } = req.validated.body
    const criticalKeys = new Set(['max_supply', 'halving_interval', 'current_reward'])

    if (criticalKeys.has(key) && !confirmCritical) {
      return res.status(400).json({ error: 'Critical parameter update requires confirmCritical=true' })
    }

    const before = await prisma.networkParam.findUnique({ where: { key } })
    const item = await prisma.networkParam.upsert({
      where: { key },
      update: { value, updatedBy: req.user.id },
      create: { key, value, updatedBy: req.user.id },
    })

    await writeAuditLog({
      userId: req.user.id,
      action: 'network_param.updated',
      entity: 'network_params',
      entityId: key,
      diffJson: { before, after: item },
    })

    return res.json({ item })
  } catch (error) {
    return next(error)
  }
}

export async function getAdminDashboard(req, res, next) {
  try {
    const [
      activeUsers,
      latestLogin,
      latestAuditLogs,
      contentBlocks,
      roadmapItems,
      faqItems,
    ] = await prisma.$transaction([
      prisma.user.count({ where: { isActive: true } }),
      prisma.user.findFirst({ where: { lastLogin: { not: null } }, orderBy: { lastLogin: 'desc' } }),
      prisma.auditLog.findMany({
        orderBy: { createdAt: 'desc' },
        take: 8,
        include: { user: { select: { email: true } } },
      }),
      prisma.contentBlock.count(),
      prisma.roadmapItem.count(),
      prisma.faqItem.count(),
    ])

    const candidateStats = await networkProvider.getStats().catch(() => null)

    return res.json({
      kpis: {
        candidateHeight: candidateStats?.height ?? null,
        activeUsers,
        lastLogin: latestLogin ? {
          email: latestLogin.email,
          at: latestLogin.lastLogin,
        } : null,
        contentBlocks,
        roadmapItems,
        faqItems,
      },
      latestAuditLogs: latestAuditLogs.map((item) => ({
        id: item.id,
        action: item.action,
        entity: item.entity,
        entityId: item.entityId,
        userEmail: item.user?.email || null,
        createdAt: item.createdAt,
      })),
    })
  } catch (error) {
    return next(error)
  }
}
