import bcrypt from 'bcryptjs'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { mockPrisma } = vi.hoisted(() => ({
  mockPrisma: {
    user: {
      findUnique: vi.fn(),
      update: vi.fn(),
    },
    refreshToken: {
      create: vi.fn(),
    },
    auditLog: {
      create: vi.fn(),
    },
  },
}))

vi.mock('../src/prisma/client.js', () => ({
  prisma: mockPrisma,
}))

const { loginWithPassword } = await import('../src/services/authService.js')

describe('authService.loginWithPassword', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns null for a missing user', async () => {
    mockPrisma.user.findUnique.mockResolvedValue(null)

    await expect(loginWithPassword({ email: 'missing@alvenqis.network', password: 'Password123!' })).resolves.toBeNull()
    expect(mockPrisma.refreshToken.create).not.toHaveBeenCalled()
  })

  it('returns null for an inactive user', async () => {
    mockPrisma.user.findUnique.mockResolvedValue({
      id: 'user-1',
      email: 'off@alvenqis.network',
      passwordHash: await bcrypt.hash('Password123!', 4),
      role: 'content_editor',
      isActive: false,
    })

    await expect(loginWithPassword({ email: 'off@alvenqis.network', password: 'Password123!' })).resolves.toBeNull()
    expect(mockPrisma.user.update).not.toHaveBeenCalled()
  })

  it('updates last login, creates refresh token and returns a public session', async () => {
    const user = {
      id: 'user-1',
      email: 'admin@alvenqis.network',
      passwordHash: await bcrypt.hash('Password123!', 4),
      role: 'superadmin',
      isActive: true,
      lastLogin: null,
    }
    const updatedUser = { ...user, lastLogin: new Date('2026-07-02T10:00:00.000Z') }
    mockPrisma.user.findUnique.mockResolvedValue(user)
    mockPrisma.user.update.mockResolvedValue(updatedUser)
    mockPrisma.refreshToken.create.mockResolvedValue({})
    mockPrisma.auditLog.create.mockResolvedValue({})

    const session = await loginWithPassword({ email: user.email, password: 'Password123!' })

    expect(session.user).toEqual({
      id: updatedUser.id,
      email: updatedUser.email,
      role: updatedUser.role,
      isActive: true,
      lastLogin: updatedUser.lastLogin,
    })
    expect(session.accessToken).toEqual(expect.any(String))
    expect(session.refreshToken).toEqual(expect.any(String))
    expect(mockPrisma.refreshToken.create).toHaveBeenCalledOnce()
    expect(mockPrisma.auditLog.create).toHaveBeenCalledWith(expect.objectContaining({
      data: expect.objectContaining({ action: 'auth.login' }),
    }))
  })
})
