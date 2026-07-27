import bcrypt from 'bcryptjs'
import { prisma } from '../prisma/client.js'
import { hashToken, refreshExpiryDate, signAccessToken, signRefreshToken, verifyRefreshToken } from '../utils/token.js'
import { writeAuditLog } from './auditService.js'

function publicUser(user) {
  return {
    id: user.id,
    email: user.email,
    role: user.role,
    isActive: user.isActive,
    lastLogin: user.lastLogin,
  }
}

export async function loginWithPassword({ email, password }) {
  const user = await prisma.user.findUnique({ where: { email } })

  if (!user || !user.isActive) {
    return null
  }

  const valid = await bcrypt.compare(password, user.passwordHash)
  if (!valid) {
    return null
  }

  const updatedUser = await prisma.user.update({
    where: { id: user.id },
    data: { lastLogin: new Date() },
  })

  const accessToken = signAccessToken(updatedUser)
  const refreshToken = signRefreshToken(updatedUser)

  await prisma.refreshToken.create({
    data: {
      userId: updatedUser.id,
      tokenHash: hashToken(refreshToken),
      expiresAt: refreshExpiryDate(),
    },
  })

  await writeAuditLog({
    userId: updatedUser.id,
    action: 'auth.login',
    entity: 'users',
    entityId: updatedUser.id,
    diffJson: { email: updatedUser.email },
  })

  return {
    user: publicUser(updatedUser),
    accessToken,
    refreshToken,
  }
}

export async function refreshSession(refreshToken) {
  const payload = verifyRefreshToken(refreshToken)
  const tokenHash = hashToken(refreshToken)

  const storedToken = await prisma.refreshToken.findUnique({
    where: { tokenHash },
    include: { user: true },
  })

  if (!storedToken || storedToken.revokedAt || storedToken.expiresAt < new Date() || !storedToken.user.isActive) {
    return null
  }

  if (storedToken.userId !== payload.sub) {
    return null
  }

  await prisma.refreshToken.update({
    where: { id: storedToken.id },
    data: { revokedAt: new Date() },
  })

  const accessToken = signAccessToken(storedToken.user)
  const nextRefreshToken = signRefreshToken(storedToken.user)

  await prisma.refreshToken.create({
    data: {
      userId: storedToken.user.id,
      tokenHash: hashToken(nextRefreshToken),
      expiresAt: refreshExpiryDate(),
    },
  })

  return {
    user: publicUser(storedToken.user),
    accessToken,
    refreshToken: nextRefreshToken,
  }
}

export async function logoutSession(refreshToken) {
  if (!refreshToken) return

  await prisma.refreshToken.updateMany({
    where: { tokenHash: hashToken(refreshToken), revokedAt: null },
    data: { revokedAt: new Date() },
  })
}
