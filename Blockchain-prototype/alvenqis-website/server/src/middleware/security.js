import rateLimit from 'express-rate-limit'
import { env } from '../config/env.js'

export const globalRateLimiter = rateLimit({
  windowMs: env.RATE_LIMIT_WINDOW_MS,
  max: env.RATE_LIMIT_MAX,
  standardHeaders: true,
  legacyHeaders: false,
  message: { error: 'Too many requests. Try again later.' },
})

export function corsOrigin(origin, callback) {
  if (!origin && env.NODE_ENV !== 'production') {
    return callback(null, true)
  }

  if (origin && env.CORS_ORIGINS.includes(origin)) {
    return callback(null, true)
  }

  return callback(new Error('CORS origin not allowed'))
}
