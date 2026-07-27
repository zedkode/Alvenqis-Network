import crypto from 'node:crypto'
import jwt from 'jsonwebtoken'
import { env } from '../config/env.js'

export function signAccessToken(user) {
  return jwt.sign(
    { sub: user.id, email: user.email, role: user.role },
    env.JWT_SECRET,
    { expiresIn: '15m' },
  )
}

export function signRefreshToken(user) {
  return jwt.sign(
    { sub: user.id, type: 'refresh' },
    env.JWT_REFRESH_SECRET,
    { expiresIn: '7d' },
  )
}

export function verifyAccessToken(token) {
  return jwt.verify(token, env.JWT_SECRET)
}

export function verifyRefreshToken(token) {
  return jwt.verify(token, env.JWT_REFRESH_SECRET)
}

export function hashToken(token) {
  return crypto.createHash('sha256').update(token).digest('hex')
}

export function refreshExpiryDate() {
  const date = new Date()
  date.setDate(date.getDate() + 7)
  return date
}
