export function notFoundHandler(req, res) {
  res.status(404).json({ error: 'Route not found' })
}

export function errorHandler(error, req, res, next) {
  if (res.headersSent) return next(error)

  const status = error.status || 500
  const payload = {
    error: error.message || 'Internal server error',
  }

  if (process.env.NODE_ENV !== 'production' && error.details) {
    payload.details = error.details
  }

  return res.status(status).json(payload)
}
