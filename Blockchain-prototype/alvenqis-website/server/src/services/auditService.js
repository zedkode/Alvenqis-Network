import { prisma } from '../prisma/client.js'

export async function writeAuditLog({ userId, action, entity, entityId, diffJson }) {
  return prisma.auditLog.create({
    data: {
      userId,
      action,
      entity,
      entityId,
      diffJson,
    },
  })
}
