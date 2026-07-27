import { prisma } from '../prisma/client.js'
import { writeAuditLog } from '../services/auditService.js'

const roadmapSelect = {
  id: true,
  phase: true,
  title: true,
  description: true,
  status: true,
  order: true,
  targetDate: true,
  createdAt: true,
  updatedAt: true,
}

function roadmapPayload(item) {
  return item
}

function roadmapData(body) {
  return {
    ...body,
    targetDate: body.targetDate === undefined
      ? undefined
      : body.targetDate
        ? new Date(body.targetDate)
        : null,
  }
}

export async function listRoadmapItems(req, res, next) {
  try {
    const items = await prisma.roadmapItem.findMany({
      orderBy: [{ order: 'asc' }, { createdAt: 'asc' }],
      select: roadmapSelect,
    })

    return res.json({ items: items.map(roadmapPayload) })
  } catch (error) {
    return next(error)
  }
}

export async function createRoadmapItem(req, res, next) {
  try {
    const item = await prisma.roadmapItem.create({
      data: roadmapData(req.validated.body),
      select: roadmapSelect,
    })

    await writeAuditLog({
      userId: req.user.id,
      action: 'roadmap_item.created',
      entity: 'roadmap_items',
      entityId: item.id,
      diffJson: { after: roadmapPayload(item) },
    })

    return res.status(201).json({ item: roadmapPayload(item) })
  } catch (error) {
    return next(error)
  }
}

export async function updateRoadmapItem(req, res, next) {
  try {
    const { id } = req.validated.params
    const before = await prisma.roadmapItem.findUnique({ where: { id }, select: roadmapSelect })

    if (!before) {
      return res.status(404).json({ error: 'Roadmap item not found' })
    }

    const item = await prisma.roadmapItem.update({
      where: { id },
      data: roadmapData(req.validated.body),
      select: roadmapSelect,
    })

    await writeAuditLog({
      userId: req.user.id,
      action: 'roadmap_item.updated',
      entity: 'roadmap_items',
      entityId: item.id,
      diffJson: { before: roadmapPayload(before), after: roadmapPayload(item) },
    })

    return res.json({ item: roadmapPayload(item) })
  } catch (error) {
    return next(error)
  }
}

export async function deleteRoadmapItem(req, res, next) {
  try {
    const { id } = req.validated.params
    const before = await prisma.roadmapItem.findUnique({ where: { id }, select: roadmapSelect })

    if (!before) {
      return res.status(404).json({ error: 'Roadmap item not found' })
    }

    await prisma.roadmapItem.delete({ where: { id } })
    await writeAuditLog({
      userId: req.user.id,
      action: 'roadmap_item.deleted',
      entity: 'roadmap_items',
      entityId: id,
      diffJson: { before: roadmapPayload(before) },
    })

    return res.status(204).send()
  } catch (error) {
    return next(error)
  }
}
