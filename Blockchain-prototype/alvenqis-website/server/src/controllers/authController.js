import { loginWithPassword, logoutSession, refreshSession } from '../services/authService.js'

const cookieOptions = {
  httpOnly: true,
  sameSite: 'lax',
  secure: process.env.NODE_ENV === 'production',
  path: '/',
  maxAge: 7 * 24 * 60 * 60 * 1000,
}

const clearCookieOptions = {
  httpOnly: true,
  sameSite: 'lax',
  secure: process.env.NODE_ENV === 'production',
  path: '/',
}

export async function login(req, res) {
  const { email, password } = req.validated.body
  const session = await loginWithPassword({ email, password })

  if (!session) {
    return res.status(401).json({ error: 'Invalid email or password' })
  }

  res.cookie('alvenqis_refresh_token', session.refreshToken, cookieOptions)
  return res.json({
    user: session.user,
    accessToken: session.accessToken,
  })
}

export async function refresh(req, res) {
  const refreshToken = req.cookies.alvenqis_refresh_token

  if (!refreshToken) {
    return res.status(401).json({ error: 'Missing refresh token' })
  }

  try {
    const session = await refreshSession(refreshToken)

    if (!session) {
      res.clearCookie('alvenqis_refresh_token', clearCookieOptions)
      return res.status(401).json({ error: 'Invalid refresh token' })
    }

    res.cookie('alvenqis_refresh_token', session.refreshToken, cookieOptions)
    return res.json({
      user: session.user,
      accessToken: session.accessToken,
    })
  } catch {
    res.clearCookie('alvenqis_refresh_token', clearCookieOptions)
    return res.status(401).json({ error: 'Invalid refresh token' })
  }
}

export async function logout(req, res) {
  await logoutSession(req.cookies.alvenqis_refresh_token)
  res.clearCookie('alvenqis_refresh_token', clearCookieOptions)
  return res.status(204).send()
}

export async function me(req, res) {
  return res.json({ user: req.user })
}
