import request from 'supertest'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { authMocks } = vi.hoisted(() => ({
  authMocks: {
    loginWithPassword: vi.fn(),
    refreshSession: vi.fn(),
    logoutSession: vi.fn(),
  },
}))

vi.mock('../src/services/authService.js', () => authMocks)

vi.mock('../src/prisma/client.js', () => ({
  prisma: {
    user: {
      findUnique: vi.fn(),
    },
  },
}))

const app = (await import('../src/app.js')).default

describe('auth routes', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('logs in and sets an httpOnly refresh cookie', async () => {
    authMocks.loginWithPassword.mockResolvedValue({
      user: { id: 'user-1', email: 'admin@alvenqis.network', role: 'superadmin', isActive: true },
      accessToken: 'access-token',
      refreshToken: 'refresh-token',
    })

    const response = await request(app)
      .post('/auth/login')
      .send({ email: 'admin@alvenqis.network', password: 'Password123!' })
      .expect(200)

    expect(response.body.accessToken).toBe('access-token')
    expect(response.headers['set-cookie'].join(';')).toContain('alvenqis_refresh_token=refresh-token')
    expect(response.headers['set-cookie'].join(';')).toContain('HttpOnly')
  })

  it('rejects invalid credentials', async () => {
    authMocks.loginWithPassword.mockResolvedValue(null)

    await request(app)
      .post('/auth/login')
      .send({ email: 'admin@alvenqis.network', password: 'Password123!' })
      .expect(401)
  })

  it('rejects refresh without cookie', async () => {
    await request(app)
      .post('/auth/refresh')
      .expect(401)
  })

  it('logs out and clears the refresh cookie', async () => {
    await request(app)
      .post('/auth/logout')
      .set('Cookie', ['alvenqis_refresh_token=refresh-token'])
      .expect(204)

    expect(authMocks.logoutSession).toHaveBeenCalledWith('refresh-token')
  })

  it('serves the OpenAPI document', async () => {
    const response = await request(app)
      .get('/openapi.json')
      .expect(200)

    expect(response.body.openapi).toBe('3.0.3')
    expect(response.body.paths['/auth/login']).toBeDefined()
  })
})
