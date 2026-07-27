import { prisma } from '../prisma/client.js'
import { writeAuditLog } from '../services/auditService.js'

function contentBlockPayload(block) {
  return {
    id: block.id,
    pageSlug: block.pageSlug,
    sectionKey: block.sectionKey,
    contentJson: block.contentJson,
    lang: block.lang,
    updatedAt: block.updatedAt,
  }
}

export async function getPageContent(req, res, next) {
  try {
    const { page_slug: pageSlug } = req.validated.params
    const { lang } = req.validated.query

    const blocks = await prisma.contentBlock.findMany({
      where: {
        pageSlug: { in: ['global', pageSlug] },
        lang,
      },
      orderBy: [{ pageSlug: 'asc' }, { sectionKey: 'asc' }],
    })

    const sections = {}
    for (const block of blocks) {
      sections[block.sectionKey] = block.contentJson
    }

    return res.json({
      pageSlug,
      lang,
      sections,
      blocks: blocks.map(contentBlockPayload),
    })
  } catch (error) {
    return next(error)
  }
}

export async function listContentBlocks(req, res, next) {
  try {
    const { pageSlug, lang } = req.validated.query
    const blocks = await prisma.contentBlock.findMany({
      where: {
        ...(pageSlug ? { pageSlug } : {}),
        ...(lang ? { lang } : {}),
      },
      orderBy: [{ pageSlug: 'asc' }, { sectionKey: 'asc' }, { lang: 'asc' }],
    })

    return res.json({ items: blocks.map(contentBlockPayload) })
  } catch (error) {
    return next(error)
  }
}

export async function createContentBlock(req, res, next) {
  try {
    const block = await prisma.contentBlock.create({
      data: req.validated.body,
    })

    await writeAuditLog({
      userId: req.user.id,
      action: 'content_block.created',
      entity: 'content_blocks',
      entityId: block.id,
      diffJson: { after: contentBlockPayload(block) },
    })

    return res.status(201).json({ item: contentBlockPayload(block) })
  } catch (error) {
    if (error.code === 'P2002') {
      return res.status(409).json({ error: 'Content block already exists for this page, section and language' })
    }

    return next(error)
  }
}

export async function updateContentBlock(req, res, next) {
  try {
    const { id } = req.validated.params
    const before = await prisma.contentBlock.findUnique({ where: { id } })

    if (!before) {
      return res.status(404).json({ error: 'Content block not found' })
    }

    const block = await prisma.contentBlock.update({
      where: { id },
      data: req.validated.body,
    })

    await writeAuditLog({
      userId: req.user.id,
      action: 'content_block.updated',
      entity: 'content_blocks',
      entityId: block.id,
      diffJson: {
        before: contentBlockPayload(before),
        after: contentBlockPayload(block),
      },
    })

    return res.json({ item: contentBlockPayload(block) })
  } catch (error) {
    if (error.code === 'P2002') {
      return res.status(409).json({ error: 'Content block already exists for this page, section and language' })
    }

    return next(error)
  }
}

export async function deleteContentBlock(req, res, next) {
  try {
    const { id } = req.validated.params
    const before = await prisma.contentBlock.findUnique({ where: { id } })

    if (!before) {
      return res.status(404).json({ error: 'Content block not found' })
    }

    await prisma.contentBlock.delete({ where: { id } })
    await writeAuditLog({
      userId: req.user.id,
      action: 'content_block.deleted',
      entity: 'content_blocks',
      entityId: id,
      diffJson: { before: contentBlockPayload(before) },
    })

    return res.status(204).send()
  } catch (error) {
    return next(error)
  }
}
