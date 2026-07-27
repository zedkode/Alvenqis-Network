import { prisma } from '../prisma/client.js'
import { writeAuditLog } from '../services/auditService.js'

function faqPayload(item) {
  return {
    id: item.id,
    question: item.question,
    contentJson: item.contentJson,
    order: item.order,
    lang: item.lang,
    createdAt: item.createdAt,
    updatedAt: item.updatedAt,
  }
}

export async function listFaqItems(req, res, next) {
  try {
    const { lang } = req.validated.query
    const items = await prisma.faqItem.findMany({
      where: lang ? { lang } : {},
      orderBy: [{ order: 'asc' }, { createdAt: 'asc' }],
    })

    return res.json({ items: items.map(faqPayload) })
  } catch (error) {
    return next(error)
  }
}

export async function createFaqItem(req, res, next) {
  try {
    const item = await prisma.faqItem.create({
      data: req.validated.body,
    })

    await writeAuditLog({
      userId: req.user.id,
      action: 'faq_item.created',
      entity: 'faq_items',
      entityId: item.id,
      diffJson: { after: faqPayload(item) },
    })

    return res.status(201).json({ item: faqPayload(item) })
  } catch (error) {
    return next(error)
  }
}

export async function updateFaqItem(req, res, next) {
  try {
    const { id } = req.validated.params
    const before = await prisma.faqItem.findUnique({ where: { id } })

    if (!before) {
      return res.status(404).json({ error: 'FAQ item not found' })
    }

    const item = await prisma.faqItem.update({
      where: { id },
      data: req.validated.body,
    })

    await writeAuditLog({
      userId: req.user.id,
      action: 'faq_item.updated',
      entity: 'faq_items',
      entityId: item.id,
      diffJson: { before: faqPayload(before), after: faqPayload(item) },
    })

    return res.json({ item: faqPayload(item) })
  } catch (error) {
    return next(error)
  }
}

export async function deleteFaqItem(req, res, next) {
  try {
    const { id } = req.validated.params
    const before = await prisma.faqItem.findUnique({ where: { id } })

    if (!before) {
      return res.status(404).json({ error: 'FAQ item not found' })
    }

    await prisma.faqItem.delete({ where: { id } })
    await writeAuditLog({
      userId: req.user.id,
      action: 'faq_item.deleted',
      entity: 'faq_items',
      entityId: id,
      diffJson: { before: faqPayload(before) },
    })

    return res.status(204).send()
  } catch (error) {
    return next(error)
  }
}
