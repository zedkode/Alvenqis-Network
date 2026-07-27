import { prisma } from '../prisma/client.js'
import { verifyAccessToken } from '../utils/token.js'

export async function requireAuth(req, res, next) {
  const header = req.headers.authorization
  const token = header?.startsWith('Bearer ') ? header.slice(7) : null

  if (!token) {
    return res.status(401).json({ error: 'Missing access token' })
  }

  try {
    const payload = verifyAccessToken(token)
    const user = await prisma.user.findUnique({
      where: { id: payload.sub },
      select: { id: true, email: true, role: true, isActive: true },
    })

    if (!user || !user.isActive) {
      return res.status(401).json({ error: 'Invalid access token user' })
    }

    req.user = user
    return next()
  } catch {
    return res.status(401).json({ error: 'Invalid or expired access token' })
  }
}

export function requireRole(...roles) {
  return (req, res, next) => {
    if (!req.user) {
      return res.status(401).json({ error: 'Authentication required' })
    }

    if (req.user.role === 'superadmin' || roles.includes(req.user.role)) {
      return next()
    }

    return res.status(403).json({ error: 'Insufficient permissions' })
  }
}
